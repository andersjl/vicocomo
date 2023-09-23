//! Structs and traits used to implement an HTTP server adapter with a
//! `config` macro.

use super::{AppConfigVal, HttpStatus};
use crate::{map_error, DatabaseIf, DbType, Error};
use chrono::{Duration, Local, NaiveDateTime};
use core::{
    convert::TryFrom,
    fmt::{self, Display, Formatter},
};
use quote::format_ident;
use rand::{thread_rng, Rng};
use regex::Regex;
use serde_json::Value as JsonValue;
use std::cell::RefCell;
use std::cmp::Ordering;
use std::collections::HashMap;
use syn::{
    braced, bracketed, parenthesized,
    parse::{Parse, ParseStream},
    parse_quote, token, Expr, Ident, LitBool, LitChar, LitFloat, LitInt,
    LitStr, Path, Type,
};
use url::Url;
use vicocomo_derive_utils::*;

// --- HttpServer ------------------------------------------------------------

/// Everything Vicocomo needs from an HTTP server.
///
/// There is an example implementation for [`actix-web`
/// ](https://crates.io/crates/actix-web) [here
/// ](../../../vicocomo_actix/struct.AxServer.html).
///
pub trait HttpServer {
    /// See [`HttpServerIf::app_config()`
    /// ](../server/struct.HttpServerIf.html#method.app_config).
    ///
    fn app_config(&self, id: &str) -> Option<AppConfigVal>;

    /// See [`HttpServerIf::param_val()`
    /// ](../server/struct.HttpServerIf.html#method.param_val), but this one
    /// returns a `String`.
    ///
    fn param_val(&self, name: &str) -> Option<String>;

    /// All parameter values in the URL (get) or body (post) as a vector of
    /// URL-decoded key-value pairs.
    ///
    fn param_vals(&self) -> Vec<(String, String)>;

    /// prepend file_root if not starts with '/', strip mtime if strip_mtime
    fn prepend_file_root(&self, file_path: &str) -> String {
        lazy_static::lazy_static! {
            static ref MTIME: Regex =
                Regex::new(r"([^/]+)-\d{10}(\.[^/.]+)?$").unwrap();
        }
        let stripped =
            if self.app_config("strip_mtime").unwrap().bool().unwrap()
                && MTIME.is_match(file_path)
            {
                MTIME.replace(file_path, "$1$2")
            } else {
                file_path.into()
            };
        if stripped.starts_with('/') {
            stripped.to_string()
        } else {
            self.app_config("file_root").unwrap().str().unwrap() + &stripped
        }
    }

    /// prepend url_root if starts with '/'
    fn prepend_url_root(&self, url_path: &str) -> String {
        if url_path.starts_with('/') {
            self.app_config("url_root").unwrap().str().unwrap() + url_path
        } else {
            url_path.to_string()
        }
    }

    /// See [`HttpServerIf::req_path()`
    /// ](../server/struct.HttpServerIf.html#method.req_path), but this one
    /// <b>does not strip</b> the `url_root` [attribute](#level-1-app_config).
    ///
    fn req_path(&self) -> String;

    /// See [`HttpServerIf::req_route_par_val()`
    /// ](../server/struct.HttpServerIf.html#method.req_route_par_val), but
    /// this one returns a `String`.
    ///
    fn req_route_par_val(&self, par: &str) -> Option<String>;

    /// [`req_path()`](#tymethod.req_path) with [`url_root`
    /// ](../server/struct.HttpServerIf.html#url_root) removed.
    ///
    fn req_path_impl(&self) -> String {
        self.strip_url_root(&self.req_path())
    }

    /// See [`HttpServerIf::req_route_par_vals()`
    /// ](../server/struct.HttpServerIf.html#method.req_route_par_vals).
    ///
    fn req_route_par_vals(&self) -> Vec<(String, String)>;

    /// See [`HttpServerIf::req_body()`
    /// ](../server/struct.HttpServerIf.html#method.req_body).
    ///
    fn req_body(&self) -> String;

    /// See [`HttpServerIf::req_url()`
    /// ](../server/struct.HttpServerIf.html#method.req_url).
    ///
    fn req_url(&self) -> String;

    /// See [`HttpServerIf::resp_body()`
    /// ](../server/struct.HttpServerIf.html#method.resp_body).
    ///
    fn resp_body(&self, txt: &str);

    /// See [`HttpServerIf::resp_error()`
    /// ](../server/struct.HttpServerIf.html#method.resp_error).
    ///
    fn resp_error(&self, status: HttpStatus, err: Option<&Error>);

    /// Serve a static file, ignoring the body.
    ///
    /// `file_path` is the absolute path of the file if it starts with '`/`',
    /// or relative to the HTTP server's working directory if it does not.
    ///
    fn resp_file(&self, file_path: &str);

    /// See [`HttpServerIf::resp_file()`
    /// ](../server/struct.HttpServerIf.html#method.resp_file).
    ///
    fn resp_file_impl(&self, file_path: &str) {
        self.resp_file(&self.prepend_file_root(file_path))
    }

    /// See [`HttpServerIf::resp_ok()`
    /// ](../server/struct.HttpServerIf.html#method.resp_ok).
    ///
    fn resp_ok(&self);

    /// See [`HttpServerIf::resp_redirect()`
    /// ](../server/struct.HttpServerIf.html#method.resp_redirect).
    ///
    fn resp_redirect(&self, url: &str);

    /// See [`HttpServerIf::session_clear()`
    /// ](../server/struct.HttpServerIf.html#method.session_clear).
    ///
    fn session_clear(&self);

    /// See [`HttpServerIf::session_get()`
    /// ](../server/struct.HttpServerIf.html#method.session_get), but the JSON
    /// is not decoded.
    ///
    fn session_get(&self, key: &str) -> Option<String>;

    /// See [`HttpServerIf::session_remove()`
    /// ](../server/struct.HttpServerIf.html#method.session_remove).
    ///
    fn session_remove(&self, key: &str);

    /// See [`HttpServerIf::session_set()`
    /// ](../server/struct.HttpServerIf.html#method.session_set), but the
    /// value is already JSON-serialized on entry.
    ///
    fn session_set(&self, key: &str, value: &str) -> Result<(), Error>;

    /// Handle files routed by the `config` macro's [`route_static`
    /// ](../server/struct.HttpServerIf.html#level-1-route_static) entries. If
    /// the `app_config` attribute [`strip_mtime`
    /// ](../server/struct.HttpServerIf.html#strip_mtime) is `true`, this is
    /// needed to strip the mtime. If not, the use of this function is
    /// optional.
    ///
    /// On entry, [`req_path()`
    /// ](../server/struct.HttpServerIf.html#method.req_path) is required to
    /// have the form `"<url_path>/<file>"` where `<url_path>` is the first
    /// string in a [`route_static`
    /// ](../server/struct.HttpServerIf.html#level-1-route_static) entry in
    /// the `config` macro.
    ///
    /// First, tries to find  a file system directory by
    /// [`url_path_to_dir_impl()`](#method.url_path_to_dir_impl). If not
    /// found, [`resp_error()`](#tymethod.resp_error).
    ///
    /// Then, appends `file` to the directory and [`resp_file()`
    /// ](#tymethod.resp_file).
    ///
    fn static_file_handler(&self) {
        lazy_static::lazy_static! {
            static ref SPLIT: Regex = Regex::new(r"/").unwrap();
        }
        let (url_path, file) = {
            let orig_path = self.req_path_impl();
            let mut pieces: Vec<&str> = SPLIT.split(&orig_path).collect();
            let file = pieces.pop().unwrap();
            (pieces.join("/").to_string(), file.to_string())
        };
        self.url_path_to_dir_impl(&url_path)
            .map(|dir| self.resp_file_impl(&(dir + &file)))
            .unwrap_or_else(|| {
                self.resp_error(
                    HttpStatus::NotFound,
                    Some(&Error::this_cannot_happen("static-route-not-found")),
                );
            });
    }

    /// Strip file_root if at beginning
    fn strip_file_root(&self, file_path: &str) -> String {
        file_path
            .strip_prefix(
                &self.app_config("file_root").unwrap().str().unwrap(),
            )
            .map(|dir| dir.to_string())
            .unwrap_or_else(|| file_path.to_string())
    }

    /// Strip url_root if at beginning
    fn strip_url_root(&self, url_path: &str) -> String {
        url_path
            .strip_prefix(
                &self.app_config("url_root").unwrap().str().unwrap(),
            )
            .map(|url| url.to_string())
            .unwrap_or_else(|| url_path.to_string())
    }

