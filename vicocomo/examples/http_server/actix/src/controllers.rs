//use chrono::NaiveDate;
use vicocomo::{
    delegate_to_view, Controller, DatabaseIf, HttpResponse, HttpServerIf,
    TemplEngIf,
};

pub struct Dynamic;

impl Dynamic {
    pub fn attach(
        _db: DatabaseIf,
        _srv: HttpServerIf,
        _teng: TemplEngIf,
    ) -> HttpResponse {
        HttpResponse::plain("foo").attach(Some("foo.txt"))
    }
}

impl Controller for Dynamic {
    delegate_to_view!(index);
}

pub struct Static;

impl Static {
    delegate_to_view!(pub date, static_views);

    pub fn file(
        _db: DatabaseIf,
        srv: HttpServerIf,
        _teng: TemplEngIf,
    ) -> HttpResponse {
        use regex::Regex;
        use std::env::temp_dir;
        use vicocomo::Error;
        match srv.req_route_par_val::<String>("filename") {
            Some(s) if Regex::new(r"tmp-\d{10}").unwrap().is_match(&s) => srv
                .resp_download(
                    &temp_dir().join("timestamp.txt").display().to_string(),
                ),
            Some(s) => srv.resp_download(&s),
            None => {
                srv.resp_error(None, Some(Error::other("not a filename")))
            }
        }
    }

    delegate_to_view!(pub home);
}
