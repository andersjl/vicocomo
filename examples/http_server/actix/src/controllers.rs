use chrono::NaiveDate;
use vicocomo::{
    t, view::render_template, DatabaseIf, HttpServerIf, TemplEngIf,
};

pub struct Static;

impl Static {
    pub fn date(_db: DatabaseIf, srv: HttpServerIf, _teng: TemplEngIf) {
        use chrono::Duration;
        use vicocomo::Error;
        match srv.req_route_par_val::<NaiveDate>("date") {
            Some(d) => {
                srv.resp_body(&(d + Duration::days(1)).to_string());
                srv.resp_ok();
            }
            None => srv.resp_error(Some(&Error::other("not a date"))),
        }
    }

    pub fn file(_db: DatabaseIf, srv: HttpServerIf, _teng: TemplEngIf) {
        use regex::Regex;
        use std::env::temp_dir;
        use vicocomo::Error;
        match srv.req_route_par_val::<String>("filename") {
            Some(s) if Regex::new(r"tmp-\d{10}").unwrap().is_match(&s) => {
                srv.resp_file(
                    &temp_dir().join("timestamp.txt").display().to_string(),
                );
            }
            Some(s) => srv.resp_file(&s),
            None => srv.resp_error(Some(&Error::other("not a filename"))),
        }
    }

    pub fn home(_db: DatabaseIf, srv: HttpServerIf, teng: TemplEngIf) {
        use vicocomo::view::make_href;
        #[derive(serde::Serialize)]
        struct Data {
            hej: &'static str,
            path: String,
            url: String,
            partial: String,
            date: NaiveDate,
            href: String,
        }
        if srv.req_path().contains("redirect") {
            //let timestamp = chrono::Utc::now().timestamp().to_string();
            //std::fs::write("public/txt/timestamp.txt", &timestamp).unwrap();
            match make_href(srv, "static/txt", "timestamp", None) {
                Ok(href) => render_template(
                    srv,
                    teng,
                    "home",
                    &Data {
                        hej: r#"hej "hopp"!"#,
                        path: srv.req_path(),
                        url: srv.req_url(),
                        partial: format!("header-{}", t!("lang")),
                        date: NaiveDate::from_num_days_from_ce(737843),
                        href,
                    },
                ),
                Err(e) => srv.resp_error(Some(&e)),
            }
        } else {
            srv.resp_redirect(&format!(
                "redirect-from-{}",
                srv.req_route_par_val::<u32>("parameter").unwrap(),
            ));
        }
    }
}