    /// Map URL to file directory for serving static files as defined by the
    /// `config` macro's [`route_static`
    /// ](../server/struct.HttpServerIf.html#level-1-route_static) entries.
    ///
    /// `url_path` is the <b>absolute</b> URL, without [`url_root`
    /// ](../server/struct.HttpServerIf.html#url_root). A missing leading
    /// slash is inserted.
    ///
    /// The returned file system path is guaranteed to end with a slash. If it
    /// starts with a slash it is absolute, if not it is relative to
    /// [`file_root`](../server/struct.HttpServerIf.html#file_root).
    ///
    ///
    fn url_path_to_dir_impl(&self, url_path: &str) -> Option<String> {
        use crate::fix_slashes;
        let url_path = fix_slashes(url_path, 1, -1);
        self.url_path_to_dir(&self.prepend_url_root(&url_path))
            .as_ref()
            .map(|dir| fix_slashes(&self.strip_file_root(dir), 0, 1))
    }

    /// See [`url_path_to_dir_impl()`](#tymethod.url_path_to_dir_impl), but
    /// - `url_path` includes [`url_root`
    ///   ](../server/struct.HttpServerIf.html#url_root) and is guaranteed not
    ///   to end with a slash, and
    /// - the returned string includes [`file_root`
    ///   ](../server/struct.HttpServerIf.html#file_root).
    ///
    fn url_path_to_dir(&self, url_path: &str) -> Option<String>;
}

// --- TemplEng --------------------------------------------------------------

/// Methods to render via a template engine.
///
pub trait TemplEng: Send + Sync {
    /// Override this if you override [`register_templ_dir`
    /// ](#method.register_templ_dir) to return `false` until the templates
    /// directory has been registered.
    ///
    /// The provided method always returns `true`.
    ///
    #[allow(unused_variables)]
    fn initialized(&self) -> bool {
        true
    }

    /// Override if your implementation uses a template directory the path of
    /// which is not available at initialization.
    ///
    /// `ext` is the template file extension, only files with that extension
    /// should be registered.
    ///
    /// `path` is the path to a directory with template files.
    ///
    /// <b>Errors</b>
    ///
    /// The provided method always returns an error.
    ///
    #[allow(unused_variables)]
    fn register_templ_dir(&self, path: &str, ext: &str) -> Result<(), Error> {
        Err(Error::nyi())
    }

    /// Override to render.
    ///
    /// `data` is data for the template `tmpl` as `serde_json::Value`.
    ///
    fn render(&self, tmpl: &str, json: &JsonValue) -> Result<String, Error>;
}

// --- Authorized ------------------------------------------------------------

/// Used by a web server adapter to implement [role based access control
/// ](../server/struct.HttpServerIf.html#role-based-access-control), utilizing
/// the role `enum`'s [`is_authenticated()`
/// ](../../authorization/trait.UserRole.html#method.is_authenticated) method.
///
/// A user that has the (optional) role `Disabled` shall be allowed access
/// only if `Disabled` is present in `allow`.
///
/// A user that does not have the role `Disabled` shall be allowed access if
/// any of its roles is present in `allow`.
///
/// Whether `Disabled` or not, if `filter` is `true` access shall be allowed
/// only after [filtering](#filtering-access-control).
///
/// The role `Superuser` is guaranteed always to be present in `allow`.
///
#[derive(Clone, Debug)]
pub struct Authorized {
    /// Each `Expr` evaluates to a [role `enum`
    /// ](../server/struct.HttpServerIf.html#role-based-access-control).
    pub allow: Vec<Expr>,

    /// Allow access to the `allow` roles only after [filtering
    /// ](../server/struct.HttpServerIf.html#filtering-access-control).
    pub filter: bool,
}

// --- Config ----------------------------------------------------------------

/// A syntax tree node for configuring an HTTP server. Intended for use in a
/// server specific `config` macro.
///
/// It is the adapter developer's responsibility to ensure that the macro
/// meets the requirements in the [`HttpServerIf` documentation!
/// ](../server/struct.HttpServerIf.html) There is an implementation for
/// [`actix-web`](https://crates.io/crates/actix-web) [here
/// ](../../../vicocomo_actix/macro.config.html).
///
// TODO: named routes and url_for().
//
#[derive(Clone, Debug)]
pub struct Config {
    /// Parsed `app_config` attributes.  This will always contain the
    /// predefined entries as documented for [`HttpServerIf::app_config()`
    /// ](../server/struct.HttpServerIf.html#method.app_config).
    ///
    pub app_config: HashMap<String, ConfigAttrVal>,

    /// Optional custom handler for failed routes.
    ///
    pub not_found: Option<(Path, Ident)>,

    /// The `Type` implements a trait.
    ///
    /// The `Expr` evaluates to the `Type`.
    ///
    pub plug_ins: HashMap<String, (Type, Expr)>,

    /// All configured handlers.
    ///
    pub handlers: Vec<HttpHandler>,

    /// The first `String` in each pair is the part of the URL path preceding
    /// the file name with leading but no trailing slash. The `app_config`
    /// attribute [`url_root`](../server/struct.HttpServerIf.html#url_root) is
    /// already added by the parser.
    ///
    /// <small>Note that the complete URL, including the file name, may
    /// possibly coincide with an URL in a `HttpHandler` in `routes`. In such
    /// cases the HTTP server is expected to call the handler and ignore the
    /// file. </small>
    ///
    /// The second `String` is the directory that the file is served from. If
    /// it begins with a slash it is an absolute file path. If not, it is
    /// relative to the HTTP server's working directory. No trailing slash.
    ///
    /// The `app_config` attribute [`file_root`
    /// ](../server/struct.HttpServerIf.html#file_root) is already prepended
    /// by [`parse()`](#method.parse).
    ///
    pub static_routes: Vec<(String, String)>,
}

macro_rules! prepend_root {
    // prepend file_root if not starts with '/'
    (file $root:expr, $path:expr) => {
        if ($path).starts_with('/') {
            ($path).to_string()
        } else {
            ($root).to_string() + ($path)
        }
    };
    // prepend url_root if starts with '/'
    (url $root:expr, $path:expr) => {
        if ($path).starts_with('/') {
            ($root).to_string() + ($path)
        } else {
            ($path).to_string()
        }
    };
}

