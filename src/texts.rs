//! Macros to simplify translating texts

/// Register texts for access with the [`t`](macro.t.html) macro.
///
/// # Raison d'être
///
/// Dead simple.  Of course, if you need to serve different languages without
/// recompiling, this is of no use.  Try [`fluent`
/// ](https://crates.io/crates/fluent).
///
/// # Usage
///
/// ### Note on namespacing
///
/// `register_texts` creates a module `vicocomo_text` in the module where it
/// is invoked.  [`t`](macro.t.html) expects this module to be
/// `crate::vicocomo_text`.  So, unless you invoke `register_texts` in your
/// `main.rs` or `lib.rs`, you must put a line
/// ```text
/// pub use the::module::you::invoke::register_texts::in::vicocomo_text
/// ```
/// in `main.rs` or `lib.rs`.
///
/// ### Macro input
///
/// The macro input is a number of comma-separated key-value pairs optionally
/// defining parameterized substitution like so:
/// ```text
/// register_texts! {
///     "simple"         => "some text without parameters",
///     "parameterized"  => "some text with <p1> two <p2> parameters",
///     "literal_angles" => "some text containing \"\\<\" with <par> \\>",
/// }
/// ```
///
#[macro_export]
macro_rules! register_texts {
    ( $( $key: literal => $text: literal ),* $( , )? ) => {
        pub mod vicocomo_text {
            use ::lazy_static::lazy_static;
            use ::std::collections::HashMap;
            lazy_static! {
                pub static ref TEXTS: HashMap <
                    &'static str, (
                        Vec<(&'static str, &'static str)>,
                        &'static str,
                    )
                > = {
                    let mut map = HashMap::new();
                $(  map.insert($key, find_params($text)); )*
                    map
                };
            }

            fn find_params(text: &'static str)
                -> (Vec<(&'static str, &'static str)>, &'static str)
            {
                use ::regex::Regex;
                lazy_static! {
                    static ref ANGLES: Regex =
                        Regex::new(r"[^\\]?<([^>]*[^\\])>").unwrap();
                }
                let mut befores_names = Vec::new();
                let mut last = 0;
                for captures in ANGLES.captures_iter(&text) {
                    let par = captures.get(1).unwrap();
                    befores_names.push(
                        (&text[last..(par.start() - 1)], par.as_str())
                    );
                    last = par.end() + 1;
                }
                (befores_names, &text[last..])
            }
        }
    }
}

/// Access a text defined by [`register_texts`](macro.register_texts.html) as
/// a `String`.
///
/// The first parameter is the key as defined in [`register_texts`
/// ](macro.register_texts.html).
///
/// If the text is parameterized, name-value pairs follow, like so:
/// ```text
/// register_texts! {
///     /* ... */
///     "example" => "example <p1> a <p2> text",
///     /* ... */
/// }
///
/// assert_eq!(
///     t!("example", "p1": "of", "p2": "parameterized"),
///     "example of a parameterized text",
/// );
/// ```
///
#[macro_export]
macro_rules! t {
    ($key: literal) => {
        t!($key,)
    };
    ($key: literal, $( $name: literal : $value: expr ),* ) => {
        {
            let entry = &crate::vicocomo_text::TEXTS.get($key).unwrap();
            let mut result = String::new();
            if entry.0.len() > 0 {
            $(
                result.extend(
                    entry.0.iter()
                        .find(|(_, name)| name == &$name)
                        .unwrap()
                        .0
                        .chars(),
                );
                result.extend($value.to_string().chars());
            )*
            }
            result.extend((entry.1).chars());
            result.replace("\\<", "<").replace("\\>", ">")
        }
    };
}
