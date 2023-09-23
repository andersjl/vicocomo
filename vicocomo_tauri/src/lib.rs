//! # Tauri application configuration and generation

use proc_macro::TokenStream;

/// A macro that generates the contents of a [Tauri (version 1)
/// ](https://tauri.app/v1/guides) application's `main.rs`.
///
/// It uses [`vicocomo::Config`
/// ](../vicocomo/http/server/struct.HttpServerIf.html#config-macro-input-syntax)
/// to produce
/// - a `main()` function,
/// - a Tauri command `request()` (see below), and
/// - glue code to make the `request()` call the controllers defined in the
///   `vicocomo::Config` [documentation.
///   ](../vicocomo/http/server/struct.HttpServerIf.html#controller-path-and-handling-method)
///
/// ### How to use `request()` in Javascript
///
/// The Rust declaration is
/// ```text
/// #[tauri::command]
/// fn request(
///     app: tauri::AppHandle,
///     storage: tauri::State<_>,
///     method: &str,
///     url: &str,
///     body: &str,
/// ) -> (u32, String) {
///     // glue code calling your controllers
/// }
/// ```
/// The parameters `app` and `storage` are supplied by Tauri and should not be
/// sent from Javascript. The command is invoked in Javascript as
/// ```text
/// let response = invoke(
///   'request',
///   {
///     // an HTTP method
///     method: 'get',
///     // If the host is not a loopback the link is opened in a new window
///     url: 'http://localhost/local/path?p1=foo&p2=bar',
///     // parameters in the body should be URL encoded
///     body: 'p3=baz&p4=qux',
///   }
/// );
/// let status = response[0];   // an integer
/// let content = response[1];  // a string
/// ```
/// The idea is to mimic an HTTP request.
///
/// The response is a Javascript array with two elements, the HTTP status code
/// and the response body. No headers!
///
/// If the `url` is not [`config`
/// ](../vicocomo/http/server/struct.HttpServerIf.html#level-1-route-and-not-found)ured
/// to call a controller, a 404 HTTP error message will be returned.
///
/// The body is a string, so if you wish to respond with a JSON object using
/// [`resp_body()`
/// ](../vicocomo/http/server/struct.HttpServerIf.html#method.resp_body),
/// you should use [`serde_json::to_string()`
/// ](https://docs.rs/serde_json/latest/serde_json/fn.to_string.html) to
/// serialize it, and your Javascript should [`JSON.parse()`
/// ](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/JSON/parse)
/// the response body.
///
/// `request()` concats the `body` parameters to those in the `url`, so it
/// does not matter where you put the parameters.
///
/// ### External URLs
///
/// If the host part of the `url` is present and is not a loopback address or
/// `localhost`, `request()` neither calls a controller nor returns an error.
/// Instead it tries to open a web browser and point it to the URL. The
/// returned Javascript array is `[0, ""]`.
///
/// ### Simple glue Javascript
///
/// The code below, if loaded by hour HTML pages, will turn a server-based
/// HTML application to a Tauri desktop as long as you do not use Javascript
/// to do requests.
///
/// ```text
/// const { invoke } = window.__TAURI__.tauri
///
/// const invoke_request = (evnt, method, url, body) => {
///   if (evnt) {
///     evnt.preventDefault();
///   }
///   invoke('request', { method: method, url: url, body: body })
///     .then((response) => {
///       if (response[0]) {
///           document.querySelector('html').innerHTML = response[1];
///           capture_links();
///           capture_forms();
///       }
///     })
/// }
///
/// const capture_links = () => {
///   let links = document.querySelectorAll('a');
///   links.forEach((link) => {
///     link.onclick = function(e) {
///       invoke_request(e, 'get', e.target.href, '');
///     }
///   });
/// }
///
/// const capture_forms = () => {
///   let forms = document.querySelectorAll('form');
///   forms.forEach((form) => {
///     form.onsubmit = function(e) {
///       let body = '';
///       for (let ix = 0; ix < e.target.elements.length; ix++) {
///         let elem = e.target.elements[ix];
///         if (body.length > 0) {
///           body += '&';
///         }
///         body += elem.name + '=' + elem.value;
///       }
///       invoke_request(
///         e,
///         e.target.method,
///         e.target.action,
///         body,
///       );
///     }
///   });
/// }
/// ```
/// ### Adapter specific attributes
///
/// See[`vicocomo::Config`
/// ](../vicocomo/http/server/struct.HttpServerIf.html#level-1-app_config)
/// documentation.
///
/// #### `db_file`
///
/// The value should be a string that is the name of the Sqlite database file.
/// Default `"tauri.sqlite"`.
///
/// #### `template_dir`
///
/// The value should be an array of two strings, the name of a directory
/// containing the templates and the template file extension. This is needed
/// because the template engine is initialized before the Tauri application,
/// and the files cannot be loaded before application initialization.
///
/// This requires the template engine implementation to override
/// [`TemplEng::register_templ_dir()`
/// ](../vicocomo/http/config/trait.TemplEng.html#method.register_templ_dir).
///
/// #### persistent
///
/// The value should be a boolean that determines whether the data stored by
/// [`HttpServerIf::session_set()`
/// ](../vicocomo/http/server/struct.HttpServerIf.html#method.session_set) is
/// [stored in the database
/// ](../vicocomo/http/config/struct.HttpDbSession.html). Default `false`.
///
#[proc_macro]
pub fn config(input: TokenStream) -> TokenStream {
    use proc_macro2::Span;
    use quote::quote;
    use syn::{parse_macro_input, parse_quote, LitStr, Path};
    use vicocomo::{Config, ConfigAttrVal, HttpServerImpl};

    const DB_FILE_DEFAULT: &'static str = "tauri.sqlite";

    let config = parse_macro_input!(input as Config);
    let db_file = config
        .app_config
        .get("db_file")
        .and_then(|val| {
            if let ConfigAttrVal::Str(db_file) = val {
                Some(db_file.clone())
            } else {
                None
            }
        })
        .unwrap_or_else(|| LitStr::new(DB_FILE_DEFAULT, Span::call_site()));
    let persistent = config
        .app_config
        .get("persistent")
        .map(|val| val.get_bool().expect("persistent value should be bool"))
        .unwrap_or(false);
    let build_server = HttpServerImpl::build_expr(
        &config,
        persistent.then(|| parse_quote!(db)),
    );
    let (_, teng_init) = config.plug_ins.get("TemplEng").unwrap();
    let (templ_dir, templ_ext) = {
        let mut result: (LitStr, LitStr) = (
            LitStr::new("", Span::call_site()),
            LitStr::new("", Span::call_site()),
        );
        if let Some(val) = config.app_config.get("template_dir") {
            if let ConfigAttrVal::Arr(arr) = val {
                if arr.len() == 2 {
                    if let ConfigAttrVal::Str(path) = &arr[0] {
                        if let ConfigAttrVal::Str(ext) = &arr[1] {
                            result = (path.clone(), ext.clone());
                        }
                    }
                }
            }
        }
        result
    };
    let mut handler_string = Vec::new();
    let mut handler_path: Vec<Path> = Vec::new();
    for handler in &config.handlers {
        handler_string
            .push(LitStr::new(&handler.call_string, Span::call_site()));
        let path = &handler.contr_path;
        let meth = &handler.contr_method;
        handler_path.push(parse_quote!(#path::#meth));
    }

    TokenStream::from(quote! {

        struct Storage {
            db: ::vicocomo::DatabaseIf,
            server: ::std::sync::Mutex<::vicocomo::HttpServerImpl>,
            teng: ::vicocomo::TemplEngIf,
            teng_ok: ::std::sync::Mutex<bool>,
        }

        fn main() {
            let db_conn = ::vicocomo_sqlite::SqliteConn::new(#db_file)
                .expect("cannot open Sqlite file");
            let db =
                ::vicocomo::DatabaseIf::new(::std::sync::Arc::new(db_conn));
            let mut server = #build_server;
            ::tauri::Builder::default()
                .manage({
                    let teng = #teng_init;
                    let teng_ok = ::vicocomo::TemplEng::initialized(&teng);
                    Storage {
                        db: db.clone(),
                        server: ::std::sync::Mutex::new(server),
                        teng: ::vicocomo::TemplEngIf::new(
                            ::std::sync::Arc::new(teng)
                        ),
                        teng_ok: ::std::sync::Mutex::new(teng_ok),
                    }
                })
                .invoke_handler(::tauri::generate_handler![log, request])
                .run(::tauri::generate_context!())
                .expect("error while running tauri application");
        }

        #[::tauri::command]
        fn log(msg: &str) -> () {
          eprintln!("{}", msg);
        }

        #[::tauri::command]
        fn request(
            app: ::tauri::AppHandle,
            storage: ::tauri::State<Storage>,
            mut method: &str,
            url: &str,
            mut body: &str,
        ) -> (u32, String) {
            use std::ops::Deref;

            let mut server = storage.server.lock().unwrap();
            let mut response: ::vicocomo::HttpResponse;
            let mut url_str = String::from(url);
            let mut url: ::url::Url;
            loop {
                let url = {
                    let mut result = ::url::Url::parse(&url_str);
                    if let Err(e) = result {
                        if e == ::url::ParseError::RelativeUrlWithoutBase {
                            result = ::url::Url::parse(
                                &("http://localhost".to_string() + &url_str),
                            );
                        }
                    }
                    if let Err(e) = result {
                        return (
                            ::vicocomo::HttpStatus::BadRequest as u32,
                            e.to_string(),
                        );
                    }
                    result.unwrap()
                };
                if ::vicocomo::HttpServerImpl::is_loopback(&url) {
                    use ::vicocomo::Controller;
                    let db = storage.db.clone();
                    let teng = storage.teng.clone();
                    let teng_ok = *storage.teng_ok.lock().unwrap();
                    if !teng_ok {
                        let path = app.path_resolver()
                            .resolve_resource(#templ_dir)
                            .expect("failed to resolve resource")
                            .to_str()
                            .expect("template directory is not valid UTF8")
                            .to_string();
                        teng.register_templ_dir(&path, #templ_ext)
                            .expect("failed to register templates directory");
                        *storage.teng_ok.lock().unwrap() = true;
                    }
                    match server.receive(method, &url, body) {
                        Ok(handler_string) => {
                            match handler_string.as_str() {
                            #(
                                #handler_string => #handler_path(
                                    db.clone(),
                                    ::vicocomo::HttpServerIf::new(server.deref()),
                                    teng.clone(),
                                ),
                            )*
                                _ => panic!("This cannot happen"),
                            }
                        }
                        Err(e) => server.not_found(method, &url),
                    }
                    response = server.response();
                    if response.status ==
                        ::vicocomo::HttpResponseStatus::Redirect
                    {
                        method = "get";
                        url_str = response.text.clone();
                        body = "";
                    } else {
                        break (response.http_status() as u32, response.text);
                    };
                } else {
                    use ::rand::Rng;
                    let _ = ::tauri::WindowBuilder::new(
                        &app,
                        format!("{:0>16x}", rand::thread_rng().gen::<u64>()),
                        ::tauri::WindowUrl::External(url.clone()),
                    )
                        .build()
                        .map(|win| { let _ = win.set_title(&url.path()); });
                    break (0, String::new());
                }
            }
        }
    })
}