impl Parse for Config {
    /// In addition to simple parsing, this function also implements rules as
    /// documented for [`HttpServerIf`
    /// ](../server/struct.HttpServerIf.html#config-macro-input-syntax).
    ///
    fn parse(stream: ParseStream) -> syn::Result<Self> {
        let mut app_config = HashMap::new();
        let mut authorizers = Authorizers::new();
        let mut not_found = None;
        let mut plug_ins = HashMap::new();
        let mut parsed_handlers = Vec::new();
        let mut static_routes = Vec::new();
        for item in stream
            .parse_terminated::<ConfigItem, token::Comma>(ConfigItem::parse)?
        {
            match item.level_1 {
                ConfigItemId::AppConfig => {
                    item.get_app_conf(&mut app_config)?
                }
                ConfigItemId::Authorize => {
                    item.get_authorizer(&mut authorizers)?
                }
                ConfigItemId::NotFound => {
                    item.get_not_found(&mut not_found)?
                }
                ConfigItemId::PlugIn => item.get_plug_in(&mut plug_ins)?,
                ConfigItemId::Routes => {
                    item.get_handlers(&mut parsed_handlers)?
                }
                ConfigItemId::StaticRoutes => {
                    item.get_static_routes(&mut static_routes)?
                }
            }
        }

        // - - app_config defaults and business rules  - - - - - - - - - - - -

        let contr_prefix: Path = match app_config.get("controller_prefix") {
            Some(val) => val.get_path()?,
            None => {
                let path: Path = parse_quote!(crate::controllers);
                app_config.insert(
                    "controller_prefix".to_string(),
                    ConfigAttrVal::Path(path.clone()),
                );
                path
            }
        };
        if let Some(val) = app_config.get("create_session_table") {
            if val.get_bool().unwrap_or(false) {
                app_config.insert(
                    "create_session_table".to_string(),
                    ConfigAttrVal::Str(LitStr::new(
                        "CREATE TABLE __vicocomo__sessions(\
                            id BIGINT, data TEXT, time BIGINT\
                        )",
                        proc_macro2::Span::call_site(),
                    )),
                );
            }
        }
        let file_root = fix_root(&mut app_config, "file_root", 0, 1);
        let role_type: Option<Type> = {
            let mut default = true;
            let mut role_type: Option<Type> = None;
            if let Some(val) = app_config.get_mut("role_enum") {
                if let Ok(p) = val.get_path() {
                    default = false;
                    role_type = Some(parse_quote!(#p));
                } else {
                    if let Ok(b) = val.get_bool() {
                        if b {
                            *val = ConfigAttrVal::Path(parse_quote!(
                                crate::models::UserRole
                            ));
                        } else {
                            default = false;
                        }
                    }
                }
            } else if app_config.contains_key("role_variants") {
                app_config.insert(
                    "role_enum".to_string(),
                    ConfigAttrVal::Path(parse_quote!(
                        crate::models::UserRole
                    )),
                );
            } else {
                app_config.insert(
                    "role_enum".to_string(),
                    ConfigAttrVal::Bool(parse_quote!(false)),
                );
                default = false;
            }
            if default {
                role_type = Some(parse_quote!(crate::models::UserRole));
            }
            role_type
        };
        if role_type.is_some() {
            let mut roles = match app_config.get("role_variants") {
                Some(v) => v.get_id_strings()?,
                None => Vec::new(),
            };
            for predefined in &["Superuser"] {
                let predef = predefined.to_string();
                if !roles.contains(&predef) {
                    roles.push(predef);
                }
            }
            authorizers.sanitize(roles.as_slice())?;
            match app_config.get("unauthorized_route") {
                Some(r) => {
                    r.get_string()?;
                }
                None => {
                    app_config.insert(
                        "unauthorized_route".to_string(),
                        ConfigAttrVal::Str(parse_quote!("/")),
                    );
                }
            }
        }
        app_config
            .get("strip_mtime")
            .map(|val| val.get_bool().map(|_| ()))
            .unwrap_or_else(|| {
                Ok({
                    app_config
                        .insert("strip_mtime".to_string(), false.into());
                })
            })?;
        let url_root = fix_root(&mut app_config, "url_root", 1, -1);

        // - - plugin defaults - - - - - - - - - - - - - - - - - - - - - - - -

        if !plug_ins.contains_key("DbConn") {
            plug_ins.insert(
                "DbConn".to_string(),
                (
                    parse_quote!(vicocomo::NullConn),
                    parse_quote!(vicocomo::NullConn),
                ),
            );
        }
        if !plug_ins.contains_key("TemplEng") {
            plug_ins.insert(
                "TemplEng".to_string(),
                (
                    parse_quote!(vicocomo::NullTemplEng),
                    parse_quote!(vicocomo::NullTemplEng),
                ),
            );
        }

        // - - handler defaults and business rules - - - - - - - - - - - - - -

        let mut handlers = Vec::new();
        for mut handler in parsed_handlers.drain(..) {
            match handler.contr_path.get_ident() {
                Some(id) => {
                    handler.contr_path.segments =
                        parse_quote!(#contr_prefix::#id);
                }
                None => (),
            }
            // must do authorize before prepending url_root
            if role_type.is_some() {
                handler.authorize(
                    authorizers.get(handler.http_method)?,
                    role_type.as_ref().unwrap(),
                )?;
            }
            handler.route =
                prepend_root!(url url_root.as_str(), &handler.route);
            handler.pattern = format!(
                "{}__{}",
                handler.http_method,
                route_to_pattern(&handler.route),
            );
            handlers.push(handler);
        }

        // - - static route business rules - - - - - - - - - - - - - - - - - -

        for (url, file) in static_routes.iter_mut() {
            *url = prepend_root!(url url_root.as_str(), url.as_str());
            *file = prepend_root!(file file_root.as_str(), file.as_str());
        }

        Ok(Self {
            app_config,
            not_found,
            plug_ins,
            handlers,
            static_routes,
        })
    }
}

// --- ConfigAttrVal ---------------------------------------------------------

/// The possible values of a level 3 configuration attribute.
///
#[derive(Clone, Debug)]
pub enum ConfigAttrVal {
    /// The elements of the array are guaranteed to be the same variant.
    Arr(Vec<ConfigAttrVal>),
    Bool(LitBool),
    Char(LitChar),
    Float(LitFloat),
    /// The contained `Ident` is guaranteed not to be `false` or `true`.
    Ident(Ident),
    Int(LitInt),
    /// The contained `Path` is guaranteed not to be a single `Ident`.
    Path(Path),
    Str(LitStr),
    /// The expression should evaluate to an instance of the type.
    TypeExpr(Type, Expr),
}

macro_rules! literal_extractor {
    ($self: ident, $variant: ident $( , )? ) => {
        if let ConfigAttrVal::$variant(v) = $self {
            Ok(v.clone())
        } else {
            Err($self.error("Literal error"))
        }
    };
}

impl ConfigAttrVal {
    fn error(&self, msg: &str) -> syn::Error {
        syn_error(&format!("{}: {}", msg, &self))
    }

    /// Return the contained array or an error. The elements of the array are
    /// guaranteed to be the same variant.
    ///
    pub fn get_array(&self) -> syn::Result<Vec<ConfigAttrVal>> {
        match self {
            ConfigAttrVal::Arr(a) => Ok(a.clone()),
            _ => Err(self.error("Not an array")),
        }
    }

    /// - A clone of the contained Arr of Ident as String, or
    /// - the contained Ident as a vector with one String, or
    /// - an error.
    ///
    pub fn get_id_strings(&self) -> syn::Result<Vec<String>> {
        match self {
            ConfigAttrVal::Arr(a) => {
                let mut result = Vec::new();
                for elem in a {
                    if let Self::Ident(id) = elem {
                        result.push(id.to_string());
                    } else {
                        return Err(self.error("Not an Ident array"));
                    }
                }
                Ok(result)
            }
            ConfigAttrVal::Ident(id) => Ok(vec![id.to_string()]),
            _ => Err(self.error("Not an Ident (array)")),
        }
    }

    /// Return the contained boolean or an error.
    ///
    pub fn get_bool(&self) -> syn::Result<bool> {
        literal_extractor!(self, Bool).map(|b| b.value())
    }

    /// Return the contained character or an error.
    ///
    pub fn get_char(&self) -> syn::Result<char> {
        literal_extractor!(self, Char).map(|c| c.value())
    }

    /// Return the contained float or an error.
    ///
    pub fn get_f64(&self) -> syn::Result<f64> {
        literal_extractor!(self, Float).and_then(|f| f.base10_parse().into())
    }

    /// Return the contained integer or an error.
    ///
    pub fn get_i64(&self) -> syn::Result<i64> {
        literal_extractor!(self, Int).and_then(|i| i.base10_parse().into())
    }

    /// Return the contained identifier or an error.
    ///
    pub fn get_ident(&self) -> syn::Result<Ident> {
        match self {
            ConfigAttrVal::Ident(i) => Ok(i.clone()),
            _ => Err(self.error("Not an Ident")),
        }
    }

    /// Return the contained `LitStr` or an error.
    ///
    pub fn get_litstr(&self) -> syn::Result<LitStr> {
        literal_extractor!(self, Str)
    }

    /// Return the contained Rust path, or identifier as a path, or an error.
    ///
    pub fn get_path(&self) -> syn::Result<Path> {
        match self {
            ConfigAttrVal::Ident(i) => Ok(parse_quote!(#i)),
            ConfigAttrVal::Path(p) => Ok(p.clone()),
            _ => Err(self.error("Not a Path")),
        }
    }

    /// Return the contained string or an error.
    ///
    pub fn get_string(&self) -> syn::Result<String> {
        literal_extractor!(self, Str).map(|s| s.value())
    }

    // get_string() and then fix_slashes()
    fn get_fix_slashes(&self, lead: i32, trail: i32) -> syn::Result<String> {
        Ok(crate::fix_slashes(&self.get_string()?, lead, trail))
    }

    /// Return the contained (Type, Expr) pair or an error.
    ///
    pub fn get_type_expr(&self) -> syn::Result<(Type, Expr)> {
        match self {
            ConfigAttrVal::TypeExpr(t, e) => Ok((t.clone(), e.clone())),
            _ => Err(syn_error(&format!("{} is not a (Type, Expr)", &self))),
        }
    }

    /// Use in the HTTP server adapter's `config` macro to initialize the
    /// container behind the implementation of [`HttpServer::app_config()`
    /// ](trait.HttpServer.html#tymethod.app_config).
    ///
    /// Convert to an `Expr` that evaluates to an [`AppConfigVal`
    /// ](../server/enum.AppConfigVal.html).
    ///
    /// Guaranteed to return Some(_) for all variants except for `TypeExpr`
    /// returning `None`.
    ///
    pub fn to_app_config_val_expr(&self) -> Option<Expr> {
        use proc_macro2::Span;
        use syn::punctuated::Punctuated;
        match self {
            Self::Arr(vals) => {
                let mut elems: Punctuated<Expr, syn::token::Comma> =
                    Punctuated::new();
                for v in vals {
                    if let Some(elem) = v.to_app_config_val_expr() {
                        elems.push(elem);
                    } else {
                        return None;
                    }
                }
                Some(parse_quote!(vicocomo::AppConfigVal::Arr(vec![#elems])))
            }
            Self::Bool(val) => {
                Some(parse_quote!(vicocomo::AppConfigVal::Bool(#val)))
            }
            Self::Char(val) => {
                Some(parse_quote!(vicocomo::AppConfigVal::Char(#val)))
            }
            Self::Float(val) => {
                Some(parse_quote!(vicocomo::AppConfigVal::Float(#val)))
            }
            Self::Ident(val) => {
                let id_str = LitStr::new(&val.to_string(), Span::call_site());
                Some(parse_quote!(vicocomo::AppConfigVal::Ident(
                    #id_str.to_string()
                )))
            }
            Self::Int(val) => {
                Some(parse_quote!(vicocomo::AppConfigVal::Int(#val)))
            }
            Self::Path(val) => {
                let p_str = LitStr::new(
                    &vicocomo_derive_utils::tokens_to_string(&val),
                    Span::call_site(),
                );
                Some(parse_quote!(vicocomo::AppConfigVal::Path(
                    #p_str.to_string()
                )))
            }
            Self::Str(val) => Some(
                parse_quote!(vicocomo::AppConfigVal::Str(#val.to_string())),
            ),
            Self::TypeExpr(..) => None,
        }
    }
}

impl Display for ConfigAttrVal {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        use itertools::Itertools;
        match self {
            Self::Arr(v) => {
                write!(f, "[{}]", v.iter().map(|i| i.to_string()).join(", "))
            }
            Self::Bool(b) => write!(f, "{}", b.value().to_string()),
            Self::Char(c) => write!(f, "{}", c.value().to_string()),
            Self::Float(e) => {
                write!(f, "{}", e.base10_parse::<f64>().unwrap().to_string())
            }
            Self::Ident(i) => write!(f, "{}", i.to_string()),
            Self::Int(i) => {
                write!(f, "{}", i.base10_parse::<i64>().unwrap().to_string())
            }
            Self::Path(p) => write!(f, "{}", tokens_to_string(p)),
            Self::Str(s) => write!(f, "{}", s.value()),
            Self::TypeExpr(t, e) => write!(
                f,
                "({}, {})",
                tokens_to_string(t),
                tokens_to_string(e),
            ),
        }
    }
}

macro_rules! cav_from_value {
    ($typ:ty, $variant:ident, $lit:ident) => {
        impl From<$typ> for ConfigAttrVal {
            fn from(val: $typ) -> Self {
                Self::$variant(syn::$lit::new(
                    val,
                    proc_macro2::Span::call_site(),
                ))
            }
        }
    };
}

impl<T: Into<ConfigAttrVal>> From<Vec<T>> for ConfigAttrVal {
    fn from(arr: Vec<T>) -> Self {
        Self::Arr(arr.into_iter().map(|val| val.into()).collect())
    }
}

cav_from_value! { bool, Bool, LitBool }
cav_from_value! { char, Char, LitChar }

impl From<&str> for ConfigAttrVal {
    fn from(s: &str) -> Self {
        Self::Str(LitStr::new(s, proc_macro2::Span::call_site()))
    }
}

// --- HttpHandler -----------------------------------------------------------

/// Information needed for implementing an HTTP server configuration macro
/// using [`Config`](struct.Config.html).
///
#[derive(Clone, Debug)]
pub struct HttpHandler {
    /// If `Some`, defines access control for `route`, see [`Authorized`
    /// ](struct.Authorized.html). If `None`, there is no access control.
    pub authorized: Option<Authorized>,

    /// controller method name.
    pub contr_method: Ident,

    /// The full rust path to the controller, e.g. `path::to::controller`.
    pub contr_path: Path,

    /// The full path to `contr_method` as a `String`, e.g.
    /// `"path::to::controller::method"`.
    pub call_string: String,

    /// only tested for Get and Post.
    ///
    pub http_method: HttpMethod,

    /// Route, possibly with path parameters in angle brackets. The
    /// `app_config` attribute [`url_root`
    /// ](../server/struct.HttpServerIf.html#url_root) is prepended if
    /// defined. Use [`HttpServer::strip_url_root()`
    /// ](trait.HttpServer.html#tymethod.strip_url_root) to get the relative
    /// URL.
    ///
    pub route: String,

    /// A Regex pattern to match `http_method` and `route` and capture the
    /// route parameter values.
    ///
    /// Example: if `http_method` is `Get` and `route` is `/foo/<bar>/baz`,
    /// the `pattern` will be `"get__/foo/([^/]+)/baz"`.
    ///
    pub pattern: String,

    /// Route parameter names.
    ///
    pub route_par_names: Vec<LitStr>,
}

impl HttpHandler {
    fn authorize(
        &mut self,
        authorizers: &[Authorizer],
        role_type: &Type,
    ) -> syn::Result<()> {
        for auth in authorizers {
            if auth.pattern.is_match(&self.route) {
                if auth.roles == vec!["Public".to_string()] {
                    return Ok(());
                }
                let mut allow: Vec<Expr> = auth
                    .roles
                    .iter()
                    .map(|s| {
                        let variant = format_ident!("{}", s);
                        parse_quote!(#role_type::#variant)
                    })
                    .collect();
                if !auth.roles.contains(&"Superuser".to_string()) {
                    allow.push(parse_quote!(#role_type::Superuser));
                }
                self.authorized = Some(Authorized {
                    allow,
                    filter: auth.filter,
                });
                return Ok(());
            }
        }
        Err(syn_error(&format!(
            "no authorization for route {}",
            self.route
        )))
    }
}

// --- HttpDbSession ---------------------------------------------------------

/// Intended for implementing an HTTP session that stores all data in a
/// database table `"__vicocomo__sessions"`.  The table has three columns,
/// `id` storing a 64 bit integer primary key, `data` storing the serialized
/// session data as an unlimited UTF-8 text, and `time` storing the last
/// access time as a 64 bit integer.
///
pub struct HttpDbSession {
    db: DatabaseIf,
    id: i64,
    cache: HashMap<String, String>,
}

impl HttpDbSession {
    /// Try to create.
    ///
    /// `db` is remembered for use by other methods.
    ///
    /// `id` is a key to the database row for this session, typically
    /// retrieved from a cookie session. If given, we try to retrieve session
    /// data from the database to a cache in the returned object.
    ///
    /// `prune`, if positive, removes all session data older than that many
    /// seconds from the database, possibly including the one with `id`.
    ///
    /// `create_sql`, if `Some(_)`, is an SQL string used on error to try to
    /// create a table for storing session data in the database. E.g., for
    /// Postgres the following should work:
    /// `CREATE TABLE __vicocomo__sessions(id BIGINT, data TEXT, time BIGINT)`.
    ///
    /// On success, the returned object always has a valid [`id`](#method.id)
    /// corresponding to a session stored in the database. If `id` was `Some`
    /// it is never changed. If it was `None` a random one is generated and an
    /// empty session is stored. In that case, the caller is responsible for
    /// persisting the new `id`, e.g. in a cookie.
    ///
    /// Returns `Error::Other("cannot-create-db-session")` on failure,
    /// translated with one [`parameter`](../../texts/index.html) `error`, the
    /// error reported from the database.
    ///
    pub fn new(
        db: DatabaseIf,
        id: Option<i64>,
        prune: i64,
        create_sql: Option<&str>,
    ) -> Result<Self, Error> {
        use crate::t;

        if prune > 0 {
            let count = db
                .clone()
                .query_column(DB_SESSION_ROW_COUNT, &[], DbType::Int)
                .and_then(|count| i64::try_from(count.clone()).ok())
                .unwrap_or(0);
            // The frequency calling this function is ~ the number of users ~
            // the number of rows in the database. So, to keep the pruning
            // frequency independent of the number of users:
            if count > 0 && thread_rng().gen_range(0..count) == 0 {
                let _ = db.clone().exec(
                    DB_SESSION_PRUNE,
                    &[(Self::now() - Duration::seconds(prune)).into()],
                );
            }
        }
        let mut cache: Option<HashMap<String, String>> = None;
        let id = id
            .map(|old_id| {
                cache = db
                    .clone()
                    .query_column(
                        DB_SESSION_SELECT,
                        &[old_id.into()],
                        DbType::Text,
                    )
                    .and_then(|data| String::try_from(data.clone()).ok())
                    .and_then(|map_str| serde_json::from_str(&map_str).ok());
                old_id
            })
            .unwrap_or_else(|| thread_rng().gen());
        if cache.is_some() {
            let _ = db
                .clone()
                .exec(DB_SESSION_TOUCH, &[id.into(), Self::now().into()]);
        } else {
            let mut tried_create = false;
            loop {
                match db.clone().exec(
                    DB_SESSION_INSERT,
                    &[id.into(), "{}".to_string().into(), Self::now().into()],
                ) {
                    Ok(count) => {
                        if count == 1 {
                            cache = Some(HashMap::new());
                            break;
                        } else {
                            return Err(Error::this_cannot_happen(""));
                        }
                    }
                    Err(e) if create_sql.is_none() || tried_create => {
                        return Err(Error::other(&t!(
                            "cannot-create-db-session",
                            "error": &e.to_string(),
                        )));
                    }
                    _ => {
                        let _ = db.clone().exec(create_sql.unwrap(), &[]);
                        tried_create = true;
                        continue;
                    }
                }
            }
        }
        Ok(Self { db, id, cache: cache.unwrap() })
    }

    /// Clear session data with [`id`](#method.id) from the database and the
    /// cached data.
    ///
    pub fn clear(&mut self) {
        self.cache = HashMap::new();
        let _ = self.update();
    }

    /// Get the current `id` of the session in the database.
    ///
    pub fn id(&self) -> i64 {
        self.id
    }

    /// Get value from cache.
    ///
    pub fn get(&self, key: &str) -> Option<String> {
        self.cache.get(key).map(|s| s.to_string())
    }

    /// Remove value from cache and database.
    ///
    pub fn remove(&mut self, key: &str) {
        self.cache.remove(key);
        let _ = self.update();
    }

    /// Insert `value` in cache.
    ///
    pub fn set(&mut self, key: &str, value: &str) -> Result<(), Error> {
        self.cache.insert(key.to_string(), value.to_string());
        self.update()
    }

    fn now() -> NaiveDateTime {
        Local::now().naive_utc()
    }

    fn update(&self) -> Result<(), Error> {
        self.db
            .clone()
            .exec(
                DB_SESSION_UPDATE,
                &[
                    self.id.into(),
                    map_error!(Other, serde_json::to_string(&self.cache))?
                        .into(),
                    Self::now().into(),
                ],
            )
            .and_then(|count| {
                if count == 1 {
                    Ok(())
                } else {
                    Err(Error::other("actix-db-session--cannot-update"))
                }
            })
    }
}

// --- HttpMethod ------------------------------------------------------------

/// A simple enum with the official methods.
///
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Hash)]
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

// --- HttpParamVals ---------------------------------------------------------

/// Facilitates implementing [`HttpServer`](trait.HttpServer.html), see e.g.
/// [`vicocomo_actix`](../../../vicocomo_actix/index.html).
///
#[derive(Clone, Debug)]
pub struct HttpParamVals(HashMap<String, Vec<String>>);

impl HttpParamVals {
    pub fn new() -> Self {
        Self(HashMap::new())
    }

    /// Empty string(s) is OK
    pub fn from_request(req_body: &str, req_query: &str) -> Self {
        let mut result = Self::new();
        result.set_request(req_body, req_query);
        result
    }

    pub fn get(&self, name: &str) -> Option<&Vec<String>> {
        self.0.get(name)
    }

    /// Empty string(s) is OK
    pub fn set_request(&mut self, req_body: &str, req_query: &str) {
        use lazy_static::lazy_static;

        lazy_static! {
            static ref QUERY: Regex =
                Regex::new(r"([^&=]+=[^&=]+&)*[^&=]+=[^&=]+").unwrap();
        }

        let mut param_vals: HashMap<String, Vec<String>> = HashMap::new();
        let body_vals = QUERY
            .captures(&req_body)
            .and_then(|c| c.get(0))
            .and_then(|m| HttpRequest::decode_url_parameter(m.as_str()).ok());
        let uri_vals = HttpRequest::decode_url_parameter(req_query).ok();
        for key_value in match uri_vals {
            Some(u) => match body_vals {
                Some(b) => u + "&" + b.as_ref(),
                None => u,
            },
            None => body_vals.unwrap_or_else(|| String::new()),
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
        self.0 = param_vals;
    }

    pub fn vals(&self) -> Vec<(String, String)> {
        let mut result: Vec<(String, String)> = Vec::new();
        for (key, vals) in &self.0 {
            for val in vals {
                result.push((key.clone(), val.clone()));
            }
        }
        result
    }
}

// --- HttpRequest -----------------------------------------------------------

/// Helper functions for handling an HTTP request.
///
pub struct HttpRequest;

impl HttpRequest {
    /// Change `"+"` to `"%20"`, then [`urlencoding::decode()`
    /// ](https://docs.rs/urlencoding/latest/urlencoding/fn.decode.html).
    ///
    pub fn decode_url_parameter(par: &str) -> Result<String, Error> {
        lazy_static::lazy_static! {
            static ref PLUS: Regex = Regex::new(r"\+").unwrap();
        }
        urlencoding::decode(&PLUS.replace_all(par, "%20"))
            .map(|s| s.to_string())
            .map_err(|e| Error::invalid_input(&e.to_string()))
    }
}

// --- HttpResponse ----------------------------------------------------------

/// Facilitates implementing [`HttpServer`](trait.HttpServer.html), see e.g.
/// [`vicocomo_actix`](../../../vicocomo_actix/index.html).
///
#[derive(Clone, Debug, Default)]
pub struct HttpResponse {
    pub status: HttpResponseStatus,
    pub text: String,
}

impl HttpResponse {
    pub fn new() -> Self {
        Self {
            status: HttpResponseStatus::NoResponse,
            text: String::new(),
        }
    }

    pub fn set_body(&mut self, text: &str) {
        self.text = text.to_string();
    }

    pub fn clear(&mut self) {
        self.status = HttpResponseStatus::NoResponse;
        self.text.clear();
    }

    pub fn error(&mut self, status: HttpStatus, err: Option<&Error>) {
        use crate::t;

        self.status = HttpResponseStatus::Error(status);
        self.text = format!(
            "{}: {}",
            t!(&status.to_string()),
            match err {
                Some(e) => e.to_string(),
                None => "Unknown".to_string(),
            }
        );
    }

    pub fn file(&mut self, path: &str) {
        self.status = HttpResponseStatus::File;
        self.text = path.to_string();
    }

    pub fn http_status(&self) -> HttpStatus {
        match self.status {
            HttpResponseStatus::Error(s) => s,
            HttpResponseStatus::File => HttpStatus::Ok,
            HttpResponseStatus::NoResponse => HttpStatus::BadRequest,
            HttpResponseStatus::Ok => HttpStatus::Ok,
            HttpResponseStatus::Redirect => HttpStatus::SeeOther,
        }
    }

    pub fn ok(&mut self) {
        self.status = HttpResponseStatus::Ok;
    }

    pub fn redirect(&mut self, url: &str) {
        self.status = HttpResponseStatus::Redirect;
        self.text = url.to_string();
    }
}

/// See [`HttpResponse`](struct.HttpResponse.html).
///
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum HttpResponseStatus {
    Error(HttpStatus),
    File,
    #[default]
    NoResponse,
    Ok,
    Redirect,
}

// --- HttpServerImpl --------------------------------------------------------

/// A default implementation of [`HttpServer`](trait.HttpServer.html).
///
/// Primarily intended for cases where there is no better alternative, see
/// e.g. [`vicocomo_tauri`](../../../vicocomo_tauri/index.html). Generally
/// *not* useful when writing an HTTP server adapter, see e.g.
/// [`vicocomo_actix`](../../../vicocomo_actix/index.html).
///
pub struct HttpServerImpl {
    // - - set by the expression returned by build_expr(), never changed - - -
    app_config: HashMap<String, AppConfigVal>,
    targets: Vec<HttpRouteTarget>,
    static_routes: HashMap<String, String>,
    session: RefCell<HttpSession>,
    // - - set by receive()  - - - - - - - - - - - - - - - - - - - - - - - - -
    req_url: Url,
    route_pars: Vec<(String, String)>,
    req_body: String,
    param_vals: HttpParamVals,
    response: RefCell<HttpResponse>,
}

impl HttpServerImpl {
    /// Returns an `Expr` that will construct an instance that is [configured
    /// ](../server/struct.HttpServerIf.html) but has not received any request
    /// yet.
    ///
    /// `config` is the [`Config`](../server/struct.HttpServerIf.html).
    ///
    /// `db` is `None` or an identifier identifying a [`DatabaseIf`
    /// ](../../database/struct.DatabaseIf.html) instance. If `Some(_)`, data
    /// stored in the session by [`session_set()`
    /// ](../server/struct.HttpServerIf.html#method.session_set) is
    /// [persisted in the database](struct.HttpDbSession.html).
    ///
    pub fn build_expr(config: &Config, db: Option<Ident>) -> Expr {
        use proc_macro2::Span;
        const SESSION_PRUNE_DEFAULT: i64 = 8640000; // 100 days

        let mut config_attr: Vec<LitStr> = Vec::new();
        let mut config_val: Vec<Expr> = Vec::new();
        let mut method: Vec<LitStr> = Vec::new();
        let mut pattern: Vec<LitStr> = Vec::new();
        let mut params: Vec<Expr> = Vec::new();
        let mut target: Vec<LitStr> = Vec::new();
        let mut static_url: Vec<LitStr> = Vec::new();
        let mut static_dir: Vec<LitStr> = Vec::new();
        for (attr, val) in &config.app_config {
            if let Some(expr) = val.to_app_config_val_expr() {
                config_attr.push(LitStr::new(&attr, Span::call_site()));
                config_val.push(expr);
            }
        }
        for handler in &config.handlers {
            let met_str = handler.http_method.to_string();
            method.push(LitStr::new(&met_str, Span::call_site()));
            pattern.push(LitStr::new(&handler.pattern, Span::call_site()));
            let names = &handler.route_par_names;
            params.push(parse_quote!([ #( #names ),* ]));
            target.push(LitStr::new(&handler.call_string, Span::call_site()));
        }
        for (url, dir) in &config.static_routes {
            let url_lit = LitStr::new(&url, Span::call_site());
            static_url.push(url_lit.clone());
            let dir_lit = LitStr::new(&dir, Span::call_site());
            static_dir.push(dir_lit.clone());
        }
        let persistent: Expr = if db.as_ref().map(|_| true).unwrap_or(false) {
            let db = db.unwrap();
            let secs = config
                .app_config
                .get("prune")
                .map(|val| {
                    val.get_string()
                        .ok()
                        .and_then(|dur| {
                            ::vicocomo_derive_utils::parse_duration(&dur)
                        })
                        .expect(
                            "prune should be of the form [dDhHmMsS]?[0-9]+",
                        )
                })
                .unwrap_or(SESSION_PRUNE_DEFAULT)
                .to_string();
            let prune = LitInt::new(&secs, Span::call_site());
            parse_quote!(
                server.set_session(
                    ::vicocomo::HttpSession::new(Some(#db.clone()), #prune)
                        .expect("cannot create HttpDbSession"),
                )
            )
        } else {
            parse_quote!(())
        };
        parse_quote!({
            let mut server = ::vicocomo::HttpServerImpl::new();
        #(  server.add_cfg(#config_attr, #config_val); )*
        #(  server.add_tgt(#pattern, &#params, #target).unwrap(); )*
        #(  server.add_static(#static_url, #static_dir); )*
            #persistent;
            server
        })
    }

    /// Create an HTTP 404 response.
    ///
    pub fn not_found(&self, http_method: &str, url: &Url) {
        self.resp_error(
            HttpStatus::NotFound,
            Some(&Error::from(
                format!("{}--{}", http_method, url.path()).as_str(),
            )),
        );
    }

    /// Find out if `url` is `localhost` or `127.0.0.x` or `::1`.
    ///
    pub fn is_loopback(url: &Url) -> bool {
        fn is_lb(addr: std::net::IpAddr) -> bool {
            addr.is_loopback()
        }
        if let Some(host) = url.host() {
            return match host {
                url::Host::Domain(host) => host == "localhost",
                url::Host::Ipv4(ipv4_addr) => is_lb(ipv4_addr.into()),
                url::Host::Ipv6(ipv6_addr) => is_lb(ipv6_addr.into()),
            };
        }
        true
    }

    /// Reset state from the received `body` and `url`.
    ///
    /// On success, the return value is `Ok("path::to::controller::method")`.
    ///
    pub fn receive(
        &mut self,
        method: &str,
        url: &Url,
        body: &str,
    ) -> Result<String, Error> {
        self.route_pars.clear();
        self.req_url = url.clone();
        self.req_body = body.to_string();
        self.param_vals.set_request(body, url.query().unwrap_or(""));
        self.response.borrow_mut().clear();
        let route = format!("{}__{}", method, url.path());
        self.targets
            .iter()
            .find(|t| {
                t.pattern
                    .captures(&route)
                    .map(|vals| {
                        let mut vals = vals.iter();
                        vals.next(); // skip match of entire URL path
                        for (nam, val) in t
                            .route_par_names
                            .iter()
                            .zip(vals.map(|val| val.unwrap().as_str()))
                        {
                            self.route_pars
                                .push((nam.clone(), val.to_string()));
                        }
                        true
                    })
                    .unwrap_or(false)
            })
            .map(|t| t.target.clone())
            .ok_or_else(|| Error::other(""))
    }

    /// Return and reset the response.
    ///
    pub fn response(&self) -> HttpResponse {
        self.response.take()
    }

    // - - intended for internal use by the code generated by build_expr() - -

    #[doc(hidden)]
    pub fn new() -> Self {
        Self {
            app_config: HashMap::new(),
            targets: Vec::new(),
            static_routes: HashMap::new(),
            session: HttpSession::new(None, 0).unwrap().into(),
            req_url: Url::parse("http://localhost").unwrap(),
            route_pars: Vec::new(),
            req_body: String::new(),
            param_vals: HttpParamVals::new(),
            response: HttpResponse::new().into(),
        }
    }

    #[doc(hidden)]
    pub fn add_cfg(&mut self, attr: &str, val: AppConfigVal) {
        self.app_config.insert(attr.to_string(), val);
    }

    #[doc(hidden)]
    pub fn add_tgt(
        &mut self,
        pat: &str,
        pars: &[&str],
        target: &str,
    ) -> Result<(), Error> {
        self.targets.push(HttpRouteTarget::new(pat, pars, target)?);
        Ok(())
    }

    #[doc(hidden)]
    pub fn add_static(&mut self, url: &str, dir: &str) {
        self.static_routes.insert(url.to_string(), dir.to_string());
    }

    #[doc(hidden)]
    pub fn set_session(&mut self, session: HttpSession) {
        self.session = RefCell::new(session);
    }
}

impl HttpServer for HttpServerImpl {
    fn app_config(&self, id: &str) -> Option<AppConfigVal> {
        self.app_config.get(id).cloned()
    }

    fn param_val(&self, name: &str) -> Option<String> {
        self.param_vals.get(name).map(|v| v[0].clone())
    }

    fn param_vals(&self) -> Vec<(String, String)> {
        self.param_vals.vals()
    }

    fn req_path(&self) -> String {
        self.req_url.path().to_string()
    }

    fn req_route_par_val(&self, par: &str) -> Option<String> {
        self.route_pars
            .iter()
            .find(|(nam, _)| nam == par)
            .map(|(_, val)| val.clone())
    }

    fn req_route_par_vals(&self) -> Vec<(String, String)> {
        self.route_pars.clone()
    }

    fn req_body(&self) -> String {
        self.req_body.clone()
    }

    fn req_url(&self) -> String {
        self.req_url.to_string()
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
        self.session.borrow().get(key)
    }

    fn session_remove(&self, key: &str) {
        self.session.borrow_mut().remove(key);
    }

    fn session_set(&self, key: &str, value: &str) -> Result<(), Error> {
        self.session.borrow_mut().set(key, value)
    }

    fn url_path_to_dir(&self, url_path: &str) -> Option<String> {
        self.static_routes.get(url_path).map(|s| s.clone())
    }
}

// --- NullTemplEng ----------------------------------------------------------

/// An implementation of [`TemplEng`](trait.TemplEng.html) that does nothing
/// and returns [`Error`](../../error/enum.Error.html).
///
#[derive(Clone, Copy, Debug)]
pub struct NullTemplEng;

impl TemplEng for NullTemplEng {
    fn render(
        &self,
        _tmpl: &str,
        _json: &JsonValue,
    ) -> Result<String, Error> {
        Err(Error::render("no template engine"))
    }
}

// --- private --------------------------------------------------------------

const DB_SESSION_INSERT: &'static str =
    "INSERT INTO __vicocomo__sessions (id, data, time) VALUES ($1, $2, $3)";
const DB_SESSION_PRUNE: &'static str =
    "DELETE FROM __vicocomo__sessions WHERE time < $1";
const DB_SESSION_ROW_COUNT: &'static str =
    "SELECT COUNT(id) FROM __vicocomo__sessions";
const DB_SESSION_SELECT: &'static str =
    "SELECT data FROM __vicocomo__sessions WHERE id = $1";
const DB_SESSION_TOUCH: &'static str =
    "UPDATE __vicocomo__sessions SET time = $2 WHERE id = $1";
const DB_SESSION_UPDATE: &'static str =
    "UPDATE __vicocomo__sessions SET data = $2, time = $3 WHERE id = $1";

lazy_static::lazy_static! {
    static ref ROUTE_PARAM: Regex = Regex::new(r"<[^>]*>").unwrap();
}

// - - HttpSession - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// Conceptually private, but needs to be public because it is used by the code
// generated by HttpServerImpl::build_expr().

#[doc(hidden)]
pub enum HttpSession {
    Persistent(HttpDbSession),
    Transient(HashMap<String, String>),
}

impl HttpSession {
    pub fn new(db: Option<DatabaseIf>, prune: i64) -> Result<Self, Error> {
        match db {
            Some(db) => HttpDbSession::new(
                db.clone(),
                Some(0),
                prune,
                Some(
                    "CREATE TABLE __vicocomo__sessions(\
                        id BIGINT, data TEXT, time BIGINT\
                    )",
                ),
            )
            .map(|dbs| Self::Persistent(dbs)),
            None => Ok(Self::Transient(HashMap::new())),
        }
    }

    fn clear(&mut self) {
        match self {
            HttpSession::Transient(map) => map.clear(),
            HttpSession::Persistent(dbs) => dbs.clear(),
        }
    }

    fn get(&self, key: &str) -> Option<String> {
        match self {
            HttpSession::Transient(map) => map.get(key).cloned(),
            HttpSession::Persistent(dbs) => dbs.get(key),
        }
    }

    fn remove(&mut self, key: &str) {
        match self {
            HttpSession::Transient(map) => {
                map.remove(key);
            }
            HttpSession::Persistent(dbs) => dbs.remove(key),
        }
    }

    fn set(&mut self, key: &str, value: &str) -> Result<(), Error> {
        match self {
            HttpSession::Transient(map) => {
                map.insert(key.to_string(), value.to_string());
                Ok(())
            }
            HttpSession::Persistent(dbs) => dbs.set(key, value),
        }
    }
}

// - - HttpRouteTarget - - - - - - - - - - - - - - - - - - - - - - - - - - - -

#[derive(Debug)]
struct HttpRouteTarget {
    pattern: Regex,
    route_par_names: Vec<String>,
    target: String,
}

impl HttpRouteTarget {
    fn new(
        pattern: &str,
        par_nams: &[&str],
        target: &str,
    ) -> Result<Self, Error> {
        Ok(Self {
            pattern: map_error!(
                Other,
                Regex::new((String::from("^") + pattern + "$").as_str(),)
            )?,
            route_par_names: par_nams.iter().map(|n| n.to_string()).collect(),
            target: target.to_string(),
        })
    }
}

/// Extract the parameter names from `route`.
///
fn get_route_par_names(route: &str) -> Vec<LitStr> {
    use proc_macro2::Span;
    ROUTE_PARAM
        .find_iter(route)
        .map(|p| {
            let s = p.as_str();
            LitStr::new(&s[1..s.len() - 1], Span::call_site())
        })
        .collect()
}

// ensure that there is a root string and fix_slashes()
fn fix_root(
    app_config: &mut HashMap<String, ConfigAttrVal>,
    root: &str,
    lead: i32,
    trail: i32,
) -> String {
    let inserted = app_config.get(root).map(|val| val.get_string().unwrap());
    let result = inserted
        .as_ref()
        .map(|s| crate::fix_slashes(s, lead, trail))
        .unwrap_or_else(|| String::new());
    if match inserted {
        Some(s) => s != result,
        None => true,
    } {
        app_config.insert(root.to_string(), result.as_str().into());
    }
    result
}

#[derive(Clone, Debug)]
struct ConfigItem {
    level_1: ConfigItemId,
    value: Option<ConfigAttrVal>,
    level_2: Vec<Level2>,
}

#[derive(Clone, Copy, Debug)]
enum ConfigItemId {
    AppConfig,
    Authorize,
    NotFound,
    PlugIn,
    Routes,
    StaticRoutes,
}

impl ConfigItem {
    // assumes level_1 to be AppConfig
    fn get_app_conf(
        &self,
        app_config: &mut HashMap<String, ConfigAttrVal>,
    ) -> syn::Result<()> {
        self.level_2.first().map(|l2| {
            for attr in &l2.attrs {
                app_config.insert(attr.id.to_string(), attr.val.clone());
            }
        });
        Ok(())
    }

    // assumes level_1 to be Authorize
    fn get_authorizer(
        &self,
        authorizers: &mut Authorizers,
    ) -> syn::Result<()> {
        let mut pat_str = self
            .value
            .as_ref()
            .ok_or_else(|| syn_error("missing authorize pattern"))?
            .get_string()?;
        // ensure slash at beginning
        if pat_str.get(0..1).map(|c| c != "/").unwrap_or(true) {
            pat_str.insert(0, '/');
        }
        let wild = pat_str.len() >= 2
            && pat_str
                .get((pat_str.len() - 2)..)
                .map(|s| s == "/*")
                .unwrap_or(false);
        let pat_tail;
        if wild {
            pat_str.truncate(pat_str.len() - 2);
            pat_tail = r"(/.*)?$";
        } else {
            pat_tail = r"$";
        }
        pat_str += pat_tail;
        pat_str = format!("^{}", route_to_pattern(&pat_str));
        let pattern =
            Regex::new(&pat_str).map_err(|e| syn_error(&e.to_string()))?;
        let priority = 2 * (pat_str.len() - pat_tail.len()) + wild as usize;
        for l2 in self.level_2.as_slice() {
            if l2.id.is_some() {
                let method =
                    Self::ident_to_http_method(l2.id.as_ref().unwrap())?;
                for attr in &l2.attrs {
                    let mut filter = false;
                    let attr_id_str = attr.id.to_string();
                    match attr_id_str.as_str() {
                        "allow" => (),
                        "filter" => filter = true,
                        _ => {
                            return Err(syn_error(&format!(
                                "unknown authorization attribute: {}",
                                attr_id_str,
                            )));
                        }
                    }
                    authorizers.insert(
                        method,
                        Authorizer {
                            priority,
                            pattern: pattern.clone(),
                            roles: attr.val.get_id_strings()?,
                            filter,
                        },
                    );
                }
            } else {
                for attr in &l2.attrs {
                    authorizers.insert(
                        Self::ident_to_http_method(&attr.id)?,
                        Authorizer {
                            priority,
                            pattern: pattern.clone(),
                            roles: attr.val.get_id_strings()?,
                            filter: false,
                        },
                    );
                }
            }
        }
        Ok(())
    }

    // assumes level_1 to be NotFound
    fn get_not_found(
        &self,
        not_found: &mut Option<(Path, Ident)>,
    ) -> syn::Result<()> {
        Ok(*not_found = Some((
            self.value
                .as_ref()
                .ok_or_else(|| syn_error("missing not_found controller Path"))
                .and_then(|v| v.get_path())?,
            self.level_2
                .first()
                .and_then(|l2| l2.id.clone())
                .ok_or_else(|| {
                    syn_error("missing not_found function Ident")
                })?,
        )))
    }

    // assumes level_1 to be PlugIn
    fn get_plug_in(
        &self,
        plug_ins: &mut HashMap<String, (Type, Expr)>,
    ) -> syn::Result<()> {
        let id = self
            .value
            .as_ref()
            .ok_or_else(|| syn_error("missing plug in identifier"))
            .and_then(|path| path.get_ident().map(|id| id.to_string()))?;
        let l2 = self
            .level_2
            .first()
            .ok_or_else(|| syn_error("missing plug in attributes"))?;
        plug_ins.insert(
            id,
            l2.attrs
                .iter()
                .find(|ci| ci.id.to_string() == "def")
                .ok_or_else(|| syn_error("missing plug in attribute 'def'"))?
                .val
                .get_type_expr()?,
        );
        Ok(())
    }

    // assumes level_1 to be Routes
    fn get_handlers(
        &self,
        handlers: &mut Vec<HttpHandler>,
    ) -> syn::Result<()> {
        use case::CaseExt;
        let contr_path = self
            .value
            .as_ref()
            .ok_or_else(|| syn_error("missing route controller path"))
            .and_then(|v| v.get_path())?;
        let contr_id = &contr_path.segments.last().unwrap().ident.clone();
        let contr_id_snake = contr_id.to_string().to_snake();
        for l2 in &self.level_2 {
            let contr_method: Ident = l2
                .id
                .clone()
                .ok_or_else(|| syn_error("missing route handler function"))?;
            let mut http_method = HttpMethod::Get;
            let mut route_str: Option<&str> = None;
            let meth_string = contr_method.to_string();
            match meth_string.as_str() {
                "new_form" => route_str = Some("new"),
                "copy_form" => route_str = Some("<id>/copy"),
                "create" => {
                    http_method = HttpMethod::Post;
                    route_str = Some("");
                }
                "ensure" => {
                    http_method = HttpMethod::Post;
                    route_str = Some("ensure");
                }
                "index" => route_str = Some(""),
                "show" => route_str = Some("<id>"),
                "edit_form" => route_str = Some("<id>/edit"),
                "patch" => {
                    http_method = HttpMethod::Post;
                    route_str = Some("<id>");
                }
                "replace" => {
                    http_method = HttpMethod::Post;
                    route_str = Some("<id>/replace");
                }
                "delete" => {
                    http_method = HttpMethod::Post;
                    route_str = Some("<id>/delete");
                }
                _ => (),
            }
            let mut route = route_str.map(|s| s.to_string());
            for attr in &l2.attrs {
                let attr_nam = attr.id.to_string();
                match attr_nam.as_str() {
                    "http_method" => {
                        http_method = attr.val.get_ident().and_then(|i| {
                            HttpMethod::try_from(i.to_string().as_str())
                                .map_err(|e| syn_error(&e.to_string()))
                        })?;
                    }
                    "path" => {
                        let mut s = attr.val.get_string()?;
                        if 1 < s.len() && s.chars().last() == Some('/') {
                            s.remove(s.len() - 1);
                        }
                        route = Some(s);
                    }
                    _ => {
                        return Err(syn_error(&format!(
                            "unknown handler attribute {}",
                            &attr_nam,
                        )));
                    }
                }
            }
            let route = {
                let mut r = match route {
                    Some(r) => r,
                    None => return Err(syn_error("missing path")),
                };
                if !r.starts_with('/') {
                    if !r.is_empty() {
                        r.insert(0, '/');
                    }
                    r.insert_str(0, &contr_id_snake);
                    r.insert(0, '/');
                }
                r
            };
            let route_par_names = get_route_par_names(&route);
            handlers.push(HttpHandler {
                authorized: None,
                contr_method,
                contr_path: contr_path.clone(),
                call_string: tokens_to_string(&contr_path)
                    + "::"
                    + &meth_string,
                http_method,
                route,
                pattern: String::new(),
                route_par_names,
            });
        }
        Ok(())
    }

    // assumes level_1 to be StaticRoutes
    fn get_static_routes(
        &self,
        routes: &mut Vec<(String, String)>,
    ) -> syn::Result<()> {
        use crate::fix_slashes;
        if let Some(v) = &self.value {
            let url_path = v.get_fix_slashes(1, -1)?;
            let mut fs_path = None;
            if let Some(l2) = self.level_2.first() {
                if let Some(attr) = l2.attrs.first() {
                    if attr.id.to_string() == "fs_path" {
                        fs_path = Some(attr.val.get_fix_slashes(0, -1)?);
                    } else {
                        return Err(syn_error(
                            "Expecting route_static(...) { fs_path: \"...\" }"
                        ));
                    }
                }
            }
            let fs_path = fix_slashes(
                &fs_path.unwrap_or(fix_slashes(&url_path, -1, 0)),
                0,
                -1,
            );
            routes.push((url_path, fs_path));
            Ok(())
        } else {
            Err(syn_error("Expecting route_static(\"...\")"))
        }
    }

    fn ident_to_http_method(id: &Ident) -> syn::Result<HttpMethod> {
        HttpMethod::try_from(id.to_string().as_str())
            .map_err(|e| syn_error(&e.to_string()))
    }
}

impl Parse for ConfigItem {
    fn parse(stream: ParseStream) -> syn::Result<Self> {
        let level_1: ConfigItemId = {
            let id = stream.parse::<Ident>()?.to_string();
            match id.as_str() {
                "app_config" => ConfigItemId::AppConfig,
                "authorize" => ConfigItemId::Authorize,
                "not_found" => ConfigItemId::NotFound,
                "plug_in" => ConfigItemId::PlugIn,
                "route" => ConfigItemId::Routes,
                "route_static" => ConfigItemId::StaticRoutes,
                _ => {
                    return Err(syn_error(&format!(
                        "'{}' cannot start a Config item",
                        &id
                    )));
                }
            }
        };
        let value: Option<ConfigAttrVal> = if stream.peek(token::Paren) {
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
        let mut level_2: Vec<Level2> = Vec::new();
        if stream.peek(token::Brace) {
            if {
                let fork = stream.fork();
                let content;
                braced!(content in fork);
                content.peek(Ident) && content.peek2(token::Colon)
            } {
                // only lev 3
                level_2.push(Level2 {
                    id: None,
                    attrs: get_config_attrs(stream)?,
                });
            } else {
                let content;
                braced!(content in stream);
                level_2 = content
                    .parse_terminated::<Level2, token::Comma>(Level2::parse)?
                    .into_iter()
                    .collect();
            }
        }
        Ok(Self {
            level_1,
            value,
            level_2,
        })
    }
}

impl Parse for ConfigAttrVal {
    fn parse(stream: ParseStream) -> syn::Result<Self> {
        use std::mem::discriminant;
        if stream.peek(token::Paren) {
            let content;
            parenthesized!(content in stream);
            let ty: Type = content.parse()?;
            content.parse::<token::Comma>()?;
            let expr: Expr = content.parse()?;
            let _ = content.parse::<token::Comma>();
            return Ok(Self::TypeExpr(ty, expr));
        }
        if stream.peek(token::Bracket) {
            let content;
            bracketed!(content in stream);
            return Ok(Self::Arr({
                let mut arr = Vec::new();
                let mut element_variant = None;
                for val in content
                    .parse_terminated::<ConfigAttrVal, token::Comma>(
                        ConfigAttrVal::parse,
                    )?
                {
                    let this_variant = discriminant(&val);
                    if let Some(dscr) = element_variant {
                        if this_variant != dscr {
                            return Err(syn_error(
                                "array elements must all be the same \
                                variant",
                            ));
                        }
                    } else {
                        element_variant = Some(this_variant)
                    }
                    arr.push(val);
                }
                arr
            }));
        }
        let expr: Expr = stream.parse()?;
        match &expr {
            Expr::Lit(lit) => match &lit.lit {
                syn::Lit::Bool(b) => return Ok(Self::Bool(b.clone())),
                syn::Lit::Char(c) => return Ok(Self::Char(c.clone())),
                syn::Lit::Float(f) => return Ok(Self::Float(f.clone())),
                syn::Lit::Int(i) => return Ok(Self::Int(i.clone())),
                syn::Lit::Str(s) => return Ok(Self::Str(s.clone())),
                _ => (),
            },
            Expr::Path(p) => {
                return Ok(p
                    .path
                    .get_ident()
                    .map(|i| Self::Ident(i.clone()))
                    .unwrap_or_else(|| Self::Path(p.path.clone())))
            }
            _ => (),
        }
        Err(syn_error(&format!(
            "cannot handle attribute value '{}'",
            tokens_to_string(&expr.clone()),
        )))
    }
}

#[derive(Clone, Debug)]
struct Authorizer {
    priority: usize,
    pattern: Regex,
    roles: Vec<String>,
    filter: bool,
}

impl Eq for Authorizer {}

impl Ord for Authorizer {
    fn cmp(&self, other: &Self) -> Ordering {
        other.priority.cmp(&self.priority) // sic!
    }
}

impl PartialEq for Authorizer {
    fn eq(&self, other: &Self) -> bool {
        self.priority == other.priority
    }
}

impl PartialOrd for Authorizer {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

struct Authorizers(HashMap<HttpMethod, Vec<Authorizer>>);

impl Authorizers {
    fn new() -> Self {
        Self(HashMap::new())
    }

    fn get(&self, method: HttpMethod) -> syn::Result<&[Authorizer]> {
        Ok(self
            .0
            .get(&method)
            .ok_or_else(|| {
                syn_error(&format!(
                    "no authorizations for HTTP method {}",
                    method.to_string()
                ))
            })?
            .as_slice())
    }

    fn insert(&mut self, method: HttpMethod, authorizer: Authorizer) {
        match self.0.get_mut(&method) {
            Some(auths) => auths.push(authorizer),
            None => {
                self.0.insert(method, vec![authorizer]);
            }
        }
    }

    // Resolve all pseudo roles except `Public`, which is handled later.
    // Sort.
    fn sanitize(&mut self, defined_roles: &[String]) -> syn::Result<()> {
        let enabled: Vec<String> = defined_roles
            .iter()
            .filter(|r| r.as_str() != "Disabled")
            .map(|r| r.clone())
            .collect();
        for (_, auths) in &mut self.0 {
            for auth in auths.iter_mut() {
                for role in &auth.roles {
                    match role.as_str() {
                        "Authenticated" => {
                            Self::assert_single(
                                &role,
                                auth.roles.as_slice(),
                            )?;
                            auth.roles = defined_roles.to_vec();
                            break;
                        }
                        "Enabled" => {
                            Self::assert_single(
                                &role,
                                auth.roles.as_slice(),
                            )?;
                            auth.roles = enabled.clone();
                            break;
                        }
                        "Public" => {
                            Self::assert_single(
                                &role,
                                auth.roles.as_slice(),
                            )?;
                            break;
                        }
                        _ if defined_roles.contains(&role) => (),
                        _ => {
                            return Err(syn_error(&format!(
                                "{} is not a role",
                                role
                            )));
                        }
                    }
                }
            }
            auths.sort();
        }
        Ok(())
    }

    fn assert_single(role: &str, roles: &[String]) -> syn::Result<()> {
        if roles.len() == 1 {
            return Ok(());
        }
        Err(syn_error(&format!("{} should be the only role", role)))
    }
}

#[derive(Clone, Debug)]
struct ConfigAttr {
    id: Ident,
    val: ConfigAttrVal,
}

impl Parse for ConfigAttr {
    fn parse(stream: ParseStream) -> syn::Result<Self> {
        let id: Ident = stream.parse()?;
        stream.parse::<token::Colon>()?;
        Ok(Self {
            id,
            val: stream.parse()?,
        })
    }
}

#[derive(Clone, Debug)]
struct Level2 {
    id: Option<Ident>,
    attrs: Vec<ConfigAttr>,
}

impl Parse for Level2 {
    fn parse(stream: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            id: stream.parse()?,
            attrs: get_config_attrs(stream)?,
        })
    }
}

fn get_config_attrs(stream: ParseStream) -> syn::Result<Vec<ConfigAttr>> {
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

// convert a route path to a regex pattern extracting the parameters
fn route_to_pattern(route: &str) -> String {
    ROUTE_PARAM.replace_all(&route, r"([^/]+)").to_string()
}

fn syn_error(e: &str) -> syn::Error {
    syn::Error::new(proc_macro2::Span::call_site(), e)
}
