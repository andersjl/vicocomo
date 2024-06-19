//! Structs and traits used to implement an HTTP server adapter with a
//! `config` macro. For HTTP server adapter developers only!

use super::{AppConfigVal, HttpReqBody, HttpResponse, HttpStatus};
use crate::{map_error, t, DatabaseIf, DbType, Error};
use chrono::{Local, NaiveDateTime, TimeDelta};
use core::convert::TryFrom;
use core::fmt::{self, Display, Formatter};
use quote::format_ident;
use rand::{thread_rng, Rng};
use regex::Regex;
use serde_json::Value as JsonValue;
use std::cell::RefCell;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::OnceLock;
use syn::parse::{Parse, ParseStream};
use syn::{
    braced, bracketed, parenthesized, parse_quote, token, Expr, Ident,
    LitBool, LitChar, LitFloat, LitInt, LitStr, Path, Type,
};
use url::Url;
use vicocomo_derive_utils::*;

// --- local macros ----------------------------------------------------------

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

// collect values for the same key in a vector mapping both key and value
// $key_value_pairs should evaluate to a slice &[(&str, &str)].
// key_map and val_map should be the name of a function (&str) -> String.
macro_rules! multi_val_map {
    ($key_value_pairs:expr) => {
        multi_val_map!($key_value_pairs, to_string, to_string)
    };
    ($key_value_pairs:expr, $key_map:ident) => {
        multi_val_map!($key_value_pairs, $key_map, to_string)
    };
    ($key_value_pairs:expr, $key_map:ident, $val_map:ident) => {{
        let mut result: HashMap<String, Vec<String>> = HashMap::new();
        for (key, val) in $key_value_pairs.iter() {
            let key = key.$key_map();
            let val = val.$val_map();
            match result.get_mut(&key) {
                Some(vals) => vals.push(val),
                None => {
                    result.insert(key, vec![val]);
                }
            }
        }
        result
    }};
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
        fix_root(&mut app_config, "data_dir", 0, 1);
        fix_root(&mut app_config, "resource_dir", 0, 1);
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

        // - - texts configuration business rules  - - - - - - - - - - - - - -

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
        Ok(ljumvall_utils::fix_slashes(
            &self.get_string()?,
            lead,
            trail,
        ))
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
                    &[(Self::now() - TimeDelta::try_seconds(prune).unwrap())
                        .into()],
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
        Ok(Self {
            db,
            id,
            cache: cache.unwrap(),
        })
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

// --- HttpHandler -----------------------------------------------------------

/// Information needed for implementing an HTTP server configuration macro
/// using [`Config`](struct.Config.html).
///
#[derive(Clone, Debug)]
pub struct HttpHandler {
    /// If `Some`, defines access control for `route`, see [`Authorized`
    /// ](struct.Authorized.html). If `None`, there is no access control.
    ///
    pub authorized: Option<Authorized>,

    /// controller method name.
    ///
    pub contr_method: Ident,

    /// The full rust path to the controller, e.g. `path::to::controller`.
    ///
    pub contr_path: Path,

    /// The full path to `contr_method` as a `String`, e.g.
    /// `"path::to::controller::method"`.
    ///
    pub call_string: String,

    /// Only tested for Get and Post.
    ///
    /// Guaranteed to never be HttpMethod::None.
    ///
    pub http_method: HttpMethod,

    /// Route, possibly with path parameters in angle brackets. The
    /// `app_config` attribute [`url_root`
    /// ](../server/struct.HttpServerIf.html#url_root) is prepended if
    /// defined. Use [`HttpServerIf::strip_url_root()`
    /// ](struct.HttpServerIf.html#tymethod.strip_url_root) to get the
    /// relative URL.
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

    /// If Some("field") the handler should expect a `multipart/form-data`
    /// containing files to upload, see [`HttpServerIf`
    /// ](../server/struct.HttpServerIf.html#file-upload).
    ///
    pub upload: Option<String>,
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

// --- HttpMethod ------------------------------------------------------------

/// A simple enum with the official methods.
///
/// HttpMethod::None is only for internal use by `ConfigItem::parse()`.
///
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Hash)]
pub enum HttpMethod {
    Connect,
    Delete,
    Get,
    Head,
    None,
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
                Self::None => "none",
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

impl TryFrom<String> for HttpMethod {
    type Error = Error;
    fn try_from(s: String) -> Result<Self, Error> {
        Self::try_from(s.as_str())
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

    /// If Some(vec) vec is never empty
    pub fn get(&self, name: &str) -> Option<&Vec<String>> {
        self.0.get(name)
    }

    /// Empty string(s) is OK. `body_par_string` should only contain
    /// `parameters=value` pairs, e.g. generated from the body by
    /// [`HttpServer::body_par_string()`
    /// ](trait.HttpServer.html#method.body_par_string)
    ///
    pub fn set_request(&mut self, body_par_string: &str, req_query: &str) {
        static QUERY: OnceLock<Regex> = OnceLock::new();
        let query = QUERY.get_or_init(|| {
            Regex::new(r"([^&=]+=[^&=]+&)*[^&=]+=[^&=]+").unwrap()
        });

        let body_vals = query
            .captures(body_par_string)
            .and_then(|c| c.get(0))
            .and_then(|m| decode_url_parameter(m.as_str()).ok());
        let uri_vals = decode_url_parameter(req_query).ok();
        let mut param_vals = Vec::new();
        let par_string = match uri_vals {
            Some(u) => match body_vals {
                Some(b) => u + "&" + b.as_ref(),
                None => u,
            },
            None => body_vals.unwrap_or_else(|| String::new()),
        };
        for key_value in par_string.split('&') {
            if key_value.len() == 0 {
                continue;
            }
            let mut k_v = key_value.split('=');
            param_vals.push((k_v.next().unwrap(), k_v.next().unwrap_or("")));
        }
        self.0 = multi_val_map!(param_vals.as_slice());
    }

    pub fn vals(&self) -> Vec<(String, String)> {
        let mut result = Vec::new();
        for (key, vals) in &self.0 {
            for val in vals {
                result.push((key.clone(), val.clone()));
            }
        }
        result
    }
}

// --- HttpRequest -----------------------------------------------------------

/// Everything Vicocomo needs from an HTTP server that depends on the current
/// request.
///
/// There is an example implementation for [`actix-web`
/// ](https://crates.io/crates/actix-web) [here
/// ](../../../vicocomo_actix/struct.AxRequest.html).
///
pub trait HttpRequest<'req> {
    /// Returns the body as a `String` if no
    /// `Content-Type: multipart/form-data` header is present and the body is
    /// an UTF8 string.
    ///
    fn body_par_string(&self) -> String {
        let body = self.body();
        if self
            .header("content-type")
            .map(|h| h.starts_with("multipart/form-data;"))
            .unwrap_or(false)
        {
            String::new()
        } else {
            String::from_utf8_lossy(body.bytes).to_string()
        }
    }

