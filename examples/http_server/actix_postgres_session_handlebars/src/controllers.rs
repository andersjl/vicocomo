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
            i: i32,
            array: JsonValue,
            empty: JsonValue,
            more: Option<&'static str>,
            partial: String,
        }
        assert!(
            srv.session_set(
                "just_to_ensure_we_can_have_more_than_one_session_key",
                &42,
            ).is_ok(),
        );
        assert!(
            srv.session_set(
                "test",
                &match srv.session_get::<i32>("test") {
                    Some(i) => i + 1,
                    None => 0,
                },
            ).is_ok(),
        );
        render_template(
            srv,
            teng,
            "home",
            &Data {
                content: Some("partial".to_string()),
                hej: "hopp",
                i: srv.session_get("test").unwrap(),
                array: json!(["ett", "två"]),
                empty: json!([]),
                more: Some("mera"),
                partial: format!("header-{}", t!("lang")),
            },
        );
    }
}
