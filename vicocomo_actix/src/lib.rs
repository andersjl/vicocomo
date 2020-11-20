//! (Ab)use `actix-web` as the web server for a vicocomo application.
//!
//! Implements [`vicocomo::HttpServer`](../../vicocomo/trait.HttpServer.html)
//! for [`actix-web`](../../actix-web/index.html).
//!

use ::vicocomo::{Error, HttpServer};
pub use ::vicocomo_actix_config::config;
use std::{cell::RefCell, collections::HashMap};

pub struct AxServer<'a> {
    request: &'a ::actix_web::HttpRequest,
    req_body: String,
    path_vals: Vec<String>,
    param_vals: HashMap<String, Vec<String>>,
    response: RefCell<Response>,
    session: Option<::actix_session::Session>,
}

impl<'a> AxServer<'a> {
    pub fn new(
        request: &'a ::actix_web::HttpRequest,
        req_body: &str,
        path_vals: &[String],
        sess: Option<::actix_session::Session>,
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
            None => body_vals.unwrap_or("".to_string()),
        }
        .split('&')
        {
            if key_value.len() == 0 {
                continue;
            }
            let mut k_v = key_value.split('=');
            let key = k_v.next().unwrap();
            let val = k_v.next().unwrap();
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
            path_vals: path_vals.to_vec(),
            param_vals,
            response: RefCell::new(Response::new()),
            session: sess,
        }
    }

    pub fn response(self) -> ::actix_web::HttpResponse {
        self.response.borrow().get()
    }
}

impl HttpServer for AxServer<'_> {
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

    fn req_path_vals(&self) -> std::slice::Iter<String> {
        self.path_vals.iter()
    }

    fn req_body(&self) -> String {
        self.req_body.clone()
    }

    fn req_uri(&self) -> String {
        self.request.uri().to_string()
    }

    fn url_for(
        &self,
        path: &str,
        params: &[&str],
    ) -> Result<String, Error> {
        self.request
            .url_for(path, params) // we did set name = path
            .map(|u| u.to_string())
            .map_err(|e| Error::invalid_input(&e.to_string()))
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
        self.session.as_ref().map(|s| s.clear()).unwrap_or(());
    }

    fn session_get(&self, key: &str) -> Option<String> {
        self.session.as_ref().and_then(|s| s.get(key).unwrap_or(None))
    }

    fn session_remove(&self, key: &str) {
        self.session.as_ref().map(|s| s.remove(key)).unwrap_or(())
    }

    fn session_set(
        &self,
        key: &str,
        value: &str,
    ) -> Result<(), ::vicocomo::Error> {
        self.session.as_ref()
            .map(|s| {
                s.set(key, value)
                    .map_err(|e| ::vicocomo::Error::other(&e.to_string()))
            })
            .unwrap_or_else(|| Err(Error::other("no session store defined")))
    }
}

enum ResponseStatus {
    InternalServerError,
    NoResponse,
    Ok,
    Redirect,
}

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
                HttpResponse::InternalServerError().body(&self.text)
            }
            ResponseStatus::NoResponse => {
                HttpResponse::InternalServerError().body(
                    "Internal server error: No response"
                )
            }
            ResponseStatus::Ok => {
                HttpResponse::Ok()
                    .content_type("text/html; charset=utf-8")
                    .body(&self.text)
            }
            ResponseStatus::Redirect => {
                HttpResponse::Found()
                    .header(header::LOCATION, self.text.clone())
                    .finish()
                    .into_body()
            }
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
