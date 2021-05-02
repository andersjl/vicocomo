use ::vicocomo::{
    view::render_template, DatabaseIf, HttpServerIf, TemplEngIf,
};
use serde::Serialize;
use serde_json::json;
use serde_json::value::Value as JsonValue;

pub struct Static;

impl Static {
    pub fn home(_db: DatabaseIf, srv: HttpServerIf, teng: TemplEngIf) {
        use ::vicocomo::t;
        #[derive(Serialize)]
        struct Data {
            content: Option<String>,
            hej: &'static str,
            i: &'static str,
            array: JsonValue,
            empty: JsonValue,
            more: Option<&'static str>,
            partial: String,
        }
        render_template(
            srv,
            teng,
            "home",
            &Data {
                content: Some("partial".to_string()),
                hej: "hopp",
                i: "lingonskogen",
                array: json!(["ett", "två"]),
                empty: json!([]),
                more: Some("mera"),
                partial: format!("header-{}", t!("lang")),
            },
        );
    }
}
