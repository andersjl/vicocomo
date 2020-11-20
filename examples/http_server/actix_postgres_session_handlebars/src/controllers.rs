use ::vicocomo::{
    view::render_template, DatabaseIf, HttpServerIf, TemplEngIf,
};
use serde::Serialize;

pub struct Static;

impl Static {
    pub fn home(
        _db: DatabaseIf,
        srv: HttpServerIf,
        teng: TemplEngIf,
    ) {
        use ::vicocomo::t;
        #[derive(Serialize)]
        struct Data {
            hej: &'static str,
            i: &'static str,
            more: Option<&'static str>,
            partial: String,
            root: String,
        }
        render_template(
            srv,
            teng,
            "home",
            &Data {
                hej: "hopp",
                i: "lingonskogen",
                more: Some("mera"),
                partial: format!("header-{}", t!("lang")),
                root: match srv.url_for("/", Some(&[""])) {
                    Ok(url) => url,
                    Err(e) => e.to_string(),
                },
            },
        );
    }
}