    /// The HTTP method of the request.
    ///
    fn http_method(&self) -> HttpMethod;

    /// See [`HttpServerIf::param_val()`
    /// ](../server/struct.HttpServerIf.html#method.param_val), but this one
    /// always returns a `String`.
    ///
    fn param_val(&self, name: &str) -> Option<String>;

    /// All parameter values in the URL (get) or body (post) as a vector of
    /// URL-decoded key-value pairs.
    ///
    fn param_vals(&self) -> Vec<(String, String)>;

    /// The request body.
    ///
    fn body(&self) -> HttpReqBody<'req>;

    /// See [`HttpServerIf::req_header()`
    /// ](../server/struct.HttpServerIf.html#method.req_header).
    ///
    fn header(&self, name: &str) -> Option<String>;

    /// See [`HttpServerIf::req_path()`
    /// ](../server/struct.HttpServerIf.html#method.req_path), but this one
    /// <b>does not strip</b> the `url_root` [attribute](#level-1-app_config).
    ///
    fn path(&self) -> String;

    /// See [`HttpServerIf::req_route_par_val()`
    /// ](../server/struct.HttpServerIf.html#method.req_route_par_val), but
    /// this one returns a `String`.
    ///
    fn route_par_val(&self, par: &str) -> Option<String>;

    /// See [`HttpServerIf::req_route_par_vals()`
    /// ](../server/struct.HttpServerIf.html#method.req_route_par_vals).
    ///
    fn route_par_vals(&self) -> Vec<(String, String)>;

    /// See [`HttpServerIf::req_url()`
    /// ](../server/struct.HttpServerIf.html#method.req_url).
    ///
    fn url(&self) -> String;
}

// --- HttpRequestImpl -------------------------------------------------------

/// A default implementation of [`HttpRequest`](trait.HttpRequest.html).
///
/// Primarily intended for cases where there is no better alternative, see
/// e.g. [`vicocomo_tauri`](../../../vicocomo_tauri/index.html). Generally
/// *not* useful when writing an HTTP server adapter, see e.g.
/// [`vicocomo_actix`](../../../vicocomo_actix/index.html).
///
#[derive(Debug)]
pub struct HttpRequestImpl<'req> {
    handler: String,
    headers: HashMap<String, Vec<String>>,
    method: HttpMethod,
    param_vals: HttpParamVals,
    body: HttpReqBody<'req>,
    url: Url,
    route_pars: Vec<(String, String)>,
}

