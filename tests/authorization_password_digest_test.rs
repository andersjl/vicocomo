#[test]
fn test_authorization_password_digest() {
    vicocomo::test_utils::test_crate(
        "../vicocomo/examples/authorization/password_digest",
        false,
        "run",
    );
}
