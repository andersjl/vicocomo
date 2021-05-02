mod http_server;

#[test]
fn test_http_server_actix_postgres_session_handlebars() {
    http_server::actix_postgres_session_handlebars::test();
}

