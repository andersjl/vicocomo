use std::sync::Arc;
use vicocomo_example_model_common::*;

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();
    let (pg_client, pg_conn) =
        ::futures::executor::block_on(tokio_postgres::connect(
            &std::env::var("VICOCOMO_TEST_DB_POSTGRES")
                .expect("VICOCOMO_TEST_DB_POSTGRES must be set"),
            tokio_postgres::NoTls,
        ))
        .expect("cannot connect");
    tokio::spawn(async move {
        if let Err(e) = pg_conn.await {
            eprintln!("connection error: {}", e);
        }
    });
    let pg_conn = ::vicocomo_postgres::PgConn::new(pg_client);
    let db = ::vicocomo::DatabaseIf::new(Arc::new(pg_conn));
    setup(db.clone(), "BIGSERIAL PRIMARY KEY");

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
