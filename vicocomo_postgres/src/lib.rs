//! Implement `::vicocomo::DbConn` by way of the `tokio-postgres` crate.

use ::vicocomo::{DbConn, DbType, DbValue, Error};
use futures::executor::block_on;
use postgres_types;

/// A wrapping of `::tokio_postgres::Client` that implements
/// `::vicocomo::DbConn`.
///
pub struct PgConn(::tokio_postgres::Client);

impl PgConn {
    pub fn new(client: ::tokio_postgres::Client) -> Self {
        Self(client)
    }

    fn error(&self, err: &::tokio_postgres::error::Error) -> Error {
        self.rollback().unwrap_or(());
        Error::database(&err.to_string())
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
        match block_on(self.0.execute(sql, from_values!(vals))) {
            Ok(i) => Ok(i as usize),
            Err(e) => Err(self.error(&e)),
        }
    }

    fn query(
        &self,
        sql: &str,
        values: &[DbValue],
        types: &[DbType],
    ) -> Result<Vec<Vec<DbValue>>, Error> {
        match block_on(self.0.query(sql, from_values!(values))) {
            Ok(pg_rows) => {
                let mut vicocomo_rows = Vec::new();
                for postgres_row in pg_rows {
                    let mut vicocomo_row = Vec::new();
                    for (ix, typ) in types.iter().enumerate() {
                        vicocomo_row.push(match typ {
                            DbType::Float => DbValue::Float(
                                match postgres_row.try_get::<_, f64>(ix) {
                                    Ok(val) => val,
                                    Err(e) => return Err(self.error(&e)),
                                },
                            ),
                            DbType::Int => DbValue::Int(
                                match postgres_row.try_get::<_, i64>(ix) {
                                    Ok(val) => val,
                                    Err(e) => return Err(self.error(&e)),
                                },
                            ),
                            DbType::Text => DbValue::Text(match postgres_row
                                .try_get::<_, String>(
                                ix,
                            ) {
                                Ok(val) => val,
                                Err(e) => return Err(self.error(&e)),
                            }),
                            DbType::NulFloat => DbValue::NulFloat(
                                match postgres_row
                                    .try_get::<_, Option<f64>>(ix)
                                {
                                    Ok(val) => val,
                                    Err(e) => return Err(self.error(&e)),
                                },
                            ),
                            DbType::NulInt => DbValue::NulInt(
                                match postgres_row
                                    .try_get::<_, Option<i64>>(ix)
                                {
                                    Ok(val) => val,
                                    Err(e) => return Err(self.error(&e)),
                                },
                            ),
                            DbType::NulText => DbValue::NulText(
                                match postgres_row
                                    .try_get::<_, Option<String>>(ix)
                                {
                                    Ok(val) => val,
                                    Err(e) => return Err(self.error(&e)),
                                },
                            ),
                        });
                    }
                    vicocomo_rows.push(vicocomo_row);
                }
                Ok(vicocomo_rows)
            }
            Err(e) => Err(self.error(&e)),
        }
    }
}
