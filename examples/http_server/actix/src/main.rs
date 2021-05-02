mod controllers;

::vicocomo_actix::config! {
    plug_in(Session) {
        def: (
            (),
            ::actix_session::CookieSession::signed(&[0; 32])
                .secure(false).expires_in(60),
        ),
    },
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
