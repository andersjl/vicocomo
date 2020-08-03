use actix_web;
use std::collections::HashMap;
use vicocomo::{Request, Response, SessionStore};
pub use vicocomo_actix_config::config;

/// Implements [`vicocomo::HttpServer`](../../vicocomo/module.HttpServer.html)
/// traits for [`actix-web`](../../actix-web/index.html).
///
pub struct AxRequest {
    body: String,
    uri: String,
    path_vals: Vec<String>,
    param_vals: HashMap<String, Vec<String>>,
}

impl AxRequest {
    pub fn new(
        body: &str,
        uri: &actix_web::http::Uri,
        path_vals: &[String],
    ) -> Self {
        use lazy_static::lazy_static;
        use regex::Regex;
        use urlencoding::decode;
        lazy_static! {
            static ref QUERY: Regex =
                Regex::new(r"([^&=]+=[^&=]+&)*[^&=]+=[^&=]+").unwrap();
        }
        let mut param_vals: HashMap<String, Vec<String>> = HashMap::new();
        let uri_vals = uri.query().and_then(|q| decode(q).ok());
        let body_vals = QUERY
            .captures(&body)
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
            body: body.to_string(),
            uri: uri.to_string(),
            path_vals: path_vals.to_vec(),
            param_vals,
        }
    }
}

impl Request for AxRequest {
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

    fn path_vals(&self) -> std::slice::Iter<String> {
        self.path_vals.iter()
    }

    fn req_body(&self) -> String {
        self.body.clone()
    }

    fn uri(&self) -> String {
        self.uri.clone()
    }
}

pub struct AxResponse {
    body: String,
    response: Option<actix_web::HttpResponse>,
}

impl AxResponse {
    pub fn new() -> Self {
        Self {
            body: String::new(),
            response: None,
        }
    }

    pub fn response(self) -> actix_web::HttpResponse {
        self.response.unwrap_or_else(|| {
            actix_web::HttpResponse::InternalServerError()
                .body("Internal server error: No response")
        })
    }
}

impl Response for AxResponse {
    fn resp_body(&mut self, txt: &str) {
        self.body = txt.to_string();
    }

    fn internal_server_error(&mut self, err: Option<&vicocomo::Error>) {
        self.response = Some(
            actix_web::HttpResponse::InternalServerError().body(format!(
                "Internal server error: {}",
                match err {
                    Some(e) => e.to_string(),
                    None => "Unknown".to_string(),
                }
            )),
        );
    }

    fn ok(&mut self) {
        self.response = Some(
            actix_web::HttpResponse::Ok()
                .content_type("text/html; charset=utf-8")
                .body(self.body.clone()),
        );
    }

    fn redirect(&mut self, url: &str) {
        use actix_web::http::header;
        self.response = Some(
            actix_web::HttpResponse::Found()
                .header(header::LOCATION, url)
                .finish()
                .into_body(),
        );
    }
}

pub struct AxSessionStore(actix_session::Session);

impl AxSessionStore {
    pub fn new(sess: actix_session::Session) -> Self {
        Self(sess)
    }
}

impl SessionStore for AxSessionStore {
    fn clear(&self) {
        self.0.clear();
    }

    fn get(&self, key: &str) -> Option<String> {
        self.0.get(key).unwrap_or(None)
    }

    fn remove(&self, key: &str) {
        self.0.remove(key)
    }

    fn set(&self, key: &str, value: &str) -> Result<(), vicocomo::Error> {
        self.0
            .set(key, value)
            .map_err(|e| vicocomo::Error::other(&e.to_string()))
    }
}
