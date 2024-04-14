//! Structs and traits implemented by HTTP server adapter and used by web
//! application developers.
//!

use super::{HttpRequest, HttpRespBody, HttpServer, TemplEng};
use crate::{map_error, Error};
use regex::{CaptureMatches, Regex};
use serde::{de::DeserializeOwned, Serialize};
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::iter::Peekable;
use std::path::PathBuf;
use std::str::from_utf8;
use std::sync::{Arc, OnceLock};

// --- AppConfigVal ----------------------------------------------------------

/// The return type of [`HttpServerIf::app_config()`
/// ](struct.HttpServerIf.html#method.app_config).
///
#[derive(Clone, Debug)]
pub enum AppConfigVal {
    Arr(Vec<AppConfigVal>),
    Bool(bool),
    Char(char),
    Float(f64),
    Ident(String),
    Int(i64),
    Path(String),
    Str(String),
}

macro_rules! get_app_config_val_variant {
    ($name:ident ( $variant:ident ) -> $ret_type:ty) => {
        /// Return the contained value if the eponymous variant or `None`
        pub fn $name(&self) -> Option<$ret_type> {
            if let Self::$variant(v) = self {
                Some(v.clone())
            } else {
                None
            }
        }
    };
}

impl AppConfigVal {
    get_app_config_val_variant! { arr(Arr) -> Vec<AppConfigVal> }
    get_app_config_val_variant! { bool(Bool) -> bool }
    get_app_config_val_variant! { char(Char) -> char }
    get_app_config_val_variant! { float(Float) -> f64 }
    get_app_config_val_variant! { ident(Ident) -> String }
    get_app_config_val_variant! { int(Int) -> i64 }
    get_app_config_val_variant! { path(Path) -> String }
    get_app_config_val_variant! { str(Str) -> String }
}

// --- HttpReqBody -----------------------------------------------------------

/// The body of an HTTP request.
///
#[derive(Clone, Debug)]
pub struct HttpReqBody<'req> {
    /// The entire body. Empty if the request is a [file upload
    /// ](struct.HttpServerIf.html#file-upload).
    pub bytes: &'req [u8],

    /// The parts of a `multipart` request.
    pub parts: Vec<HttpReqBodyPart<'req>>,
}

impl<'req> HttpReqBody<'req> {
    /// Create from a raw HTTP request body.
    ///
    /// `body` is stored as `self.bytes`.
    ///
    /// `boundary` is an optional multipart boundary. If present, the parts of
    /// the multipart request will be retreived from `body` and stored in
    /// `self.parts`.
    ///
    #[rustfmt::skip]
    pub fn from_bytes(body: &'req[u8], boundary: Option<&str>) -> Self {
        let foldex = Regex::new(r"\r\n\s+").unwrap();
        let mut result = Self { bytes: body, parts: Vec::new() };
        if let Some(boundary) = boundary {
            let bound_str = String::from("--") + boundary;
            let starter = bound_str.as_bytes();
            let mut start = 0usize;
            loop {
                if let Some(mut boundary_start) =
                    body[start..]
                        .windows(starter.len())
                        .position(|w| w == starter)
                {
                    boundary_start += start;
                    let boundary_end = boundary_start + starter.len();
                    if boundary_start >= start + 2
                        && body[(boundary_start - 2)..boundary_start]
                            == [13, 10]
                    {
                        // remove trailing CRLF from preceding contents
                        boundary_start -= 2;
                    }
                    if start > 0 {
                        let last = &body[start..boundary_start];
                        let mut headers = "";
                        let mut name = String::new();
                        let mut filename = String::new();
                        let mut content_type = String::new();
                        let mut contents: &[u8] = &[];
                        if let Some(crlfcrlf) = last
                            .windows(4)
                            .position(|w| w == &[13, 10, 13, 10])
                        {
                            if crlfcrlf >= 2 {
                                // remove leading CRLF in headers
                                let hbeg =
                                    if last[0..2] == [13, 10] {
                                        2
                                    } else {
                                        0
                                    };
                                // include one CRLF after the last header
                                let hend = crlfcrlf + 2;
                                headers = from_utf8(&last[hbeg..hend]).unwrap_or("");
                                for hdr in foldex.replace_all(headers, " ").split("\r\n") {
                                    if let Some((nam, val_pars)) = hdr.split_once(":") {
                                        match nam.to_lowercase().as_str() {
                                            "content-disposition" => {
                                                let v_p = HttpHeaderVal::from_str(val_pars);
                                                if v_p.value == "form-data" {
                                                    name = v_p.get_param("name")
                                                        .unwrap_or_else(|| String::new());
                                                    filename = v_p.get_param("filename")
                                                        .unwrap_or_else(|| String::new());
                                                }
                                            }
                                            "content-type" => {
                                                content_type =
                                                    HttpHeaderVal::from_str(val_pars).value;
                                            }
                                            _ => (),
                                        }
                                    }
                                }
                            }
                            contents = &body[(start + crlfcrlf + 4)..boundary_start];
                        }
                        result.parts.push(HttpReqBodyPart::FormData {
                            headers,
                            name,
                            filename,
                            content_type,
                            contents,
                        });
                    }
                    if body.len() < boundary_end + 2
                        || body[boundary_end..(boundary_end + 2)] == [45, 45]
                    {
                        break;
                    }
                    start = boundary_end;
                } else {
                    break;
                }
            }
        }
        result
    }

    /// Return `bytes` as an UTF8 string if possible.
    ///
    pub fn try_as_str(&'req self) -> Result<&'req str, Error> {
        map_error!(InvalidInput, from_utf8(self.bytes))
    }
}

impl<'req> std::fmt::Display for HttpReqBody<'req> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", String::from_utf8_lossy(self.bytes))
    }
}

// --- HttpReqBodyPart -------------------------------------------------------

/// Represents one part of the body of a multipart HTTP request. Currently
/// restricted to `multipart/form-data`.
///
#[derive(Clone, Debug)]
pub enum HttpReqBodyPart<'req> {
    /// All data in one part of a multipart/form-data HTTP request.
    ///
    FormData {
        /// The headers within this part's boundary, not including the headers
        /// of the HTTP request. Each header is CRLF-terminated. Folded values
        /// are not unfolded.
        headers: &'req str,

        /// Empty if there is no `Content-Disposition` header with value
        /// `form-data` in `headers`.
        name: String,

        /// Empty if there is no `Content-Disposition` header with value
        /// `form-data` in `headers` or no parameter `filename` in that
        /// header.
        filename: String,

        /// The value of a `Content-Type` header in `headers` or empty.
        content_type: String,

        /// The contents of this part. Does not include any of the two leading
        /// CRLFs. Does not include the CRLF before the trailing boundary.
        contents: &'req [u8],
    },

    /// Information about an uploaded file.
    ///
    Uploaded {
        /// The field name, possibly not unique.
        name: String,

        /// The original filename before uploading if provided.
        filename: Option<String>,

        /// The value of the parts `Content-Type` header if provided.
        content_type: Option<String>,
    },
}

// --- HttpHeaderVal ---------------------------------------------------------

/// The part after the `':'` in an HTTP header, structured.
///
/// See [`from_str()`](#method.from_str).
///
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HttpHeaderVal {
    /// The text between the first `:` and the following `;` or CRLF.
    pub value: String,
    /// `(`lowercase name`, `value`)` pairs from `; name="value"` sequences.
    pub params: Vec<(String, String)>,
}