impl<'req> HttpRequestImpl<'req> {
    /// Create from the received `method`, `url`, `headers`, and `body`.
    ///
    pub fn new(
        method: HttpMethod,
        url: &Url,
        headers: &[(&str, &str)],
        body: HttpReqBody<'req>,
        targets: &[HttpRouteTarget],
    ) -> Result<Self, Error> {
        let mut route_pars = Vec::new();
        let route = format!("{method}__{}", url.path());
        let mut parameter_error: Option<String> = None;
        targets
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
                            route_pars.push((
                                nam.clone(),
                                decode_url_parameter(val).unwrap_or_else(
                                    |e| {
                                        parameter_error =
                                            Some(e.to_string() + ": " + val);
                                        String::new()
                                    },
                                ),
                            ));
                        }
                        parameter_error.is_none()
                    })
                    .unwrap_or(false)
            })
            .map(|t| {
                let mut result = Self {
                    handler: t.target.clone(),
                    headers: multi_val_map!(headers, to_lowercase),
                    method,
                    param_vals: HttpParamVals::new(),
                    body: body,
                    url: url.clone(),
                    route_pars,
                };
                result.param_vals.set_request(
                    &result.body_par_string(),
                    url.query().unwrap_or(""),
                );
                result
            })
            .ok_or_else(|| {
                Error::InvalidInput(
                    parameter_error.unwrap_or("route-not-found".to_string()),
                )
            })
    }

    /// Return the handler created by [`new()`](#method.new).
    ///
    /// The returned value is of the form `"path::to::controller::method"`.
    ///
    pub fn handler(&'req self) -> &'req str {
        self.handler.as_str()
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
}

impl<'req> HttpRequest<'req> for HttpRequestImpl<'req> {
    fn http_method(&self) -> HttpMethod {
        self.method
    }

    fn param_val(&self, name: &str) -> Option<String> {
        self.param_vals.get(name).map(|vec| vec[0].clone())
    }

    fn param_vals(&self) -> Vec<(String, String)> {
        self.param_vals.vals()
    }

    fn body(&self) -> HttpReqBody<'req> {
        self.body.clone()
    }

    fn header(&self, name: &str) -> Option<String> {
        self.headers
            .get(&name.to_lowercase())
            .map(|vec| vec[0].clone())
    }

    fn path(&self) -> String {
        self.url.path().to_string()
    }

    fn route_par_val(&self, par: &str) -> Option<String> {
        self.route_pars
            .iter()
            .find(|(nam, _)| nam == par)
            .map(|(_, val)| val.clone())
    }

    fn route_par_vals(&self) -> Vec<(String, String)> {
        self.route_pars.clone()
    }

    fn url(&self) -> String {
        self.url.to_string()
    }
}

// --- HttpRespBody ----------------------------------------------------------

/// An interface for the HTTP server to access the response body.
///
#[derive(Clone, Debug, PartialEq)]
pub enum HttpRespBody {
    Bytes(Vec<u8>),

    /// The "body" is the path to a file, the contents of which should be the
    /// response body.
    ///
    /// If the path starts with "/" it is an absolute path in the HTTP
    /// server's file system. If not, it is relative to the HTTP server's
    /// working directory
    ///
    Download(PathBuf),

    Str(String),

    /// An empty response body.
    ///
    None,
}

// TODO: skip the Display implementation and write a method as_str() that does
// not allocate
impl Display for HttpRespBody {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Self::Bytes(b) => {
                let s: String;
                write!(
                    f,
                    "{}",
                    match std::str::from_utf8(b) {
                        Ok(s) => s,
                        Err(e) => {
                            s = e.to_string();
                            s.as_str()
                        }
                    },
                )
            }
            Self::Download(p) => {
                write!(f, "cannot display contents of {}", p.display())
            }
            Self::Str(s) => write!(f, "{}", s),
            Self::None => write!(f, ""),
        }
    }
}

// --- HttpRouteTarget -------------------------------------------------------

