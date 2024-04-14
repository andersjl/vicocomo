use chrono::NaiveDate;
use vicocomo::{t, DatabaseIf, HttpResponse, HttpServerIf, TemplEngIf};

pub fn home(
    _db: DatabaseIf,
    srv: HttpServerIf,
    teng: TemplEngIf,
) -> HttpResponse {
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
        HttpResponse::from_result(
            make_href(srv, "static/txt", "timestamp", None).and_then(
                |href| {
                    teng.render(
                        "home",
                        &Data {
                            hej: r#"hej och "hå"!"#,
                            path: srv.req_path(),
                            url: srv.req_url(),
                            partial: format!("header-{}", t!("lang")),
                            date: NaiveDate::from_num_days_from_ce_opt(
                                737843,
                            )
                            .unwrap(),
                            href,
                        },
                    )
                },
            ),
            "html",
        )
    } else {
        srv.resp_redirect(&format!(
            "redirect-from-{}",
            srv.req_route_par_val::<u32>("parameter").unwrap(),
        ))
    }
}

pub fn index(
    _db: DatabaseIf,
    srv: HttpServerIf,
    _teng: TemplEngIf,
) -> HttpResponse {
    srv.resp_ok(format!(
        "{}\n{}\n",
        srv.req_url(),
        srv.param_val::<String>("foo").unwrap().as_str(),
    ))
}

pub mod static_views {
    use super::*;
    pub fn date(
        _db: DatabaseIf,
        srv: HttpServerIf,
        _teng: TemplEngIf,
    ) -> HttpResponse {
        use chrono::TimeDelta;
        use vicocomo::Error;
        match srv.req_route_par_val::<NaiveDate>("date") {
            Some(d) => {
                srv.resp_ok((d + TimeDelta::try_days(1).unwrap()).to_string())
            }
            None => srv.resp_error(None, Some(Error::other("not a date"))),
        }
    }
}
