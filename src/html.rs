use serde::{Deserialize, Serialize};
use std::fmt;

pub trait PathTag {
    // Set (replace) the path attribute value of the tag.
    fn set_path(&mut self, a_path: &str);

    // Add an attribute (not the path attribute) to the tag.
    fn add_attr(&mut self, attr: &HtmlAttr);
}

const VOID_ELEMENTS: [&str; 14] = [
    "area", "base", "br", "col", "embed", "hr", "img", "input", "link",
    "meta", "param", "source", "track", "wbr",
];

#[derive(Clone, Debug)]
pub struct HtmlAttr {
    name: String,
    values: Vec<String>,
}

impl HtmlAttr {
    pub fn new(a_name: &str, value: Option<&str>) -> Self {
        Self {
            name: a_name.to_string(),
            values: match value {
                Some(val) => vec![val.to_string()],
                None => vec![],
            },
        }
    }

    pub fn name(&self) -> String {
        self.name.clone()
    }

    pub fn value(&self) -> String {
        self.values.join(" ")
    }

    pub fn clear(&mut self) {
        self.values.clear();
    }

    pub fn push(&mut self, val: &str) {
        self.values.push(val.to_string());
    }
}

impl fmt::Display for HtmlAttr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let disp = if self.values.is_empty() {
            self.name.clone()
        } else {
            self.name.clone() + "=\"" + &self.value() + "\""
        };
        write!(f, "{}", disp)
    }
}

#[derive(Clone, Debug)]
pub struct PathTagData {
    tag: String,
    path: HtmlAttr,
    attrs: Vec<HtmlAttr>,
    void: bool,
}

impl PathTagData {
    pub fn new(a_tag: &str, path_attr_name: &str) -> Self {
        Self {
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

#[derive(Clone, Debug, vicocomo_derive::PathTag)]
#[path_tag_data("script", "src")]
pub struct ScriptTag(PathTagData);

#[derive(Clone, Debug, vicocomo_derive::PathTag)]
#[path_tag_data("link", "href")]
#[path_tag_attr("rel", "stylesheet")]
pub struct StyleTag(PathTagData);

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