/// A data structure created by [`HttpServerImpl::build_expr()`
/// ](struct.HttpServerImpl.html#method.build_expr) and used by
/// [`HttpRequestImpl::new()`](struct.HttpRequestImpl.html#method.new) to look
/// up the [`handler()`](struct.HttpRequestImpl.html#method.handler) and set
/// the values of the route parameters for
/// [`HttpRequestImpl::route_par_val()`
/// ](struct.HttpRequestImpl.html#method.route_par_val) and
/// [`HttpRequestImpl::route_par_vals()`
/// ](struct.HttpRequestImpl.html#method.route_par_vals).
///
#[derive(Debug)]
pub struct HttpRouteTarget {
    // from HttpHandler.pattern
    pattern: Regex,
    // the names of the route parameters
    route_par_names: Vec<String>,
    // the Rust path of the handler as a string
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

// --- HttpServer ------------------------------------------------------------

/// Everything Vicocomo needs from an HTTP server that does not depend on the
/// current request.
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

    /// See [`HttpServerIf::handle_upload()`
    /// ](../server/struct.HttpServerIf.html#method.handle_upload).
    ///
    fn handle_upload(
        &self,
        files: &[Option<&std::path::Path>],
    ) -> Result<(), Error>;

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

    /// Map URL to file directory for serving static files as defined by the
    /// `config` macro's [`route_static`
    /// ](../server/struct.HttpServerIf.html#level-1-route_static) entries.
    ///
    /// `url_path` is the <b>absolute</b> URL, including [`url_root`
    /// ](../server/struct.HttpServerIf.html#url_root) and is guaranteed not
    /// to end with a slash.
    ///
    /// The returned file system path is guaranteed to end with a slash, and
    /// includes [`file_root`](../server/struct.HttpServerIf.html#file_root).
    ///
    fn url_path_to_dir(&self, url_path: &str) -> Option<String>;
}

// --- HttpServerImpl --------------------------------------------------------

/// A default implementation of [`HttpServer`](trait.HttpServer.html).
///
/// Primarily intended for cases where there is no better alternative, see
/// e.g. [`vicocomo_tauri`](../../../vicocomo_tauri/index.html). Generally
/// *not* useful when writing an HTTP server adapter, see e.g.
/// [`vicocomo_actix`](../../../vicocomo_actix/index.html).
///
/// There is always a session, but unless you give a database connection to
/// [`build_expr()`](#method.build_expr) it will only live as long as this
/// instance.
///
pub struct HttpServerImpl {
    app_config: HashMap<String, AppConfigVal>,
    session: RefCell<HttpSession>,
    static_routes: HashMap<String, String>,
    targets: Vec<HttpRouteTarget>,
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
        let session_expr: Expr = if db.as_ref().map(|_| true).unwrap_or(false)
        {
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
            parse_quote!(server.set_session(
                ::vicocomo::HttpSession::new(None, 0)
                    .expect("cannot create HttpDbSession"),
            ))
        };
        parse_quote!({
            let mut server = ::vicocomo::HttpServerImpl::new();
        #(  server.add_cfg(#config_attr, #config_val); )*
        #(  server.add_tgt(#pattern, &#params, #target).unwrap(); )*
        #(  server.add_static(#static_url, #static_dir); )*
            #session_expr;
            server
        })
    }

    /// Create an HTTP 404 response.
    ///
    #[rustfmt::skip]
    pub fn not_found(&self, method: HttpMethod, url: &Url) -> HttpResponse {
eprintln!("{}", Error::from(format!("not found: {}--{}", method, url.path()).as_str()));
        HttpResponse::error(
            Some(HttpStatus::NotFound),
            None,
            Some(Error::from(
                format!("{}--{}", method, url.path()).as_str(),
            )),
        )
    }

    /// Return a list of [targets](struct.HttpRouteTarget.html).
    ///
    pub fn targets<'srv>(&'srv self) -> &'srv [HttpRouteTarget] {
        self.targets.as_slice()
    }

    // - - intended for internal use by the code generated by build_expr() - -

    #[doc(hidden)]
    pub fn new() -> Self {
        Self {
            app_config: HashMap::new(),
            session: HttpSession::new(None, 0).unwrap().into(),
            static_routes: HashMap::new(),
            targets: Vec::new(),
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
        self.session = session.into();
    }
}

impl HttpServer for HttpServerImpl {
    fn app_config(&self, id: &str) -> Option<AppConfigVal> {
        self.app_config.get(id).cloned()
    }

