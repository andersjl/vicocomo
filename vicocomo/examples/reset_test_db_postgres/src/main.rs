#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();
    let (pg_client, pg_conn) =
        futures::executor::block_on(tokio_postgres::connect(
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
    let pg_conn = vicocomo_postgres::PgConn::new(pg_client);
    let db = vicocomo::DatabaseIf::new(&pg_conn);
    assert!(db
        .exec("DROP TABLE IF EXISTS __vicocomo__sessions", &[])
        .is_ok());
    assert!(db
        .exec(
            "CREATE TABLE __vicocomo__sessions\
            ( id    BIGINT  PRIMARY KEY\
            , data  TEXT    NOT NULL\
            , time  BIGINT  NOT NULL\
            )",
            &[],
        )
        .is_ok());
}
