#[test]
fn test_http_server_tauri_sqlite() {
    const TEST_DIR: &'static str =
        "../vicocomo/examples/http_server/tauri_sqlite/webdriver/webdriverio";
    ljumvall_test_utils::test_command(
        TEST_DIR,
        "./init.sh",
        &[],
        false,
        false,
        ljumvall_test_utils::TestCommandOutput::Whatever,
        None,
    );
    ljumvall_test_utils::test_command(
        TEST_DIR,
        "npm",
        &["test"],
        false,
        false,
        ljumvall_test_utils::TestCommandOutput::Whatever,
        None,
    );
}
