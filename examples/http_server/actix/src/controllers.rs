use ::chrono::NaiveDate;
use ::serde::{Deserialize, Serialize};
use ::vicocomo::{
    t, view::render_template, DatabaseIf, HttpServerIf, SessionModel,
    TemplEngIf,
};

pub struct Static;

impl Static {
    pub fn home(_db: DatabaseIf, srv: HttpServerIf, teng: TemplEngIf) {
        #[derive(
            Copy, Clone, Debug, Default, Deserialize, Serialize, SessionModel,
        )]
        #[vicocomo_session_model_accessors]
        struct Sess {
            data: f32,
        }

        #[derive(Serialize)]
        struct Data {
            hej: &'static str,
            path: String,
            more: Option<f32>,
            partial: String,
            date: NaiveDate,
        }
        let mut sess = Sess::load(srv);
        render_template(
            srv,
            teng,
            "home",
            &Data {
                hej: r#"hej "hopp"!"#,
                path: srv.req_path(),
                more: Some(sess.data),
                partial: format!("header-{}", t!("lang")),
                date: NaiveDate::from_num_days_from_ce(737843),
            },
        );
        if !srv.req_path().starts_with("/redirect") {
            /*
            sess.data =
                srv.req_path_val("p1").unwrap().parse().unwrap_or(0.0);
            let _ = sess.store(srv);
            */
            let _ = sess.set_data(
                srv,
                &srv.req_path_val("p1").unwrap().parse().unwrap_or(0.0),
            );
            srv.resp_redirect(&format!(
                "/redirect-from-{}",
                srv.req_path_val("p1").unwrap(),
            ));
        }
    }
}
