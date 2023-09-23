use vicocomo_example_http_server_local::*;

vicocomo_actix::config! {
    app_config {
        session: None,
    },
    plug_in(DbConn) {
        def: (
            vicocomo_sqlite::SqliteConn,
            vicocomo_sqlite::SqliteConn::new("test.sqlite").unwrap(),
        ),
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
    }
}

fn main() -> std::io::Result<()> {
    actix_main()
}
