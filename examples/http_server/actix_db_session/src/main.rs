mod controllers {
    use ::vicocomo::{DatabaseIf, HttpServerIf, TemplEngIf };

    pub struct Static;

    impl Static {
        pub fn home(_db: DatabaseIf, srv: HttpServerIf, _teng: TemplEngIf) {
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
            srv.resp_body(&format!("{}", count));
            srv.resp_ok();
        }
    }
}

::vicocomo_actix::config! {
    app_config { session: [Database, h1] },
    plug_in(DbConn) {
        def: (
            vicocomo_postgres::PgConn,
            {
                let (client, connection) = tokio_postgres::connect(
                        &std::env::var("VICOCOMO_TEST_DB")
                            .expect("VICOCOMO_TEST_DB must be set"),
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
    route(Static) { home { path: "/" } },
}

fn main() -> std::io::Result<()> {
    dotenv::dotenv().ok();
    actix_main()
}
