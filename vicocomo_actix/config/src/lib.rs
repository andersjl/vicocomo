//! # Actix web application configuration and generation

use proc_macro::TokenStream;

/// A macro that uses [`vicocomo::server::Config`
/// ](../vicocomo/http/config/struct.Config.html) to implement `actix_main()`.
/// ```text
/// pub fn actix_main() -> std::io::Result<()>
/// ```
/// ### Web sessions under Actix
///
/// `config` accepts two adapter-specific [`app_config`
/// ](../vicocomo/http/server/struct.HttpServerIf.html#level-1-app_config)
/// attributes:
///
/// #### `session`
///
/// Allowed values:
/// - <b>`None`</b>: No session support.
/// - <b>`Cookie`</b>: [`actix_session::storage::CookieSessionStore`
///   ](../actix_session/storage/struct.CookieSessionStore.html) is used for
///   session support. This is the default if the attribute is not defined.
/// - <b>`Database`</b>: Still using `CookieSessionStore` to store a session
///   ID, the actual session data is stored in the database. This requires
///   that the plugin `DbConn` is defined, and that the database has a table
///   named `"__vicocomo__sessions"` to store the sessions, see
///   [`HttpDbSession`](../vicocomo/http/config/struct.HttpDbSession.html).
///
///   For `Database`, the value may be an array `[Database, `*max age*`]`,
///   where *max age* indicates a duration after which untouched session data
///   are pruned from the database. The default is `d100`, meaning one hundred
///   days. The identifier should begin with `d`, `h`, `m`, or `s` for days,
///   hours, minutes, or seconds, followed by a number of decimal digits.
///
/// #### `session_middleware`
///
/// Unless `session` is `None`, you also have to define this attribute as a
/// tuple `(actix_session::SessionMiddleware, `*an expression evaluating to
/// `SessionMiddleware`*`)`. See the example below.
///
/// # Usage
/// ```text
/// config! {
///     app_config {
///         session: Database,
///         session_middleware: (
///             actix_session::SessionMiddleware,
///             actix_session::SessionMiddleware::builder(
///                 actix_session::storage::CookieSessionStore::default(),
///                 // a real application should use a secure key, of course!
///                 actix_web::cookie::Key::from(&[0; 64]),
///             )
///             // here you use configuration method calls as needed
///             .build(),
///         ),
///     },
///     plug_in(DbConn) {
///         def: (
///             vicocomo_postgres::PgConn,
///             {
///                 let (client, connection) = tokio_postgres::connect(
///                         "postgres://my_usr:my_pwd@localhost/my_db",
///                         tokio_postgres::NoTls,
///                     )
///                     .await
///                     .expect("could not get connection");
///                 tokio::spawn(async move {
///                     if let Err(e) = connection.await {
///                         eprintln!("could not init connection: {}", e);
///                     }
///                 });
///                 vicocomo_postgres::PgConn::new(client)
///             },
///         ),
///     },
///     plug_in(TemplEng) {
///         def: (
///             vicocomo_handlebars::HbTemplEng,
///             vicocomo_handlebars::HbTemplEng::new(None),
///         ),
///     },
///     app_config { role_enum: true },
///     authorize("/*") { get: Public, post: Authorized },
/// }
///
/// fn main() -> std::io::Result<()> {
///     actix_main()
/// }
/// ```
/// (see [`vicocomo::server::Config`
/// ](../vicocomo/http/config/struct.Config.html)).
///
#[proc_macro]
pub fn config(input: TokenStream) -> TokenStream {
    use case::CaseExt;
    use proc_macro2::Span;
    use quote::{format_ident, quote};
    use syn::{
        parse_macro_input, parse_quote, punctuated::Punctuated, token, Expr,
        FnArg, Ident, LitInt, LitStr, Path, Type,
    };
    use vicocomo::{Config, ConfigAttrVal, HttpHandler};

    const ERROR_SESSION: &'static str =
        "expected None, Cookie, Database, or [Database, <max age>]";
    const ERROR_SESSION_MW: &'static str =
        "expected a tuple (actix_session::SessionMiddleware, <expression>";
    const SESSION_DB_DEFAULT: &'static str = "8640000"; // 100 days
    const SESSION_DB_NONE: &'static str = "0";

    let Config {
        plug_ins,
        app_config,
        handlers,
        mut static_routes,
        not_found: _,
    } = parse_macro_input!(input as Config);
    let (db_type, db_init) = plug_ins.get("DbConn").unwrap();
    let (has_session, db_session) = app_config
        .get("session")
        .map(|val| {
            let mut result: Option<(bool, String)> = None;
            match val {
                ConfigAttrVal::Ident(sess_id) => {
                    match sess_id.to_string().as_str() {
                        "None" => {
                            result =
                                Some((false, SESSION_DB_NONE.to_string()))
                        }
                        "Cookie" => {
                            result = Some((true, SESSION_DB_NONE.to_string()))
                        }
                        "Database" => {
                            result =
                                Some((true, SESSION_DB_DEFAULT.to_string()))
                        }
                        _ => (),
                    }
                }
                ConfigAttrVal::Arr(a) => {
                    if a.len() == 2 && a[0].to_string() == "Database" {
                        if let Some(secs) =
                            ::vicocomo_derive_utils::parse_duration(
                                &a[1].to_string(),
                            )
                        {
                            result = Some((true, secs.to_string()));
                        }
                    }
                }
                _ => (),
            }
            result.unwrap_or_else(|| panic!("{}", ERROR_SESSION))
        })
        .unwrap_or_else(|| (true, SESSION_DB_NONE.to_string())); // Cookie
    let (templ_type, templ_init) = plug_ins.get("TemplEng").unwrap();
    let mut role_enum: Type = parse_quote!(());
    let mut disabled_expr: Expr = parse_quote!(None);
    let mut unauthorized_route: LitStr = parse_quote!("*** no route ***");
    match app_config.get("role_enum") {
        Some(val) => match val {
            ConfigAttrVal::Path(p) => {
                role_enum = parse_quote!(#p);
                unauthorized_route = app_config
                    .get("unauthorized_route")
                    .unwrap()
                    .get_litstr()
                    .unwrap();
                if app_config
                    .get("role_variants")
                    .map(|cav| {
                        cav.get_id_strings()
                            .unwrap()
                            .contains(&"Disabled".to_string())
                    })
                    .unwrap_or(false)
                {
                    disabled_expr = parse_quote!(Some(#role_enum::Disabled))
                }
            }
            _ => (),
        },
        _ => (),
    }
    let mut handler_fn_vec: Vec<Ident> = Vec::new();
    let mut http_meth_vec: Vec<Ident> = Vec::new();
    let mut http_path_vec: Vec<LitStr> = Vec::new();
    let mut hndl_pars_vec: Vec<Punctuated<FnArg, token::Comma>> = Vec::new();
    let mut authorize_expr_vec: Vec<Expr> = Vec::new();
    let mut controller_vec: Vec<Path> = Vec::new();
    let mut contr_meth_vec: Vec<Ident> = Vec::new();
    let mut route_pars_expr_vec: Vec<Expr> = Vec::new();
    let mut hndl_pars_min: Punctuated<FnArg, token::Comma> = parse_quote!(
        conf_extr:
            actix_web::web::Data<
                std::collections::HashMap<String, vicocomo::AppConfigVal>,
            >,
        stro_extr:
            actix_web::web::Data<std::collections::HashMap<String, String>>
    );
    hndl_pars_min.push(parse_quote!(
        db_extr: actix_web::web::Data<#db_type>
    ));
    let mut session_middleware: Vec<Expr> = Vec::new();
    let mut session: Expr = parse_quote!(None);
    let mut session_db: Expr = parse_quote!(None);
    let mut session_prune = LitInt::new("0", Span::call_site());
    if has_session {
        hndl_pars_min.push(parse_quote!(sess: actix_session::Session));
        match app_config
            .get("session_middleware")
            .ok_or_else(|| {
                syn::Error::new(
                    proc_macro2::Span::call_site(),
                    ERROR_SESSION_MW,
                )
            })
            .and_then(|val| val.get_type_expr())
        {
            Ok((_ignore, expr)) => session_middleware.push(expr),
            Err(e) => panic!("{}", e),
        }
        session = parse_quote!(Some(sess));
        if db_session != SESSION_DB_NONE {
            session_db = parse_quote!(Some(db_if.clone()));
            session_prune = LitInt::new(&db_session, Span::call_site());
        }
    }
    hndl_pars_min.push(parse_quote!(
        te_extr: actix_web::web::Data<#templ_type>
    ));
    hndl_pars_min.push(parse_quote!(ax_req: actix_web::HttpRequest));
    hndl_pars_min.push(parse_quote!(body: String));
    for HttpHandler {
        authorized,
        contr_method,
        contr_path,
        call_string: _,
        http_method,
        route,
        pattern: _,
        route_par_names,
    } in &handlers {
        let contr_path_snake = contr_path
            .segments
            .iter()
            .map(|segm| segm.ident.to_string().to_snake())
            .collect::<Vec<_>>()
            .join("_");
        handler_fn_vec.push(format_ident!(
            "{}__{}",
            contr_path_snake,
            contr_method,
        ));
        http_meth_vec.push(format_ident!("{}", http_method.to_string()));
        http_path_vec.push(LitStr::new(
            &route.replace("<", "{").replace(">", "}"),
            Span::call_site(),
        ));
        let mut hndl_pars = hndl_pars_min.clone();
        let mut route_pars_expr: Expr = parse_quote!(Vec::new());
        if !route_par_names.is_empty() {
            route_pars_expr = parse_quote!(
                [ #( #route_par_names ),* ]
                    .iter()
                    .map(|s| s.to_string())
                    .zip(route_par_vals.drain(..))
                    .collect()
            );
            hndl_pars.push(parse_quote!(
                mut route_par_vals: actix_web::web::Path<Vec<String>>
            ));
        }
        let authorize_expr: Expr = match authorized {
            Some(a) => {
                let allow = a.allow.clone();
                let allow_slice: Expr = parse_quote!(&[ #( #allow ),* ]);
                let mut cond: Expr = parse_quote!(
                    <#role_enum as vicocomo::UserRole>::is_authorized(
                        #allow_slice,
                        db_if.clone(),
                        srv_if,
                        #disabled_expr,
                        #role_enum::Superuser,
                    )
                );
                if a.filter {
                    cond = parse_quote!(
                        #cond && #contr_path::filter_access(db_if, srv_if)
                    );
                }
                parse_quote!(
                    if !(#cond) {
                        return actix_web::HttpResponse::Found()
                            .append_header((
                                actix_web::http::header::LOCATION,
                                #unauthorized_route.clone(),
                            ))
                            .finish()
                    }
                )
            }
            None => parse_quote!(()),
        };
        route_pars_expr_vec.push(route_pars_expr);
        hndl_pars_vec.push(hndl_pars);
        authorize_expr_vec.push(authorize_expr);
        controller_vec.push(contr_path.clone());
        contr_meth_vec.push(contr_method.clone());
    }
    // TODO: use not_found from Config and fix the signature of the default
    let not_found_handler: Path = parse_quote!(crate::not_found);
    let mut static_url = Vec::new();
    let mut static_dir = Vec::new();
    let mut static_expr: Vec<Expr> = Vec::new();
    let strip_mtime =
        app_config.get("strip_mtime").unwrap().get_bool().unwrap();
    for (url, dir) in static_routes.drain(..) {
        let url_lit = LitStr::new(&url, Span::call_site());
        static_url.push(url_lit.clone());
        let dir_lit = LitStr::new(&dir, Span::call_site());
        static_dir.push(dir_lit.clone());
        static_expr.push(if strip_mtime {
            let url_file = LitStr::new(&(url + "/{file}"), Span::call_site());
            parse_quote!(
                route(
                    #url_file,
                    actix_web::web::get().to(
                        __vicocomo__handlers::static_file_handler
                    )
                )
            )
        } else {
            parse_quote!(
                service(actix_files::Files::new(
                    #url_lit,
                    #dir_lit,
                ))
            )
        });
    }
    let mut conf_key = Vec::new();
    let mut conf_val: Vec<Expr> = Vec::new();
    for (key, val) in app_config {
        if let Some(expr) = val.to_app_config_val_expr() {
            conf_key.push(LitStr::new(&key, Span::call_site()));
            conf_val.push(expr);
        }
    }
    TokenStream::from(quote! {

        #[::actix_rt::main]
        pub async fn actix_main() -> std::io::Result<()> {
            #[cfg(debug_assertions)]
            {
                eprintln!("debugging");
                //env_logger::init();
            }
            use std::collections::HashMap;
            let mut conf: HashMap<String, vicocomo::AppConfigVal> =
                HashMap::new();
        #(  conf.insert(#conf_key.to_string(), #conf_val); )*
            let mut stro: HashMap<String, String> = HashMap::new();
        #(  stro.insert(#static_url.to_string(), #static_dir.to_string()); )*
            let conf_ref = actix_web::web::Data::new(conf);
            let stro_ref = actix_web::web::Data::new(stro);
            let database_ref = actix_web::web::Data::new(#db_init);
            let templ_ref = actix_web::web::Data::new(#templ_init);
            let port_str = std::env::var("PORT").unwrap_or_default();
            actix_web::HttpServer::new(move || {
                actix_web::App::new()
                    .app_data(conf_ref.clone())
                    .app_data(stro_ref.clone())
                    .app_data(database_ref.clone())
                    .app_data(templ_ref.clone())
                #(  .wrap(#session_middleware) )*
                #(
                    .route(
                        #http_path_vec,
                        actix_web::web::#http_meth_vec()
                        .to(__vicocomo__handlers::#handler_fn_vec)
                    )
                )*
                #(  .#static_expr )*
                    .default_service(actix_web::web::route().to(
                        __vicocomo__handlers::not_found
                    ))
            })
            .bind(format!(
                "0.0.0.0:{}",
                std::str::FromStr::from_str(&port_str).unwrap_or(3000)
            ))
            .map_err(|e| { eprintln!("{}", e); e })?
            .run()
            .await
        }

        fn not_found(
            method: &actix_web::http::Method,
            uri: &actix_web::http::uri::Uri,
        ) -> actix_web::HttpResponse {
            actix_web::HttpResponse::NotFound()
                .content_type("text; charset=utf-8")
                .body(format!("404 Not Found: {} {}", method, uri))
        }

        #[allow(non_snake_case)]
        #[doc(hidden)]
        mod __vicocomo__handlers {
            use ::vicocomo::Controller;
        #(
            pub async fn #handler_fn_vec(
                #hndl_pars_vec
            ) -> ::actix_web::HttpResponse {
                let conf = conf_extr.into_inner();
                let stro = stro_extr.into_inner();
                let db_arc = db_extr.into_inner();
                let db_if = vicocomo::DatabaseIf::new(db_arc);
                let te_arc = te_extr.into_inner();
                let te_if = vicocomo::TemplEngIf::new(te_arc);
                let route_pars: Vec<(String, String)> =
                    #route_pars_expr_vec;
                let server = vicocomo_actix::AxServer::new(
                    &conf,
                    &stro,
                    &ax_req,
                    body.as_str(),
                    route_pars.as_slice(),
                    #session,
                    #session_db,
                    #session_prune,
                );
                let srv_if = vicocomo::HttpServerIf::new(&server);
                #authorize_expr_vec;
                #controller_vec::#contr_meth_vec(
                    db_if.clone(),
                    srv_if,
                    te_if,
                );
                server.response()
            }
        )*
            pub async fn static_file_handler(
                conf_extr: actix_web::web::Data<
                    std::collections::HashMap<String, vicocomo::AppConfigVal>,
                >,
                stro_extr: actix_web::web::Data<
                    std::collections::HashMap<String, String>,
                >,
                ax_req: actix_web::HttpRequest,
            ) -> actix_web::HttpResponse {
                use ::vicocomo::HttpServer;
                let conf = conf_extr.into_inner();
                let stro = stro_extr.into_inner();
                let server = vicocomo_actix::AxServer::new(
                    &conf,
                    &stro,
                    &ax_req,
                    "",
                    &[],
                    None,
                    None,
                    0,
                );
                server.static_file_handler();
                server.response()
            }

            pub async fn not_found(
                req: actix_web::HttpRequest
            ) -> actix_web::HttpResponse {
                #not_found_handler(req.method(), req.uri())
            }
        }
    })
}
