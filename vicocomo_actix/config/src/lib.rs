//! # Actix web application configuration and generation

use ::proc_macro::TokenStream;

/// A macro that uses [`vicocomo::http_server::Config`
/// ](../vicocomo/http_server/struct.Config.html) to implement `actix_main()`.
/// ```text
/// pub fn actix_main() -> std::io::Result<()>
/// ```
/// ### Web sessions under Actix
///
/// Implementing the trait [`vicocomo::Session`
/// ](../vicocomo/http_server/trait.Session.html) does not work well for
/// `actix`. Instead, `config` accepts an adapter-specific [`app_config`
/// ](../vicocomo/http_server/struct.Config.html#level-1-app_config) attribute
/// `session`, which can have the values
/// - <b>`None`</b>: No session support.
/// - <b>`Cookie`</b>: [`actix_session::CookieSession`
///   ](../actix_session/struct.CookieSession.html) is used for session
///   support. This is the default if the attribute is not defined.
/// - <b>`Database`</b>: Still using `CookieSession` to store a session ID,
///   the actual session data is stored in the database. This requires that
///   the plugin `DbConn` is defined, and that the database has a table named
///   `"__vicocomo__sessions"` to store the sessions. The table should have
///   three columns, `id` storing a 64 bit integer primary key, `data` storing
///   the serialized session data as an unlimited ascii text, and `time`
///   storing the last access time as a 64 bit integer.
///
///   For `Database`, the value may be an array `[Database, `*max age*`]`,
///   where *max age* indicates a duration after which untouched session data
///   are pruned from the database. The default is `d100`, meaning one hundred
///   days. The identifier should begin with `d`, `h`, `m`, or `s` for days,
///   hours, minutes, or seconds, followed by a number of decimal digits.
///
/// Unless `session` is `None`, you *also* have to define the plug-in
/// `Session` in a non-standard way, see the example below.
///
/// # Usage
/// ```text
/// config! {
///     app_config {
///         session: Database,
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
///     plug_in(Session) {
///         def: (
///             // The type is ignored by vicocomo_actix, but still required!
///             (),
///             // vicocomo_actix requires the initialization expression to
///             // evaluate to CookieSession rather than vicocomo::Session.
///             ::actix_session::CookieSession::signed(&[0; 32])
///                 .secure(false),
///         ),
///     },
///     plug_in(TemplEng) {
///         def: (
///             ::vicocomo_handlebars::HbTemplEng<'_>,
///             ::vicocomo_handlebars::HbTemplEng::new(None),
///         ),
///     },
///     route(static) { home { path: "/" }},
///     app_config { role_enum: true },
///     authorize("/*") { get: Public, post: Authorized },
/// }
///
/// fn main() -> std::io::Result<()> {
///     actix_main()
/// }
/// ```
/// (see [`vicocomo::http_server::Config`
/// ](../vicocomo/http_server/struct.Config.html)).
///
#[proc_macro]
pub fn config(input: TokenStream) -> TokenStream {
    use ::case::CaseExt;
    use ::proc_macro2::Span;
    use ::quote::{format_ident, quote};
    use ::syn::{
        parse_macro_input, parse_quote, punctuated::Punctuated, token, Expr,
        FnArg, Ident, LitInt, LitStr, Path, Type,
    };
    use ::vicocomo::{http_server::ConfigAttrVal, Config, Handler};

    const ERROR_SESSION: &'static str =
        "expected None, Cookie, Database, or [Database, <max age>]";
    const SESSION_DB_DEFAULT: &'static str = "8640000"; // 100 days
    const SESSION_DB_NONE: &'static str = "0";

    let Config {
        plug_ins,
        app_config,
        routes,
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
                ConfigAttrVal::Array(a) => {
                    if a.len() == 2 && a[0].to_string() == "Database" {
                        let mut age = a[1].to_string();
                        if age.len() > 1 {
                            let factor: i64 = match age.remove(0) {
                                'd' | 'D' => 24 * 60 * 60,
                                'h' | 'H' => 60 * 60,
                                'm' | 'M' => 60,
                                's' | 'S' => 1,
                                _ => 0,
                            };
                            if factor > 0 {
                                result = age
                                    .parse::<i64>()
                                    .ok()
                                    .map(|i| (true, (factor * i).to_string()))
                            }
                        }
                    }
                }
                _ => (),
            }
            result.unwrap_or_else(|| panic!(ERROR_SESSION))
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
                        cav.get_array_strings()
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
    let mut name_vec: Vec<LitStr> = Vec::new();
    let mut hndl_pars_vec: Vec<Punctuated<FnArg, token::Comma>> = Vec::new();
    let mut authorize_expr_vec: Vec<Expr> = Vec::new();
    let mut controller_vec: Vec<Path> = Vec::new();
    let mut contr_meth_vec: Vec<Ident> = Vec::new();
    let mut path_pars_expr_vec: Vec<Expr> = Vec::new();
    let mut hndl_pars_min: Punctuated<FnArg, token::Comma> = parse_quote!(
        db_extr: ::actix_web::web::Data<#db_type>,
    );
    let mut session_middleware: Vec<Expr> = Vec::new();
    let mut session: Expr = parse_quote!(None);
    let mut session_db: Expr = parse_quote!(None);
    let mut session_prune = LitInt::new("0", Span::call_site());
    if has_session {
        hndl_pars_min.push(parse_quote!(sess: ::actix_session::Session));
        session_middleware.push(plug_ins.get("Session").unwrap().1.clone());
        session = parse_quote!(Some(sess));
        if db_session != SESSION_DB_NONE {
            session_db = parse_quote!(Some(db_if));
            session_prune = LitInt::new(&db_session, Span::call_site());
        }
    }
    hndl_pars_min.push(parse_quote!(
        teng: ::actix_web::web::Data<#templ_type>
    ));
    hndl_pars_min.push(parse_quote!(ax_req: ::actix_web::HttpRequest));
    hndl_pars_min.push(parse_quote!(body: String));
    for contr_path in routes.keys() {
        let contr_path_snake = contr_path
            .segments
            .iter()
            .map(|segm| segm.ident.to_string().to_snake())
            .collect::<Vec<_>>()
            .join("_");
        for handler in routes.get(contr_path).unwrap() {
            let Handler {
                http_method,
                route,
                route_par_count,
                authorized,
                contr_method,
            } = handler;
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
            name_vec.push(LitStr::new(&route, Span::call_site()));
            let mut hndl_pars = hndl_pars_min.clone();
            let mut path_pars_expr: Expr = parse_quote!(Vec::new());
            if *route_par_count > 0 {
                path_pars_expr = parse_quote!(path_par_vals
                    .iter()
                    .enumerate()
                    .map(|(ix, val)| (format!("p{}", ix + 1), val.clone()))
                    .collect());
                hndl_pars.push(parse_quote!(
                    path_par_vals: ::actix_web::web::Path<Vec<String>>
                ));
            }
            let authorize_expr: Expr = match authorized {
                Some(a) => {
                    let allow = a.allow.clone();
                    let allow_slice: Expr = parse_quote!(&[ #( #allow ),* ]);
                    let mut cond: Expr = parse_quote!(
                        <#role_enum as ::vicocomo::UserRole>::is_authorized(
                            #allow_slice,
                            db_if,
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
                            return ::actix_web::HttpResponse::Found()
                                .header(
                                    ::actix_web::http::header::LOCATION,
                                    #unauthorized_route.clone(),
                                )
                                .finish()
                                .into_body();
                        }
                    )
                }
                None => parse_quote!(()),
            };
            path_pars_expr_vec.push(path_pars_expr);
            hndl_pars_vec.push(hndl_pars);
            authorize_expr_vec.push(authorize_expr);
            controller_vec.push(contr_path.clone());
            contr_meth_vec.push(contr_method.clone());
        }
    }
    // TODO: use not_found from Config and fix the signature of the default
    let not_found_handler: Path = parse_quote!(crate::not_found);
    TokenStream::from(quote! {

        #[actix_rt::main]
        pub async fn actix_main() -> std::io::Result<()> {
            let database_ref = ::actix_web::web::Data::new(#db_init);
            let handlebars_ref = ::actix_web::web::Data::new(#templ_init);
            let port_str = ::std::env::var("PORT").unwrap_or_default();
            ::actix_web::HttpServer::new(move || {
                ::actix_web::App::new()
                    .app_data(database_ref.clone())
                    .app_data(handlebars_ref.clone())
                #(  .wrap(#session_middleware))*
                #(
                    .service(
                        ::actix_web::web::resource(#http_path_vec)
                        .name(#name_vec)
                        .route(
                            ::actix_web::web::#http_meth_vec()
                            .to(__vicocomo__handlers::#handler_fn_vec)
                        )
                    )
                )*
                    .service(::actix_files::Files::new("/", "./static"))
                    .default_service(::actix_web::web::route().to(
                        __vicocomo__handlers::not_found
                    ))
            })
            .bind(format!(
                "0.0.0.0:{}",
                std::str::FromStr::from_str(&port_str).unwrap_or(3000)
            ))?
            .run()
            .await
        }

        fn not_found(
            method: &::actix_web::http::Method,
            uri: &::actix_web::http::uri::Uri,
        ) -> ::actix_web::HttpResponse {
            ::actix_web::HttpResponse::NotFound()
                .content_type("text; charset=utf-8")
                .body(format!("404 Not Found: {} {}", method, uri))
        }

        #[allow(non_snake_case)]
        mod __vicocomo__handlers {
            use ::vicocomo::Controller;
            #(
                pub async fn #handler_fn_vec(
                    #hndl_pars_vec
                ) -> ::actix_web::HttpResponse {
                    let pg_arc = db_extr.into_inner();
                    let db_if = ::vicocomo::DatabaseIf::new(pg_arc.as_ref());
                    let te_arc = teng.into_inner();
                    let te_if = ::vicocomo::TemplEngIf::new(te_arc.as_ref());
                    let path_pars: Vec<(String, String)> = #path_pars_expr_vec;
                    let server = ::vicocomo_actix::AxServer::new(
                        &ax_req,
                        body.as_str(),
                        path_pars.as_slice(),
                        #session,
                        #session_db,
                        #session_prune,
                    );
                    let srv_if = ::vicocomo::HttpServerIf::new(&server);
                    #authorize_expr_vec;
                    #controller_vec::#contr_meth_vec(db_if, srv_if, te_if);
                    server.response()
                }
            )*

            pub async fn not_found(
                req: ::actix_web::HttpRequest
            ) -> ::actix_web::HttpResponse {
                #not_found_handler(req.method(), req.uri())
            }
        }
    })
}