impl HttpHeaderVal {
    /// Partition the value of an HTTP header, supposing that it is a value
    /// optionally followed by any number of `; par_name="par_value"`:
    /// ```
    /// assert_eq!(
    ///     vicocomo::HttpHeaderVal::from_str(
    ///         "\t value; \n p1=foo ;P2 ;p3=\"bar   \" "
    ///     ),
    ///     vicocomo::HttpHeaderVal {
    ///         value: String::from("value"),
    ///         params: vec![
    ///             (String::from("p1"), String::from("foo")),
    ///             (String::from("p2"), String::new()),
    ///             (String::from("p3"), String::from("\"bar   \""))
    ///         ]
    ///     },
    /// );
    /// ```
    /// Note that `val_pars` is unfolded before partitioning.
    ///
    pub fn from_str(val_pars: &str) -> Self {
        // unfold
        let val_pars = Regex::new(r"\r\n\s+")
            .unwrap()
            .replace_all(val_pars, " ")
            .trim()
            .to_string();
        let mut val_pars = val_pars.split(";");
        let value = val_pars.next().unwrap().trim().to_string();
        let mut params = Vec::new();
        for nam_val in val_pars {
            let mut nam_val = nam_val.trim().split("=");
            let nam = nam_val.next().unwrap().trim().to_lowercase();
            params.push((
                nam,
                nam_val
                    .next()
                    .map(|v| v.trim().to_string())
                    .unwrap_or_else(|| String::new()),
            ))
        }
        Self { value, params }
    }

    /// Get the parameter with name `name`.
    ///
    pub fn get_param(&self, name: &str) -> Option<String> {
        self.params
            .iter()
            .find(|(n, _)| *n == name.to_lowercase())
            .map(|(_, v)| {
                let b = v.as_bytes();
                if b.len() > 2 && b[0] == b'"' && b[b.len() - 1] == b'"' {
                    from_utf8(&b[1..(b.len() - 1)]).unwrap()
                } else {
                    v
                }
                .to_string()
            })
    }
}

// --- HttpResponse ----------------------------------------------------------

/// To be returned from a [route handler
/// ](struct.HttpServerIf.html#controller-path-and-handling-method).
///
#[derive(Clone, Debug, PartialEq)]
pub struct HttpResponse {
    status: HttpStatus,
    headers: Vec<(String, String)>,
    body: HttpRespBody,
}

impl HttpResponse {

    // - - constructors  - - - - - - - - - - - - - - - - - - - - - - - - - - -

    /// Construct a response from a vector of `u8`.
    ///
    /// The default status is `HttpStatus::Ok`.
    ///
    /// The default `Content-Type` header is `application/octet-stream`.
    ///
    pub fn bytes(bytes: Vec<u8>) -> Self {
        Self {
            status: HttpStatus::Ok,
            headers: vec![(
                "Content-Type".to_string(),
                "application/octet-stream".to_string(),
            )],
            body: HttpRespBody::Bytes(bytes),
        }
    }

    /// Construct a response from a slice of `u8`.
    ///
    /// The default status is `HttpStatus::Ok`.
    ///
    /// The default `Content-Type` header is `application/octet-stream`.
    ///
    pub fn byteslice(bytes: &[u8]) -> Self {
        Self {
            status: HttpStatus::Ok,
            headers: vec![(
                "Content-Type".to_string(),
                "application/octet-stream".to_string(),
            )],
            body: HttpRespBody::Bytes(bytes.to_vec()),
        }
    }

    /// Construct a response with HTTP status `status` and a `Content-Type`
    /// header with value `text/<content_type>; charset=utf-8` from `error`.
    ///
    /// The default status is `HttpStatus::InternalServerError`.
    ///
    /// The default `Content-Type` is `text/plain; charset=utf-8`.
    ///
    /// The response body will be the [localized
    /// ](../../error/enum.Error.html#method.localize) `error`, or `unknown`.
    ///
    pub fn error(
        status: Option<HttpStatus>,
        content_type: Option<&str>,
        error: Option<Error>,
    ) -> Self {
        use crate::t;
        let status = status.unwrap_or(HttpStatus::InternalServerError);
        Self {
            status: status,
            headers: vec![(
                "Content-Type".to_string(),
                format!(
                    "text/{}; charset=utf-8",
                    content_type.unwrap_or("plain"),
                ),
            )],
            body: HttpRespBody::Str(format!(
                "{}: {}",
                t!(&status.to_string()),
                match error {
                    Some(e) => e.localize(),
                    None => "unknown".to_string(),
                }
            )),
        }
    }

    /// Construct a response that sends a file.
    ///
    /// `path` is the absolute file system path if it starts with `/`,
    /// relative to the HTTP server's working directory if not.
    ///
    /// The HTTP server adapter is expected to provide a `Content-Type`
    /// header.
    ///
    pub fn download(path: String) -> Self {
        Self {
            status: HttpStatus::Ok,
            headers: Vec::new(),
            body: HttpRespBody::Download(PathBuf::from(path)),
        }
    }

    /// Construct a response from a `Result<String, Error`.
    ///
    /// If `result` is `Ok` this is [`utf8()`](#method.utf8) with
    /// `content_type` and status `200`.
    ///
    /// If not, this is [`error()`](#method.error) with content type
    /// `text/plain` and status `500`.
    ///
    pub fn from_result(
        body: Result<String, Error>,
        content_type: &str,
    ) -> Self {
        match body {
            Ok(s) => Self::utf8(None, Some(content_type), s),
            Err(e) => Self::error(None, None, Some(e)),
        }
    }

    /// Construct a response with
    /// - status `HttpStatus::Ok`,
    /// - `Content-Type` header `text/html; charset=utf-8`.
    /// - body `body`
    ///
    pub fn html(body: String) -> Self {
        Self::utf8(None, Some("html"), body)
    }

    /// Construct a response with
    /// - status `HttpStatus::Ok`,
    /// - `Content-Type` header `text/json; charset=utf-8`.
    /// - body `body`
    ///
    pub fn json(json: String) -> Self {
        Self::utf8(None, Some("json"), json)
    }

    /// Construct an empty response.
    ///
    /// The default status is `HttpStatus::InternalServerError`.
    ///
    pub fn new() -> Self {
        Self {
            status: HttpStatus::InternalServerError,
            headers: Vec::new(),
            body: HttpRespBody::None,
        }
    }

    /// Construct an empty response with status `HttpStatus::Ok`.
    ///
    pub fn ok() -> Self {
        Self {
            status: HttpStatus::Ok,
            headers: Vec::new(),
            body: HttpRespBody::None,
        }
    }

    /// Construct a redirect response.
    ///
    /// `url` is the url to redirect to. The HTTP status is set to
    /// `HttpStatus::SeeOther`, meaning that the browser should send a `GET`
    /// request for `url`.
    ///
    pub fn redirect(url: String) -> Self {
        Self {
            status: HttpStatus::SeeOther,
            headers: vec![(String::from("Location"), url)],
            body: HttpRespBody::None,
        }
    }

    /// Construct a response with
    /// - status `HttpStatus::Ok`,
    /// - `Content-Type` header `text/plain; charset=utf-8`.
    /// - body `text`
    ///
    pub fn plain(text: &str) -> Self {
        Self::utf8(None, None, text.to_string())
    }

