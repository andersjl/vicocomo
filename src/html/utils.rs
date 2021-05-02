//! Helper types for producing HTML code.
//!
#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use std::fmt;

// --- HtmlTagAttr -----------------------------------------------------------

/// Represents an HTML attribute.
///
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct HtmlTagAttr {
    name: String,
    values: Option<Vec<String>>,
}

impl HtmlTagAttr {
    /// Create an HtmlTagAttr.
    ///
    /// `name` is the attribute name, `values` are whitespace-separated
    /// values.
    ///
    /// ```
    /// # use ::vicocomo::HtmlTagAttr;
    ///
    /// assert_eq!(
    ///     HtmlTagAttr::new("some-name", None).to_string(),
    ///     "some-name",
    /// );
    ///
    /// assert_eq!(
    ///     HtmlTagAttr::new("some-name", Some("")).to_string(),
    ///     "some-name=\"\"",
    /// );
    ///
    /// assert_eq!(
    ///     HtmlTagAttr::new("some-name", Some("val1  \n\t  val2"))
    ///         .to_string(),
    ///     "some-name=\"val1 val2\"",
    /// );
    ///
    /// assert_eq!(
    ///     HtmlTagAttr::new("some-name", Some("val1 val2 val1")).to_string(),
    ///     "some-name=\"val1 val2\"",
    /// );
    /// ```
    ///
    pub fn new(name: &str, values: Option<&str>) -> Self {
        let mut result = Self {
            name: name.to_string(),
            values: None,
        };
        match values {
            Some(vals) => result.add(vals),
            None => (),
        };
        result
    }

    /// Add one or more values to the attribute.
    ///
    /// `values` contains the values, separated by whitespace.
    ///
    /// After `add()` the attribute will display as
    /// `<`value of name`>="val1 val2"`.
    ///
    /// If this is the first add and `values` is empty, the attribute will
    /// display as `<`value of name`>=""`.
    ///
    pub fn add(&mut self, values: &str) {
        use ::itertools::Itertools;
        let mut vals = match &self.values {
            Some(_) => self.values.take().unwrap(),
            None => Vec::new(),
        };
        vals.extend(&mut values.split_whitespace().map(|s| s.to_string()));
        self.values = Some(vals.into_iter().unique().collect::<Vec<_>>());
    }

    /// Clear all values.  After `clear()`, the attribute will display as only
    /// a name with no value.
    ///
    pub fn clear(&mut self) {
        self.values = None;
    }

    /// Get the first value or `None`.
    ///
    pub fn first(&self) -> Option<String> {
        self.values
            .as_ref()
            .and_then(|vals| vals.first().map(|v| v.clone()))
    }

    /// The attribute name.
    ///
    pub fn name(&self) -> String {
        self.name.clone()
    }

    /// Get all values as `Some(`space-separated string`)` or `None`.
    ///
    pub fn values(&self) -> Option<String> {
        self.values.as_ref().map(|vals| vals.join(" "))
    }
}

impl fmt::Display for HtmlTagAttr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self.values.as_ref() {
                Some(vals) => {
                    self.name.clone() + r#"=""# + &vals.join(" ") + r#"""#
                }
                None => self.name.clone(),
            }
        )
    }
}

// --- HtmlTagInner ----------------------------------------------------------

/// Represents a part of the content of an HTML tag.
///
#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum HtmlTagInner {
    Text(String),
    Tag(HtmlTag),
}

impl fmt::Display for HtmlTagInner {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                HtmlTagInner::Text(text) => text.clone(),
                HtmlTagInner::Tag(tag) => tag.to_string(),
            }
        )
    }
}

// --- PathTag ---------------------------------------------------------------

/// A tag with a URL-valued attribute but no content.  See the
/// [`PathTag`](../../vicocomo_html_macros/derive.PathTag.html) derive.
///
pub trait PathTag {
    /// Set (replace) the path attribute value of the tag.
    fn set_path(&mut self, path: &str);

    /// Set (replace) an attribute (not the path attribute) to the tag.
    fn set_attr(&mut self, attr: &str, values: Option<&str>);
}

