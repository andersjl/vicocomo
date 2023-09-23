//! A stub implementation of `vicococmo::HttpServer`

use ::std::{cell::RefCell, collections::HashMap};
use ::vicocomo::{AppConfigVal, Error, HttpResponse, HttpServer, HttpStatus};

#[derive(Clone, Debug)]
pub struct Server {
    config: RefCell<HashMap<String, AppConfigVal>>,
    params: RefCell<HashMap<String, Vec<String>>>,
    route_pars: RefCell<HashMap<String, String>>,
    request: RefCell<Request>,
    response: RefCell<HttpResponse>,
    session: RefCell<HashMap<String, String>>,
    static_routes: RefCell<HashMap<String, String>>,
}

impl Server {
    pub fn new() -> Self {
        let result = Self {
            config: RefCell::new(HashMap::new()),
            params: RefCell::new(HashMap::new()),
            route_pars: RefCell::new(HashMap::new()),
            request: RefCell::new(Request {
                scheme: String::new(),
                host: String::new(),
                path: String::new(),
                parameters: String::new(),
                body: String::new(),
            }),
            response: RefCell::new(HttpResponse::new()),
            session: RefCell::new(HashMap::new()),
            static_routes: RefCell::new(HashMap::new()),
        };
        result.defaults();
        result
    }

    pub fn add_config(&self, attr: &str, val: AppConfigVal) {
        self.config.borrow_mut().insert(attr.to_string(), val);
    }

    pub fn defaults(&self) {
        macro_rules! null_string {
            () => {
                AppConfigVal::Str(String::new())
            };
        }
        let mut map = self.config.borrow_mut();
        map.clear();
        map.insert("file_root".to_string(), null_string!());
        map.insert("strip_mtime".to_string(), AppConfigVal::Bool(false));
        map.insert("url_root".to_string(), null_string!());
    }

    pub fn get_response(&self) -> HttpResponse {
        self.response.borrow().clone()
    }

    pub fn set_config(&self, vals: &[(&str, &AppConfigVal)]) {
        self.defaults();
        let mut map = self.config.borrow_mut();
        for (par, val) in vals {
            map.insert(par.to_string(), (*val).clone());
        }
    }

    pub fn set_params(&self, vals: &[(&str, &str)]) {
        let mut map = self.params.borrow_mut();
        map.clear();
        for (par, val) in vals {
            map.get_mut(*par)
                .map(|arr| arr.push(val.to_string()))
                .unwrap_or_else(|| {
                    map.insert(par.to_string(), vec![val.to_string()]);
                });
        }
    }

    pub fn set_route_pars(&self, vals: &[(&str, &str)]) {
        let mut map = self.route_pars.borrow_mut();
        map.clear();
        for (par, val) in vals {
            map.insert(par.to_string(), val.to_string());
        }
    }

    pub fn set_request(&self, req: Request) {
        let mut request = self.request.borrow_mut();
        *request = req;
    }

    pub fn set_static_routes(&self, vals: &[(&str, &str)]) {
        let mut map = self.static_routes.borrow_mut();
        map.clear();
        for (par, val) in vals {
            map.insert(par.to_string(), val.to_string());
        }
    }
}

impl HttpServer for Server {
    fn app_config(&self, id: &str) -> Option<AppConfigVal> {
        self.config.borrow().get(id).map(|v| v.clone())
    }

    fn param_val(&self, name: &str) -> Option<String> {
        self.params
            .borrow()
            .get(name)
            .map(|v| v.first().unwrap().clone())
    }

    fn param_vals(&self) -> Vec<(String, String)> {
        let mut result: Vec<(String, String)> = Vec::new();
        for (par, arr) in self.params.borrow().iter() {
            for val in arr {
                result.push((par.to_string(), val.to_string()));
            }
        }
        result
    }

    fn req_path(&self) -> String {
        self.request.borrow().path.to_string()
    }

    fn req_route_par_val(&self, par: &str) -> Option<String> {
        self.route_pars.borrow().get(par).map(|v| v.to_string())
    }

    fn req_route_par_vals(&self) -> Vec<(String, String)> {
        self.route_pars
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
            result = result + &self.request.borrow().scheme + "://";
        }
        if !self.request.borrow().host.is_empty() {
            result += &self.request.borrow().host;
        }
        result = result + &self.request.borrow().path;
        if !self.request.borrow().parameters.is_empty() {
            result = result + "?" + &self.request.borrow().parameters;
        }
        result
    }

    fn resp_body(&self, txt: &str) {
        self.response.borrow_mut().set_body(txt);
    }

    fn resp_error(&self, status: HttpStatus, err: Option<&Error>) {
        self.response.borrow_mut().error(status, err);
    }

    fn resp_file(&self, file_path: &str) {
        self.response.borrow_mut().file(file_path);
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

    fn url_path_to_dir(&self, url_path: &str) -> Option<String> {
        self.static_routes
            .borrow()
            .get(url_path)
            .map(|s| s.to_string())
    }
}

#[derive(Clone, Debug)]
pub struct Request {
    pub scheme: String,
    pub host: String,
    pub path: String,
    pub parameters: String,
    pub body: String,
}
