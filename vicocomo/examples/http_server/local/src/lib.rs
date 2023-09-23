pub mod controllers {
    use crate::models::Count;
    use crate::views::{home, Form};
    use vicocomo::{
        ActiveRecord, DatabaseIf, HtmlForm, HttpServerIf, TemplEngIf,
    };

    pub struct Counts;

    impl Counts {
        pub fn show(db: DatabaseIf, srv: HttpServerIf, teng: TemplEngIf) {
            home(srv, teng, Self::get_count(db), None);
        }

        pub fn ensure(db: DatabaseIf, srv: HttpServerIf, _teng: TemplEngIf) {
            if let Ok(json) = srv.param_json() {
                let mut form = Form::new();
                if let Ok(_) = form.update(&json) {
                    if let Some(count) = form.count.get() {
                        let mut count = Count { val: count };
                        db.clone()
                            .exec("DELETE FROM counts", &[])
                            .expect("cannot DELETE");
                        count.save(db).expect("cannot save()");
                    }
                }
            }
            srv.resp_redirect("/");
        }

        pub fn delete(db: DatabaseIf, srv: HttpServerIf, _teng: TemplEngIf) {
            let _ = db.clone().exec("DROP TABLE IF EXISTS counts", &[]);
            let _ = db.exec("CREATE TABLE counts (val BIGINT)", &[]);
            srv.resp_redirect("/");
        }

        pub fn cancel(db: DatabaseIf, srv: HttpServerIf, teng: TemplEngIf) {
            home(srv, teng, Self::get_count(db), Some("cancelled"));
        }

        fn get_count(db: DatabaseIf) -> i32 {
            Count::load(db)
                .ok()
                .as_ref()
                .and_then(|c| c.first())
                .map(|c| c.val)
                .unwrap_or(0)
        }
    }
}

pub mod models {
    #[derive(Clone, Copy, Debug, vicocomo::ActiveRecord)]
    pub struct Count {
        pub val: i32,
    }
}

pub mod views {
    use serde::Serialize;
    use vicocomo::view::render_template;
    use vicocomo::{HtmlForm, HtmlInput, HttpServerIf, TemplEngIf};

    #[derive(Clone, Debug, HtmlForm, Serialize)]
    pub struct Form {
        errors: Vec<String>,
        pub count: HtmlInput<i32>,
        pub extra: Option<String>,
    }

    pub fn home(
        srv: HttpServerIf,
        teng: TemplEngIf,
        count: i32,
        extra: Option<&str>,
    ) {
        let mut form = Form::new();
        form.count.set(count);
        form.extra = extra.map(|e| e.to_string());
        render_template(srv, teng, "home", &form.to_json());
    }
}
