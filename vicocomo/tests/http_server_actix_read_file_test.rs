use ljumvall_test_utils::{
    create_dir_all, test_http_server, TestRequest, TestResponse,
};

#[test]
fn test_http_server_actix_read_file() {
    use std::fs;

    let content = vec![0xC1u8, 0xC2u8, 0xC3u8];
    let tmp_dir = std::env::temp_dir();
    let src_dir = tmp_dir.join("__vicocomo_source__");
    let _ = fs::remove_dir_all(&src_dir);
    create_dir_all(&src_dir);
    let src1 = src_dir.join("file1.txt").to_string_lossy().to_string();
    let src2 = src_dir.join("filetvå").to_string_lossy().to_string();
    assert!(fs::write(&src1, "file 1").is_ok());
    assert!(fs::write(&src2, &content).is_ok());
    test_http_server!(
        "../vicocomo/examples/http_server/actix_read_file",
        true,
        TestRequest::new("http://localhost:3000/read")
            .no_redirect()
            .form_part("commit", "OK", false)
            .form_part("file_field", &src1, true)
            .form_part("file_field", &src2, true),
        |r: &TestResponse| {
            assert_eq!(r.status(), "200");
            assert_eq!(
                r.body(),
                "\
<html>
  <head>
    <title> Actix File Test </title>
  </head>
  <body>
    <br> type: 
    <br> contents: [79, 75]
    <br> type: text/plain
    <br> contents: [102, 105, 108, 101, 32, 49]
    <br> type: application/octet-stream
    <br> contents: [193, 194, 195]
  </body>
</html>
",
            );
        },
    );
}
