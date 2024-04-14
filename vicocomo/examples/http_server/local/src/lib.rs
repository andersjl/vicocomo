pub mod controllers {
    use crate::models::Count;
    use crate::views::{home, Form};
    use vicocomo::{
        t, try_exec_sql, ActiveRecord, DatabaseIf, Error, HtmlForm,
        HttpResponse, HttpServerIf, TemplEngIf,
    };

    pub struct Counts;

    impl Counts {
        pub fn show(
            db: DatabaseIf,
            srv: HttpServerIf,
            teng: TemplEngIf,
        ) -> HttpResponse {
            home(teng, Self::get_count(db, srv), None)
        }

        pub fn ensure(
            db: DatabaseIf,
            srv: HttpServerIf,
            _teng: TemplEngIf,
        ) -> HttpResponse {
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
            srv.resp_redirect("/")
        }

        pub fn delete(
            db: DatabaseIf,
            srv: HttpServerIf,
            _teng: TemplEngIf,
        ) -> HttpResponse {
            let _ = db.clone().exec("DROP TABLE IF EXISTS counts", &[]);
            let _ = db.exec("CREATE TABLE counts (val BIGINT)", &[]);
            srv.resp_redirect("/")
        }

        pub fn cancel(
            db: DatabaseIf,
            srv: HttpServerIf,
            teng: TemplEngIf,
        ) -> HttpResponse {
            home(teng, Self::get_count(db, srv), Some(&t!("cancelled")))
        }

        fn get_count(db: DatabaseIf, srv: HttpServerIf) -> i32 {
            Count::load(db.clone())
                .or_else(|_| {
                    let init = srv.resource_dir().join("db/init.sql");
                    std::fs::read_to_string(&init)
                        .map_err(|e| {
                            Error::invalid_input(&format!(
                                "cannot read {}: {e}",
                                init.display()
                            ))
                        })
                        .and_then(|init| {
                            try_exec_sql(db.clone(), &init, None)
                        })
                        .and_then(|_| Count::load(db.clone()))
                })
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
    use vicocomo::{HtmlForm, HtmlInput, HttpResponse, TemplEngIf};

    #[derive(Clone, Debug, HtmlForm, Serialize)]
    pub struct Form {
        errors: Vec<String>,
        pub count: HtmlInput<i32>,
        pub extra: Option<String>,
    }

    pub fn home(
        teng: TemplEngIf,
        count: i32,
        extra: Option<&str>,
    ) -> HttpResponse {
        let mut form = Form::new();
        form.count.set(count);
        form.extra = extra.map(|e| e.to_string());
        HttpResponse::from_result(
            teng.render("home", &form.to_json()),
            "html",
        )
    }
}
