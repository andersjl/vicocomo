mod controllers {
    use vicocomo::{DatabaseIf, HttpResponse, HttpServerIf, TemplEngIf};

    pub struct Static;

    impl Static {
        pub fn bar(
            _db: DatabaseIf,
            srv: HttpServerIf,
            _teng: TemplEngIf,
        ) -> HttpResponse {
            srv.resp_ok(String::from("bar is authorized, too"))
        }

        pub fn foobar(
            _db: DatabaseIf,
            srv: HttpServerIf,
            _teng: TemplEngIf,
        ) -> HttpResponse {
            srv.resp_ok(String::from("foobar is public"))
        }

        pub fn home(
            _db: DatabaseIf,
            srv: HttpServerIf,
            _teng: TemplEngIf,
        ) -> HttpResponse {
            srv.resp_ok(String::from("you are authorized"))
        }

        pub fn login(
            _db: DatabaseIf,
            srv: HttpServerIf,
            _teng: TemplEngIf,
        ) -> HttpResponse {
            match srv.session_set("logged_in", &true) {
                Ok(_) => srv.resp_redirect("/"),
                _ => srv.resp_error(None, None),
            }
        }

        pub fn logout(
            _db: DatabaseIf,
            srv: HttpServerIf,
            _teng: TemplEngIf,
        ) -> HttpResponse {
            srv.session_remove("logged_in");
            srv.resp_redirect("/")
        }

        pub fn njet(
            _db: DatabaseIf,
            srv: HttpServerIf,
            _teng: TemplEngIf,
        ) -> HttpResponse {
            srv.resp_ok(String::from("*** njet ***"))
        }
    }
}

mod models {
    use vicocomo::{DatabaseIf, HttpServerIf};

    #[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
    pub enum UserRole {
        Superuser,
        User,
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

vicocomo_actix::config! {
    app_config {
        role_variants: [User],
        unauthorized_route: "/njet",
        session_middleware: (
            actix_session::SessionMiddleware,
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
        bar { path: "/bar" },
        foobar { path: "/foo/bar" },
        home { path: "/" },
        login { path: "/login" },
        logout { path: "/logout" },
        njet { path: "/njet" },
    },
    authorize("/") { get: User },
    authorize("/bar") { get: User },
}

fn main() -> std::io::Result<()> {
    actix_main()
}