    /// Construct a response with
    /// - status `HttpStatus::Ok`,
    /// - `Content-Type` header `text/plain; charset=utf-8`.
    /// - body `text`
    ///
    pub fn string(text: String) -> Self {
        Self::utf8(None, None, text)
    }

    /// Construct a response with HTTP status `status` and a `Content-Type`
    /// header with value `text/<content_type>; charset=utf-8` from `body`.
    ///
    /// The default status is `HttpStatus::Ok`.
    ///
    /// The default `Content-Type` is `text/plain; charset=utf-8`.
    ///
    pub fn utf8(
        status: Option<HttpStatus>,
        content_type: Option<&str>,
        body: String,
    ) -> Self {
        Self {
            status: status.unwrap_or(HttpStatus::Ok),
            headers: vec![(
                "Content-Type".to_string(),
                format!(
                    "text/{}; charset=utf-8",
                    content_type.unwrap_or("plain"),
                ),
            )],
            body: HttpRespBody::Str(body),
        }
    }

    // - - modifiers - - - - - - - - - - - - - - - - - - - - - - - - - - - - -

    /// Add a header with name `nam` and value `val`. Any previous header with
    /// the same name is kept.
    ///
    pub fn add_header(mut self, nam: &str, val: &str) -> Self {
        self.headers
            .push((nam.trim().to_string(), val.trim().to_string()));
        self
    }

    /// Add a `Content-Disposition: attachment` header.
    ///
    /// Do not use if `self` is constructed by [`download`](#method.download).
    ///
    pub fn attach(self, filename: Option<&str>) -> Self {
        let mut val = "attachment".to_string();
        if let Some(f) = filename {
            val = val + "; filename=\"" + f + "\"";
        }
        self.add_header("Content-Disposition", &val)
    }

    /// Substitute `body`.
    ///
    pub fn body(mut self, body: HttpRespBody) -> Self {
        self.body = body;
        self
    }

    /// Insert a header with name `nam` and value `val`. Any previous header
    /// with the same name (case insensitive) is replaced.
    ///
    pub fn insert_header(mut self, nam: &str, val: &str) -> Self {
        self.headers
            .retain(|h| h.0.to_lowercase() != nam.to_lowercase());
        self.headers
            .push((nam.trim().to_string(), val.trim().to_string()));
        self
    }

    pub fn status(mut self, status: HttpStatus) -> Self {
        self.status = status;
        self
    }

    // - - accessors - - - - - - - - - - - - - - - - - - - - - - - - - - - - -

    /// ### For HTTP server adapter developers only
    ///
    /// Consumes `self`!
    ///
    pub fn get_body(self) -> HttpRespBody {
        self.body
    }

    /// ### For HTTP server adapter developers only
    ///
    /// Return the value of a header with `name`, case insensitive. If there
    /// is more than one header with `name` the choice is unpredictable.
    ///
    pub fn get_header(&self, name: &str) -> Option<String> {
        self.headers
            .iter()
            .find(|(n, _)| n.to_lowercase() == name.to_lowercase())
            .map(|(_, v)| v.clone())
    }

    /// ### For HTTP server adapter developers only
    ///
    /// Return a string with the headers.
    ///
    /// Inserts a colon and a space between name and value, and `CRLF` after
    /// each header.
    ///
    pub fn get_headers(&self) -> String {
        let mut result = String::new();
        for header in &self.headers {
            result = result + &header.0 + ":";
            if !header.1.is_empty() {
                result = result + " " + &header.1;
            }
            result += "\r\n";
        }
        result
    }

    /// ### For HTTP server adapter developers only
    ///
    pub fn get_status(&self) -> HttpStatus {
        self.status
    }

    /// ### For HTTP server adapter developers only
    ///
    /// Return an iterator yielding pairs (headers name, header value).
    ///
    /// Drains the headers!
    ///
    pub fn drain_headers(&mut self) -> std::vec::Drain<(String, String)> {
        self.headers.drain(..)
    }
}

// --- HttpServerIf ----------------------------------------------------------

