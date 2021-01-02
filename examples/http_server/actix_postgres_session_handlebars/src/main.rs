mod controllers;

::vicocomo_actix::config! {
    plug_in(DbConn) {
        def: (
            ::vicocomo_postgres::PgConn,
            {
                let (client, connection) = ::tokio_postgres::connect(
                        &::std::env::var("DATABASE_URL")
                            .expect("DATABASE_URL must be set"),
                        ::tokio_postgres::NoTls,
                    )
                    .await
                    .expect("could not get connection");
                ::tokio::spawn(async move {
                    if let Err(e) = connection.await {
                        eprintln!("could not init connection: {}", e);
                    }
                });
                ::vicocomo_postgres::PgConn::new(client)
            },
        ),
    },
    plug_in(Session) {
        def: (
            (),
            ::actix_session::CookieSession::signed(&[0; 32]).secure(false),
        ),
    },
    plug_in(TemplEng) {
        def: (
            ::vicocomo_handlebars::HbTemplEng<'_>,
            ::vicocomo_handlebars::HbTemplEng::new(None),
        ),
    },
    route(Static) { home { http_method: get, path: "/" } },
}

fn main() -> std::io::Result<()> {
    ::dotenv::dotenv().ok();
    actix_main()
}
