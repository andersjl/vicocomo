use ljumvall_test_utils::{
    create_dir_all, test_http_server, TestRequest, TestResponse,
};

#[test]
fn test_http_server_actix_upload_file() {
    use std::fs;

    let content = vec![0xC1u8, 0xC2u8, 0xC3u8];
    let tmp_dir = std::env::temp_dir();
    let src_dir = tmp_dir.join("__vicocomo_source__");
    let _ = fs::remove_dir_all(&src_dir);
    create_dir_all(&src_dir);
    let tgt_dir = tmp_dir.join("__vicocomo_target__");
    let _ = fs::remove_dir_all(&tgt_dir);
    create_dir_all(&tgt_dir);
    let src1 = src_dir.join("file1.txt").to_string_lossy().to_string();
    let src2 = src_dir.join("filetvå").to_string_lossy().to_string();
    let tgt = tgt_dir.join("upltvå").to_string_lossy().to_string();
    assert!(fs::write(&src1, "file 1").is_ok());
    assert!(fs::write(&src2, &content).is_ok());

    test_http_server!(
        "../vicocomo/examples/http_server/actix_upload_file",
        true,
        TestRequest::new("http://localhost:3000/upload")
            .no_redirect()
            .form_part("commit", "OK", false)
            .form_part("file_field", &src1, true)
            .form_part("file_field", &src2, true),
        |r: &TestResponse| {
            assert_eq!(r.status(), "303");
            assert_eq!(
                r.redirect(),
                "http://localhost:3000/"
            );
            let tgt_cont = fs::read(tgt);
            assert!(tgt_cont.is_ok());
            assert_eq!(tgt_cont.unwrap(), content);
        },
    );
}
