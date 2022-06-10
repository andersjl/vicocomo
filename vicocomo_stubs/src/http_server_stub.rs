//! A stub implementation of `vicococmo::HttpServer`

use ::std::{cell::RefCell, collections::HashMap};
use ::vicocomo::{Error, HttpServer};

pub struct HttpServerStub {
    params: RefCell<HashMap<String, String>>,
    path_vals: RefCell<HashMap<String, String>>,
    request: RefCell<Request>,
    response: RefCell<Response>,
    session: RefCell<HashMap<String, String>>,
}

macro_rules! set_map {
    ($map:expr, $vals:expr) => {{
        let mut map = $map.borrow_mut();
        map.clear();
        for (par, val) in $vals {
            map.insert(par.to_string(), val.to_string());
        }
    }};
}

impl HttpServerStub {
    pub fn new() -> Self {
        Self {
            params: RefCell::new(HashMap::new()),
            path_vals: RefCell::new(HashMap::new()),
            request: RefCell::new(Request {
                scheme: String::new(),
                host: String::new(),
                path: String::new(),
                parameters: String::new(),
                body: String::new(),
            }),
            response: RefCell::new(Response {
                status: ResponseStatus::NoResponse,
                text: String::new(),
            }),
            session: RefCell::new(HashMap::new()),
        }
    }

    pub fn get_response(&self) -> Response {
        self.response.borrow().clone()
    }

    pub fn set_params(&self, vals: &[(&str, &str)]) {
        set_map!(self.params, vals)
    }

    pub fn set_path_vals(&self, vals: &[(&str, &str)]) {
        set_map!(self.path_vals, vals)
    }

    pub fn set_request(&self, req: Request) {
        let mut request = self.request.borrow_mut();
        *request = req;
    }
}

impl HttpServer for HttpServerStub {
    fn param_val(&self, name: &str) -> Option<String> {
        self.params.borrow().get(name).map(|v| v.to_string())
    }

    fn param_vals(&self) -> Vec<(String, String)> {
        self.params
            .borrow()
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    fn req_path(&self) -> String {
        self.request.borrow().path.to_string()
    }

    fn req_path_val(&self, par: &str) -> Option<String> {
        self.path_vals.borrow().get(par).map(|v| v.to_string())
    }

    fn req_path_vals(&self) -> Vec<(String, String)> {
        self.path_vals
            .borrow()
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    fn req_body(&self) -> String {
        self.request.borrow().body.to_string()
    }

    fn req_url(&self) -> String {
        let mut result = String::new();
        if !self.request.borrow().scheme.is_empty() {
            result = result + &self.request.borrow().scheme + "//:";
        }
        if !self.request.borrow().host.is_empty() {
            result += &self.request.borrow().host;
        }
        result = result + "/" + &self.request.borrow().path;
        if !self.request.borrow().parameters.is_empty() {
            result = result + "?" + &self.request.borrow().parameters;
        }
        result
    }

    fn resp_body(&self, txt: &str) {
        self.response.borrow_mut().set_body(txt);
    }

    fn resp_error(&self, err: Option<&Error>) {
        self.response.borrow_mut().set_error(err);
    }

    fn resp_ok(&self) {
        self.response.borrow_mut().ok();
    }

    fn resp_redirect(&self, url: &str) {
        self.response.borrow_mut().redirect(url);
    }

    fn session_clear(&self) {
        self.session.borrow_mut().clear();
    }

    fn session_get(&self, key: &str) -> Option<String> {
        self.session.borrow().get(key).map(|v| v.to_string())
    }

    fn session_remove(&self, key: &str) {
        self.session.borrow_mut().remove(key);
    }

    fn session_set(&self, key: &str, value: &str) -> Result<(), Error> {
        self.session
            .borrow_mut()
            .insert(key.to_string(), value.to_string());
        Ok(())
    }
}

#[derive(Clone)]
pub struct Request {
    pub scheme: String,
    pub host: String,
    pub path: String,
    pub parameters: String,
    pub body: String,
}

#[derive(Clone)]
pub struct Response {
    pub status: ResponseStatus,
    pub text: String,
}

impl Response {
    fn set_body(&mut self, txt: &str) {
        self.text = txt.to_string();
    }

    fn set_error(&mut self, err: Option<&::vicocomo::Error>) {
        self.status = ResponseStatus::Error;
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

#[derive(Clone, Copy)]
pub enum ResponseStatus {
    Error,
    NoResponse,
    Ok,
    Redirect,
}
