use ::vicocomo::{
    view::render_template, DatabaseIf, HttpServerIf, TemplEngIf,
};
use serde::Serialize;

pub struct Static;

impl Static {
    pub fn home(_db: DatabaseIf, srv: HttpServerIf, teng: TemplEngIf) {
        use ::vicocomo::t;
        #[derive(Serialize)]
        struct Data {
            hej: &'static str,
            path: String,
            more: Option<&'static str>,
            partial: String,
        }
        render_template(
            srv,
            teng,
            "home",
            &Data {
                hej: "hopp",
                path: srv.req_path(),
                more: Some("mera"),
                partial: format!("header-{}", t!("lang")),
            },
        );
        if !srv.req_path().starts_with("/redirect") {
            srv.resp_redirect(&format!(
                "/redirect-from-{}",
                srv.req_path_val("p1").unwrap(),
            ));
        }
    }
}
