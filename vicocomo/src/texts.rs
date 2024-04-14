//! Simplify translating texts
//!
//! # Raison d'être
//!
//! Dead simple.  Of course, if you need to serve different languages without
//! recompiling, this is of no use.  Try [`fluent`
//! ](https://crates.io/crates/fluent).
//!
//! # Defining texts
//!
//! The texts are defined in a text file, default `config/texts.cfg`, as a
//! number of comma-separated key-value pairs optionally defining
//! parameterized substitution, see the example below.
//!
//! To initialize the module you must call [`initialize`](fn.initialize.html)
//! before using the [`t!()`](macro.t.html) macro.
//!
//! # Example
//! ```
//! use vicocomo::t;
//! std::fs::create_dir_all("config").unwrap();
//!
//! let _ = std::fs::write(
//!     "config/texts.cfg",
//!     r#"
//!     "simple"           => "some text without parameters",
//!     "parameterized"    => "some text with <p1> two <p2> parameters",
//!     "beginning-w-par"  => "<par> before text",
//!     "literal-angles"   => "some text containing \< with <par> \>",
//!     "#,
//! );
//! vicocomo::texts::initialize(None);
//! assert_eq!(
//!     t!("simple"),
//!     "some text without parameters",
//! );
//! assert_eq!(
//!     t!("parameterized", "p2": "(second)", "p1": "(first)"),
//!     "some text with (first) two (second) parameters",
//! );
//! assert_eq!(
//!     t!("parameterized", "p2": "(second)"),
//!     "some text with p1: ? two (second) parameters",
//! );
//! assert_eq!(
//!     t!("beginning-w-par", "par": "parameter"),
//!     "parameter before text",
//! );
//! assert_eq!(
//!     t!("literal-angles", "par": "parameter"),
//!     "some text containing < with parameter >",
//! );
//! assert_eq!(
//!     t!("unregistered", "p1": "of", "p2": "parameterized"),
//!     "unregistered, p1: of, p2: parameterized",
//! );
//! ```

/// Access a text defined in [`config/texts.cfg`](texts/index.html) as a `String`.
///
/// The first parameter is the key as defined in `config/texts.cfg`.
///
/// If the text is parameterized, name-value pairs follow, like so:
///
/// If `config/texts.cfg` contains
/// ```text
///     "example" => "example <p1> a <p2> text",
/// ```
/// The following assertion should hold:
/// ```text
/// assert_eq!(
///     t!("example", "p2": "parameterized", "p1": "of"),
///     "example of a parameterized text",
/// );
/// ```
/// If the `$key` is not in `config/texts.cfg`, the output is the key and the
/// parameters:
/// ```text
/// assert_eq!(
///     t!("unregistered", "p1": "of", "p2": "parameterized"),
///     "unregistered, p1: of, p2: parameterized",
/// );
/// ```
///
#[macro_export]
macro_rules! t {
    ($key:expr $( , $name:literal : $value:expr )* $( , )? ) => {
        {
            #[allow(unused_mut)]
            let mut params: Vec<(&str, &str)> = Vec::new();
        $(
            let val = $value.to_string();
            params.push(($name, &val));
        )*
            $crate::texts::get_text($key, params.as_slice())
        }
    };
}

use regex::Regex;
use std::collections::HashMap;
use std::path::Path;
use std::sync::OnceLock;

static TEXTS: OnceLock<
    HashMap<&'static str, (Vec<(&'static str, &'static str)>, &'static str)>,
> = OnceLock::new();

static DEFS: OnceLock<String> = OnceLock::new();

/// Read `cfg` and initialize the localization mechanism from it.
///
/// The default file is `config/texts.cfg`.
///
pub fn initialize(cfg: Option<&Path>) {
    TEXTS.get_or_init(|| {
        let defs = DEFS.get_or_init(|| {
            std::fs::read_to_string(
                cfg.unwrap_or(Path::new("config/texts.cfg")),
            )
            .unwrap_or(String::new())
        });
        let key_val_pair: Regex = Regex::new(
            r#""((?:[^"]|\\")*)"\s*=>\s*"((?:[^"]|\\")*)"(?:,|$)"#,
        )
        .unwrap();
        let mut map = HashMap::new();
        for key_vals in key_val_pair.captures_iter(defs.as_str()) {
            map.insert(
                key_vals.get(1).unwrap().as_str(),
                find_params(key_vals.get(2).unwrap().as_str()),
            );
        }
        map
    });
}

// find < > delimited parameters in text and return a pair (
//   a vector of pairs ( text before parameter, parameter name ),
//   text after the last parameter,
// )
#[doc(hidden)]
pub fn find_params(
    text: &'static str,
) -> (Vec<(&'static str, &'static str)>, &'static str) {
    static PARAM: OnceLock<Regex> = OnceLock::new();
    let param = PARAM.get_or_init(|| {
        Regex::new(r"((?:[^\\<]|\\<)*)<((?:[^>]|\\>)*)>").unwrap()
    });
    let mut befores_names = Vec::new();
    let mut last = 0;
    for captures in param.captures_iter(&text) {
        let par = captures.get(2).unwrap();
        befores_names.push((&text[last..(par.start() - 1)], par.as_str()));
        last = par.end() + 1;
    }
    (befores_names, &text[last..])
}

// params is [ ( param name, param value ), ... ] in arbitrary order
#[doc(hidden)]
pub fn get_text(key: &str, params: &[(&str, &str)]) -> String {
    let mut result = String::new();
    match TEXTS.get().and_then(|texts| texts.get(key)) {
        Some(entry) => {
            for (piece, par) in &entry.0 {
                result += piece;
                match params.iter().find(|(name, _)| name == par) {
                    Some((_, value)) => result += value,
                    None => {
                        result += par;
                        result += ": ?";
                    }
                }
            }
            result.extend((entry.1).chars());
        }
        None => {
            result += key;
            for (name, value) in params {
                result += ", ";
                result += name;
                result += ": ";
                result.extend(value.to_string().chars());
            }
        }
    }
    result
        .replace("\\<", "<")
        .replace("\\>", ">")
        .replace(r#"\""#, r#"""#)
}
