//! # Actix web application configuration and generation

use proc_macro::TokenStream;

/// A macro that uses [`vicocomo::http_server::Config`
/// ](../vicocomo/http_server/struct.Config.html) to implement `actix_main()`.
/// ```text
/// pub fn actix_main() -> std::io::Result<()>
/// ```
/// # Usage
/// ```text
/// vicocomo_actix::config {
///     plug_in(DbConn) {
///         def: (
///             vicocomo_postgres::PgConn,
///             {
///                 let (client, connection) = tokio_postgres::connect(
///                         "postgres://my_usr:my_pwd@localhost/my_db,
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
///             // If the type is not vicocomo::NullSession, it is ignored by
///             // vicocomo_actix. But it is still required!
///             (),
///             // If the type is not vicocomo::NullSession, vicocomo_actix
///             // requires the initialization expression to evaluate to a
///             // CookieSession rather than a vicocomo::Session.
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
///     route(static) { home { path: "/" },
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
    use ::quote::{format_ident, quote};
    use ::syn::{
        export::Span, parse_macro_input, parse_quote, punctuated::Punctuated,
        token, Expr, FnArg, Ident, LitStr, Path,
    };
    use ::vicocomo::{Config, Handler};

    let Config {
        plug_ins,
        app_config,
        routes,
        not_found,
    } = parse_macro_input!(input as Config);
    let (db_type, db_init) = plug_ins.get("DbConn").unwrap();
    let (sess_type, sess_init) = plug_ins.get("Session").unwrap();
    // below is because we want no session middleware for NullSession
    let has_session =
        !(sess_type == &parse_quote!(::vicocomo::NullSession));
    let (templ_type, templ_init) = plug_ins.get("TemplEng").unwrap();
    let mut handler_fn_vec: Vec<Ident> = Vec::new();
    let mut http_meth_vec: Vec<Ident> = Vec::new();
    let mut http_path_vec: Vec<LitStr> = Vec::new();
    let mut name_vec: Vec<LitStr> = Vec::new();
    let mut hndl_pars_vec: Vec<Punctuated<FnArg, token::Comma>> = Vec::new();
    let mut controller_vec: Vec<Path> = Vec::new();
    let mut contr_meth_vec: Vec<Ident> = Vec::new();
    let mut path_pars_expr_vec: Vec<Expr> = Vec::new();
    let mut hndl_pars_min: Punctuated<FnArg, token::Comma> = parse_quote!(
        db: actix_web::web::Data<#db_type>,
    );
    if has_session {
        hndl_pars_min.push(parse_quote!(sess: actix_session::Session));
    }
    hndl_pars_min.push(parse_quote!(teng: actix_web::web::Data<#templ_type>));
    hndl_pars_min.push(parse_quote!(ax_req: actix_web::HttpRequest));
    hndl_pars_min.push(parse_quote!(body: String));
    let mut session_middleware: Vec<Expr> = Vec::new();
    let session: Expr;
    if has_session {
        session_middleware.push(sess_init.clone());
        session = parse_quote!(Some(sess));
    } else {
        session = parse_quote!(None);
    }
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
                http_path,
                path_par_count,
                contr_method,
            } = handler;
            handler_fn_vec.push(format_ident!(
                "{}__{}",
                contr_path_snake,
                contr_method,
            ));
            http_meth_vec.push(format_ident!("{}", http_method.to_string()));
            http_path_vec.push(LitStr::new(
                &http_path.replace("<", "{").replace(">", "}"),
                Span::call_site(),
            ));
            name_vec.push(LitStr::new(&http_path, Span::call_site()));
            let mut hndl_pars = hndl_pars_min.clone();
            let mut path_pars_expr: Expr = parse_quote!(&[]);
            if *path_par_count > 0 {
                path_pars_expr = parse_quote!(path_par_vals.as_slice());
                hndl_pars.push(parse_quote!(
                    path_par_vals: actix_web::web::Path<Vec<String>>
                ));
            }
            path_pars_expr_vec.push(path_pars_expr);
            hndl_pars_vec.push(hndl_pars);
            controller_vec.push(contr_path.clone());
            contr_meth_vec.push(contr_method.clone());
        }
    }
    // TODO: use not_found from Config and fix the signature of the default
    let not_found_handler: Path = parse_quote!(crate::not_found);
    TokenStream::from(quote! {

        #[actix_rt::main]
        pub async fn actix_main() -> std::io::Result<()> {
            let database_ref = actix_web::web::Data::new(#db_init);
            let handlebars_ref = actix_web::web::Data::new(#templ_init);
            let port_str = std::env::var("PORT").unwrap_or_default();
            actix_web::HttpServer::new(move || {
                actix_web::App::new()
                    .app_data(database_ref.clone())
                    .app_data(handlebars_ref.clone())
                #(  .wrap(#session_middleware))*
                #(
                    .service(
                        actix_web::web::resource(#http_path_vec)
                        .name(#name_vec)
                        .route(
                            actix_web::web::#http_meth_vec()
                            .to(__vicocomo__handlers::#handler_fn_vec)
                        )
                    )
                )*
                    .service(actix_files::Files::new("/", "./static"))
                    .default_service(actix_web::web::route().to(
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
            method: &actix_web::http::Method, uri: &actix_web::http::uri::Uri,
        ) -> actix_web::HttpResponse {
            actix_web::HttpResponse::NotFound()
                .content_type("text; charset=utf-8")
                .body(format!("404 Not Found: {} {}", method, uri))
        }

        #[allow(non_snake_case)]
        mod __vicocomo__handlers {
            use ::vicocomo::Controller;
            #(
                pub async fn #handler_fn_vec(
                    #hndl_pars_vec
                ) -> actix_web::HttpResponse {
                    let server = ::vicocomo_actix::AxServer::new(
                        &ax_req,
                        body.as_str(),
                        #path_pars_expr_vec,
                        #session,
                    );
                    #controller_vec::#contr_meth_vec(
                        ::vicocomo::DatabaseIf::new(db.into_inner().as_ref()),
                        ::vicocomo::HttpServerIf::new(&server),
                        ::vicocomo::TemplEngIf::new(teng.into_inner().as_ref()),
                    );
                    server.response()
                }
            )*

            pub async fn not_found(
                req: actix_web::HttpRequest
            ) -> actix_web::HttpResponse {
                #not_found_handler(req.method(), req.uri())
            }
        }
    })
}
