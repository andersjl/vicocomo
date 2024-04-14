//! Implement `vicocomo::DbConn` by way of the `rusqlite` crate.

use rusqlite::{Connection, ToSql};
use std::path::Path;
use std::sync::Mutex;
use vicocomo::{
    DbConn, DbType, DbValue, Error, SQLSTATE_FOREIGN_KEY_VIOLATION,
    SQLSTATE_UNIQUE_VIOLATION,
};

/// A wrapping of `sqlite::Connection` that implements `vicocomo::DbConn`.
///
// Connection is not Send, which is needed e.g. when used as managed data by
// actix-web. Hence Mutex.
//
pub struct SqliteConn(Mutex<Connection>);

impl SqliteConn {
    /// Try to create with [default flags
    /// ](https://docs.rs/rusqlite/latest/rusqlite/struct.Connection.html#flags)
    /// and enabled foreign key support.
    ///
    pub fn new(path: &Path) -> Result<Self, Error> {
        Ok(Self(Mutex::new(Self::result(
            Connection::open(path).and_then(|conn| {
                conn.execute("PRAGMA foreign_keys = ON", [])?;
                Ok(conn)
            }),
        )?)))
    }

    /// Replace `$` with `?`.
    fn fix_params(params: &str) -> String {
        params.replace("$", "?")
    }

    /// Convert `res` to `Result<T, vicocomo::Error>`.
    fn result<T>(res: rusqlite::Result<T>) -> Result<T, Error> {
        res.map_err(|e| {
            let mut sqlstate = None;
            if let rusqlite::Error::SqliteFailure(err, ref msg) = e {
                if let rusqlite::ErrorCode::ConstraintViolation = err.code {
                    if let Some(text) = msg {
                        if text.starts_with("FOREIGN KEY") {
                            sqlstate = Some(SQLSTATE_FOREIGN_KEY_VIOLATION);
                        } else if text.starts_with("UNIQUE") {
                            sqlstate = Some(SQLSTATE_UNIQUE_VIOLATION);
                        }
                    }
                }
            }
            Error::database(sqlstate, &e)
        })
    }
}

macro_rules! from_values {
    ($values:expr) => {
        &$values
            .iter()
            .map(|val| match val {
                DbValue::Float(v) => v as &dyn ToSql,
                DbValue::Int(v) => v as &dyn ToSql,
                DbValue::Text(v) => v as &dyn ToSql,
                DbValue::NulFloat(v) => v as &dyn ToSql,
                DbValue::NulInt(v) => v as &dyn ToSql,
                DbValue::NulText(v) => v as &dyn ToSql,
            })
            .collect::<Vec<_>>()[..]
    };
}

impl DbConn for SqliteConn {
    fn exec(&self, sql: &str, vals: &[DbValue]) -> Result<usize, Error> {
        Self::result(
            self.0
                .lock()
                .unwrap()
                .execute(&Self::fix_params(sql), from_values!(vals)),
        )
    }

    fn query(
        &self,
        sql: &str,
        vals: &[DbValue],
        types: &[DbType],
    ) -> Result<Vec<Vec<DbValue>>, Error> {
        Self::result(
            self.0
                .lock()
                .unwrap()
                .prepare(&Self::fix_params(sql))
                .and_then(|mut stmt| {
                    Ok({
                        let mut rows = Vec::new();
                        for row_result in stmt.query_map(
                            from_values!(vals),
                            |sqlt_row| -> rusqlite::Result<Vec<DbValue>> {
                                let mut vicocomo_row = Vec::new();
                                for (ix, typ) in types.iter().enumerate() {
                                    vicocomo_row.push(match typ {
                                        DbType::Float => DbValue::Float(
                                            sqlt_row.get::<_, f64>(ix)?,
                                        ),
                                        DbType::Int => DbValue::Int(
                                            sqlt_row.get::<_, i64>(ix)?,
                                        ),
                                        DbType::Text => DbValue::Text(
                                            sqlt_row.get::<_, String>(ix)?,
                                        ),
                                        DbType::NulFloat => {
                                            DbValue::NulFloat(
                                                sqlt_row
                                                    .get::<_, Option<f64>>(
                                                        ix,
                                                    )?,
                                            )
                                        }
                                        DbType::NulInt => DbValue::NulInt(
                                            sqlt_row
                                                .get::<_, Option<i64>>(ix)?,
                                        ),
                                        DbType::NulText => DbValue::NulText(
                                            sqlt_row
                                                .get::<_, Option<String>>(
                                                    ix,
                                                )?,
                                        ),
                                    })
                                }
                                Ok(vicocomo_row)
                            },
                        )? {
                            rows.push(row_result?)
                        }
                        rows
                    })
                }),
        )
    }
}
