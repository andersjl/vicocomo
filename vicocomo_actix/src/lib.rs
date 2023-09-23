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
use std::{cell::RefCell, collections::HashMap};
use vicocomo::{
    AppConfigVal, DatabaseIf, Error, HttpParamVals, HttpResponse,
    HttpResponseStatus, HttpServer, HttpStatus,
};
pub use vicocomo_actix_config::config;

pub struct AxServer<'conf, 'req, 'stro> {
    app_config: &'conf HashMap<String, AppConfigVal>,
    param_vals: HttpParamVals,
    req_body: String,
    request: &'req actix_web::HttpRequest,
    response: RefCell<AxResponse>,
    route_pars: HashMap<String, String>,
    session: Option<RefCell<Session>>,
    static_routes: &'stro HashMap<String, String>,
}

impl<'conf, 'req, 'stro> AxServer<'conf, 'req, 'stro> {
    pub fn new(
        app_config: &'conf HashMap<String, AppConfigVal>,
        static_routes: &'stro HashMap<String, String>,
        request: &'req actix_web::HttpRequest,
        req_body: &str,
        route_pars: &[(String, String)],
        session: Option<actix_session::Session>,
        db: Option<DatabaseIf>,
        prune: i64,
    ) -> Self {
        let param_vals = HttpParamVals::from_request(
            req_body,
            request.uri().query().unwrap_or(""),
        );
        Self {
            app_config,
            param_vals,
            req_body: req_body.to_string(),
            request,
            response: RefCell::new(AxResponse::new()),
            route_pars: route_pars
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect(),
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

    #[cfg(debug_assertions)]
    pub fn peek(&self) -> String {
        format!("{:?}", self.response.borrow())
    }

    pub fn response(self) -> actix_web::HttpResponse {
        use actix_web::Responder;
        self.response.into_inner().respond_to(self.request)
    }

    #[cfg(debug_assertions)]
    pub fn response_status(&self) -> String {
        format!("{:?}", self.response.borrow().0.status)
    }
}

impl HttpServer for AxServer<'_, '_, '_> {
    fn app_config(&self, id: &str) -> Option<AppConfigVal> {
        self.app_config.get(id).map(|v| v.clone())
    }

    fn param_val(&self, name: &str) -> Option<String> {
        self.param_vals.get(name).map(|v| v[0].clone())
    }

    fn param_vals(&self) -> Vec<(String, String)> {
        self.param_vals.vals()
    }

    fn req_path(&self) -> String {
        self.request.uri().path().to_string()
    }

    fn req_route_par_val(&self, par: &str) -> Option<String> {
        self.route_pars.get(par).map(|v| v.clone())
    }

    fn req_route_par_vals(&self) -> Vec<(String, String)> {
        self.route_pars
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect()
    }

    fn req_body(&self) -> String {
        self.req_body.clone()
    }

    fn req_url(&self) -> String {
        self.request.uri().to_string()
    }

    fn resp_body(&self, txt: &str) {
        self.response.borrow_mut().0.set_body(txt);
    }

    fn resp_error(&self, status: HttpStatus, err: Option<&vicocomo::Error>) {
        self.response.borrow_mut().0.error(status, err);
    }

    fn resp_file(&self, file_path: &str) {
        self.response.borrow_mut().0.file(file_path);
    }

    fn resp_ok(&self) {
        self.response.borrow_mut().0.ok();
    }

    fn resp_redirect(&self, url: &str) {
        self.response.borrow_mut().0.redirect(url);
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

#[derive(Clone, Debug)]
struct AxResponse(HttpResponse);

impl AxResponse {
    fn new() -> Self {
        Self(HttpResponse::new())
    }
}

impl actix_web::Responder for AxResponse {
    type Body = actix_web::body::BoxBody;

    fn respond_to(
        self,
        req: &actix_web::HttpRequest,
    ) -> actix_web::HttpResponse {
        use actix_web::http::{header, StatusCode};
        use actix_web::HttpResponse;
        match self.0.status {
            HttpResponseStatus::Error(code) => HttpResponse::build(
                StatusCode::from_u16(code as u16)
                    .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
            )
            .body(self.0.text.clone()),
            HttpResponseStatus::File => {
                match actix_files::NamedFile::open(self.0.text.clone()) {
                    Ok(resp) => resp.respond_to(req),
                    Err(e) => HttpResponse::NotFound().body(e.to_string()),
                }
            }
            HttpResponseStatus::NoResponse => {
                HttpResponse::InternalServerError()
                    .body("Internal server error: No response")
            }
            HttpResponseStatus::Ok => HttpResponse::Ok()
                .content_type("text/html; charset=utf-8")
                .body(self.0.text.clone()),
            HttpResponseStatus::Redirect => HttpResponse::Found()
                .append_header((header::LOCATION, self.0.text.clone()))
                .finish(),
        }
    }
}
