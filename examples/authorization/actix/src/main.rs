mod controllers {
    use ::vicocomo::{DatabaseIf, HttpServerIf, TemplEngIf };

    pub struct Static;

    impl Static {
        pub fn home(_db: DatabaseIf, srv: HttpServerIf, _teng: TemplEngIf) {
            srv.resp_body("you are authorized");
            srv.resp_ok();
        }

        pub fn login(_db: DatabaseIf, srv: HttpServerIf, _teng: TemplEngIf) {
            match srv.session_set("logged_in", &true) {
                Ok(_) => srv.resp_redirect("/"),
                _ => srv.resp_error(None),
            }
        }

        pub fn logout(_db: DatabaseIf, srv: HttpServerIf, _teng: TemplEngIf) {
            srv.session_remove("logged_in");
            srv.resp_redirect("/");
        }

        pub fn njet(_db: DatabaseIf, srv: HttpServerIf, _teng: TemplEngIf) {
            srv.resp_body("*** njet ***");
            srv.resp_ok();
        }
    }
}

mod models {
    use ::vicocomo::{DatabaseIf, HttpServerIf };

    #[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
    pub enum UserRole {
        Superuser
    }

    impl vicocomo::UserRole for UserRole {
        fn is_authenticated(
            &self,
            _db: DatabaseIf,
            srv: HttpServerIf,
        ) -> bool {
            srv.session_get("logged_in").unwrap_or(false)
        }
    }
}

::vicocomo_actix::config! {
    app_config {
        role_enum: true,
        unauthorized_route: "/njet",
    },
    plug_in(Session) {
        def: (
            (),
            actix_session::SessionMiddleware::builder(
                actix_session::storage::CookieSessionStore::default(),
                actix_web::cookie::Key::from(&[0; 64]),
            )
            .cookie_secure(false)
            .build(),
        ),
    },
    authorize("/*") { get: Public },
    route(Static) {
        home { path: "/" },
        login { path: "/login" },
        logout { path: "/logout" },
        njet { path: "/njet" },
    },
    authorize("/") { get: Superuser },
}

fn main() -> std::io::Result<()> {
    actix_main()
}
