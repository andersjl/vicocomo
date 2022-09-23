mod controllers;

::vicocomo_actix::config! {
    app_config {
        file_root: "public",
        session: None,
        strip_mtime: true,
        url_root: "test",
    },
    plug_in(TemplEng) {
        def: (
            ::vicocomo_handlebars::HbTemplEng<'_>,
            ::vicocomo_handlebars::HbTemplEng::new(None),
        ),
    },
    route(Static) {
        home { http_method: get, path: "/home/<parameter>" },
        file { path: "/file/<filename>" },
        date { path: "/date/<date>" },
    },
    //route(Dynamic) { create },
    route(Dynamic) { index },
    route_static("static") { fs_path: "" },
    route_static("static/txt") { fs_path: "txt" },
}

fn main() -> std::io::Result<()> {
    actix_main()
}