/// The Vicocomo interface to an HTTP server.
///
/// The server is configurated by the web application developer using a
/// `config` macro, which is written by the server adapter developer. This
/// means that the application is not 100% server independent. To minimize the
/// server dependence there is a [`Config`](../config/struct.Config.html) that
/// is used by the adapter developer to implement the `config` macro, which is
/// <b>required</b> to accept input as documented here.
///
/// An implementation of this interface is a parameter to the controller
/// methods called by the code generated by the `config macro.
///
/// <small>*Knowledge of the structure and inner workings of `Config` is not
/// necessary for developing an application using the `config` macro. See
/// examples in the `examples` directory.*</small>
//
/// # `config` Macro Input Syntax
///
/// An HTTP server adapter's `config` macro takes input of the form
/// ```text
/// level_1(value) { level_2 { level_3: value, ... }, ... }, ...
/// ```
/// where `level_1`, `level_2`, and `level_3` are identifiers while the
/// `value`s are
/// - *an identifer*,
/// - *a bool, char, float, integer, or string literal*,
/// - *a Rust path*,
/// - `[` *any of the above*`, ...` *(more of the same)*` ]`, or
/// - `(` *a type*`, `*an expression*` )`,
///
/// The level 1 `value` and its parentheses are optional as well as the braced
/// groups and braces if empty. `level_3` may be present without any
/// `level_2`, and in this case there should be a single pair of braces.
///
/// The combination of `level_1` and its `value` should be globally unique.
/// `level_2` and `level_3` should be unique within their brace group.
/// "Unique" means that a later entry will replace an earlier.
///
/// # Arguments Recognized By Any `config` Macro
///
/// ## Level 1 `app_config`
///
/// Various attributes configuring the generated application code. No level 1
/// value or level 2 identifiers. Generally recognized level 3 identifiers are
///
/// ### `controller_prefix`
///
/// The value should be a Rust path that is the controller's module prefix,
/// see [`route`](#controller-path-and-handling-method) below.
///
/// Optional, default `crate::controllers`.
///
/// ### `create_session_table`
///
/// A string literal that is the SQL to create a table to store HTTP session
/// data. The [HTTP server adapter uses this
/// ](../config/struct.HttpDbSession.html#method.new) to create the table if it
/// does not exist.
///
/// `create_session_table: true` gives the default value
/// `"CREATE TABLE __vicocomo__sessions(id BIGINT, data TEXT, time BIGINT)"`.
///
/// Optional, no default if not present.
///
/// ### `data_dir`
///
/// The direcotry root of the application's resoruces.
///
/// Optional, the default as defined by the HTTP server adapter or `""`
/// meaning the working directory of the HTTP server.
///
/// ### `file_root`
///
/// The value should be a string literal, the file system path that is the
/// root of a file system relative path given in a [`route_static`
/// ](#level-1-route_static) entry or an argument to [`resp_download()`
/// ](#method.resp_download).
///
/// If no leading slash, the working directory of the HTTP server is
/// prepended.
///
/// If not empty and no ending slash, a slash is appended.
///
/// Optional, default `""` meaning the working directory of the HTTP server.
///
/// ### `resource_dir`
///
/// The direcotry root of the application's resoruces.
///
/// Optional, the default as defined by the HTTP server adapter or `""`
/// meaning the working directory of the HTTP server.
///
/// ### `role_enum`
///
/// The value defines [role-based access control](#role-based-access-control)
/// (RBAC) as follows:
/// - <b>`false`:</b> RBAC is not used.
/// - <b>`true`:</b> RBAC is used, and the role `enum` is
///   `crate::models::UserRole`.
/// - <b>a Rust path:</b> RBAC is used, and the value is the path to the role
///   `enum`.
///
/// Optional, default `true` if `role_variants` are defined, otherwise
/// `false`.
///
/// ### `role_variants`
///
/// The value should be an array of role identifiers, needed by the
/// [authorization](#level-1-authorize) mechanism below.
///
/// Ignored if `role_enum` is `false`. Otherwise optional, default an empty
/// array.
///
/// The predefined role `Superuser` is added if omitted.
///
/// ### `strip_mtime`
///
/// The value should be `true` or `false`. Works together with
/// [`view::make_href()`](../../view/fn.make_href.html). If `true` a dash
/// followed by 10 digits at the end of the file name before the file
/// extension will be stripped before finding a file to serve.
///
/// Optional, default `false`.
///
/// ### `texts_config`
///
/// The path of the configuration file for the text translation [`t()`
/// ](../../macro.t.html) macro.
///
/// The value should be `true` or a string literal that is a file system path.
///
/// The HTTP server adapter uses this to initialize translations:
///
/// - If no value is given text translation is not avaialble.
///
/// - If the value is `true` the text translation is initialized with the
///   [default value](../../texts/index.html#defining-texts).
///
/// - If the value is a string literal with a leading slash it is taken as the
///   file system path of the configuration file.
///
/// - If no leading slash, the working directory of the HTTP server is
///   prepended.
///
/// ### `unauthorized_route`
///
/// The value should be a string, the route to redirect to if authentications
/// fails. Note that this is relative to the attribute `url_root`, see below.
///
/// Ignored if `role_enum` is `false`. Otherwise optional, default "/".
///
/// ### `url_root`
///
/// The value should be a string literal, the URL path that is prepended to
/// any URL given in [`authorize(...)`](#level-1-authorize), [`route(...)`
/// ](#level-1-route-and-not_found), [`route_static(...)`
/// ](#level-1-route_static), `unauthorized_route` (see above), or
/// [`view::make_href()` ](../../view/fn.make_href.html).
///
/// Also prepended to an URL argument to [`resp_redirect()`
/// ](#method.resp_redirect) that starts with '/'.
///
/// A leading slash is inserted if missing, a trailing one is removed if
/// present (`""` is left alone, `"/"` is converted to `""`).
///
/// Optional, default `""`.
///
/// ### Server Adapter Specific Attributes
///
/// A server adapter may use its own `app_config` attributes as needed. All
/// attributes defined (and some default values) are accesible by
/// [`app_config()`](#method.app_config)
///
/// ## Level 1 `authorize`
///
/// ### Role Based Access Control
///
/// RBAC may be implemented by giving the `app_config` attribute [`role_enum`
/// ](#role_enum) a value that is not `false`.  The application must implement
/// the trait [`UserRole`](../../authorization/trait.UserRole.html) for the
/// role `enum`.
///
/// ### Route Pattern Authorization
///
/// The level 1 value (between parentheses) is an authorization pattern.  It
/// must match the entire route (stripping the `app_config` attribute
/// [`url_root`](#url_root) if defined). A slash at the beginning is ignored.
/// It may end with "/*", which matches "" and any string starting with "/".
///
/// If there is no level 2 identifier: The level 3 identifier should be an
/// HTTP method (case insensitive). The value is the role `enum` variant that
/// is authorized, or an array of them.
///
/// If there are level 2 identifiers, they should be case insensitive HTTP
/// methods. For each method, the level 3 identifier is either `allow` or
/// `filter`, and the value is again one or more role `enum` variants.
///
/// See below about the use of [`filter`](#filtering-access-control).
///
/// When choosing authorized roles, the longest (up to "/*") matching pattern
/// is used.  If two patterns have the same length and one of them ends in a
/// wildcard and the other not, the latter is chosen.  If none or both of them
/// ends in a wildcard, the first is chosen.
///
/// Example:
/// ```text
/// authorize("/my-route/*") { get { allow: SomeRole } },
/// authorize("/my-route/specific") { get { allow: OtherRole } },
/// authorize("/my-route/general/*") { get: ThirdRole },
/// ```
/// will authorize SomeRole to `/my-route`, `/my-route/whatever`, and
/// `/my-route/specific/whatever` but not to `/my-route/specific`,
/// `/my-route/general`, or `/my-route/general/whatever`.
///
/// OtherRole is authorized only to `/my-route/specific`, while ThirdRole is
/// authorized to `/my-route/general` and `/my-route/general/whatever`.
///
/// The route pattern may include parameters in angle brackets, e.g.
/// `"path/<id>/with/<par>/two/parameters"`, which match any value in that
/// position in the actual path.
///
/// ### Predefined and Pseudo Roles
///
/// The predefined role `Superuser` is always authorized to everything,
/// ignoring route pattern authorization except if `filter`ed, see [below
/// ](#filtering-access-control).
///
/// A user that has the (optional) role `Disabled` is denied access to all
/// routes that do not explicitly allow `Disabled`.
///
/// The pseudo role `Authenticated` is equivalent to an array containing all
/// defined roles, including `Disabled`.
///
/// The pseudo role `Enabled` is equivalent to an array containing all defined
/// roles except `Disabled`.  If `Disabled` is not defined, this is equivalent
/// to `Authenticated`.
///
/// The pseudo role `Public` authorizes access to all clients, ignoring
/// authentication.
///
/// Apart from the predefined `Superuser` there can be any number of roles
/// with arbitrary names except the pseudo role names above. The application
/// must implement the role `enum` as defined by the `app_config` attributes
/// [`role_enum`](#role_enum) and [`role_variants`](#role_variants).
///
/// ### Filtering Access Control
///
/// Using `filter` instead of `allow` when authorizing a role or an array of
/// them means that access is granted only if the handler controller's method
/// [`filter_access()`
/// ](../../controller/trait.Controller.html#method.filter_access) returns
/// `true`.  Use `filter` and override `filter_access()` e.g. to allow the
/// logged in user access to their own profile but noone elses, like so:
///
/// ```text
/// // in the config macro the route definition ...
/// route(Users) { create, delete, edit_form, index, patch, show },
/// // ... will create the following paths:
/// //     create     POST  /users
/// //     delete     POST  /users/<id>/delete
/// //     edit_form  GET   /users/<id>/edit
/// //     index      GET   /users
/// //     patch      POST  /users/<id>
/// //     show       GET   /users/<id>
/// // and the authorization ...
///     authorize("/users/<id>/*") {
///         get { filter: Authenticated },
///         post { filter: Authenticated },
///     },
///     authorize("/users/<id>/delete") {
///         post { allow: [] },
///     },
/// // ... together with the following Users controller code ...
///     impl Controller for Users {
///         fn filter_access(db: DatabaseIf, srv: HttpServerIf) -> bool {
///             use crate::models::UserSession; // supposing a UserSession ...
///             srv.req_route_par_val::<u32>("id")
///                 .map(|id| {             // ... with an auth_id() method
///                     id == UserSession::auth_id(db, srv)
///                 })
///                 .unwrap_or(false)
///         }
///         // ... handlers etc
///     }
/// // ... will authorize a logged in user with id 42 to only the following
/// // routes:
/// //     GET   /users/42
/// //     GET   /users/42/edit
/// //     POST  /users/42/patch
/// ```
/// ### Token Authentication
///
/// TODO
///
/// ## Level 1 `plug_in`
///
/// Plug in an object implementing a trait. Generally recognized level 1
/// values are:
/// - `DbConn`: The plugin implements [`DbConn`
///   ](../../database/trait.DbConn.html). Optional, default [`NullConn`
///   ](../../database/struct.NullConn.html).
/// - `TemplEng`: The plugin implements [`TemplEng`
///   ](../config/trait.TemplEng.html). Optional, default [`NullTemplEng`
///   ](../config/struct.NullTemplEng.html).
///
/// All require no level 2 and one level 3 arg `def`:
/// ```text
/// plug_in(SomeTrait) {
///     def: (
///         <a type implementing SomeTrait>,
///         <an expression evaluating to an instance of the type>,
///     ),
/// },
/// ```
///
/// ## Level 1 `route` and `not_found`
///
/// Route configuration. At least one route must obviously be defined.
/// Example follows.  See [Controller path and handling method
/// ](#controller-path-and-handling-method) below for the meaning of
/// "*Control*" in `route(`*Control*`)`.
/// ```text
///                        // HTTP | Path (params in <>)  | ctrl | method
///                        // =====+======================+======+============
/// route(Rsrc) {          //   CRUD requests, only those given are generated
/// // Create request         -----+----------------------+------+------------
///   new_form,            // get  | "/rsrc/new"          | Rsrc | new_form
///   copy_form,           // get  | "/rsrc/<id>/copy"    | Rsrc | copy_form
///   create,              // post | "/rsrc"              | Rsrc | create
///   ensure,              // post | "/rsrc/ensure"       | Rsrc | ensure
/// // Read request           -----+----------------------+------+------------
///   index,               // get  | "/rsrc"              | Rsrc | index
///   show,                // get  | "/rsrc/<id>"         | Rsrc | show
/// // Update request         -----+----------------------+------+------------
///   edit_form,           // get  | "/rsrc/<id>/edit"    | Rsrc | edit_form
///   patch,               // post | "/rsrc/<id>"         | Rsrc | patch
///   replace,             // post | "/rsrc/<id>/replace" | Rsrc | replace
/// // Delete request         -----+----------------------+------+------------
///   delete,              // post | "/rsrc/<id>/delete"  | Rsrc | delete
/// },                     // =====+======================+======+============
/// route(Cust) {          //   Methods may be customized |      |
///   custom {             // -----+----------------------+------+------------
///     http_method: post, //   Default GET
///     path: "path",      // post | "/path"              | Cust | custom
/// } },                   // =====+======================+======+============
/// route(Sing) {          //   Example: configure a singleton resource
///   new_form,            // get  | "/sing/new"          | Sing | new_form
///   ensure,              // post | "/sing/ensure"       | Sing | ensure
///   show                 //   Full path must be given if leading slash
///   { path: "/sing" },   // get  | "/sing"              | Sing | show
///   edit_form            //   Resource snake prepended if no leading slash
///   { path: "edit" },    // get  | "/sing/edit"         | Sing | edit_form
///   patch { path: "" },  // post | "/sing"              | Sing | patch
///   replace              //      |                      |      |
///   { path: "replace" }, // post | "/sing/replace"      | Sing | replace
///   delete               //      |                      |      |
///   { path: "delete" },  // post | "/sing/delete"       | Sing | delete
/// },                     // =====+======================+======+============
/// route(Othr) {          //   Other customizations
///   parm_req {           //   Customized path parameters are btw angle br.
///     path: "some/<par>" // get  | "/some/<par>"        | Othr | parm_req
///   },                   // -----+----------------------+------+------------
///   post_req {           //   Except for the standardized CRUD requests
///     http_method: post, // above, GET is the default HTTP method
///     path: "postpth",   // post | "/postpth"           | Othr | post_req
/// } },                   // =====+======================+======+============
/// route(Upl) {           //   Handle file upload if level 3 upload present:
///   hndl_upl {           //   The default http_method is now POST
///     path: "hndl_path", //   HTTP path must be given, "a-field" is the form
///     upload: "a-field", // field, req_body() -> HttpReqBodyPart::Uploaded
///   },                   // post | "/hndl_path"         | Upl  | hndl_upl
/// },                     // =====+======================+======+============
/// // Not Found handler   //      |                      |      |
/// not_found(Hand) {func} //   All not handled elsewhere,| Hand | func
///                        // no default provided by parse()
/// ```
///
/// ### Prepending an URL root
///
/// Note that the path given in a `route` entry is relative to the
/// `app_config` attribute [`url_root`](#url_root).
///
/// ### Controller Path and Handling Method
///
/// The controller is given as `some::path::to::Controller`. If the path is a
/// single identifier, as in the examples, the [`controller_prefix` attribute
/// ](#controller_prefix) value (default `crate::controllers::`) is prepended.
///
/// The handling methods are called as
/// `some::path::to::Controller::handler(...)`. So the controller may be a
/// module, struct, or enum as long as the handling method does not have a
/// receiver.
///
/// Handling method signature:
/// ```text
/// (
///     vicocomo::DatabaseIf,
///     vicocomo::HttpServerIf,
///     vicocomo::TemplEngIf,
/// ) -> vicocomo::HttpResponse
/// ```
/// See [`DatabaseIf`](../../database/struct.DatabaseIf.html), [`TemplEngIf`
/// ](struct.TemplEngIf.html), and [`HttpResponse`](struct.HttpResponse.html).
///
/// ### File upload
///
/// `route(...) { some_handler { path: ..., upload: "some-field" }}` means
/// `some_handler` will be called with limited access to the request data; the
/// [`HttpReqBody`](struct.HttpReqBody.html) returned by [`req_body()`
/// ](#method.req_body) will have an empty `bytes` field, and the parts will
/// be limited to one [`HttpReqBodyPart::Uploaded`
/// ](enum.HttpReqBodyPart.html#variant.Uploaded) per uploaded file.
///
/// The handler may use [`handle_upload()`](#method.handle_upload) to store
/// the file(s).
///
/// The level 3 "field" should be the name of the `input type="file"` form
/// field. It is optional, default "file".
///
/// ## Level 1 `route_static`
///
/// Configure the server to serve static files from a file system directory.
///
/// The value is a string literal, the URL path. Note that this is relative to
/// the `app_config` attribute [`url_root`](#url_root). Leading and trailing
/// slashes are ignored.
///
/// No level 2.
///
/// Currently only one level 3 `fs_path` which should have a value that is a
/// string literal, the file system path. If it starts with a slash it is an
/// absolute file path, if not it is relative to the one given by the
/// `app_config` attribute [`file_root`](#file_root).
///
/// `fs_path` is optional, the default is the URL path value.
///
/// All directories must be explicitly given, e.g to access files from `"dir"`
/// and `"dir/sub"` you must do `route_static("dir"), route_static("dir/sub").
///
/// The server adapter uses [`handle_static_file()`
/// ](#method.handle_static_file) to serve the files. A web application should
/// not define a handler for serving static files on a per directory basis.
///
/// <small>Note: To define an URL for serving a specific file (not all files
/// in a directory), use [level 1 `route`](#level-1-route-and-not_found) and
/// write a handler that uses [`resp_download()`](#method.resp_download).
/// </small>
///
/// If the same URL is defined by `route` and `route_static`, the HTTP server
/// adapter shall choose the `route`.
///
// TODO: named routes and url_for().
//
#[derive(Clone, Copy)]
pub struct HttpServerIf<'srv, 'req>(
    &'srv dyn HttpServer,
    &'req dyn HttpRequest<'req>,
);

