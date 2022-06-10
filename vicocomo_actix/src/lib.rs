//! (Ab)use `actix-web` as the web server for a vicocomo application.
//!
//! Implements [`vicocomo::HttpServer`
//! ](../vicocomo/http/server/trait.HttpServer.html) for [`actix-web`
//! ](https://crates.io/crates/actix-web).
//!

mod session;

use ::vicocomo::{DatabaseIf, Error, HttpServer};
pub use ::vicocomo_actix_config::config;
use session::Session;
use std::{cell::RefCell, collections::HashMap};

pub struct AxServer<'a, 'd> {
    request: &'a ::actix_web::HttpRequest,
    req_body: String,
    path_vals: HashMap<String, String>,
    param_vals: HashMap<String, Vec<String>>,
    response: RefCell<Response>,
    session: Option<RefCell<Session<'d>>>,
}

impl<'a, 'd> AxServer<'a, 'd> {
    pub fn new(
        request: &'a ::actix_web::HttpRequest,
        req_body: &str,
        path_vals: &[(String, String)],
        session: Option<::actix_session::Session>,
        db: Option<DatabaseIf<'d>>,
        prune: i64,
    ) -> Self {
        use lazy_static::lazy_static;
        use regex::Regex;
        use urlencoding::decode;
        lazy_static! {
            static ref QUERY: Regex =
                Regex::new(r"([^&=]+=[^&=]+&)*[^&=]+=[^&=]+").unwrap();
        }
        let mut param_vals: HashMap<String, Vec<String>> = HashMap::new();
        let uri_vals = request.uri().query().and_then(|q| decode(q).ok());
        let body_vals = QUERY
            .captures(&req_body)
            .and_then(|c| c.get(0))
            .and_then(|m| decode(m.as_str()).ok());
        for key_value in match uri_vals {
            Some(u) => match body_vals {
                Some(b) => u + "&" + &b,
                None => u,
            },
            None => body_vals.unwrap_or(String::new()),
        }
        .split('&')
        {
            if key_value.len() == 0 {
                continue;
            }
            let mut k_v = key_value.split('=');
            let key = k_v.next().unwrap();
            let val = k_v.next().unwrap_or("");
            match param_vals.get_mut(key) {
                Some(vals) => vals.push(val.to_string()),
                None => {
                    param_vals.insert(key.to_string(), vec![val.to_string()]);
                }
            }
        }
        Self {
            request,
            req_body: req_body.to_string(),
            path_vals: path_vals
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect(),
            param_vals,
            response: RefCell::new(Response::new()),
            session: session.and_then(|s| Session::new(s, db, prune)),
        }
    }

    #[cfg(debug_assertions)]
    pub fn peek(&self) -> String {
        format!("{:?}", self.response.borrow())
    }

    pub fn response(self) -> ::actix_web::HttpResponse {
        self.response.borrow().get()
    }

    #[cfg(debug_assertions)]
    pub fn response_status(&self) -> String {
        format!("{:?}", self.response.borrow().status)
    }
}

impl HttpServer for AxServer<'_, '_> {
    fn param_val(&self, name: &str) -> Option<String> {
        self.param_vals.get(name).map(|v| v[0].clone())
    }

    fn param_vals(&self) -> Vec<(String, String)> {
        let mut result: Vec<(String, String)> = Vec::new();
        for (key, vals) in &self.param_vals {
            for val in vals {
                result.push((key.clone(), val.clone()));
            }
        }
        result
    }

    fn req_path(&self) -> String {
        self.request.uri().path().to_string()
    }

    fn req_path_val(&self, par: &str) -> Option<String> {
        self.path_vals.get(par).map(|v| v.clone())
    }

    fn req_path_vals(&self) -> Vec<(String, String)> {
        self.path_vals
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
        self.response.borrow_mut().body(txt);
    }

    fn resp_error(&self, err: Option<&::vicocomo::Error>) {
        self.response.borrow_mut().internal_server_error(err);
    }

    fn resp_ok(&self) {
        self.response.borrow_mut().ok();
    }

    fn resp_redirect(&self, url: &str) {
        self.response.borrow_mut().redirect(url);
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
    ) -> Result<(), ::vicocomo::Error> {
        self.session
            .as_ref()
            .map(|c| c.borrow_mut().set(key, value))
            .unwrap_or_else(|| Err(Error::other("no session store defined")))
    }
}

#[derive(Clone, Copy, Debug)]
enum ResponseStatus {
    InternalServerError,
    NoResponse,
    Ok,
    Redirect,
}

#[derive(Clone, Debug)]
struct Response {
    status: ResponseStatus,
    text: String,
}

impl Response {
    fn new() -> Self {
        Self {
            status: ResponseStatus::NoResponse,
            text: String::new(),
        }
    }

    fn body(&mut self, text: &str) {
        self.text = text.to_string();
    }

    fn get(&self) -> ::actix_web::HttpResponse {
        use ::actix_web::{http::header, HttpResponse};
        match self.status {
            ResponseStatus::InternalServerError => {
                HttpResponse::InternalServerError().body(self.text.clone())
            }
            ResponseStatus::NoResponse => HttpResponse::InternalServerError()
                .body("Internal server error: No response"),
            ResponseStatus::Ok => HttpResponse::Ok()
                .content_type("text/html; charset=utf-8")
                .body(self.text.clone()),
            ResponseStatus::Redirect => HttpResponse::Found()
                .append_header((header::LOCATION, self.text.clone()))
                .finish()
        }
    }

    fn internal_server_error(&mut self, err: Option<&::vicocomo::Error>) {
        self.status = ResponseStatus::InternalServerError;
        self.text = format!(
            "Internal server error: {}",
            match err {
                Some(e) => e.to_string(),
                None => "Unknown".to_string(),
            }
        );
    }

    fn ok(&mut self) {
        self.status = ResponseStatus::Ok;
    }

    fn redirect(&mut self, url: &str) {
        self.status = ResponseStatus::Redirect;
        self.text = url.to_string();
    }
}
