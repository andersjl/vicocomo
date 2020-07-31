//! Traits and structs implemented by an HTTP server and used by applications.
//!
use crate::Error;
use core::{
    convert::TryFrom,
    fmt::{self, Display, Formatter},
};
use serde::{de::DeserializeOwned, Serialize};
use std::collections::HashMap;
use syn::{
    parse::{Parse, ParseStream},
    Ident, Path,
};

/// A custom syntax tree node for configuring an HTTP server.  Intended for
/// use in a server specific `config` macro, see an [example for `actix-web`
/// ](../../../../examples/http_server/actix/target/doc/vicocomo_example_actix/index.html).
///
/// TODO: a new field and config item templ_eng, probably a a struct.
///
/// # Code example:
///
/// ```text
/// // Route config, see below for the meaning of "Control" in route(Control)
///                        // HTTP | Actix URL            | ctrl | method
///                        // =====+======================+======+==========
/// route(Rsrc) {          // CRUD requests, only those given are generated
/// // Create request         -----+----------------------+------+----------
///   new_form,            // get  | "/rsrc/new"          | Rsrc | new_form
///   copy_form,           // get  | "/rsrc/<id>/copy"    | Rsrc | copy_form
///   create,              // post | "/rsrc"              | Rsrc | create
///   ensure,              // post | "/rsrc/ensure"       | Rsrc | ensure
/// // Read request           -----+----------------------+------+----------
///   index,               // get  | "/rsrc"              | Rsrc | index
///   show,                // get  | "/rsrc/<id>"         | Rsrc | show
/// // Update request         -----+----------------------+------+----------
///   edit_form,           // get  | "/rsrc/<id>/edit"    | Rsrc | edit_form
///   patch,               // post | "/rsrc/<id>"         | Rsrc | patch
///   replace,             // post | "/rsrc/<id>/replace" | Rsrc | replace
/// // Delete request         -----+----------------------+------+----------
///   delete,              // post | "/rsrc/<id>/delete"  | Rsrc | delete
/// },                     // =====+======================+======+==========
/// route(Cust) {          //   Methods may be customized |      |
///   custom {             // -----+----------------------+------+----------
///     http_method: get,  //   Order matters, omitted default if defined
///     path: "path",      // get  | "/path"              | Cust | custom
/// }},                    // =====+======================+======+==========
/// route(Sing) {          //   Example: configure a singleton resource
///   new_form,            // get  | "/sing/new"          | Sing | new_form
///   create,              // post | "/sing"              | Sing | create
///   ensure,              // post | "/sing/ensure"       | Sing | ensure
///   show                 //   full path must be given if leading slash
///   { path: "/sing" },   // get  | "/sing"              | Sing | show
///   edit_form            //   resource snake prepended if no leading slash
///   { path: "edit" },    // get  | "/sing/edit"         | Sing | edit_form
///   patch { path: "" },  // post | "/sing"              | Sing | patch
///   replace              //      |                      |      |
///   { path: "replace" }, // post | "/sing/replace"      | Sing | replace
///   delete               //      |                      |      |
///   { path: "delete" },  // post | "/sing/delete"       | Sing | delete
/// },                     // =====+======================+======+==========
/// route(Othr) {          //   Customized path parameters are given as
///   parm_req { path:     // <param> (parameter name ignored)
///     "some/<param>",    // get  | "/some/<p0>"         | Othr | parm_req
///   },                   // -----+----------------------+------+----------
///   post_req {           //   Except for the standardized CRUD requests
///     http_method: post, // above get is the default HTTP method
///     path: "postpth",   // post | "/postpth"           | Othr | post_req
/// }},                    // =====+======================+======+==========
/// // Not Found handler   //      |                      |      |
/// notfnd(Hand) { func }, // all not handled elsewhere   | Hand | func
///                        // default a simple 404 with text body
/// ```
///
///  Definition of "Controller" in `route(Controller)` and
///  `notfnd(Controller)`:
///
///  The controller is given as `some::path::to::Controller`.  If the path is
///  a single identifier, as in the examples, `crate::controllers::` is
///  prepended.
///
///  The handling method is called as
///  `some::path::to::Controller::handler(...)`.  So the controller may be a
///  module, struct, or enum as long as the handling method does not have a
///  receiver.  In the struct/enum case it would probably be a constructor.
///
///  Handling method signature:
///  ```text
///  (
///    &impl vicocomo::Request,       // server request
///    &impl vicocomo::TemplEng,      // template engine
///    &impl vicocomo::DbConn,        // database connection
///    &mut impl vicocomo::Session,   // session object
///    &mut impl vicocomo::Response,  // request body
///  ) -> Result<(), vicocomo::Error>
/// ```
///
#[derive(Clone, Debug)]
pub struct Config {
    /// The routing targets, mapping a controller to its route handlers.
    ///
    pub routes: HashMap<Path, Vec<Handler>>,

