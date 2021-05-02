mod model;

#[test]
fn test_model_postgres() {
    model::postgres::test();
}
