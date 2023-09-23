#[test]
fn test_authorization_password_digest() {
    ljumvall_test_utils::test_crate(
        "../vicocomo/examples/authorization/password_digest",
        &["run"],
        false,
        false,
        ljumvall_test_utils::TestCommandOutput::Whatever,
    );
}
