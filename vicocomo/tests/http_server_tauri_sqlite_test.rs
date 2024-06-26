#[test]
fn test_http_server_tauri_sqlite() {
    ljumvall_test_utils::test_command(
        "../vicocomo/examples/http_server/tauri_sqlite",
        "cargo",
        &["tauri", "build", "-b"],
        false,
        false,
        ljumvall_test_utils::TestCommandOutput::Whatever,
        None,
    );
    /*
     * While waiting for a not-so-buggy tauri-driver, test manually following
     * ../examples/http_server/tauri_sqlite/webdriver/webdriverio/test/specs/ui.js
     *
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
        true,
        ljumvall_test_utils::TestCommandOutput::Whatever,
        None,
    );
     *
     */
}
