//! Implement `vicocomo::DbConn` by way of the `tokio-postgres` crate.

use futures::executor::block_on;
use postgres_types;
use tokio_postgres;
use vicocomo::{DbConn, DbType, DbValue, Error};

/// A wrapping of `tokio_postgres::Client` that implements `vicocomo::DbConn`.
pub struct PgConn(tokio_postgres::Client);

impl PgConn {
    pub fn new(client: tokio_postgres::Client) -> Self {
        Self(client)
    }
}

macro_rules! from_values {
    ($values:expr) => {
        &$values
            .iter()
            .map(|val| match val {
                DbValue::Float(v) => v as &(dyn postgres_types::ToSql + Sync),
                DbValue::Int(v) => v as &(dyn postgres_types::ToSql + Sync),
                DbValue::Text(v) => v as &(dyn postgres_types::ToSql + Sync),
                DbValue::NulFloat(v) => {
                    v as &(dyn postgres_types::ToSql + Sync)
                }
                DbValue::NulInt(v) => {
                    v as &(dyn postgres_types::ToSql + Sync)
                }
                DbValue::NulText(v) => {
                    v as &(dyn postgres_types::ToSql + Sync)
                }
            })
            .collect::<Vec<_>>()[..]
    };
}

impl DbConn for PgConn {
    fn exec(&self, sql: &str, vals: &[DbValue]) -> Result<usize, Error> {
        /*
        print!(
            "PgConn.0.exec(\n    {:?},\n    {:?},\n)",
            sql,
            from_values!(vals));
        let result =
        */
        block_on(self.0.execute(sql, from_values!(vals)))
            .map(|i| i as usize)
            .map_err(|e| Error::Database(e.to_string()))
        /*
        ; println!(" -> {:?}", result); result
        */
    }

    fn query(&self, sql: &str, values: &[DbValue], types: &[DbType])
        -> Result<Vec<Vec<DbValue>>, Error>
    {
        /*
        print!(
            "PgConn.0.query(\n    {:?},\n    {:?},\n)",
            sql,
            from_values!(values)
        );
        let result =
        */
        do_query(block_on(self.0.query(sql, from_values!(values))), types)
        /*
        ; println!(" -> {:?}", result); result
        */
    }
}

fn do_query(
    pg_rows: Result<Vec<tokio_postgres::Row>, tokio_postgres::Error>,
    types: &[DbType],
) -> Result<Vec<Vec<DbValue>>, Error> {
    //print!("vicocomo_postgres::do_query(\n    {:?},\n    {:?},\n)", pg_rows, types);
    let mut vicocomo_rows = vec![];
    for postgres_row in
        pg_rows.map_err(|e| Error::Database(e.to_string()))?.iter()
    {
        let mut vicocomo_row = vec![];
        for (ix, typ) in types.iter().enumerate() {
            vicocomo_row.push(match typ {
                DbType::Float => DbValue::Float(
                    postgres_row
                        .try_get::<_, f64>(ix)
                        .map_err(|e| Error::Database(e.to_string()))?,
                ),
                DbType::Int => DbValue::Int(
                    postgres_row
                        .try_get::<_, i64>(ix)
                        .map_err(|e| Error::Database(e.to_string()))?,
                ),
                DbType::Text => DbValue::Text(
                    postgres_row
                        .try_get::<_, String>(ix)
                        .map_err(|e| Error::Database(e.to_string()))?,
                ),
                DbType::NulFloat => DbValue::NulFloat(
                    postgres_row
                        .try_get::<_, Option<f64>>(ix)
                        .map_err(|e| Error::Database(e.to_string()))?,
                ),
                DbType::NulInt => DbValue::NulInt(
                    postgres_row
                        .try_get::<_, Option<i64>>(ix)
                        .map_err(|e| Error::Database(e.to_string()))?,
                ),
                DbType::NulText => DbValue::NulText(
                    postgres_row
                        .try_get::<_, Option<String>>(ix)
                        .map_err(|e| Error::Database(e.to_string()))?,
                ),
            });
        }
        vicocomo_rows.push(vicocomo_row);
    }
    //println!( " -> {:?}", vicocomo_rows);
    Ok(vicocomo_rows)
}
