use ::regex::Regex;
use ::vicocomo::test_http_server;
use ::vicocomo::test_utils::{TestRequest, TestResponse};

#[test]
fn test_http_server_actix() {
    use std::fs;
    let timestamp = chrono::Utc::now().timestamp().to_string();
    fs::write(std::env::temp_dir().join("timestamp.txt"), &timestamp).unwrap();
    fs::write(
        "examples/http_server/actix/public/txt/timestamp.txt",
        &timestamp,
    ).unwrap();
    let static_file_content =
        Regex::new(
            "<html>\
                \\s*<head>\
                    \\s*<title>\\s*test static\\s*</title>\
                    \\s*<link type=\"text/css\" rel=\"stylesheet\" href=\"/test/static/application.css\" />\
                \\s*</head>\
                \\s*<body>\
                    \\s*Static file content\
                    \\s*<div class=\"hello\">\\s*hello\\s*</div>\
                \\s*</body>\
            \\s*</html>",
        )
        .unwrap();
    test_http_server!(
        "../vicocomo/examples/http_server/actix",
        true,
        TestRequest::new("http://localhost:3000/test/home/42").no_redirect(),
        |r: &TestResponse| {
            assert_eq!(r.status(), "302");
            assert_eq!(
                r.redirect(),
                "http://localhost:3000/test/home/redirect-from-42"
            );
        },
        TestRequest::new("http://localhost:3000/test/home/42"),
        |r: &TestResponse| {
            assert_eq!(
                r.status(),
                "200",
                "\n{}\n",
                r.body(),
            );
            assert!(
                Regex::new(
                    "<html>\
                        \\s*<head>\\s*<title>\\s*test\\s*</title>\\s*</head>\
                        \\s*<body>\
                            \\s*<div>\\s*head\\s*</div>\
                            \\s*<div>\\s*hej\\s*&quot;hopp&quot;!\\s*</div>\
                            \\s*<div>\\s* /home/redirect-from-42\\s*</div>\
                            \\s*<div>\\s*url: /test/home/redirect-from-42\\s*</div>\
                            \\s*<div>\\s*<a href=\"/test/static/txt/timestamp-\\d{10}.txt\">\\s*test file\\s*</a>\\s*</div>\
                            \\s*<div>\\s*date:\\s*2021-02-22\\s*</div>\
                            \\s*<div>\\s*sist\\s*</div>\
                        \\s*</body>\
                    \\s*</html>",
                )
                .unwrap()
                .is_match(r.body()),
                "got\n{}\n",
                r.body(),
            );
        },
        TestRequest::new("http://localhost:3000/test/home/redirect?foo=bar"),
        |r: &TestResponse| {
            assert_eq!(
                r.status(),
                "200",
                "\n{}\n",
                r.body(),
            );
            assert!(
                Regex::new(
                    "<html>\
                        \\s*<head>\\s*<title>\\s*test\\s*</title>\\s*</head>\
                        \\s*<body>\
                            \\s*<div>\\s*head\\s*</div>\
                            \\s*<div>\\s*hej\\s*&quot;hopp&quot;!\\s*</div>\
                            \\s*<div>\\s* /home/redirect\\s*</div>\
                            \\s*<div>\\s*url: /test/home/redirect\\?foo&#x3D;bar\\s*</div>\
                            \\s*<div>\\s*<a href=\"/test/static/txt/timestamp-\\d{10}.txt\">\\s*test file\\s*</a>\\s*</div>\
                            \\s*<div>\\s*date:\\s*2021-02-22\\s*</div>\
                            \\s*<div>\\s*sist\\s*</div>\
                        \\s*</body>\
                    \\s*</html>",
                )
                .unwrap()
                .is_match(r.body()),
                "got\n{}\n",
                r.body(),
            );
        },
        //TestRequest::new("http://localhost:3000/test/dynamic")
            //.post("foo", "a+b c"),
        TestRequest::new("http://localhost:3000/test/dynamic?foo=a%2Bb+c"),
        |r: &TestResponse| {
            assert_eq!(
                r.status(),
                "200",
                "\n{}\n",
                r.body(),
            );
            assert_eq!(
                r.body(),
                //"foo=a%2Bb+c\na+b c\n",
                "/test/dynamic?foo=a%2Bb+c\na+b c\n",
                "got\n{}\n",
                r.body(),
            );
        },
        TestRequest::new(
            "http://localhost:3000/test/static/static-1234567890.html",
        ),
        |r: &TestResponse| {
            assert_eq!(
                r.status(),
                "200",
                "\n{}\n",
                r.body(),
            );
            assert!(
                static_file_content.is_match(r.body()),
                "got\n{}\n",
                r.body(),
            );
        },
        TestRequest::new(
            &format!(
                "http://localhost:3000/test/static/txt/timestamp-{}.txt",
                &timestamp,
            ),
        ),
        |r: &TestResponse| {
            assert_eq!(
                r.status(),
                "200",
                "\n{}\n",
                r.body(),
            );
            assert_eq!(r.body(), timestamp);
        },
        TestRequest::new(
            &format!("http://localhost:3000/test/file/tmp-{}", &timestamp),
        ),
        |r: &TestResponse| {
            assert_eq!(
                r.status(),
                "200",
                "\n{}\n",
                r.body(),
            );
            assert_eq!(r.body(), timestamp);
        },
        TestRequest::new("http://localhost:3000/test/date/2021-12-31"),
        |r: &TestResponse| {
            assert_eq!(
                r.status(),
                "200",
                "\n{}\n",
                r.body(),
            );
            assert!(
                r.body() == "2022-01-01",
                "got\n{}\n",
                r.body(),
            );
        },
    );
}
