#[test]
fn test_model_postgres() {
    ljumvall_test_utils::test_crate(
        "../vicocomo/examples/model/postgres",
        &["run"],
        false,
        false,
        ljumvall_test_utils::TestCrateOutput::None,
    );
}
