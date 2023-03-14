#[test]
fn test_http_server_if() {
    ljumvall_test_utils::test_crate(
        "../vicocomo/examples/http_server/if",
        &["run"],
        false,
        false,
        ljumvall_test_utils::TestCrateOutput::None,
    );
}
