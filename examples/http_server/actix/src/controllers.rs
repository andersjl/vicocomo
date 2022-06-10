use chrono::NaiveDate;
use vicocomo::{
    t, view::render_template, DatabaseIf, HttpServerIf, TemplEngIf,
};

pub struct Static;

impl Static {
    pub fn home(_db: DatabaseIf, srv: HttpServerIf, teng: TemplEngIf) {
        #[derive(serde::Serialize)]
        struct Data {
            hej: &'static str,
            path: String,
            partial: String,
            date: NaiveDate,
        }
        render_template(
            srv,
            teng,
            "home",
            &Data {
                hej: r#"hej "hopp"!"#,
                path: srv.req_path(),
                partial: format!("header-{}", t!("lang")),
                date: NaiveDate::from_num_days_from_ce(737843),
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