impl<'srv, 'req> HttpServerIf<'srv, 'req> {
    /// ### For HTTP server adapter developers only
    ///
    /// Create an interface to the `server`.
    ///
    pub fn new(
        server: &'srv impl HttpServer,
        request: &'req impl HttpRequest<'req>,
    ) -> Self {
        Self(server, request)
    }

    /// Get an attribute value as set by the HTTP server's `config` macro's
    /// [`app_config`](#level-1-app_config) entry.
    ///
    /// Note that an implementation is free to add its own HTTP server
    /// specific attributes.
    ///
    /// Literal values are represented in the obvious way. Identifiers and
    /// rust paths are converted to strings. Arrays are represented by `Vec`.
    /// Plugins are not expected as `app_config` values and shall return
    /// `None` even if defined.
    ///
    /// Note that entries that have default values are accessible here even
    /// if they are not defined in the `config` macro.
    ///
    pub fn app_config(self, id: &str) -> Option<AppConfigVal> {
        self.0.app_config(id)
    }

    /// Return the configured [`data_dir`](#data_dir).
    ///
    pub fn data_dir(self) -> PathBuf {
        self.app_config("data_dir").unwrap().str().unwrap().into()
    }

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
    /// [`url_path_to_dir()`](#method.url_path_to_dir). If not found,
    /// [`resp_error()`](#tymethod.resp_error).
    ///
    /// Then, appends `file` to the directory and [`resp_download()`
    /// ](#method.resp_download).
    ///
    /// <b> Errors </b>
    ///
    /// Returns [`Error::ThisCannotHappen("static-route-not-found")`
    /// ](../../error/enum.Error.html#variant.ThisCannotHappen) if the static
    /// route is not registered. This is a bug in the HTTP server adapter.
    ///
    pub fn handle_static_file(&self) -> Result<HttpResponse, Error> {
        static SPLIT: OnceLock<Regex> = OnceLock::new();
        let split = SPLIT.get_or_init(|| Regex::new(r"/").unwrap());
        let (url_path, file) = {
            let orig_path = self.req_path();
            let mut pieces: Vec<&str> = split.split(&orig_path).collect();
            let file = pieces.pop().unwrap();
            (pieces.join("/").to_string(), file.to_string())
        };
        self.url_path_to_dir(&url_path)
            .map(|dir| self.resp_download(&(dir + &file)))
            .ok_or_else(|| {
                Error::this_cannot_happen("static-route-not-found")
            })
    }

