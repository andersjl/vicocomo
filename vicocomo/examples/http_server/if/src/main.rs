fn main() {
    use chrono::NaiveDate;
    use vicocomo::{AppConfigVal, HttpRespBody, HttpServerIf};
    use vicocomo_stubs::{Request, Server};

    let srv_stub = Server::new();
    srv_stub.add_config("url_root", AppConfigVal::Str("/root".to_string()));
    srv_stub.add_config("file_root", AppConfigVal::Str("root/".to_string()));
    let req_stub = Request::new();
    let srv = HttpServerIf::new(&srv_stub, &req_stub);

    req_stub.set_params(&[
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

    *req_stub.scheme.borrow_mut() = "http".to_string();
    *req_stub.host.borrow_mut() = "example.com".to_string();
    *req_stub.path.borrow_mut() = "/root/some/<par>/path".to_string();
    *req_stub.parameters.borrow_mut() = "foo=bar".to_string();
    *req_stub.body.borrow_mut() = Vec::new();

    print!("req_path() ... ");
    assert_eq!(srv.req_path(), "/some/<par>/path");
    println!("OK");

    req_stub.set_route_pars(&[
        ("foo", "42"),
        ("bar", "43"),
        ("baz", "2020-02-02"),
    ]);

    print!("req_route_par_val() ... ");
    assert_eq!(srv.req_route_par_val("foo"), Some("42".to_string()));
    assert_eq!(srv.req_route_par_val("bar"), Some(43i32));
    assert_eq!(
        srv.req_route_par_val("baz"),
        Some(NaiveDate::from_ymd_opt(2020, 02, 02).unwrap()),
    );
    println!("OK");

    print!("req_url() ... ");
    assert_eq!(
        srv.req_url(),
        "http://example.com/root/some/<par>/path?foo=bar",
    );
    println!("OK");

    print!("resp_download() ... ");
    assert_eq!(
        srv.resp_download("some/file.txt").get_body(),
        HttpRespBody::Download("root/some/file.txt".into()),
    );
    assert_eq!(
        srv.resp_download("some/file-1234567890.txt").get_body(),
        HttpRespBody::Download("root/some/file-1234567890.txt".into()),
    );
    srv_stub.add_config("strip_mtime", AppConfigVal::Bool(true));
    assert_eq!(
        srv.resp_download("some/file-1234567890.txt").get_body(),
        HttpRespBody::Download("root/some/file.txt".into()),
    );
    assert_eq!(
        srv.resp_download("some/file-123456789.txt").get_body(),
        HttpRespBody::Download("root/some/file-123456789.txt".into()),
    );
    assert_eq!(
        srv.resp_download("some/file-1234567890").get_body(),
        HttpRespBody::Download("root/some/file".into()),
    );
    println!("OK");

    print!("resp_redirect() ... ");
    assert_eq!(
        srv.resp_redirect("foo/bar")
            .drain_headers()
            .collect::<Vec<_>>(),
        vec![(String::from("Location"), String::from("foo/bar"))],
    );
    assert_eq!(
        srv.resp_redirect("/foo/bar")
            .drain_headers()
            .collect::<Vec<_>>(),
        vec![(String::from("Location"), String::from("/root/foo/bar"))],
    );
    println!("OK");

    srv_stub.set_static_routes(&[
        ("/root/some/url", "root/some/dir"),
        ("/root/other/url", "/absolute/dir"),
    ]);

    println!("\nAll OK");
}
