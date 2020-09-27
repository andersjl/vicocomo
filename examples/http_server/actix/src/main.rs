mod controllers;

::vicocomo_actix::config! {
    route(Static) { home { http_method: get, path: "/" } },
}
