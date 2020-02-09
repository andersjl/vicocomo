#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use std::fmt;

// HtmlAttr ------------------------------------------------------------------
// To conveniently add several values to an HTML attribute and get a String.

// If values is None, to_string() yields name
// If values is empty, to_string() yields name=""
// If values is not empty, to_string() yields name="val1 val2 ..."
// it is ensured that values are unique and have len() > 0
#[derive(Clone, Debug)]
pub struct HtmlAttr {
    name: String,
    values: Option<Vec<String>>,
}

impl HtmlAttr {
    // Create an HtmlAttr.
    // name is the attribute name, values are whitespace-separated values.
    // See above.
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

    // Add one or more value to the attribute, separated by whitespace.
    // After add, the attribute will display as name="val1 val2".
    // If this is the first add and values is empty, the attribute will
    // display as name="".
    pub fn add(&mut self, values: &str) {
        use itertools::Itertools;
        let mut vals = match &self.values {
            Some(_) => self.values.take().unwrap(),
            None => vec![],
        };
        vals.append(
            &mut values
                .split_whitespace()
                .map(|s| s.to_string())
                .collect::<Vec<_>>(),
        );
        self.values = Some(vals.into_iter().unique().collect::<Vec<_>>());
    }

    // Clear all values.  After Clear, the attribute will display as only a
    // name with no value.
    pub fn clear(&mut self) {
        self.values = None;
    }

    // The attribute name.
    pub fn name(&self) -> String {
        self.name.clone()
    }

    // Get all values as Some(space-separated string) or None.
    pub fn values(&self) -> Option<String> {
        match &self.values {
            Some(vals) => Some(vals.join(" ")),
            None => None,
        }
    }
}

impl fmt::Display for HtmlAttr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match &self.values {
                Some(_vals) =>
                    self.name.clone() + "=\"" + &self.values().unwrap() + "\"",
                None => self.name.clone(),
            }
        )
    }
}

// InnerHtml enum ------------------------------------------------------------

#[derive(Clone, Debug)]
pub enum InnerHtml {
    Text(String),
    Tag(HtmlTag),
}

impl fmt::Display for InnerHtml {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                InnerHtml::Text(text) => text.clone(),
                InnerHtml::Tag(tag) => tag.to_string(),
            }
        )
    }
}

// HtmlTag struct ------------------------------------------------------------
// A general tag

const VOID_ELEMENTS: [&str; 14] = [
    "area", "base", "br", "col", "embed", "hr", "img", "input", "link",
    "meta", "param", "source", "track", "wbr",
];

#[derive(Clone, Debug)]
pub struct HtmlTag {
    tag: String,
    attrs: Vec<HtmlAttr>,
    inner: Vec<InnerHtml>,
    void: bool,
}

impl HtmlTag {
    // Create an empty tag with no inner HTML.
    pub fn new(a_tag: &str) -> Self {
        Self {
            tag: a_tag.to_string(),
            attrs: vec![],
            inner: vec![],
            void: (&VOID_ELEMENTS).iter().find(|&&e| e == a_tag).is_some(),
        }
    }

    // Add values to the attribute attr, or create it if not present.
    fn add_attr_vals(&mut self, attr: &str, values: &str) {
        match self.get_attr_mut(attr) {
            Some(html_attr) => html_attr.add(values),
            None => self.attrs.push(HtmlAttr::new(attr, Some(values))),
        };
    }

    // Remove all attributes and inner HTML.
    pub fn clear(&mut self) {
        self.attrs = vec![];
        self.inner = vec![];
    }

    // Set the named attribute, removing previous values if any.
    pub fn set_attr(&mut self, attr: &str, values: Option<&str>) {
        match self.get_attr_mut(attr) {
            Some(html_attr) => {
                html_attr.clear();
                match values {
                    Some(vals) => html_attr.add(vals),
                    None => (),
                }
            }
            None => self.attrs.push(HtmlAttr::new(attr, values)),
        };
    }

    // A clone of the tag name.
    pub fn tag_name(&self) -> String {
        self.tag.clone()
    }

    // private ------------------------

    fn get_attr_mut(&mut self, attr: &str) -> Option<&mut HtmlAttr> {
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
                        InnerHtml::Text(text) => text,
                        InnerHtml::Tag(tag) => tag.to_string(),
                    })
                    .collect::<Vec<_>>()
                    .join(" ");
            end_tag = "</".to_string() + &self.tag + ">";
        };
        write!(
            f,
            "<{}{}>{}{}",
            self.tag, attr_string, inner_string, end_tag
        )
    }
}

// PathTag trait -------------------------------------------------------------
// A tag with a URL-valued attribute but no content.  See PathTag derive.

pub trait PathTag {
    // Set (replace) the path attribute value of the tag.
    fn set_path(&mut self, a_path: &str);

    // Set (replace) an attribute (not the path attribute) to the tag.
    fn set_attr(&mut self, attr: &str, values: Option<&str>);
}

/*
#[derive(Clone, Debug)]
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
            path: HtmlAttr::new(path_attr_name, Some("#")),
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
    fn add_attr(&mut self, attr: &HtmlAttr) {
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

// ScriptTag -----------------------------------------------------------------
// A script tag with a src attribute -----------------------------------------
#[derive(Clone, Debug, vicocomo_derive::PathTag)]
#[path_tag_data("script", "src")]
pub struct ScriptTag(HtmlTag);

// An encapsuled vector of ScriptTag-turned-strings accessed by to_json().
#[derive(Deserialize, Serialize)]
pub struct Scripts(Vec<String>);
impl Scripts {
    pub fn new() -> Self {
        Self(vec![])
    }

    pub fn add(&mut self, script_tag: &ScriptTag) {
        self.0.push(script_tag.to_string());
    }
}

// StyleTag ------------------------------------------------------------------
// A link tag with an href attribute and rel="stylesheet".
#[derive(Clone, Debug, vicocomo_derive::PathTag)]
#[path_tag_data("link", "href")]
#[path_tag_attr("rel", "stylesheet")]
pub struct StyleTag(HtmlTag);

// An encapsuled vector of StyleTag-turned-strings accessed by to_json().
#[derive(Deserialize, Serialize)]
pub struct Styles(Vec<String>);
impl Styles {
    pub fn new() -> Self {
        Self(vec![])
    }

    pub fn add(&mut self, style_tag: &StyleTag) {
        self.0.push(style_tag.to_string());
    }
}