//! A stub implementation of `vicococmo::HttpServer`

use ::vicocomo::{
    AppConfigVal, Error, HttpMethod, HttpReqBody, HttpRequest, HttpServer,
};
use std::cell::RefCell;
use std::collections::HashMap;
use std::path::Path;

#[derive(Clone, Debug)]
pub struct Server {
    config: RefCell<HashMap<String, AppConfigVal>>,
    session: RefCell<HashMap<String, String>>,
    static_routes: RefCell<HashMap<String, String>>,
}

impl Server {
    pub fn new() -> Self {
        let result = Self {
            config: RefCell::new(HashMap::new()),
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

    pub fn set_config(&self, vals: &[(&str, &AppConfigVal)]) {
        self.defaults();
        let mut map = self.config.borrow_mut();
        for (par, val) in vals {
            map.insert(par.to_string(), (*val).clone());
        }
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

    fn handle_upload(&self, _files: &[Option<&Path>]) -> Result<(), Error> {
        Err(Error::nyi())
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
    pub body: RefCell<Vec<u8>>,
    pub host: RefCell<String>,
    pub method: HttpMethod,
    pub parameters: RefCell<String>,
    pub params: RefCell<HashMap<String, Vec<String>>>,
    pub path: RefCell<String>,
    pub route_pars: RefCell<HashMap<String, String>>,
    pub scheme: RefCell<String>,
}

impl Request {
    pub fn new() -> Self {
        Self {
            body: RefCell::new(Vec::new()),
            host: RefCell::new(String::new()),
            method: HttpMethod::None,
            parameters: RefCell::new(String::new()),
            params: RefCell::new(HashMap::new()),
            path: RefCell::new(String::new()),
            route_pars: RefCell::new(HashMap::new()),
            scheme: RefCell::new(String::new()),
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
}

impl<'a> HttpRequest<'a> for Request {
    fn http_method(&self) -> HttpMethod {
        self.method
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

    fn body(&self) -> HttpReqBody<'a> {
        static mut BODY: Vec<u8> = Vec::new();
        unsafe {
            BODY = self.body.borrow().clone();
            HttpReqBody {
                bytes: BODY.as_slice(),
                parts: Vec::new(),
            }
        }
    }

    fn header(&self, _name: &str) -> Option<String> {
        None
    }

    fn path(&self) -> String {
        self.path.borrow().to_string()
    }

    fn route_par_val(&self, par: &str) -> Option<String> {
        self.route_pars.borrow().get(par).map(|v| v.to_string())
    }

    fn route_par_vals(&self) -> Vec<(String, String)> {
        self.route_pars
            .borrow()
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    fn url(&self) -> String {
        let mut result = String::new();
        if !self.scheme.borrow().is_empty() {
            result = result + &self.scheme.borrow() + "://";
        }
        if !self.host.borrow().is_empty() {
            result += &self.host.borrow();
        }
        result = result + &self.path.borrow();
        if !self.parameters.borrow().is_empty() {
            result = result + "?" + &self.parameters.borrow();
        }
        result
    }
}
