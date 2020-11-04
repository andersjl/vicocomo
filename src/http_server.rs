//! Traits and structs implemented by an HTTP server and used by applications.
//!
use crate::Error;
use ::core::{
    convert::TryFrom,
    fmt::{self, Display, Formatter},
};
use ::serde::{de::DeserializeOwned, Serialize};
use ::std::{collections::HashMap, slice::Iter};
use ::syn::{
    braced, parenthesized,
    parse::{Parse, ParseStream},
    parse_quote, token, Expr, Ident, Lit, LitStr, Path, Type,
};
use ::vicocomo_derive_utils::*;

/// A custom syntax tree node for configuring an HTTP server.  Intended for
/// use in a server specific `config` macro.
///
/// There is an implementation for [`actix-web`
/// ](https://crates.io/crates/actix-web) [here.
/// ](../../vicocomo_actix/macro.config.html)
///
// TODO: a new route attribute 'name' for use in Request::url_for.
// TODO: implement not_found.
/// The `Parse::parse()` method expects tokens of the form
/// ```text
/// level_1(level_2) { level_3 { level_4: value, ... }, ... }, ...
/// ```
/// where `level_1`, `level_3`, and `level_4` are identifiers while `level_2`
/// is a Rust path.  `level_2` and its parentheses are optional as well as the
/// braced groups and braces if empty.  `level_4` may be present without any
/// `level_3`, the colon disambiguates this.
///
/// The combination of `level_1` and `level_2` should be globally unique.
/// `level_3` and `level_4` should be unique within their brace group.
/// "Unique" means that a later entry will replace an earlier.
///
/// At present, the `value` of a level 4 attribute may be
/// - an identifier, or
/// - a literal (`bool`, `char`, `f64`, `i64`, or `&'static str`), or
/// - `(`*a type*`, `*an expression evaluating to the type*`)`.
///
/// # Currently recognized arguments
///
/// ## Level 1 `plug_in`
///
/// Plug in an object implementing a `vicocomo` trait. Recognized level 2
/// identifiers (no double colons allowed here) are
/// - `DbConn`: The plug in implements [`DbConn`
///   ](../database/trait.DbConn.html). Optional, default [`NullConn`
///   ](../database/struct.NullConn.html).
/// - `SessionStore`: The plug in implements [`SessionStore`
///   ](trait.SessionStore.html). Optional, default [`::vicocomo::NullStore`
///   ](struct.NullStore.html). (Note the leading double colon!)
/// - `TemplEng`: The plug in mplements [`TemplEng`
///   ](trait.TemplEng.html). Optional, default [`NullEng`
///   ](struct.NullEng.html).
///
/// All require no level 3 and one typed level 4 arg `def`:
/// ```text
/// plug_in(SomeTrait) {
///     def: (
///         <a type implementing vicocomo::SomeTrait>,
///         <an expression evaluating to an instance of the type>,
///     ),
/// },
/// ```
///
/// ## Level 1 `app_config`
///
/// Various attributes configuring the generated application code. No level 2
/// or 3 identifiers. Globally recognized level 4 identifiers are
/// - `controller_prefix`: The value should be a string literal that will be
///   interpreded as a Rust Path that is the controllers module prefix, see
///   [`route`](#controller-path-and-handling-methods) below. Optional,
///   default `crate::controllers`.
///
/// An HTTP server specific `config` macro may use other `app_config`
/// attributes as needed.
///
/// ## Level 1 `route` and `not_found`
///
/// Route configuration. At least one route must obviously be defined.
/// Example follows.  See below for the meaning of "*Control*" in
/// `route(`*Control*`)`.
/// ```text
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
///     http_method: post, // above, get is the default HTTP method
///     path: "postpth",   // post | "/postpth"           | Othr | post_req
/// }},                    // =====+======================+======+==========
/// // Not Found handler   //      |                      |      |
/// not_found(Hand) {func} // all not handled elsewhere   | Hand | func
///                        // no default provided by parse()
/// ```
///
/// ### Controller path and handling method
///
/// The controller is given as `some::path::to::Controller`. If the path is a
/// single identifier, as in the examples, the controller prefix attribute
/// value (default `crate::controllers::`) is prepended.
///
/// The handling methods are called as
/// `some::path::to::Controller::handler(...)`. So the controller may be a
/// module, struct, or enum as long as the handling method does not have a
/// receiver.
///
/// Handling method signature:
/// ```text
/// (
///   &impl ::vicocomo::Request,       // server request
///   &impl ::vicocomo::TemplEng,      // template engine
///   &impl ::vicocomo::DbConn,        // database connection
///   ::vicocomo::Session,             // session object
///   &mut impl ::vicocomo::Response,  // response
/// ) -> ()
/// ```
///
#[derive(Clone, Debug)]
pub struct Config {
    /// The `Type` implements a trait `vicocomo::`*the `String`*.
    ///
    /// The `Expr` evaluates to the `Type`.
    ///
    pub plug_ins: HashMap<String, (Type, Expr)>,

