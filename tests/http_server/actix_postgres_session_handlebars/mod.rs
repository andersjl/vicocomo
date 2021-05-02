use ::regex::Regex;
use ::vicocomo::test_http_server;
use ::vicocomo::test_utils::{TestRequest, TestResponse};

#[allow(dead_code)]
pub fn test() {
    test_http_server!(
        "../vicocomo/examples/http_server/actix_postgres_session_handlebars",
        TestRequest::new("http://localhost:3000"),
        |r: &TestResponse| {
            assert_eq!(r.status(), "200");
            println!("{}", r.body());
            assert!(
                Regex::new(
                    "<html>\
                        \\s*<head>\\s*<title>\\s*test\\s*</title>\\s*</head>\
                        \\s*<body>\
                            \\s*<div>\\s*partial\\s*</div>\
                            \\s*<div>\\s*falskt\\s*</div>\
                            \\s*<div>\\s*lingonskogen\\s*</div>\
                            \\s*firstett\\s*first\\s*last:\\s*två\
                            \\s*<div>\\s*ignoring empty array\\s*</div>\
                            \\s*<div>\\s*mera mer\\s*</div>\
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
