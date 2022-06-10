mod controllers;

vicocomo_actix::config! {
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
    plug_in(TemplEng) {
        def: (
            vicocomo_handlebars::HbTemplEng<'_>,
            vicocomo_handlebars::HbTemplEng::new(None),
        ),
    },
    route(Static) { home { http_method: get, path: "/" } },
}

fn main() -> std::io::Result<()> {
    dotenv::dotenv().ok();
    actix_main()
}
