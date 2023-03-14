use ljumvall_test_utils::test_http_server;
use ljumvall_test_utils::{TestRequest, TestResponse};

#[test]
fn test_authorization_actix() {
    test_http_server!(
        "../vicocomo/examples/authorization/actix",
        false,
        TestRequest::new("http://localhost:3000/").no_redirect(),
        |r: &TestResponse| {
            assert_eq!(r.status(), "302");
            assert_eq!(r.redirect(), "http://localhost:3000/njet");
        },
        TestRequest::new("http://localhost:3000/login").cookies(),
        |r: &TestResponse| {
            assert_eq!(r.status(), "200");
            assert_eq!(r.body(), "you are authorized");
        },
        TestRequest::new("http://localhost:3000/logout").cookies(),
        |r: &TestResponse| {
            assert_eq!(r.status(), "200");
            assert_eq!(r.body(), "*** njet ***");
        },
    );
}
