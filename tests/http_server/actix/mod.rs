use ::regex::Regex;
use ::vicocomo::test_http_server;
use ::vicocomo::test_utils::{TestRequest, TestResponse};

#[allow(dead_code)]
pub fn test() {
    test_http_server!(
        "../vicocomo/examples/http_server/actix",
        TestRequest::new("http://localhost:3000/42")
            .no_redirect(),
        |r: &TestResponse| {
            assert_eq!(r.status(), "302");
            assert_eq!(r.redirect(), "http://localhost:3000/redirect-from-42");
        },
        TestRequest::new("http://localhost:3000/42"),
        |r: &TestResponse| {
            assert_eq!(r.status(), "200");
            assert!(
                Regex::new(
                    "<html>\
                        \\s*<head>\\s*<title>\\s*test\\s*</title>\\s*</head>\
                        \\s*<body>\
                            \\s*<div>\\s*head\\s*</div>\
                            \\s*<div>\\s*hej\\s*&quot;hopp&quot;!\\s*</div>\
                            \\s*<div>\\s*/redirect-from-42\\s*</div>\
                            \\s*<div>\\s*date:\\s*2021-02-22\\s*</div>\
                            \\s*<div>\\s*sist\\s*</div>\
                        \\s*</body>\
                    \\s*</html>",
                )
                .unwrap()
                .is_match(r.body())
            );
        },
    );
}
