mod http_server;

#[test]
fn test_http_server_actix() {
    http_server::actix::test();
}

