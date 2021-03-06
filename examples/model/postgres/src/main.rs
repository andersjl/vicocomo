// TODO: test optional unique field without value

mod belongs_to;
mod delete;
mod many_to_many;
mod models;
mod multi_pk;
mod one_to_many;
mod single_pk;

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();
    let (pg_client, pg_conn) =
        ::futures::executor::block_on(tokio_postgres::connect(
            &std::env::var("DATABASE_URL").expect("DATABASE_URL must be set"),
            tokio_postgres::NoTls,
        ))
        .expect("cannot connect");
    tokio::spawn(async move {
        if let Err(e) = pg_conn.await {
            eprintln!("connection error: {}", e);
        }
    });
    let pg_conn = ::vicocomo_postgres::PgConn::new(pg_client);
    let db = ::vicocomo::DatabaseIf::new(&pg_conn);

    belongs_to::test_belongs_to(db);
    delete::test_delete(db);
    many_to_many::test_many_to_many(db);
    multi_pk::test_multi_pk(db);
    one_to_many::test_one_to_many(db);
    single_pk::test_single_pk(db);
}