    /// Handle a [file upload](#handle-file-upload) request, persisting the
    /// uploaded file(s).
    ///
    /// `files` should have one element for each [`HttpReqBodyPart`
    /// ](struct.HttpReqBodyPart.html#variant.Uploaded) in the [`HttpReqBody`
    /// ](struct.HttpReqBody) returned by [`req_body()`](#method.req_body).
    /// The element determines where to store the corresponding uploaded file:
    /// - `None`: The file is not permanently stored.
    /// - `Some(`*a path that starts with `'/'`*`)`: An absolute path.
    /// - `Some(_)`: A path relative to the [`file_root`](#file_root).
    ///
    /// After the call, the entries in [`HttpReqBody.parts`
    /// ](struct.HttpReqBody.html#structfield.parts) that correspond to
    /// persisted files are removed,
    ///
    /// <b> Errors </b>
    ///
    /// Returns an error if there are more entries in `files` than in
    /// [`HttpReqBody.parts`](struct.HttpReqBody.html#structfield.parts), or
    /// if any of them is not a [`HttpReqBodyPart::Uploaded`
    /// ](struct.HttpReqBodyPart.html#variant.Uploaded).
    ///
    /// Forwards any other errors from the HTTP server adapter, e.g. file
    /// access etc.
    ///
    pub fn handle_upload(
        self,
        files: &[Option<&std::path::Path>],
    ) -> Result<(), Error> {
        self.0.handle_upload(files)
    }

    /// The parameter values in the URL (get) or body (post) as a
    /// `serde_json::Value`.
    ///
    /// The parameters may be structured à la PHP:
    // No doc test, but see the unit test test_form_data()
    /// ```text
    /// smpl=1&arr[]=2&arr[]=3&map[a]=4&map[b]=5&deep[c][]=6&deep[c][]=7&deep[d]=8&mtrx[][]=9
    /// -> json!({
    ///     "smpl": "1",
    ///     "arr":  ["2", "3"],
    ///     "map":  { "a": "4", "b": "5" },
    ///     "deep": { "c": ["6", "7"], "d": "8" },
    ///     "mtrx": [["9"]],
    /// })
    /// ```
    /// Note that all leaf values are strings.
    ///
    pub fn param_json(&self) -> Result<JsonValue, Error> {
        FormData::parse(self.1.param_vals()).and_then(|fd| {
            serde_json::to_value(&fd)
                .map_err(|e| Error::invalid_input(&e.to_string()))
        })
    }

    /// The value of the parameter with `name` in the URL (get) or body (post)
    /// deserialized from a URL-decoded string.
    ///
    /// If more than one parameter with `name` is given, the first value is
    /// returned.
    ///
    /// For structured parameters, use [`param_json()`](#method.param_json).
    ///
    /// A parameter present in the URL or body but without a value (no equals
    /// sign) should be fetched as a `String` and will return `Some("")`. This
    /// is how PHP does it.
    ///
    pub fn param_val<T>(self, name: &str) -> Option<T>
    where
        T: DeserializeOwned,
    {
        self.1.param_val(name).and_then(|s| {
            serde_json::from_str(&s)
                .or_else(|_| serde_json::from_str(&format!("\"{s}\"")))
                .ok()
        })
    }

    /// prepend url_root if starts with '/'
    ///
    pub fn prepend_url_root(&self, url_path: &str) -> String {
        if url_path.starts_with('/') {
            self.app_config("url_root").unwrap().str().unwrap() + url_path
        } else {
            url_path.to_string()
        }
    }

