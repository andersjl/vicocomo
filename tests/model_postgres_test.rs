#[test]
fn test_model_postgres() {
    vicocomo::test_utils::test_crate(
        "../vicocomo/examples/model/postgres",
        false,
        "run",
    );
}