    /// Optional custom handler for failed routes.
    ///
    pub not_found: Option<(Path, Handler)>,
}

impl Config {}

/// Methods for getting information about and from the request.
///
pub trait Request {
    /// The body of the request
    ///
    fn req_body(&self) -> String;

    /// The requested HTTP URI, including scheme, path, and query.
    /// W.I.P. more methods for scheme, path etc TBD.
    ///
    fn uri(&self) -> String;

    /// If registered as `"a/<p1>/<p2>"` and the HTTP path of the request is
    /// `"a/42/Hello"`, this will collect as e.g.
    /// `vec![String::from("42"), String::from("Hello")]`
    ///
    fn path_vals(&self) -> std::slice::Iter<String>;

    /// NYI!!!  The values of parameters in the URI (get) or body (POST)
    fn param_val(&self, name: &str) -> Option<String>;
}

/// Methods to build the response.
///
pub trait Response {
    /// Set the body of the response
    ///
    fn resp_body(&mut self, txt: &str);

    /// Generate an internal server error response, replacing the body.
    ///
    fn internal_server_error(&mut self, err: Option<&Error>);

    /// Generate an OK response, using the body.
    ///
    fn ok(&mut self);

    /// Generate a redirect response, ignoring the body.
    ///
    fn redirect(&mut self, url: &str);
}

/// Methods to render via a template engine.
///
pub trait TemplEng {
    fn render(
        &self,
        tmpl: &str,
        data: &impl Serialize,
    ) -> Result<String, Error>;
}

/// Methods to store a cookie session.  Should be implemented by an HTTP
/// server.  Applications should use [`Session`](struct.Session.html)
///
pub trait SessionStore {
    /// Clear the entire session.
    ///
    fn clear(&self);

    /// Retreive the value for `key` or `None` if not present.
    ///
    fn get(&self, key: &str) -> Option<String>;

    /// Remove the `key`-value pair.
    ///
    fn remove(&self, key: &str);

    /// Set a `value` for `key`.
    ///
    fn set(&self, key: &str, value: &str) -> Result<(), Error>;
}

/// A cookie session.
///
pub struct Session<'a>(&'a dyn SessionStore);

impl<'a> Session<'a> {
    /// Create a `Session` from a [`SessionStore`](trait.SessionStore.html).
    ///
    pub fn new(store: &'a impl SessionStore) -> Self {
        Self(store)
    }

    /// Clear the entire session.
    ///
    pub fn clear(&self) {
        self.0.clear();
    }

    /// Return `key` as `T`.
    ///
    /// If the current value for `key` is not a `T`, an error is returned.
    /// If there is no current value for `key`, `None` is returned.
    ///
    pub fn get<T>(&self, key: &str) -> Result<Option<T>, Error>
    where
        T: DeserializeOwned,
    {
        match self.0.get(key) {
            Some(s) => Ok(Some(
                serde_json::from_str(&s)
                    .map_err(|e| Error::other(&e.to_string()))?,
            )),
            None => Ok(None),
        }
    }

