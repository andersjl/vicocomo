use ljumvall_test_utils::test_http_server;
use ljumvall_test_utils::{TestRequest, TestResponse};

#[test]
fn test_http_server_actix_db_session() {
    test_http_server!(
        "../vicocomo/examples/http_server/actix_db_session",
        false,
        TestRequest::new("http://localhost:3000/init"),
        |r: &TestResponse| {
            assert_eq!(r.status(), "200");
            assert_eq!(r.body(), "deleted session DB table");
        },
    );
    test_http_server!(
        "../vicocomo/examples/http_server/actix_db_session",
        false,
        TestRequest::new("http://localhost:3000").cookies(),
        |r: &TestResponse| {
            assert_eq!(r.status(), "200");
            assert_eq!(r.body(), "0");
        },
        TestRequest::new("http://localhost:3000").cookies(),
        |r: &TestResponse| {
            assert_eq!(r.status(), "200");
            assert_eq!(r.body(), "1");
        },
        TestRequest::new("http://localhost:3000").cookies(),
        |r: &TestResponse| {
            assert_eq!(r.status(), "200");
            assert_eq!(r.body(), "2");
        },
        TestRequest::new("http://localhost:3000").cookies(),
        |r: &TestResponse| {
            assert_eq!(r.status(), "200");
            assert_eq!(r.body(), "0");
        },
    );
}
