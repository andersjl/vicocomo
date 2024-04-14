mod controllers;
mod views;

::vicocomo_actix::config! {
    app_config {
        file_root: "public",
        session: None,
        strip_mtime: true,
        texts_config: true,
        url_root: "test",
    },
    plug_in(TemplEng) {
        def: (
            ::vicocomo_handlebars::HbTemplEng,
            ::vicocomo_handlebars::HbTemplEng::new(None),
        ),
    },
    route(Static) {
        date { path: "/date/<date>" },
        file { path: "/file/<filename>" },
        home { http_method: get, path: "/home/<parameter>" },
    },
    route(Dynamic) {
        attach { path: "/attach" },
        index,
    },
    route_static("static") { fs_path: "" },
    route_static("static/txt") { fs_path: "txt" },
}

fn main() -> std::io::Result<()> {
    actix_main()
}
