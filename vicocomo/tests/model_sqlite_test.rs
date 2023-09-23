#[test]
fn test_model_sqlite() {
    ljumvall_test_utils::test_crate(
        "../vicocomo/examples/model/sqlite",
        &["run"],
        false,
        false,
        ljumvall_test_utils::TestCommandOutput::Whatever,
    );
}
