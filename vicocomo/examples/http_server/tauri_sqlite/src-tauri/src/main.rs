// Prevents additional console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use vicocomo_example_http_server_local::*;

vicocomo_tauri::config! {
    app_config {
        session: None,
        template_dir: ["templates", "hbs"],
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