/*
// --- PathTagData -----------------------------------------------------------

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PathTagData {
    path_attr_name: String,
    tag_data: HtmlTag,
}

impl PathTagData {
    pub fn new(tag: &str, path_attr_name: &str) -> Self {
        result = Self {
            path_attr_name,
            tag_data: HtmlTag::new(tag),
            tag: a_tag.to_string(),
            path: HtmlTagAttr::new(path_attr_name, Some("#")),
            attrs: vec![],
            void: (&VOID_ELEMENTS).iter().find(|&&e| e == a_tag).is_some(),
        }
    }

    pub fn tag(&self) -> String {
        self.tag.clone()
    }

    pub fn path(&self) -> String {
        self.path.value()
    }

    pub fn clear(&mut self) {
        self.set_path("#");
        self.attrs = vec![];
    }
}

impl PathTag for PathTagData {
    fn add_attr(&mut self, attr: &HtmlTagAttr) {
        self.attrs.push(attr.clone());
    }

    fn set_path(&mut self, a_path: &str) {
        self.path.clear();
        self.path.push(a_path);
    }
}

impl fmt::Display for PathTagData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let end_tag = if self.void {
            "".to_string()
        } else {
            "</".to_string() + &self.tag + ">"
        };
        let attr_string = if self.attrs.is_empty() {
            "".to_string()
        } else {
            " ".to_string()
                + &self
                    .attrs
                    .iter()
                    .map(|x| x.to_string())
                    .collect::<Vec<_>>()
                    .join(" ")
        };
        write!(f, "<{} {}{}>{}", self.tag, self.path, attr_string, end_tag,)
    }
}
*/

// --- Script, ScriptTag -----------------------------------------------------

/// An encapsuled vector of [ScriptTag](struct.ScriptTag.html)-turned-strings.
///
#[derive(Debug, Deserialize, Serialize)]
pub struct Scripts(Vec<String>);
impl Scripts {
    pub fn new() -> Self {
        Self(vec![])
    }

    pub fn add(&mut self, script_tag: &ScriptTag) {
        self.0.push(script_tag.to_string());
    }

    /*
        pub fn iter(&self) -> ::std::slice::Iter<String> {
            self.0.iter()
        }
    */
}

/// A script tag with a `src` attribute
///
#[derive(Clone, Debug, Deserialize, Serialize, crate::PathTag)]
#[vicocomo_path_tag_data("script", "src")]
pub struct ScriptTag(HtmlTag);

// --- Styles, StyleTag ------------------------------------------------------

/// An encapsuled vector of [StyleTag](struct.StyleTag.html)-turned-strings.
///
#[derive(Debug, Deserialize, Serialize)]
pub struct Styles(Vec<String>);
impl Styles {
    pub fn new() -> Self {
        Self(vec![])
    }

    pub fn add(&mut self, style_tag: &StyleTag) {
        self.0.push(style_tag.to_string());
    }

    /*
        pub fn iter(&self) -> ::std::slice::Iter<String> {
            self.0.iter()
        }
    */
}

/// A link tag with an `href` attribute and `rel="stylesheet"`.
///
#[derive(Clone, Debug, Deserialize, Serialize, crate::PathTag)]
#[vicocomo_path_tag_data("link", "href")]
#[vicocomo_path_tag_attr("rel", "stylesheet")]
pub struct StyleTag(HtmlTag);

// --- HtmlTag ---------------------------------------------------------------

const VOID_ELEMENTS: [&str; 14] = [
    "area", "base", "br", "col", "embed", "hr", "img", "input", "link",
    "meta", "param", "source", "track", "wbr",
];

/// Represents a general tag
///
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct HtmlTag {
    tag: String,
    attrs: Vec<HtmlTagAttr>,
    inner: Vec<HtmlTagInner>,
    void: bool,
}

impl HtmlTag {
    /// Create an empty tag with no inner HTML.
    ///
    pub fn new(tag: &str) -> Self {
        Self {
            tag: tag.to_string(),
            attrs: Vec::new(),
            inner: Vec::new(),
            void: (&VOID_ELEMENTS).iter().find(|&&e| e == tag).is_some(),
        }
    }

    /// Add `values` (whitespace separated) to the attribute `attr`, or create
    /// it if not present.
    ///
    /// For details, see [`HtmlTagAttr::new()`
    /// ](struct.HtmlTagAttr.html#method.new).
    ///
    pub fn add_attr_vals(&mut self, attr: &str, values: &str) {
        match self.get_attr_mut(attr) {
            Some(html_attr) => html_attr.add(values),
            None => self.attrs.push(HtmlTagAttr::new(attr, Some(values))),
        };
    }