    fn handle_upload(
        &self,
        _files: &[Option<&std::path::Path>],
    ) -> Result<(), Error> {
        Err(Error::nyi())
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

/// Change `"+"` to `"%20"`, then [`urlencoding::decode()`
/// ](https://docs.rs/urlencoding/latest/urlencoding/fn.decode.html).
///
pub fn decode_url_parameter(par: &str) -> Result<String, Error> {
    static PLUS: OnceLock<Regex> = OnceLock::new();
    let plus = PLUS.get_or_init(|| Regex::new(r"\+").unwrap());
    urlencoding::decode(&plus.replace_all(par, "%20"))
        .map(|s| s.to_string())
        .map_err(|e| Error::invalid_input(&e.to_string()))
}

/// Returns the boundary if `content_type` is `multipart/form-data;` ...
/// `boundary="`*some boundary*`"`. For use by HTTP server adapters.
///
pub fn multipart_boundary(content_type: &str) -> Option<String> {
    let header = super::HttpHeaderVal::from_str(content_type);
    if header.value == "multipart/form-data" {
        header.get_param("boundary")
    } else {
        None
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

static ROUTE_PARAM: OnceLock<Regex> = OnceLock::new();

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

/// Extract the parameter names from `route`.
///
fn get_route_par_names(route: &str) -> Vec<LitStr> {
    use proc_macro2::Span;
    ROUTE_PARAM
        .get_or_init(route_param_init)
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
        .map(|s| ljumvall_utils::fix_slashes(s, lead, trail))
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
        *not_found = Some((
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
        ));
        Ok(())
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
            let mut method = HttpMethod::None;
            let mut route_str: Option<&str> = None;
            let mut upload = None;
            let meth_string = contr_method.to_string();
            match meth_string.as_str() {
                "new_form" => {
                    route_str = Some("new");
                }
                "copy_form" => {
                    route_str = Some("<id>/copy");
                }
                "create" => {
                    method = HttpMethod::Post;
                    route_str = Some("");
                }
                "ensure" => {
                    method = HttpMethod::Post;
                    route_str = Some("ensure");
                }
                "index" => {
                    route_str = Some("");
                }
                "show" => {
                    route_str = Some("<id>");
                }
                "edit_form" => {
                    route_str = Some("<id>/edit");
                }
                "patch" => {
                    method = HttpMethod::Post;
                    route_str = Some("<id>");
                }
                "replace" => {
                    method = HttpMethod::Post;
                    route_str = Some("<id>/replace");
                }
                "delete" => {
                    method = HttpMethod::Post;
                    route_str = Some("<id>/delete");
                }
                _ => (),
            }
            let mut route = route_str.map(|s| s.to_string());
            for attr in &l2.attrs {
                let attr_nam = attr.id.to_string();
                match attr_nam.as_str() {
                    "http_method" => {
                        method = attr.val.get_ident().and_then(|i| {
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
                    "upload" => {
                        upload = Some(attr.val.get_string()?);
                    }
                    _ => {
                        return Err(syn_error(&format!(
                            "unknown handler attribute {}",
                            &attr_nam,
                        )));
                    }
                }
            }
            if method == HttpMethod::None {
                if upload.is_some() {
                    method = HttpMethod::Post;
                } else {
                    method = HttpMethod::Get;
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
                http_method: method,
                route,
                pattern: String::new(),
                route_par_names,
                upload,
            });
        }
        Ok(())
    }

    // assumes level_1 to be StaticRoutes
    fn get_static_routes(
        &self,
        routes: &mut Vec<(String, String)>,
    ) -> syn::Result<()> {
        use ljumvall_utils::fix_slashes;
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

fn route_param_init() -> Regex {
    Regex::new(r"<[^>]*>").unwrap()
}

// convert a route path to a regex pattern extracting the parameters
fn route_to_pattern(route: &str) -> String {
    ROUTE_PARAM
        .get_or_init(route_param_init)
        .replace_all(&route, r"([^/]+)")
        .to_string()
}

fn syn_error(e: &str) -> syn::Error {
    syn::Error::new(proc_macro2::Span::call_site(), e)
}

#[cfg(test)]
mod tests {
    // TODO: unit test Config::parse()
    use super::*;

    speculate2::speculate! {
        describe "impl HttpServerImpl" {
            before {
                let test_server = HttpServerImpl::new();
            }

            context "not_found()" {
                it "returns an HTTP 404 response" {
                    let response = test_server.not_found(
                        HttpMethod::Get,
                        &Url::parse("http://localhost").unwrap(),
                    );
                    assert_eq!(response.get_status(), HttpStatus::NotFound);
                    /*
                    assert_eq!(
                        response.text,
                        "vicocomo--http_status-404: \
                            error--Other\nerror--Other--get--/"
                    );
                    */
                }
            }
        }
    }
}
