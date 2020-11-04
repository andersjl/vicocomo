use ::vicocomo::{
    view::render_template, DbConn, Request, Response, Session, TemplEng,
};
use serde::Serialize;

pub struct Static;

impl Static {
    pub fn home(
        req: &impl Request,
        teng: &impl TemplEng,
        _db: &impl DbConn,
        _sess: Session,
        resp: &mut impl Response,
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
            resp,
            teng,
            "home",
            &Data {
                hej: "hopp",
                i: "lingonskogen",
                more: Some("mera"),
                partial: format!("header-{}", t!("lang")),
                root: match req.url_for("/", Some(&[""])) {
                    Ok(url) => url,
                    Err(e) => e.to_string(),
                },
            },
        );
    }
}