    /// This will always contain the predefined entry
    /// <br>`"controller_prefix" => (::syn::Path, `*`Path`-valued
    /// expression*`)`. (note the leading double colon!)
    /// <br>An implementation is free to add HTTP server specific attributes.
    ///
    pub app_config: HashMap<String, (Type, Expr)>,

    /// The routing targets, mapping a controller to its route handlers.
    ///
    pub routes: HashMap<Path, Vec<Handler>>,

    /// Optional custom handler for failed routes.
    ///
    pub not_found: Option<(Path, Ident)>,
}

/// Information needed for implementing an HTTP server configuration macro
/// using [`Config`](struct.Config.html).
#[derive(Clone, Debug)]
pub struct Handler {
    /// only tested for Get and Post.
    pub http_method: HttpMethod,
    /// HTTP path, possibly with path parameters in angle brackets, normalized
    /// to `path/<p1>/with/<p2>/two/parameters`.
    pub http_path: String,
    /// number of path parameters.
    pub path_par_count: usize,
    /// controller method name.
    pub contr_method: Ident,
}

/// Methods for getting information about and from the request.
///
pub trait Request {
    /// The parameter values in the URI (get) or body (post) as a json string.
    /// The parameters may be structured à la PHP:
    /// ```text
    /// simple=1&arr[]=2&arr[]=3&map[a]=4&map[b]=5&deep[c][]=6&deep[c][]=7&deep[d]=8
    /// // -> {
    /// //     "simple": "1",
    /// //     "arr":    ["2", "3"],
    /// //     "map":    {"a": "4", "b": "5"},
    /// //     "deep:    {"c": ["6", "7"], "d": "8"}
    /// // }
    /// ```
    /// Note that all values are strings.
    ///
    fn json(&self) -> Result<String, Error> {
        FormData::parse(self.param_vals()).and_then(|fd| {
            serde_json::to_string(&fd)
                .map_err(|e| Error::invalid_input(&e.to_string()))
        })
    }

    /// The value of the parameter with `name` in the URI (get) or body (post)
    /// as a URL-decoded string.  For structured paramters, use [`json()`
    /// ](#method.json)
    ///
    fn param_val(&self, name: &str) -> Option<String>;

    /// All parameter values in the URI (get) or body (post) as a vector of
    /// URL-decoded key-value pairs.  Primarily intended for internal use by
    /// [`json()`](#method.json).
    ///
    /// ### Note to implementors
    ///
    /// For array parameters to work as described in [`json()`](#method.json)
    /// it is required that the same key kan occur more than once in the
    /// vector, if that is what is received in the request URI or body.
    ///
    fn param_vals(&self) -> Vec<(String, String)>;

    /// If registered as `"a/<p1>/<p2>"` and the HTTP path of the request is
    /// `"a/42/Hello"`, this will collect as e.g.
    /// `vec![String::from("42"), String::from("Hello")]`
    ///
    fn path_vals(&self) -> Iter<String>;

    /// The body of the request
    ///
    fn req_body(&self) -> String;

    /// The requested HTTP URI, including scheme, path, and query.
    /// W.I.P. more methods for scheme, path etc TBD.
    ///
    fn uri(&self) -> String;

