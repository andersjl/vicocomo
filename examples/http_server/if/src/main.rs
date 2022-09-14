fn main() {
    use chrono::NaiveDate;
    use vicocomo::{AppConfigVal, HttpServerIf};
    use vicocomo_stubs::{Request, Server};

    let stub = Server::new();
    stub.add_config("url_root", AppConfigVal::Str("/root".to_string()));
    stub.add_config("file_root", AppConfigVal::Str("root/".to_string()));
    let srv = HttpServerIf::new(&stub);

    stub.set_params(&[
        ("smpl", "1"),
        ("arr[]", "2"),
        ("arr[]", "3"),
        ("map[a]", "4"),
        ("map[b]", "5"),
        ("deep[c][]", "6"),
        ("deep[c][]", "7"),
        ("deep[d]", "8"),
        ("mtrx[][]", "9"),
    ]);

    print!("param_json() ... ");
    let json = srv.param_json();
    assert!(json.is_ok());
    assert_eq!(
        json.unwrap(),
        serde_json::json!({
            "smpl": "1",
            "arr":  ["2", "3"],
            "map":  { "a": "4", "b": "5" },
            "deep": { "c": ["6", "7"], "d": "8" },
            "mtrx": [["9"]],
        })
    );
    println!("OK");

    print!("param_val() ... ");
    // param_val() returns the first value in a vector
    let i: Option<u32> = srv.param_val("arr[]");
    assert!(i.is_some());
    assert_eq!(i.unwrap(), 2);
    let i: Option<u32> = srv.param_val("mtrx[][]");
    assert!(i.is_some());
    assert_eq!(i.unwrap(), 9);
    println!("OK");

    stub.set_request(Request {
        scheme: "http".to_string(),
        host: "example.com".to_string(),
        path: "/root/some/<par>/path".to_string(),
        parameters: "foo=bar".to_string(),
        body: String::new(),
    });

    print!("req_path() ... ");
    assert_eq!(srv.req_path(), "/some/<par>/path");
    println!("OK");

    stub.set_route_pars(&[
        ("foo", "42"),
        ("bar", "43"),
        ("baz", "2020-02-02"),
    ]);

    print!("req_route_par_val() ... ");
    assert_eq!(srv.req_route_par_val("foo"), Some("42".to_string()));
    assert_eq!(srv.req_route_par_val("bar"), Some(43i32));
    assert_eq!(
        srv.req_route_par_val("baz"),
        Some(NaiveDate::from_ymd(2020, 02, 02)),
    );
    println!("OK");

    print!("req_url() ... ");
    assert_eq!(
        srv.req_url(),
        "http://example.com/root/some/<par>/path?foo=bar",
    );
    println!("OK");

    print!("resp_file() ... ");
    srv.resp_file("some/file.txt");
    assert_eq!(stub.get_response().text, "root/some/file.txt");
    srv.resp_file("some/file-1234567890.txt");
    assert_eq!(stub.get_response().text, "root/some/file-1234567890.txt");
    stub.add_config("strip_mtime", AppConfigVal::Bool(true));
    srv.resp_file("some/file-1234567890.txt");
    assert_eq!(stub.get_response().text, "root/some/file.txt");
    srv.resp_file("some/file-123456789.txt");
    assert_eq!(stub.get_response().text, "root/some/file-123456789.txt");
    srv.resp_file("some/file-1234567890");
    assert_eq!(stub.get_response().text, "root/some/file");
    println!("OK");

    print!("resp_redirect() ... ");
    srv.resp_redirect("foo/bar");
    assert_eq!(stub.get_response().text, "foo/bar");
    srv.resp_redirect("/foo/bar");
    assert_eq!(stub.get_response().text, "/root/foo/bar");
    println!("OK");

    stub.set_static_routes(&[
        ("/root/some/url", "root/some/dir"),
        ("/root/other/url", "/absolute/dir"),
    ]);

    print!("url_path_to_dir() ... ");
    assert!(srv.url_path_to_dir("foo/bar").is_none());
    assert_eq!(
        srv.url_path_to_dir("/some/url"),
        Some("some/dir/".to_string())
    );
    assert_eq!(
        srv.url_path_to_dir("/other/url"),
        Some("/absolute/dir/".to_string()),
    );
    println!("OK");

    println!("\nAll OK");
}
