use ljumvall_test_utils::{test_http_server, TestRequest, TestResponse};
use regex::Regex;

#[test]
fn test_http_server_actix_sqlite() {
    test_http_server!(
        "../vicocomo/examples/http_server/actix_sqlite",
        true,
        TestRequest::new("http://localhost:3000/delete")
            .no_redirect()
            .post(),
        |r: &TestResponse| {
            assert_eq!(r.status(), "303");
            assert_eq!(r.redirect(), "http://localhost:3000/");
        },
        TestRequest::new("http://localhost:3000/"),
        |r: &TestResponse| {
            assert_eq!(r.status(), "200");
            assert!(check_count(0).is_match(r.body()), "got\n{}\n", r.body());
        },
        TestRequest::new("http://localhost:3000/")
            .post()
            .no_redirect()
            .data("count", "42"),
        |r: &TestResponse| {
            assert_eq!(r.status(), "303");
            assert_eq!(r.redirect(), "http://localhost:3000/");
        },
        TestRequest::new("http://localhost:3000/"),
        |r: &TestResponse| {
            assert_eq!(r.status(), "200");
            assert!(
                check_count(42).is_match(r.body()),
                "got\n{}\n",
                r.body()
            );
        },
        TestRequest::new("http://localhost:3000/cancel"),
        |r: &TestResponse| {
            assert_eq!(r.status(), "200");
            assert!(
                check_count(42).is_match(r.body()),
                "got\n{}\n",
                r.body()
            );
            assert!(r.body().contains("avbrutet"), "{}", r.body());
        },
        TestRequest::new("http://localhost:3000/")
            .post()
            .no_redirect()
            .data("count", "4711"),
        |r: &TestResponse| {
            assert_eq!(r.status(), "303");
            assert_eq!(r.redirect(), "http://localhost:3000/");
        },
        TestRequest::new("http://localhost:3000/"),
        |r: &TestResponse| {
            assert_eq!(r.status(), "200");
            assert!(
                check_count(4711).is_match(r.body()),
                "got\n{}\n",
                r.body()
            );
        },
    );
}

fn check_count(count: i32) -> Regex {
    Regex::new(&format!(
        r#"<input type="number" id="count" name="count" value="{}">"#,
        count,
    ))
    .unwrap()
}