    /// Remove all attributes and inner HTML.
    ///
    pub fn clear(&mut self) {
        self.clear_attrs();
        self.clear_inner();
    }

    /// Remove all attributes.
    ///
    pub fn clear_attrs(&mut self) {
        self.attrs = Vec::new();
    }

    /// Remove all inner HTML.
    ///
    pub fn clear_inner(&mut self) {
        self.inner = Vec::new();
    }

    /// Forget `attr`. This is useful for boolean attributes.
    ///
    pub fn drop_attr(&mut self, attr: &str) {
        self.attrs.retain(|a| a.name != attr);
    }

    /// Get the current values of `attr` as a space-separated list.
    ///
    /// Returns `None` if the attribute is not known.
    ///
    /// Returns `Some("") if the attribute has no values, e.g. because it is
    /// boolean.
    ///
    pub fn get_attr(&self, attr: &str) -> Option<String> {
        self.attrs
            .iter()
            .find(|a| attr == a.name)
            .map(|a| a.values().unwrap_or_else(|| String::new()))
    }

    /// Get the first value of `attr` as a space-separated list.
    ///
    /// Returns `None` if the attribute is not known.
    ///
    /// Returns `Some("") if the attribute has no values, e.g. because it is
    /// boolean.
    ///
    pub fn get_attr_first(&self, attr: &str) -> Option<String> {
        self.attrs
            .iter()
            .find(|a| attr == a.name)
            .map(|a| a.first().unwrap_or_else(|| String::new()))
    }

    /// Iterate over all attrs.
    ///
    pub fn get_attrs(&self) -> ::std::slice::Iter<'_, HtmlTagAttr> {
        self.attrs.iter()
    }

    /// Iterate over inner HTML.
    ///
    pub fn get_inner(&self) -> ::std::slice::Iter<'_, HtmlTagInner> {
        self.inner.iter()
    }

    /// Push a tag to the tag's inner HTML
    ///
    pub fn push_tag(&mut self, tag: Self) {
        self.inner.push(HtmlTagInner::Tag(tag));
    }

    /// Push text to the tag's inner HTML
    ///
    pub fn push_text(&mut self, txt: &str) {
        self.inner.push(HtmlTagInner::Text(txt.to_string()));
    }

    /// Set the attribute `attr` to `values` (whitespace separated), removing
    /// previous values if any.
    ///
    /// To set a boolean attribute, use `set_attr("some_attr", None)`.
    ///
    /// For details, see [`HtmlTagAttr::add()`
    /// ](struct.HtmlTagAttr.html#method.add) and [`HtmlTagAttr::clear()`
    /// ](struct.HtmlTagAttr.html#method.clear).
    ///
    pub fn set_attr(&mut self, attr: &str, values: Option<&str>) {
        match self.get_attr_mut(attr) {
            Some(html_attr) => {
                html_attr.clear();
                match values {
                    Some(vals) => html_attr.add(vals),
                    None => (),
                }
            }
            None => self.attrs.push(HtmlTagAttr::new(attr, values)),
        };
    }

    /// A clone of the tag name.
    ///
    pub fn tag_name(&self) -> String {
        self.tag.clone()
    }

    // private ---------------------------------------------------------------

    fn get_attr_mut(&mut self, attr: &str) -> Option<&mut HtmlTagAttr> {
        self.attrs.iter_mut().find(|a| attr == a.name)
    }
}

impl fmt::Display for HtmlTag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let attr_string = if self.attrs.is_empty() {
            String::new()
        } else {
            " ".to_string()
                + &self
                    .attrs
                    .iter()
                    .map(|a| a.to_string())
                    .collect::<Vec<_>>()
                    .join(" ")
        };
        let inner_string: String;
        let end_tag: String;
        if self.void {
            inner_string = String::new();
            end_tag = String::new();
        } else {
            inner_string = String::new()
                + &self
                    .inner
                    .clone()
                    .into_iter()
                    .map(|i| match i {
                        HtmlTagInner::Text(text) => text,
                        HtmlTagInner::Tag(tag) => tag.to_string(),
                    })
                    .collect::<Vec<_>>()
                    .join("");
            end_tag = "</".to_string() + &self.tag + ">";
        };
        write!(
            f,
            "<{}{}>{}{}",
            self.tag, attr_string, inner_string, end_tag
        )
    }
}