    /// The URL, including scheme and host, without ending slash.
    ///
    /// 'path` should be as given to [`Config`](struct.Config.html).  if it
    /// has path parameters these should be in angle brackets.  The path
    /// parameter names are ignored and may be omitted:
    /// `path/<>/with/<>/two/parameters`.
    ///
    fn url_for(
        &self,
        path: &str,
        params: Option<&[&str]>,
    ) -> Result<String, Error> {
        use crate::texts::get_text;
        let (http_path, expected_count) = normalize_http_path(path);
        let param_count = match params {
            Some(p) => p.len(),
            None => 0,
        };
        if param_count == expected_count {
            self.url_for_impl(&http_path, params.unwrap_or(&[])).map(
                |mut u| {
                    if u.ends_with('/') {
                        u.pop();
                    }
                    u
                },
            )
        } else {
            Err(Error::invalid_input(&get_text(
                "error--parameter-count",
                &[
                    ("expected", &expected_count.to_string()),
                    ("actual", &param_count.to_string()),
                ],
            )))
        }
    }

    /// For web server adapter developers only.  Like [`url_for()`
    /// ](tymethod.url_for.html), but:
    ///
    /// - `path` parameter names are normalized to
    ///   `path/<p1>/with/<p2>/two/parameters`, and
    ///
    /// - the number of `params` is verified on entry.
    ///
    fn url_for_impl(
        &self,
        path: &str,
        params: &[&str],
    ) -> Result<String, Error>;
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

/// Methods to store a session.
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

/// An implementation of [`SessionStore`](trait.SessionStore.html) that does
/// nothing and returns `()`, `None`, or [`Error`](../error/enum.Error.html).
///
#[derive(Clone, Copy, Debug)]
pub struct NullStore;

impl SessionStore for NullStore {
    fn clear(&self) {
        ()
    }
    fn get(&self, _key: &str) -> Option<String> {
        None
    }
    fn remove(&self, _key: &str) {
        ()
    }
    fn set(&self, _key: &str, _value: &str) -> Result<(), Error> {
        Err(Error::other("no session store defined"))
    }
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

/// An implementation of [`TemplEng`](trait.TemplEng.html) that does nothing
/// and returns [`Error`](../error/enum.Error.html).
///
#[derive(Clone, Copy, Debug)]
pub struct NullEng;

impl TemplEng for NullEng {
    fn render(
        &self,
        _tmpl: &str,
        _data: &impl ::serde::Serialize,
    ) -> Result<String, Error> {
        Err(Error::render("no template engine"))
    }
}

/// A cookie session.
///
#[derive(Clone)]
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

// --- private --------------------------------------------------------------

impl Parse for Config {
    fn parse(stream: ParseStream) -> ::syn::Result<Self> {
        let mut plug_ins = HashMap::new();
        let mut app_config = HashMap::new();
        let mut parsed_routes = Vec::new();
        let mut not_found = None;
        for item in stream
            .parse_terminated::<ConfigItem, token::Comma>(ConfigItem::parse)?
        {
            match item.level_1 {
                ConfigItemId::AppConfig => {
                    item.get_app_conf(&mut app_config)?
                }
                ConfigItemId::NotFound => {
                    item.get_not_found(&mut not_found)?
                }
                ConfigItemId::PlugIn => item.get_plug_in(&mut plug_ins)?,
                ConfigItemId::Routes => {
                    item.get_routes(&mut parsed_routes)?
                }
            }
        }
        if !plug_ins.contains_key("DbConn") {
            plug_ins.insert(
                "DbConn".to_string(),
                (
                    parse_quote!(::vicocomo::NullConn),
                    parse_quote!(::vicocomo::NullConn),
                ),
            );
        }
        if !plug_ins.contains_key("SessionStore") {
            plug_ins.insert(
                "SessionStore".to_string(),
                (
                    parse_quote!(::vicocomo::NullStore),
                    parse_quote!(::vicocomo::NullStore),
                ),
            );
        }
        if !plug_ins.contains_key("TemplEng") {
            plug_ins.insert(
                "TemplEng".to_string(),
                (
                    parse_quote!(::vicocomo::NullEng),
                    parse_quote!(::vicocomo::NullEng),
                ),
            );
        }
        let contr_prefix = {
            let expr: Expr = app_config
                .get("controller_prefix")
                .and_then(|(t, e)| {
                    if t == &static_str_type() {
                        let s: LitStr = parse_quote!(#e);
                        match s.parse::<Path>() {
                            Ok(p) => Some(parse_quote!(#p)),
                            Err(_) => None,
                        }
                    } else {
                        None
                    }
                })
                .unwrap_or_else(|| parse_quote!(crate::controllers));
            app_config.insert(
                "controller_prefix".to_string(),
                (parse_quote!(Path), expr.clone()),
            );
            expr
        };
        let mut routes: HashMap<Path, Vec<Handler>> = HashMap::new();
        for (mut contr_path, handler) in parsed_routes.drain(..) {
            match contr_path.get_ident() {
                Some(id) => {
                    contr_path.segments = parse_quote!(#contr_prefix::#id);
                }
                None => (),
            }
            match routes.get_mut(&contr_path) {
                Some(hands) => hands.push(handler),
                None => {
                    routes.insert(contr_path, vec![handler]);
                }
            }
        }
        Ok(Self {
            plug_ins,
            app_config,
            routes,
            not_found,
        })
    }
}

#[derive(Clone, Debug)]
struct ConfigItem {
    level_1: ConfigItemId,
    level_2: Option<Path>,
    // level_3.first().unwrap().id == None => level_3.len() == 1.
    level_3: Vec<Level3>,
}

#[derive(Clone, Copy, Debug)]
enum ConfigItemId {
    AppConfig,
    NotFound,
    PlugIn,
    Routes,
}

impl ConfigItem {
    // expects level_1 to be AppConfig
    fn get_app_conf(
        &self,
        app_config: &mut HashMap<String, (Type, Expr)>,
    ) -> ::syn::Result<()> {
        match self.level_3.first() {
            Some(l3) => {
                for attr in &l3.attrs {
                    app_config.insert(
                        attr.id.to_string(),
                        (attr.ty.clone(), attr.expr.clone()),
                    );
                }
            }
            _ => (),
        }
        Ok(())
    }

    // expects level_1 to be NotFound
    fn get_not_found(
        &self,
        not_found: &mut Option<(Path, Ident)>,
    ) -> ::syn::Result<()> {
        let mut func = None;
        match self.level_3.first() {
            Some(l3) => func = l3.id.clone(),
            _ => (),
        }
        Ok(*not_found = Some((
            self.level_2.clone().ok_or_else(|| {
                syn_error("missing not_found controller Path")
            })?,
            func.ok_or_else(|| {
                syn_error("missing not_found function Ident")
            })?,
        )))
    }

    // expects level_1 to be PlugIn
    fn get_plug_in(
        &self,
        plug_ins: &mut HashMap<String, (Type, Expr)>,
    ) -> ::syn::Result<()> {
        let id = self
            .level_2
            .as_ref()
            .and_then(|path| path.get_ident().map(|id| id.to_string()))
            .ok_or_else(|| syn_error("missing plug in identifier"))?;
        let l3 = self
            .level_3
            .first()
            .ok_or_else(|| syn_error("missing plug in attributes"))?;
        plug_ins.insert(id, {
            let attr = l3
                .attrs
                .iter()
                .find(|ci| ci.id.to_string() == "def")
                .ok_or_else(|| {
                    syn_error("missing plug in attribute 'def'")
                })?;
            (attr.ty.clone(), attr.expr.clone())
        });
        Ok(())
    }

    // expects level_1 to be Routes
    fn get_routes(
        &self,
        routes: &mut Vec<(Path, Handler)>,
    ) -> ::syn::Result<()> {
        use ::case::CaseExt;
        let contr_path = self
            .level_2
            .clone()
            .ok_or_else(|| syn_error("missing route controller path"))?;
        let contr_id = &contr_path.segments.last().unwrap().ident.clone();
        let contr_id_snake = contr_id.to_string().to_snake();
        for l3 in &self.level_3 {
            let contr_method: Ident = l3
                .id
                .clone()
                .ok_or_else(|| syn_error("missing route handler function"))?;
            let mut http_method = HttpMethod::Get;
            let mut path_str: Option<&str> = None;
            match contr_method.to_string().as_str() {
                "new_form" => path_str = Some("new"),
                "copy_form" => path_str = Some("<id>/copy"),
                "create" => {
                    http_method = HttpMethod::Post;
                    path_str = Some("");
                }
                "ensure" => {
                    http_method = HttpMethod::Post;
                    path_str = Some("ensure");
                }
                "index" => path_str = Some(""),
                "show" => path_str = Some("<id>"),
                "edit_form" => path_str = Some("<id>/edit"),
                "patch" => {
                    http_method = HttpMethod::Post;
                    path_str = Some("<id>");
                }
                "replace" => {
                    http_method = HttpMethod::Post;
                    path_str = Some("<id>/replace");
                }
                "delete" => {
                    http_method = HttpMethod::Post;
                    path_str = Some("<id>/delete");
                }
                _ => (),
            }
            let mut path_string = path_str.map(|s| s.to_string());
            for attr in &l3.attrs {
                match attr.id.to_string().as_str() {
                    "http_method" => {
                        let mut error = Some(syn_error(&format!(
                            "{} is not an HTTP method",
                            tokens_to_string(&attr.expr),
                        )));
                        match &attr.expr {
                            Expr::Path(expr_path) => {
                                match expr_path.path.get_ident() {
                                    Some(i) => {
                                        match HttpMethod::try_from(
                                            i.to_string().as_str(),
                                        ) {
                                            Ok(meth) => {
                                                http_method = meth;
                                                error = None;
                                            }
                                            Err(e) => {
                                                error = Some(syn_error(
                                                    &e.to_string(),
                                                ))
                                            }
                                        }
                                    }
                                    None => (),
                                }
                            }
                            _ => (),
                        }
                        if error.is_some() {
                            return Err(error.unwrap());
                        }
                    }
                    "path" => {
                        let mut error = Some(syn_error(&format!(
                            "{} is not a valid path string",
                            tokens_to_string(&attr.expr),
                        )));
                        match &attr.expr {
                            Expr::Lit(lit) => match &lit.lit {
                                Lit::Str(ls) => {
                                    let mut s = ls.value();
                                    if 1 < s.len()
                                        && s.chars().last() == Some('/')
                                    {
                                        s.remove(s.len() - 1);
                                    }
                                    path_string = Some(s);
                                    error = None;
                                }
                                _ => (),
                            },
                            _ => (),
                        }
                        if error.is_some() {
                            return Err(error.unwrap());
                        }
                    }
                    _ => {
                        return Err(syn_error(&format!(
                            "unknown handler attribute {}",
                            tokens_to_string(&attr.id),
                        )));
                    }
                }
            }
            match path_string {
                Some(ref mut s) if s.chars().nth(0) != Some('/') => {
                    if !s.is_empty() {
                        s.insert(0, '/');
                    }
                    s.insert_str(0, &contr_id_snake);
                    s.insert(0, '/');
                }
                Some(_) => (),
                None => return Err(syn_error("missing path")),
            }
            let (http_path, path_par_count) =
                normalize_http_path(path_string.as_ref().unwrap());
            routes.push((
                contr_path.clone(),
                Handler {
                    http_method,
                    http_path,
                    path_par_count,
                    contr_method,
                },
            ));
        }
        Ok(())
    }
}

impl Parse for ConfigItem {
    fn parse(stream: ParseStream) -> ::syn::Result<Self> {
        let level_1: ConfigItemId = {
            let id = stream.parse::<Ident>()?.to_string();
            match id.as_str() {
                "app_config" => ConfigItemId::AppConfig,
                "not_found" => ConfigItemId::NotFound,
                "plug_in" => ConfigItemId::PlugIn,
                "route" => ConfigItemId::Routes,
                _ => {
                    return Err(syn_error(&format!(
                        "'{}' cannot start a Config item",
                        &id
                    )));
                }
            }
        };
        let level_2: Option<Path> = if stream.peek(token::Paren) {
            let content;
            parenthesized!(content in stream);
            if content.is_empty() {
                None
            } else {
                Some(content.parse()?)
            }
        } else {
            None
        };
        let mut level_3: Vec<Level3> = Vec::new();
        if stream.peek(token::Brace) && {
            let fork = stream.fork();
            let content;
            braced!(content in fork);
            content.peek(Ident) && content.peek2(token::Colon)
        } {
            // only lev 4
            level_3.push(Level3 {
                id: None,
                attrs: get_config_attrs(stream)?,
            });
        } else {
            let content;
            braced!(content in stream);
            level_3 = content
                .parse_terminated::<Level3, token::Comma>(Level3::parse)?
                .into_iter()
                .collect();
        }
        Ok(Self {
            level_1,
            level_2,
            level_3,
        })
    }
}

#[derive(Clone, Debug)]
struct Level3 {
    id: Option<Ident>,
    attrs: Vec<ConfigAttr>,
}

impl Parse for Level3 {
    fn parse(stream: ParseStream) -> ::syn::Result<Self> {
        Ok(Self {
            id: stream.parse()?,
            attrs: get_config_attrs(stream)?,
        })
    }
}

fn get_config_attrs(stream: ParseStream) -> ::syn::Result<Vec<ConfigAttr>> {
    Ok(if stream.peek(token::Brace) {
        let content;
        braced!(content in stream);
        content
            .parse_terminated::<ConfigAttr, token::Comma>(ConfigAttr::parse)?
            .into_iter()
            .collect()
    } else {
        Vec::new()
    })
}

#[derive(Clone, Debug)]
struct ConfigAttr {
    id: Ident,
    ty: Type,
    expr: Expr,
}

impl Parse for ConfigAttr {
    fn parse(stream: ParseStream) -> ::syn::Result<Self> {
        let id: Ident = stream.parse()?;
        stream.parse::<token::Colon>()?;
        let mut ty: Option<Type> = None;
        let expr: Option<Expr>;
        if stream.peek(token::Paren) {
            let content;
            parenthesized!(content in stream);
            ty = Some(content.parse()?);
            content.parse::<token::Comma>()?;
            expr = Some(content.parse()?);
            let _ = content.parse::<token::Comma>();
        } else {
            expr = Some(stream.parse()?);
            let mut error = Some(syn_error(&format!(
                "cannot handle attribute value '{}'",
                tokens_to_string(&expr.clone().unwrap()),
            )));
            match expr.clone().unwrap() {
                Expr::Lit(l) => {
                    error = None;
                    match l.lit {
                        Lit::Bool(_) => ty = Some(parse_quote!(bool)),
                        Lit::Char(_) => ty = Some(parse_quote!(char)),
                        Lit::Float(_) => ty = Some(parse_quote!(f64)),
                        Lit::Int(_) => ty = Some(parse_quote!(i64)),
                        Lit::Str(_) => ty = Some(static_str_type()),
                        _ => {
                            error = Some(syn_error(&format!(
                                "cannot handle literal '{}'",
                                tokens_to_string(&l.lit),
                            )));
                        }
                    }
                }
                Expr::Path(p) if p.path.get_ident().is_some() => {
                    ty = Some(parse_quote!(()));
                    error = None;
                }
                _ => (),
            }
            if error.is_some() {
                return Err(error.unwrap());
            }
        }
        Ok(Self {
            id,
            ty: ty.clone().unwrap(),
            expr: expr.clone().unwrap(),
        })
    }
}

fn syn_error(e: &str) -> ::syn::Error {
    ::syn::Error::new(::proc_macro2::Span::call_site(), e)
}

/// Normalize an HTTP path from e.g. `"/a/<`...`>/b/<`...`>/c"` to a pair <br>
/// `( String::from("/a/<p1>/b/<p2>/c"), 2 /* the number of params */ )`
///
fn normalize_http_path(http_path: &str) -> (String, usize) {
    ::lazy_static::lazy_static! {
        static ref ANGLES: ::regex::Regex =
            ::regex::Regex::new(r"<[^>]*>").unwrap();
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

#[derive(Clone, Debug)]
enum FormData {
    Arr(Vec<FormData>),
    Map(HashMap<String, FormData>),
    Leaf(String),
}

impl FormData {
    fn new() -> Self {
        Self::Map(HashMap::new())
    }

    fn branch(&mut self, nam: &str) -> Result<&mut Self, Error> {
        if let FormData::Map(ref mut map) = self {
            match map.get(nam) {
                Some(val) => match val {
                    Self::Map(_) => (),
                    _ => {
                        return Err(Error::invalid_input(&format!(
                            "get(\"{}\") is not a Map variant",
                            nam,
                        )));
                    }
                },
                None => {
                    map.insert(nam.to_string(), Self::new());
                }
            }
            Ok(self.get_mut(nam).unwrap())
        } else {
            Err(Error::invalid_input("self is not a Map variant"))
        }
    }

    /*
    fn get(&self, nam: &str) -> Option<&Self> {
        if let FormData::Map(map) = self {
            map.get(nam)
        } else {
            None
        }
    }
    */

    fn get_mut(&mut self, nam: &str) -> Option<&mut Self> {
        if let FormData::Map(map) = self {
            map.get_mut(nam)
        } else {
            None
        }
    }

    fn insert(&mut self, nam: &str, val: &FormData) -> Result<(), Error> {
        if let FormData::Map(ref mut map) = self {
            if map.insert(nam.to_string(), val.clone()).is_none() {
                Ok(())
            } else {
                Err(Error::invalid_input(&format!(
                    "\"{}\" is already set",
                    nam
                )))
            }
        } else {
            Err(Error::invalid_input("self is not a Map variant"))
        }
    }

    fn parse(vals: Vec<(String, String)>) -> Result<Self, Error> {
        ::lazy_static::lazy_static! {
            static ref BRACKETS: ::regex::Regex =
                ::regex::Regex::new(r"\[([^]]*)\]").unwrap();
        }
        let mut result = FormData::new();
        for (raw_key, raw_val) in vals {
            let val = Self::Leaf(raw_val);
            let mut nested = BRACKETS.captures_iter(&raw_key).peekable();
            let mut key = if nested.peek().is_none() {
                &raw_key
            } else {
                &raw_key[0..nested.peek().unwrap().get(0).unwrap().start()]
            };
            let mut branch = &mut result;
            loop {
                match nested.peek() {
                    Some(c) => {
                        let next_key = c.get(1).unwrap().as_str();
                        if next_key.len() == 0 {
                            branch.push(key, &val)?;
                            break;
                        }
                        branch = branch.branch(key)?;
                        key = next_key;
                    }
                    None => {
                        branch.insert(key, &val)?;
                        break;
                    }
                }
            }
        }
        Ok(result)
    }

    fn push(&mut self, nam: &str, val: &FormData) -> Result<(), Error> {
        match self {
            FormData::Map(ref mut map) => match map.get_mut(nam) {
                Some(old_val) => {
                    if let FormData::Arr(ref mut old_arr) = old_val {
                        old_arr.push(val.clone());
                        return Ok(());
                    }
                }
                None => {
                    map.insert(
                        nam.to_string(),
                        FormData::Arr(vec![val.clone()]),
                    );
                    return Ok(());
                }
            },
            _ => (),
        }
        Err(Error::invalid_input(&format!(
            "get(\"{}\") is not an Arr variant",
            nam
        )))
    }
}

impl Serialize for FormData {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::{SerializeMap, SerializeSeq};
        match self {
            Self::Arr(a) => {
                let mut seq = serializer.serialize_seq(Some(a.len()))?;
                for e in a {
                    seq.serialize_element(e)?;
                }
                seq.end()
            }
            Self::Map(m) => {
                let mut map = serializer.serialize_map(Some(m.len()))?;
                for (k, v) in m {
                    map.serialize_entry(k, v)?;
                }
                map.end()
            }
            Self::Leaf(l) => serializer.serialize_str(l),
        }
    }
}

fn static_str_type() -> Type {
    parse_quote!(&'static str)
}
