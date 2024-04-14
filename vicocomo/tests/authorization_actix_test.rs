use ljumvall_test_utils::test_http_server;
use ljumvall_test_utils::{TestRequest, TestResponse};

#[test]
fn test_authorization_actix() {
    test_http_server!(
        "../vicocomo/examples/authorization/actix",
        false,
        TestRequest::new("http://localhost:3000/").no_redirect(),
        |r: &TestResponse| {
            assert_eq!(r.status(), "303");
            assert_eq!(r.redirect(), "http://localhost:3000/njet");
        },
        TestRequest::new("http://localhost:3000/bar").no_redirect(),
        |r: &TestResponse| {
            assert_eq!(r.status(), "303");
            assert_eq!(r.redirect(), "http://localhost:3000/njet");
        },
        TestRequest::new("http://localhost:3000/foo/bar").cookies(),
        |r: &TestResponse| {
            assert_eq!(r.status(), "200");
            assert_eq!(r.body(), "foobar is public");
        },
        TestRequest::new("http://localhost:3000/login").cookies(),
        |r: &TestResponse| {
            assert_eq!(r.status(), "200");
            assert_eq!(r.body(), "you are authorized");
        },
        TestRequest::new("http://localhost:3000/").cookies(),
        |r: &TestResponse| {
            assert_eq!(r.status(), "200");
            assert_eq!(r.body(), "you are authorized");
        },
        TestRequest::new("http://localhost:3000/bar").cookies(),
        |r: &TestResponse| {
            assert_eq!(r.status(), "200");
            assert_eq!(r.body(), "bar is authorized, too");
        },
        TestRequest::new("http://localhost:3000/foo/bar").cookies(),
        |r: &TestResponse| {
            assert_eq!(r.status(), "200");
            assert_eq!(r.body(), "foobar is public");
        },
        TestRequest::new("http://localhost:3000/logout").cookies(),
        |r: &TestResponse| {
            assert_eq!(r.status(), "200");
            assert_eq!(r.body(), "*** njet ***");
        },
        TestRequest::new("http://localhost:3000/bar").no_redirect(),
        |r: &TestResponse| {
            assert_eq!(r.status(), "303");
            assert_eq!(r.redirect(), "http://localhost:3000/njet");
        },
        TestRequest::new("http://localhost:3000/foo/bar").cookies(),
        |r: &TestResponse| {
            assert_eq!(r.status(), "200");
            assert_eq!(r.body(), "foobar is public");
        },
    );
}
