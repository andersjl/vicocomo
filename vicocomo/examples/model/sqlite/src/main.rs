use std::sync::Arc;
use vicocomo_example_model_common::*;

fn main() {
    let sqlite_conn =
        vicocomo_sqlite::SqliteConn::new(std::path::Path::new("test.sqlite"))
            .unwrap();
    let db = vicocomo::DatabaseIf::new(Arc::new(sqlite_conn));
    setup(db.clone(), "INTEGER  PRIMARY KEY AUTOINCREMENT");

    test_belongs_to(db.clone());
    test_delete(db.clone());
    test_many_to_many(db.clone());
    test_multi_pk(db.clone());
    test_no_pk(db.clone());
    test_nonstandard_parent(db.clone());
    test_one_to_many(db.clone());
    test_random(db.clone());
    test_single_pk(db.clone());
    test_csv(db.clone());

    println!("\ntest completed successfully -----------------------------\n");
}
