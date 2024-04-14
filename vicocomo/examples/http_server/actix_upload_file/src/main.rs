mod controllers;
mod views;

::vicocomo_actix::config! {
    app_config {
        file_root: "public",
        session: None,
    },
    plug_in(TemplEng) {
        def: (
            ::vicocomo_handlebars::HbTemplEng,
            ::vicocomo_handlebars::HbTemplEng::new(None),
        ),
    },
    route(Test) {
        home { path: "/" },
        upload { upload: "file_field", path: "/upload" },
    },
}

fn main() -> std::io::Result<()> {
    actix_main()
}
