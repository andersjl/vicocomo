// Prevents additional console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod controllers {
    use crate::models;
    use crate::views;
    use vicocomo::{
        ActiveRecord, DatabaseIf, HtmlForm, HttpResponse, HttpServerIf,
        TemplEngIf,
    };
    pub use vicocomo_example_http_server_local::controllers::*;

    pub struct Tough;

    impl Tough {
        pub fn create(
            db: DatabaseIf,
            srv: HttpServerIf,
            _teng: TemplEngIf,
        ) -> HttpResponse {
            use views::ToughForm;

            let mut tough = models::Tough::load(db.clone())
                .unwrap()
                .drain(..1)
                .next()
                .unwrap();
            let pars = srv.param_json().unwrap();
            let form =
                views::ToughForm::update_session(srv, &pars, ToughForm::init)
                    .unwrap();
            tough.set_data(models::ToughData {
                selec: form.selec.get().unwrap(),
                multi: form.multi.get_mult(),
                radio: form.radio.get().unwrap(),
                chbox: form.chbox.get_mult(),
            });
            let _ = db.clone().transaction(|db| {
                db.clone().exec("DELETE FROM toughs", &[])?;
                tough.save(db.clone())?;
                Ok(())
            });
            srv.resp_redirect("/tough")
        }

        pub fn index(
            db: DatabaseIf,
            _srv: HttpServerIf,
            teng: TemplEngIf,
        ) -> HttpResponse {
            views::tough(teng, models::Tough::data(db.clone()))
        }
    }
}

mod models {
    use vicocomo::{ActiveRecord, DatabaseIf};

    #[derive(Clone, Debug, vicocomo::ActiveRecord)]
    pub struct Tough {
        pub selec: String,
        pub multi: String, // space separated values
        pub radio: String,
        pub chbox: String, // space separated values
    }

    impl Tough {
        pub fn data(db: DatabaseIf) -> ToughData {
            let db_data = Self::load(db.clone()).unwrap();
            let dbd = db_data.first().unwrap();
            ToughData {
                selec: dbd.selec.clone(),
                multi: dbd
                    .multi
                    .split_whitespace()
                    .map(|s| s.to_string())
                    .collect::<Vec<_>>(),
                radio: dbd.radio.clone(),
                chbox: dbd
                    .chbox
                    .split_whitespace()
                    .map(|s| s.to_string())
                    .collect::<Vec<_>>(),
            }
        }

        pub fn set_data(&mut self, data: ToughData) {
            self.selec = data.selec;
            self.multi = data.multi.join(" ");
            self.radio = data.radio;
            self.chbox = data.chbox.join(" ");
        }
    }

    pub struct ToughData {
        pub selec: String,
        pub multi: Vec<String>,
        pub radio: String,
        pub chbox: Vec<String>,
    }
}

mod views {
    use super::models::ToughData;
    use serde::{Deserialize, Serialize};
    use vicocomo::{
        HtmlForm, HtmlInput, HttpResponse, SessionModel, TemplEngIf,
    };

    #[derive(
        Clone, Debug, Deserialize, HtmlForm, Serialize, SessionModel,
    )]
    pub struct ToughForm {
        pub errors: Vec<String>,
        #[vicocomo_html_input_type = "Select"]
        pub selec: HtmlInput<String>,
        #[vicocomo_html_input_type = "SelectMult"]
        pub multi: HtmlInput<String>,
        #[vicocomo_html_input_type = "Radio"]
        pub radio: HtmlInput<String>,
        #[vicocomo_html_input_type = "Checkbox"]
        pub chbox: HtmlInput<String>,
    }

    impl ToughForm {
        pub(crate) fn init() -> Self {
            let mut result = Self::with_labels(None);
            result.selec.set_options(&[
                ("one", "one".to_string()),
                ("two", "two".to_string()),
                ("thr", "three 3".to_string()),
            ]);
            result.multi.set_options(&[
                ("one", "one".to_string()),
                ("two", "two".to_string()),
                ("thr", "three 3".to_string()),
            ]);
            result.radio.set_options(&[
                ("one", "one".to_string()),
                ("two", "two".to_string()),
                ("thr", "three 3".to_string()),
            ]);
            result
                .radio
                .add_attr_vals("class", "vicocomo--submit-on-change");
            result.chbox.set_options(&[
                ("one", "one".to_string()),
                ("two", "two".to_string()),
                ("thr", "three 3".to_string()),
            ]);
            result
        }
    }

    pub fn tough(teng: TemplEngIf, data: ToughData) -> HttpResponse {
        let mut form = ToughForm::init();
        form.selec.set(data.selec.clone());
        form.multi.set_mult(data.multi.as_slice());
        form.radio.set(data.radio.clone());
        form.chbox.set_mult(data.chbox.as_slice());
        HttpResponse::from_result(
            teng.render("tough", &form.to_json()),
            "html",
        )
    }
}

vicocomo_tauri::config! {
    app_config {
        session: None,
        template_dir: ["templates", "hbs"],
        texts_config: true,
    },
    plug_in(TemplEng) {
        def: (
            vicocomo_handlebars::HbTemplEng,
            vicocomo_handlebars::HbTemplEng::new(None),
        ),
    },
    route(Counts) {
        show { path: "/" },
        ensure { path: "/" },
        delete { path: "/delete" },
        cancel { path: "/cancel" },
    },
    route(Tough) { create, index },
}