    /// The body of the request.
    ///
    pub fn req_body(self) -> HttpReqBody<'req> {
        self.1.body()
    }

    /// Get a header. The key is case insensitive. If there are more than one
    /// header with `name` one value is arbitrarily chosen.
    ///
    pub fn req_header(self, name: &str) -> Option<HttpHeaderVal> {
        self.1.header(name).map(|s| HttpHeaderVal::from_str(&s))
    }

    /// True iff the request `Content-Type` is `multipart/form-data`.
    ///
    pub fn req_is_multipart(self) -> bool {
        self.req_header("content-type")
            .map(|hdr| hdr.value == "multipart/form-data")
            .unwrap_or(false)
    }

    /// The path part of the request, without scheme, host, or parameters.
    /// The [`url_root`](#url_root) <b>is stripped</b> from the path.
    ///
    pub fn req_path(self) -> String {
        self.strip_url_root(&self.1.path())
    }

    /// If registered as `"a/<p1>/<p2>"` and the HTTP path of the request is
    /// `"a/42/Hello"`, and a local variable `srv: HttpServerIf`, the
    /// following holds:
    /// ```text
    /// assert!(srv.req_route_par_val::<i32>("p1").unwrap() == 42);
    /// assert!(srv.req_route_par_val::<String>("p2").unwrap() == "Hello");
    /// ```
    ///
    pub fn req_route_par_val<T>(self, par: &str) -> Option<T>
    where
        T: DeserializeOwned,
    {
        self.1.route_par_val(par).and_then(|s| {
            serde_json::from_str(&s)
                .or_else(|_| {
                    serde_json::from_str(&("\"".to_string() + &s + "\""))
                })
                .ok()
        })
    }

    /// If registered as `"a/<p1>/<p2>"` and the HTTP path of the request is
    /// `"a/42/Hello"`, this will return
    /// ```text
    /// vec![
    ///     (String::from("p1"), String::from("42")),
    ///     (String::from("p2"), String::from("Hello")),
    /// ]
    /// ```
    pub fn req_route_par_vals(self) -> Vec<(String, String)> {
        self.1.route_par_vals()
    }

    /// The requested HTTP URL, including preferably scheme and host, always
    /// path, and, if applicable, query. The [`url_root`](#url_root) <b>is not
    /// stripped</b> from the path.
    ///
    pub fn req_url(self) -> String {
        self.1.url()
    }

    /// Return the configured [`resource_dir`](#resource_dir).
    ///
    pub fn resource_dir(self) -> PathBuf {
        self.app_config("resource_dir")
            .unwrap()
            .str()
            .unwrap()
            .into()
    }

    /// Generate a response to serve a static file.
    ///
    /// `file_path` is the absolute path of the file if it starts with '`/`',
    /// or relative to the [`file_root`](#file_root) if it does not.
    ///
    /// If [`strip_mtime`](#strip_mtime) is `true` and the `file_path` matches
    /// `[^/]+(-\d{10})(\.[^/.]+)?$`, the `-\d{10}` group is removed.
    ///
    pub fn resp_download(&self, file_path: &str) -> HttpResponse {
        HttpResponse::download(self.prepend_file_root(file_path))
    }

    /// Generate an [error response](struct.HttpResponse.html#method.error).
    ///
    pub fn resp_error(
        self,
        status: Option<HttpStatus>,
        err: Option<Error>,
    ) -> HttpResponse {
        HttpResponse::error(status, None, err)
    }

    /// Generate an [OK response](struct.HttpResponse.html#method.string) with
    /// body `txt`.
    ///
    pub fn resp_ok(self, txt: String) -> HttpResponse {
        HttpResponse::string(txt)
    }

    /// Generate a [redirect response
    /// ](struct.HttpResponse.html#method.redirect).
    ///
    /// If `url` starts with `/` the [`url_root`](#url_root) is prepended.
    ///
    pub fn resp_redirect(self, url: &str) -> HttpResponse {
        HttpResponse::redirect(self.prepend_url_root(url))
    }

    /// Clear the entire session.
    ///
    pub fn session_clear(self) {
        self.0.session_clear();
    }

    /// Return the session value for `key` as `T`.
    ///
    /// If there is no current value for `key`, or  the current value for
    /// `key` is not a `T`,`None` is returned.
    ///
    pub fn session_get<T>(self, key: &str) -> Option<T>
    where
        T: DeserializeOwned,
    {
        self.0
            .session_get(key)
            .and_then(|s| serde_json::from_str(&s).ok())
    }

    /// Remove the `key`-value pair.
    ///
    pub fn session_remove(self, key: &str) {
        self.0.session_remove(key)
    }

    /// Set a `value` for `key`.
    ///
    /// Returns an error if serializing fails.
    ///
    pub fn session_set<T>(self, key: &str, value: &T) -> Result<(), Error>
    where
        T: Serialize,
    {
        self.0.session_set(
            key,
            &serde_json::to_string(value)
                .map_err(|e| Error::other(&e.to_string()))?,
        )
    }

    /// Strip file_root if at beginning
    ///
    fn strip_file_root(&self, file_path: &str) -> String {
        file_path
            .strip_prefix(
                &self.app_config("file_root").unwrap().str().unwrap(),
            )
            .map(|dir| dir.to_string())
            .unwrap_or_else(|| file_path.to_string())
    }

    /// Strip url_root if at beginning.
    ///
    pub fn strip_url_root(self, url_path: &str) -> String {
        url_path
            .strip_prefix(
                &self.app_config("url_root").unwrap().str().unwrap(),
            )
            .map(|url| url.to_string())
            .unwrap_or_else(|| url_path.to_string())
    }

    /// Map URL to file directory for serving static files as defined by the
    /// `config` macro's [`route_static`](#level-1-route_static) entries.
    ///
    /// Before calling [`HttpServer::url_path_to_dir()`
    /// ](../config/trait.HttpServer.html#tymethod.url_path_to_dir)
    /// - a leading slash is added to `url_path` if missing,
    /// - a trailing slash is removed from `url_path` if present,
    /// - [`url_root`](#url_root) is prepended to `url_path`.
    ///
    /// Before returning the result, ensures that
    /// - the return value has a trailing slash,
    /// - the return value is absolute if it starts with a slash, relative to
    ///   [`file_root`](#file_root) if not.
    ///
    pub fn url_path_to_dir(&self, url_path: &str) -> Option<String> {
        use ljumvall_utils::fix_slashes;
        let url_path = fix_slashes(url_path, 1, -1);
        self.0
            .url_path_to_dir(&self.prepend_url_root(&url_path))
            .as_ref()
            .map(|dir| fix_slashes(&self.strip_file_root(dir), 0, 1))
    }

    // --- crate internal ----------------------------------------------------

    // prepend file_root if not starts with '/', strip mtime if strip_mtime
    pub(crate) fn prepend_file_root(&self, file_path: &str) -> String {
        static MTIME: OnceLock<Regex> = OnceLock::new();
        let mtime = MTIME.get_or_init(|| {
            Regex::new(r"([^/]+)-\d{10}(\.[^/.]+)?$").unwrap()
        });
        let stripped =
            if self.app_config("strip_mtime").unwrap().bool().unwrap()
                && mtime.is_match(file_path)
            {
                mtime.replace(file_path, "$1$2")
            } else {
                file_path.into()
            };
        if stripped.starts_with('/') {
            stripped.to_string()
        } else {
            self.app_config("file_root").unwrap().str().unwrap() + &stripped
        }
    }
}

// --- HttpStatus ------------------------------------------------------------

