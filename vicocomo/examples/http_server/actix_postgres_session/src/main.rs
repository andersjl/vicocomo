mod controllers {
    use ::vicocomo::{DatabaseIf, HttpResponse, HttpServerIf, TemplEngIf};

    pub struct Static;

    impl Static {
        pub fn home(
            _db: DatabaseIf,
            srv: HttpServerIf,
            _teng: TemplEngIf,
        ) -> HttpResponse {
            let count = match srv.session_get::<i32>("count") {
                Some(i) if i < 2 => i + 1,
                Some(_) => {
                    srv.session_clear();
                    0
                }
                None => 0,
            };
            srv.session_clear();
            let _ = srv.session_set("count", &count);
            srv.resp_ok(format!("{}", count))
        }

        pub fn init(
            db: DatabaseIf,
            srv: HttpServerIf,
            _teng: TemplEngIf,
        ) -> HttpResponse {
            let _ = db.exec("DROP TABLE IF EXISTS __vicocomo__sessions", &[]);
            srv.resp_ok(String::from("deleted session DB table"))
        }
    }
}

::vicocomo_actix::config! {
    app_config {
        session: [Database, h1],
        create_session_table: true,
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
    plug_in(DbConn) {
        def: (
            vicocomo_postgres::PgConn,
            {
                let (client, connection) = tokio_postgres::connect(
                        &std::env::var("VICOCOMO_TEST_DB_POSTGRES")
                            .expect("VICOCOMO_TEST_DB_POSTGRES must be set"),
                        tokio_postgres::NoTls,
                    )
                    .await
                    .expect("could not get connection");
                tokio::spawn(async move {
                    if let Err(e) = connection.await {
                        eprintln!("could not init connection: {}", e);
                    }
                });
                vicocomo_postgres::PgConn::new(client)
            },
        ),
    },
    route(Static) { home { path: "/" } },
    route(Static) { init { path: "/init" } },
}

fn main() -> std::io::Result<()> {
    dotenv::dotenv().ok();
    actix_main()
}
