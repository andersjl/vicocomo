mod controllers;

::vicocomo_actix::config! {
    app_config { session: None },
    plug_in(TemplEng) {
        def: (
            ::vicocomo_handlebars::HbTemplEng<'_>,
            ::vicocomo_handlebars::HbTemplEng::new(None),
        ),
    },
    route(Static) { home { http_method: get, path: "/<parameter>" } },
}

fn main() -> std::io::Result<()> {
    actix_main()
}
