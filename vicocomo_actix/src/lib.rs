use actix_web;
use vicocomo::{Request, Response, SessionStore};
pub use vicocomo_actix_config::config;

/// Implements [`vicocomo::HttpServer`](../../vicocomo/module.HttpServer.html)
/// traits for [`actix-web`](../../actix-web/index.html).
///
pub struct AxRequest {
    body: String,
    uri: String,
    path_vals: Vec<String>,
    param_vals: std::collections::HashMap<String, String>,
}

impl AxRequest {
    /// TODO: GET and POST parameters to param_vals.
    ///
    pub fn new(
        body: &str,
        uri: &actix_web::http::Uri,
        path_vals: &[String],
    ) -> Self {
        Self {
            body: body.to_string(),
            uri: uri.to_string(),
            path_vals: path_vals.to_vec(),
            param_vals: std::collections::HashMap::new(),
        }
    }
}

impl Request for AxRequest {
    fn req_body(&self) -> String {
        self.body.clone()
    }

    fn uri(&self) -> String {
        self.uri.clone()
    }

    fn path_vals(&self) -> std::slice::Iter<String> {
        self.path_vals.iter()
    }

    fn param_val(&self, name: &str) -> Option<String> {
        self.param_vals.get(name).map(|s| s.clone())
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