ljumvall_utils::back_to_enum! {

    /// The HTTP status codes as an `enum` that can be cast to the
    /// corresponding integer. It `Display`s that integer, too:
    /// ```
    /// assert_eq!(vicocomo::HttpStatus::Ok as usize, 200usize);
    /// assert_eq!(
    ///     vicocomo::HttpStatus::Ok.to_string(),
    ///     "vicocomo--http_status-200",
    /// );
    /// ```
    /// It also implements `TryFrom<i32>`.
    ///
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    pub enum HttpStatus {
        Continue = 100,
        SwitchingProtocols = 101,
        EarlyHints = 103,
        Ok = 200,
        Created = 201,
        Accepted = 202,
        NonAuthorativeInformation = 203,
        NoContent = 204,
        ResetContent = 205,
        PartialContent = 206,
        MultipleChoices = 300,
        MovedPermanently = 301,
        Found = 302,
        SeeOther = 303,
        NotModified = 304,
        UseProxy = 305,
        TemporaryRedirect = 307,
        PermanentRedirect = 308,
        BadRequest = 400,
        Unauthorized = 401,
        PaymentRequired = 402,
        Forbidden = 403,
        NotFound = 404,
        MethodNotAllowed = 405,
        NotAcceptable = 406,
        ProxyAuthenticationRequired = 407,
        RequestTimeout = 408,
        Conflict = 409,
        Gone = 410,
        LengthRequired = 411,
        PreconditionFailed = 412,
        RequestEntityTooLarge = 413,
        RequestUriTooLong = 414,
        UnsupportedMediaType = 415,
        RequestedRangeNotSatisfiable = 416,
        ExpectationFailed = 417,
        MisdirectedRequest = 421,
        UnprocessableEntity = 422,
        Locked = 423,
        TooManyRequests = 429,
        UnavailableForLegalReasons = 451,
        InternalServerError = 500,
        NotImplemented = 501,
        BadGateway = 502,
        ServiceUnavailable = 503,
        GatewayTimeout = 504,
        HttpVersionNotSupported = 505,
        InsufficientStorage = 507,
        NetworkAuthenticationRequired = 511,
    }
}

impl std::fmt::Display for HttpStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "vicocomo--http_status-{}", *self as u32)
    }
}

// --- TemplEngIf ------------------------------------------------------------

/// The Vicocomo interface to a template rendering engine.  Parameter to the
/// controller methods called by the code generated by a server specific
/// [`config`](struct.HttpServerIf.html#config-macro-input-syntax) macro.
///
#[derive(Clone)]
pub struct TemplEngIf(Arc<dyn TemplEng + Send + Sync>);

impl TemplEngIf {
    /// ### For HTTP server adapter developers only
    ///
    /// Create from a [`TemplEng`](../config/trait.TemplEng.html).
    ///
    pub fn new(eng: Arc<dyn TemplEng + Send + Sync>) -> Self {
        Self(Arc::clone(&eng))
    }

    /// ### For HTTP server adapter developers only
    ///
    /// See [`TemplEng::initialized()`
    /// ](../config/trait.TemplEng.html#method.initialized).
    ///
    pub fn initialized(&self) -> bool {
        self.0.initialized()
    }

    /// ### For HTTP server adapter developers only
    ///
    /// See [`TemplEng::register_templ_dir()`
    /// ](../config/trait.TemplEng.html#method.register_templ_dir).
    ///
    pub fn register_templ_dir(
        &self,
        path: &str,
        ext: &str,
    ) -> Result<(), Error> {
        self.0.register_templ_dir(path, ext)
    }

    /// Render, filling out `tmpl` with `data`.
    ///
    pub fn render(
        self,
        tmpl: &str,
        data: &impl Serialize,
    ) -> Result<String, Error> {
        self.0.render(
            tmpl,
            &serde_json::to_value(data)
                .map_err(|e| Error::render(&e.to_string()))?,
        )
    }
}

// --- private --------------------------------------------------------------

#[derive(Clone, Debug)]
enum FormData {
    Arr(Vec<FormData>),
    Map(HashMap<String, FormData>),
    Leaf(String),
}

impl FormData {
    // Expect self to be a Map and value to be a Leaf.
    // Depending on more_keys.next():
    // - None => insert value in self at key
    // - "" => recurse to push() to the Arr in self at key, or create the Arr
    // - => recurse to insert() in the Map in self at key
    fn insert(
        &mut self,
        key: String,
        mut more_keys: Peekable<CaptureMatches>,
        value: Self,
    ) -> Result<(), Error> {
        if let FormData::Map(ref mut map) = self {
            match more_keys.next() {
                None => {
                    map.insert(key, value);
                }
                Some(next_match) => {
                    let next_key = next_match.get(1).unwrap().as_str();
                    if next_key.len() == 0 {
                        if map.get(&key).is_none() {
                            map.insert(key.clone(), Self::Arr(Vec::new()));
                        }
                        map.get_mut(&key).unwrap().push(more_keys, value)?
                    } else {
                        if map.get(&key).is_none() {
                            map.insert(
                                key.clone(),
                                Self::Map(HashMap::new()),
                            );
                        }
                        map.get_mut(&key).unwrap().insert(
                            next_key.to_string(),
                            more_keys,
                            value,
                        )?
                    }
                }
            }
            Ok(())
        } else {
            Err(Error::invalid_input("self is not a Map variant"))
        }
    }

    // Expect self to be an Arr and value to be a Leaf.
    // Depending on more_keys.next():
    // - None => push value on self
    // - "" => recurse to push() to the Arr last in self
    // - => recurse to insert() in the Map last in self
    fn push(
        &mut self,
        mut more_keys: Peekable<CaptureMatches>,
        value: Self,
    ) -> Result<(), Error> {
        if let FormData::Arr(ref mut arr) = self {
            match more_keys.next() {
                None => arr.push(value),
                Some(next_match) => {
                    let next_key = next_match.get(1).unwrap().as_str();
                    if next_key.len() == 0 {
                        if arr.is_empty() {
                            arr.push(Self::Arr(Vec::new()));
                        }
                        arr.last_mut().unwrap().push(more_keys, value)?
                    } else {
                        if arr.is_empty() {
                            arr.push(Self::Map(HashMap::new()));
                        }
                        arr.last_mut().unwrap().insert(
                            next_key.to_string(),
                            more_keys,
                            value,
                        )?
                    }
                }
            }
            Ok(())
        } else {
            Err(Error::invalid_input("self is not an Arr variant"))
        }
    }

    // vals is [(<URL or body parameter name>, <URL-decoded value>), ...].
    // The parameter name should be e.g. "foo[bar][]" indicating that the
    // value is an element in an array that is a value with key "bar" in a
    // map that is the value with key "foo" in the returned result, which is
    // guaranteed to be Self::Map.
    fn parse(vals: Vec<(String, String)>) -> Result<Self, Error> {
        static BRACKETS: OnceLock<Regex> = OnceLock::new();
        let brackets =
            BRACKETS.get_or_init(|| Regex::new(r"\[([^]]*)\]").unwrap());
        let mut result = FormData::Map(HashMap::new());
        for (raw_key, raw_val) in vals {
            let val = Self::Leaf(raw_val);
            let mut nested = brackets.captures_iter(&raw_key).peekable();
            let key = if let Some(mtch) = nested.peek() {
                &raw_key[0..mtch.get(0).unwrap().start()]
            } else {
                &raw_key
            };
            result.insert(key.to_string(), nested, val)?;
        }
        Ok(result)
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_form_data() {
        assert_eq!(
            serde_json::to_value(
                FormData::parse(
                    vec![
                        ("smpl", "1"),
                        ("arr[]", "2"),
                        ("arr[]", "3"),
                        ("map[a]", "4"),
                        ("map[b]", "5"),
                        ("deep[c][]", "6"),
                        ("deep[c][]", "7"),
                        ("deep[d]", "8"),
                        ("mtrx[][]", "9"),
                    ]
                    .iter()
                    .map(|(k, v)| (k.to_string(), v.to_string()))
                    .collect(),
                )
                .unwrap(),
            )
            .unwrap(),
            json!({
                "smpl": "1",
                "arr":  ["2", "3"],
                "map":  { "a": "4", "b": "5" },
                "deep": { "c": ["6", "7"], "d": "8" },
                "mtrx": [["9"]],
            }),
        );
    }
}