    /// Remove the `key`-value pair.
    ///
    pub fn remove(&self, key: &str) {
        self.0.remove(key)
    }

    /// Set a `value` for `key`.
    ///
    /// Returns an error if serializing fails.
    ///
    pub fn set<T>(&self, key: &str, value: &T) -> Result<(), Error>
    where
        T: Serialize,
    {
        self.0.set(
            key,
            &serde_json::to_string(value)
                .map_err(|e| Error::other(&e.to_string()))?,
        )
    }
}

/// A simple enum with the official methods.
///
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub enum HttpMethod {
    Connect,
    Delete,
    Get,
    Head,
    Options,
    Patch,
    Post,
    Put,
    Trace,
}

impl Copy for HttpMethod {}

impl Display for HttpMethod {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Connect => "connect",
                Self::Delete => "delete",
                Self::Get => "get",
                Self::Head => "head",
                Self::Options => "options",
                Self::Patch => "patch",
                Self::Post => "post",
                Self::Put => "put",
                Self::Trace => "trace",
            }
        )
    }
}

impl TryFrom<&str> for HttpMethod {
    type Error = Error;
    fn try_from(s: &str) -> Result<Self, Error> {
        match s.to_lowercase().as_str() {
            "connect" => Ok(HttpMethod::Connect),
            "delete" => Ok(HttpMethod::Delete),
            "get" => Ok(HttpMethod::Get),
            "head" => Ok(HttpMethod::Head),
            "options" => Ok(HttpMethod::Options),
            "patch" => Ok(HttpMethod::Patch),
            "post" => Ok(HttpMethod::Post),
            "put" => Ok(HttpMethod::Put),
            "trace" => Ok(HttpMethod::Trace),
            &_ => Err(Error::other(&format!("{} is not an HTTP method", s))),
        }
    }
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
enum ConfigItem {
    NotFnd {
        controller: Path,
        handlers: Vec<Handler>,
    },
    Route {
        controller: Path,
        handlers: Vec<Handler>,
    },
}

/// Information needed for implementing an HTTP server configuration macro
/// using [`Config`](struct.Config.html).
#[derive(Clone, Debug)]
pub struct Handler {
    /// only tested for Get and Post.
    pub http_method: HttpMethod,
    /// HTTP path, possibly with path parameters in angle brackets.
    pub http_path: String,
    /// number of path parameters.
    pub path_par_count: usize,
    /// controller method name.
    pub contr_method: Ident,
}

impl Parse for Config {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        use syn::{parse_quote, token};
        let mut routes: HashMap<Path, Vec<Handler>> = HashMap::new();
        let mut not_found: Option<(Path, Handler)> = None;
        for item in input
            .parse_terminated::<ConfigItem, token::Comma>(ConfigItem::parse)?
        {
            match item {
                ConfigItem::NotFnd {
                    controller,
                    mut handlers,
                } => {
                    not_found = Some((controller, handlers.pop().unwrap()));
                }
                ConfigItem::Route {
                    mut controller,
                    mut handlers,
                } => {
                    let segments = &controller.segments;
                    if 1 == segments.len() {
                        let contr_id =
                            &segments.last().unwrap().ident.clone();
                        controller.segments =
                            parse_quote!(crate::controllers::#contr_id);
                    }
                    match routes.get_mut(&controller) {
                        Some(hands) => hands.extend(handlers.drain(..)),
                        None => {
                            routes.insert(controller, handlers);
                        }
                    }
                }
            }
        }
        Ok(Self { routes, not_found })
    }
}

impl Parse for ConfigItem {
    fn parse(input: ParseStream) -> syn::Result<Self> {
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

fn get_handlers(input: ParseStream) -> syn::Result<(Path, Vec<Handler>)> {
    use case::CaseExt;
    use syn::{braced, parenthesized, parse_quote, token};
    let content;
    parenthesized!(content in input);
    let mut controller: Path = content.parse()?;
    let segments = &controller.segments;
    let contr_id = &segments.last().unwrap().ident.clone();
    if 1 == segments.len() {
        controller.segments = parse_quote!(crate::controllers::#contr_id);
    }
    let contr_id_snake = contr_id.to_string().to_snake();
    let content;
    braced!(content in input);
    let mut handlers: Vec<Handler> = content
        .parse_terminated::<Handler, token::Comma>(Handler::parse)?
        .into_iter()
        .collect();
    if handlers.len() > 0 {
        for handler in &mut handlers {
            let http_path = &mut handler.http_path;
            if http_path.chars().nth(0) != Some('/') {
                if !http_path.is_empty() {
                    http_path.insert(0, '/');
                }
                http_path.insert_str(0, &contr_id_snake);
                http_path.insert(0, '/');
            }
        }
        Ok((controller, handlers))
    } else {
        Err(input.error("missing handler"))
    }
}

impl Parse for Handler {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        use syn::{braced, token, LitStr};
        let contr_method: Ident = input.parse()?;
        let mut http_method: Option<HttpMethod> = None;
        let mut path_str: Option<&str> = None;
        match contr_method.to_string().as_str() {
            "new_form" => {
                http_method = Some(HttpMethod::Get);
                path_str = Some("new");
            }
            "copy_form" => {
                http_method = Some(HttpMethod::Get);
                path_str = Some("<id>/copy");
            }
            "create" => {
                http_method = Some(HttpMethod::Post);
                path_str = Some("");
            }
            "ensure" => {
                http_method = Some(HttpMethod::Post);
                path_str = Some("ensure");
            }
            "index" => {
                http_method = Some(HttpMethod::Get);
                path_str = Some("");
            }
            "show" => {
                http_method = Some(HttpMethod::Get);
                path_str = Some("<id>");
            }
            "edit_form" => {
                http_method = Some(HttpMethod::Get);
                path_str = Some("<id>/edit");
            }
            "patch" => {
                http_method = Some(HttpMethod::Post);
                path_str = Some("<id>");
            }
            "replace" => {
                http_method = Some(HttpMethod::Post);
                path_str = Some("<id>/replace");
            }
            "delete" => {
                http_method = Some(HttpMethod::Post);
                path_str = Some("<id>/delete");
            }
            _ => (),
        }
        let mut path_string;
        if input.peek(token::Brace) {
            let content;
            braced!(content in input);
            match parse_entry::<Ident>(&content, "http_method")? {
                Some(val) => {
                    http_method = Some(
                        HttpMethod::try_from(val.to_string().as_str())
                            .map_err(|e| input.error(e.to_string()))?,
                    );
                }
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
        let (http_path, path_par_count) =
            normalize_http_path(&path_str.unwrap());
        Ok(Self {
            http_method,
            http_path,
            path_par_count,
            contr_method,
        })
    }
}

fn parse_entry<T>(input: ParseStream, nam: &str) -> syn::Result<Option<T>>
where
    T: Parse,
{
    use syn::token;
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

/// Normalize an HTTP path from e.g. `"/a/<`...`>/b/<`...`>/c"` to a pair <br>
/// `( String::from("/a/<p1>/b/<p2>/c"), 2 /* the number of params */ )`
///
fn normalize_http_path(http_path: &str) -> (String, usize) {
    use lazy_static::lazy_static;
    use regex::Regex;
    lazy_static! {
        static ref ANGLES: Regex = Regex::new(r"<[^>]*>").unwrap();
    }
    let mut result: (String, usize) = (String::new(), 0);
    let mut last = 0;
    for mat in ANGLES.find_iter(http_path) {
        result.0.extend(http_path[last..mat.start()].chars());
        result.1 += 1;
        result.0.extend(format!("<p{}>", result.1).chars());
        last = mat.end();
    }
    if last < http_path.len() {
        result.0.extend(http_path[last..http_path.len()].chars());
    }
    result
}
