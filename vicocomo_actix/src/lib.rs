//! (Ab)use `actix-web` as the web server for a vicocomo application.
//!
//! Implements [`vicocomo::HttpServer`
//! ](../vicocomo/http/config/trait.HttpServer.html) for [`actix-web`
//! ](https://crates.io/crates/actix-web).
//!
//! An application developer should not have to use this crate, the macro
//! [`config`](macro.config.html) generates code that does.

mod session;

use session::Session;
use std::cell::RefCell;
use std::collections::HashMap;
use std::path::Path;
use vicocomo::{
    map_error, t, AppConfigVal, DatabaseIf, Error, HttpMethod, HttpParamVals,
    HttpReqBody, HttpRequest, HttpRespBody, HttpResponse, HttpServer,
};
pub use vicocomo_actix_config::config;

pub struct AxServer<'conf, 'stro> {
    app_config: &'conf HashMap<String, AppConfigVal>,
    uploaded: RefCell<Vec<tempfile::TempPath>>,
    session: Option<RefCell<Session>>,
    static_routes: &'stro HashMap<String, String>,
}

impl<'conf, 'stro> AxServer<'conf, 'stro> {
    pub fn new(
        app_config: &'conf HashMap<String, AppConfigVal>,
        static_routes: &'stro HashMap<String, String>,
        uploaded: Vec<tempfile::TempPath>,
        session: Option<actix_session::Session>,
        db: Option<DatabaseIf>,
        prune: i64,
    ) -> Self {
        Self {
            app_config,
            uploaded: RefCell::new(uploaded),
            session: session.and_then(|actix_session| {
                Session::new(
                    actix_session,
                    db,
                    prune,
                    app_config
                        .get("create_session_table")
                        .and_then(|val| val.str())
                        .as_deref(),
                )
                .map(|s| RefCell::new(s))
            }),
            static_routes,
        }
    }
}

impl HttpServer for AxServer<'_, '_> {
    fn app_config(&self, id: &str) -> Option<AppConfigVal> {
        self.app_config.get(id).map(|v| v.clone())
    }

    fn handle_upload(&self, files: &[Option<&Path>]) -> Result<(), Error> {
        for (tmp, path) in
            self.uploaded.borrow_mut().drain(..).zip(files.iter())
        {
            if let Some(p) = path {
                map_error!(Other, tmp.persist(p))?;
            }
        }
        Ok(())
    }

    fn session_clear(&self) {
        let _ = self.session.as_ref().map(|c| c.borrow_mut().clear());
    }

    fn session_get(&self, key: &str) -> Option<String> {
        self.session.as_ref().and_then(|c| c.borrow().get(key))
    }

    fn session_remove(&self, key: &str) {
        let _ = self.session.as_ref().map(|c| c.borrow_mut().remove(key));
    }

    fn session_set(
        &self,
        key: &str,
        value: &str,
    ) -> Result<(), vicocomo::Error> {
        self.session
            .as_ref()
            .map(|c| c.borrow_mut().set(key, value))
            .unwrap_or_else(|| Err(Error::other("no session store defined")))
    }

    fn url_path_to_dir(&self, url_path: &str) -> Option<String> {
        self.static_routes.get(url_path).map(|s| s.clone())
    }
}

pub struct AxRequest<'req> {
    param_vals: HttpParamVals,
    body: HttpReqBody<'req>,
    request: &'req actix_web::HttpRequest,
    route_pars: HashMap<String, String>,
}

impl<'req> AxRequest<'req> {
    pub fn new(
        request: &'req actix_web::HttpRequest,
        body: HttpReqBody<'req>,
        route_pars: &[(String, String)],
    ) -> Self {
        let mut result = Self {
            param_vals: HttpParamVals::new(),
            body,
            request,
            route_pars: route_pars
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect(),
        };
        result.param_vals.set_request(
            &result.body_par_string(),
            request.uri().query().unwrap_or(""),
        );
        result
    }
}

impl<'req> HttpRequest<'req> for AxRequest<'req> {
    fn http_method(&self) -> HttpMethod {
        self.request
            .method()
            .as_str()
            .try_into()
            .expect(&t!("http-method--unknown"))
    }

    fn param_val(&self, name: &str) -> Option<String> {
        self.param_vals.get(name).map(|v| v[0].clone())
    }

    fn param_vals(&self) -> Vec<(String, String)> {
        self.param_vals.vals()
    }

    fn body(&self) -> HttpReqBody<'req> {
        self.body.clone()
    }

    fn header(&self, name: &str) -> Option<String> {
        self.request
            .headers()
            .get(&name.to_lowercase())
            .and_then(|s| s.to_str().map(|s| s.to_string()).ok())
    }

    fn path(&self) -> String {
        self.request.uri().path().to_string()
    }

    fn route_par_val(&self, par: &str) -> Option<String> {
        self.route_pars.get(par).map(|v| v.clone())
    }

    fn route_par_vals(&self) -> Vec<(String, String)> {
        self.route_pars
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect()
    }

    fn url(&self) -> String {
        self.request.uri().to_string()
    }
}

#[derive(Clone, Debug)]
pub struct AxResponse(HttpResponse);

impl AxResponse {
    pub fn new(resp: HttpResponse) -> Self {
        Self(resp)
    }
}

impl actix_web::Responder for AxResponse {
    type Body = actix_web::body::BoxBody;

    fn respond_to(
        mut self,
        req: &actix_web::HttpRequest,
    ) -> actix_web::HttpResponse {
        use actix_web::http::StatusCode;
        use actix_web::HttpResponse;

        let mut builder = HttpResponse::build(
            StatusCode::from_u16(self.0.get_status() as u16)
                .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
        );
        for header in self.0.drain_headers() {
            builder.append_header(header);
        }
        match self.0.get_body() {
            HttpRespBody::Bytes(b) => builder.body(b),
            HttpRespBody::Download(f) => {
                match actix_files::NamedFile::open(&f) {
                    Ok(resp) => resp.respond_to(req),
                    Err(e) => HttpResponse::NotFound().body(e.to_string()),
                }
            }
            HttpRespBody::Str(s) => builder.body(s),
            HttpRespBody::None => builder.finish(),
        }
    }
}
