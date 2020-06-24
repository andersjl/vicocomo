#![allow(dead_code)]

use proc_macro::TokenStream;
use regex::Regex;
use syn::{
    braced,
    parse::{Parse, ParseStream},
    parse_macro_input, parse_quote,
    punctuated::Punctuated,
    token, Expr, FnArg, Ident, LitStr, Path, Result, Type,
};

pub fn configure_impl(input: TokenStream) -> TokenStream {
    use case::CaseExt;
    use quote::{format_ident, quote};
    use syn::export::Span;
    let ConfigInput { items } = parse_macro_input!(input as ConfigInput);
    let mut handler_fn_vec: Vec<Ident> = vec![];
    let mut http_meth_vec: Vec<Ident> = vec![];
    let mut actix_path_vec: Vec<LitStr> = vec![];
    let mut hndl_pars_vec: Vec<Punctuated<FnArg, token::Comma>> = vec![];
    let mut controller_vec: Vec<Path> = vec![];
    let mut contr_meth_vec: Vec<Ident> = vec![];
    let mut meth_args_vec: Vec<Punctuated<Expr, token::Comma>> = vec![];
    let hndl_pars_min: Punctuated<FnArg, token::Comma> = parse_quote!(
        pool: actix_web::web::Data<crate::Pool>,
        sess: actix_session::Session,
        hb: actix_web::web::Data<handlebars::Handlebars>,
        body: String,
    );
    let meth_args_min: Punctuated<Expr, token::Comma> =
        parse_quote!(&pool.get().unwrap(), sess, hb.into_inner(), body,);
    let not_found_handler: Path = parse_quote!(crate::not_found);
    for item in items {
        match item {
            ConfigItem::Route {
                controller,
                handlers,
            } => {
                let mut contr_path = controller.clone();
                let segments = &contr_path.segments;
                let contr_id = &segments.last().unwrap().ident.clone();
                if 1 == segments.len() {
                    contr_path.segments =
                        parse_quote!(crate::controllers::#contr_id);
                }
                let contr_path_snake = contr_path
                    .segments
                    .iter()
                    .map(|segm| segm.ident.to_string().to_snake())
                    .collect::<Vec<_>>()
                    .join("_");
                let contr_id_snake = contr_id.to_string().to_snake();
                for handler in handlers {
                    handler_fn_vec.push(format_ident!(
                        "{}__{}",
                        contr_path_snake,
                        handler.contr_meth
                    ));
                    http_meth_vec.push(handler.http_method);
                    let mut actix_path = handler.actix_path;
                    if actix_path.chars().nth(0) != Some('/') {
                        if !actix_path.is_empty() {
                            actix_path.insert(0, '/');
                        }
                        actix_path.insert_str(0, &contr_id_snake);
                        actix_path.insert(0, '/');
                    }
                    actix_path_vec
                        .push(LitStr::new(&actix_path, Span::call_site()));
                    let path_types = handler.path_types;
                    let mut hndl_pars = hndl_pars_min.clone();
                    if !path_types.is_empty() {
                        hndl_pars.push(parse_quote!(
                            params: actix_web::web::Path<(#path_types)>
                        ));
                        /*
                            if 1 == path_types.len() {
                                hndl_pars.push(parse_quote!(params: #path_types));
                            } else {
                                hndl_pars.push(parse_quote!(params: (#path_types)));
                            }
                        */
                    }
                    hndl_pars_vec.push(hndl_pars);
                    controller_vec.push(contr_path.clone());
                    contr_meth_vec.push(handler.contr_meth);
                    let mut meth_args = meth_args_min.clone();
                    meth_args.extend(handler.path_args);
                    meth_args_vec.push(meth_args);
                }
            }
            _ => panic!("NYI ConfigItem variant"),
        }
    }
    TokenStream::from(quote! {

        #[actix_rt::main]
        async fn main() -> std::io::Result<()> {
            use actix_files;
            use actix_web;
            use actix_rt;
            use actix_session;
            use dotenv;
            use handlebars;
            dotenv::dotenv().ok();
            let database_url = std::env::var("DATABASE_URL")
                .expect("DATABASE_URL must be set");
            let mut conn = vicocomo::PgConn::connect(&database_url).expect(
                "cannot connect"
            );
            let mut handlebars = handlebars::Handlebars::new();
            handlebars
                .register_templates_directory(".hbs", "templates")
                .unwrap();
            let handlebars_ref = actix_web::web::Data::new(handlebars);
            let port_str = std::env::var("PORT").unwrap_or_default();
            actix_web::HttpServer::new(move || {
                actix_web::App::new()
                    .data(pool.clone())
                    .app_data(handlebars_ref.clone())
                    .wrap( actix_session::CookieSession::signed(&[0; 32])
                        .secure(false)
                    )
                 #( .service(__vicocomo__handlers::#handler_fn_vec) )*
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
            #(
                #[actix_web::#http_meth_vec(#actix_path_vec)]
                pub fn #handler_fn_vec(
                    #hndl_pars_vec
                ) -> actix_web::HttpResponse {
                    #controller_vec::#contr_meth_vec(#meth_args_vec)
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

struct ConfigInput {
    items: Vec<ConfigItem>,
}

enum ConfigItem {
    NotFnd {
        controller: Path,
        handlers: Vec<RouteHandler>,
    },
    Route {
        controller: Path,
        handlers: Vec<RouteHandler>,
    },
}

struct RouteHandler {
    http_method: Ident, // only tested for get and post
    actix_path: String, // actix-web path, possibly with untyped parameters
    path_types: Punctuated<Type, token::Comma>, // path parameter types
    contr_meth: Ident,  // controller method name
    path_args: Punctuated<Expr, token::Comma>, //  method path args
}

impl Parse for ConfigInput {
    fn parse(input: ParseStream) -> Result<Self> {
        Ok(Self {
            items: input
                .parse_terminated::<ConfigItem, token::Comma>(
                    ConfigItem::parse,
                )?
                .into_iter()
                .collect(),
        })
    }
}

impl Parse for ConfigItem {
    fn parse(input: ParseStream) -> Result<Self> {
        match input.parse::<Ident>()?.to_string().as_str() {
            "notfnd" => {
                let (controller, handlers) = get_handlers(input)?;
                Ok(Self::NotFnd {
                    controller,
                    handlers,
                })
            }
            "route" => {
                let (controller, handlers) = get_handlers(input)?;
                Ok(Self::Route {
                    controller,
                    handlers,
                })
            }
            _ => Err(input.error("expected `route( ... ) { ... }`")),
        }
    }
}

fn get_handlers(input: ParseStream) -> Result<(Path, Vec<RouteHandler>)> {
    use syn::parenthesized;
    let content;
    parenthesized!(content in input);
    let controller: Path = content.parse()?;
    let content;
    braced!(content in input);
    let handlers: Vec<RouteHandler> = content
        .parse_terminated::<RouteHandler, token::Comma>(RouteHandler::parse)?
        .into_iter()
        .collect();
    Ok((controller, handlers))
}

impl Parse for RouteHandler {
    fn parse(input: ParseStream) -> Result<Self> {
        use quote::format_ident;
        let contr_meth: Ident = input.parse()?;
        let get_method = format_ident!("get");
        let post_method = format_ident!("post");
        let mut http_method: Option<Ident> = None;
        let mut path_str: Option<&str> = None;
        match contr_meth.to_string().as_str() {
            "new_form" => {
                http_method = Some(get_method);
                path_str = Some("new");
            }
            "copy_form" => {
                http_method = Some(get_method);
                path_str = Some("{i32}/copy");
            }
            "create" => {
                http_method = Some(post_method);
                path_str = Some("");
            }
            "ensure" => {
                http_method = Some(post_method);
                path_str = Some("ensure");
            }
            "index" => {
                http_method = Some(get_method);
                path_str = Some("");
            }
            "show" => {
                http_method = Some(get_method);
                path_str = Some("{i32}");
            }
            "edit_form" => {
                http_method = Some(get_method);
                path_str = Some("{i32}/edit");
            }
            "patch" => {
                http_method = Some(post_method);
                path_str = Some("{i32}");
            }
            "replace" => {
                http_method = Some(post_method);
                path_str = Some("{i32}/replace");
            }
            "delete" => {
                http_method = Some(post_method);
                path_str = Some("{i32}/delete");
            }
            _ => (),
        }
        let mut path_string;
        if input.peek(token::Brace) {
            let content;
            braced!(content in input);
            match parse_entry::<Ident>(&content, "http_method")? {
                Some(val) => http_method = Some(val),
                None => (),
            }
            match parse_entry::<LitStr>(&content, "path")? {
                Some(val) => {
                    path_string = val.value();
                    if 1 < path_string.len()
                        && path_string.chars().last() == Some('/')
                    {
                        path_string.remove(path_string.len() - 1);
                    }
                    path_str = Some(&path_string);
                }
                None => (),
            }
        }
        if http_method.is_none() {
            return Err(input.error("missing http_method"));
        }
        let http_method = http_method.unwrap();
        if path_str.is_none() {
            return Err(input.error("missing path"));
        }
        let path_parts =
            split_keep(&Regex::new(r"\{[^}]*\}").unwrap(), path_str.unwrap());
        let mut actix_path = String::new();
        let mut path_types: Punctuated<Type, token::Comma> =
            Punctuated::new();
        let mut param_nr = 0;
        for part in path_parts {
            if Some('{') == part.chars().nth(0) {
                let tokens: TokenStream = part.parse().unwrap();
                let PathParam { param_type } =
                    syn::parse_macro_input::parse::<PathParam>(tokens)?;
                path_types.push(param_type);
                actix_path.push('{');
                actix_path.push_str(&format!("p{}", param_nr));
                actix_path.push('}');
                param_nr += 1;
            } else {
                actix_path.push_str(part);
            }
        }
        let PathArgs { path_args } = {
            let token_str = match param_nr {
                0 => String::new(),
                1 => "params.into_inner()".to_string(),
                _ => (0..param_nr).fold(String::new(), |s, i| {
                    format!("{}params.{},", s, i)
                }),
            };
            syn::parse_macro_input::parse(token_str.parse().unwrap())?
        };
        Ok(Self {
            http_method,
            actix_path,
            path_types,
            contr_meth,
            path_args,
        })
    }
}

struct PathParam {
    param_type: Type,
}

impl Parse for PathParam {
    fn parse(input: ParseStream) -> Result<Self> {
        let content;
        braced!(content in input);
        Ok(Self {
            param_type: content.parse().unwrap(),
        })
    }
}

pub fn split_keep<'a>(re: &Regex, text: &'a str) -> Vec<&'a str> {
    let (last, mut parts) =
        re.find_iter(text).fold((0, vec![]), |(ix, mut acc), mat| {
            acc.push(text.get(ix..mat.start()).unwrap());
            acc.push(mat.as_str());
            (mat.end(), acc)
        });
    parts.push(text.get(last..text.len()).unwrap());
    parts
}

struct PathArgs {
    path_args: Punctuated<Expr, token::Comma>,
}

impl Parse for PathArgs {
    fn parse(input: ParseStream) -> Result<Self> {
        Ok(Self {
            path_args: Punctuated::parse_terminated(input)?,
        })
    }
}

fn parse_entry<T>(input: ParseStream, nam: &str) -> Result<Option<T>>
where
    T: Parse,
{
    if !input.is_empty() {
        let nam_id: Ident = input.fork().parse()?;
        if &nam_id.to_string() == nam {
            input.parse::<Ident>()?;
            input.parse::<token::Colon>()?;
            let value: T = input.parse()?;
            if !input.is_empty() {
                input.parse::<token::Comma>()?;
            }
            return Ok(Some(value));
        }
    }
    Ok(None)
}
