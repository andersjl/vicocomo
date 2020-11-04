//! Trait and helper types to abstract an SQL database.
//!
use crate::{db_value_convert, Error};
use chrono::{Datelike, NaiveDate, NaiveDateTime, NaiveTime, Timelike};
use std::fmt;

/// An SQL abstraction trait for use by other `vicocomo` modules as well as
/// applications.
///
pub trait DbConn {
    /// Begin a transaction.  The default method simply uses `exec()` to send
    /// `BEGIN` to the database.
    ///
    fn begin(&self) -> Result<(), Error> {
        self.exec("BEGIN", &[]).map(|_| ())
    }

    /// Commit the present transaction.  The default method simply uses
    /// `exec()` to send `COMMIT` to the database.
    ///
    fn commit(&self) -> Result<(), Error> {
        self.exec("COMMIT", &[]).map(|_| ())
    }

    /// Execute an SQL statement.
    ///
    /// `sql` is the statement, which may be parameterized using `$1`, `$2`,
    /// ... to indicate the position of the parameter in `values`.
    ///
    /// `values` are the values for the parameters in `sql`.
    ///
    /// Returns the number of affected rows.
    ///
    fn exec(&self, sql: &str, values: &[DbValue]) -> Result<usize, Error>;

    /// Execute an SQL query and return the result.
    ///
    /// `sql` is the query, which may be parameterized using `$1`, `$2`, ...
    /// to indicate the position of the parameter in `values`.
    ///
    /// `values` are the values for the parameters in `sql`.
    ///
    /// `types` indicates how the implementation should convert the result to
    /// `DbValue` vectors.  `types.len()` must equal the length of each of the
    /// returned `DbValue` vectors.
    ///
    /// Returns the result as a vector of vectors of `DbValue`.
    ///
    fn query(
        &self,
        sql: &str,
        values: &[DbValue],
        types: &[DbType],
    ) -> Result<Vec<Vec<DbValue>>, Error>;

    /// Rollback the present transaction.  The default method simply uses
    /// `exec()` to send `ROLLBACK` to the database.
    ///
    fn rollback(&self) -> Result<(), Error> {
        self.exec("ROLLBACK", &[]).map(|_| ())
    }
}

/// The possible types as seen by the database.
///
/// See [`DbConn::query()`](trait.DbConn.html#tymethod.query)
#[derive(Clone, Debug)]
pub enum DbType {
    /// `f64`
    Float,
    /// `i64`
    Int,
    /// `String`
    Text,
    /// `Option<f64>`
    NulFloat,
    /// `Option<i64>`
    NulInt,
    /// `Option<String>`
    NulText,
}

impl Copy for DbType {}

/// The obvious conversion.
///
impl From<DbValue> for DbType {
    fn from(v: DbValue) -> Self {
        match v {
            DbValue::Float(_) => Self::Float,
            DbValue::Int(_) => Self::Int,
            DbValue::Text(_) => Self::Text,
            DbValue::NulFloat(_) => Self::NulFloat,
            DbValue::NulInt(_) => Self::NulInt,
            DbValue::NulText(_) => Self::NulText,
        }
    }
}

/// The values sent to and from the database by a
/// [`DbConn`](trait.DbConn.html) implementation.
///
/// Implements conversions to and from many Rust types.  The macro
/// [`db_value_convert`](../macro.db_value_convert.html) can be used to
/// implement more conversions, here or in application code.
///
#[derive(Clone, Debug)]
pub enum DbValue {
    Float(f64),
    Int(i64),
    Text(String),
    NulFloat(Option<f64>),
    NulInt(Option<i64>),
    NulText(Option<String>),
}

macro_rules! write_opt {
    ($f:ident, $o:ident) => {
        match $o {
            Some(v) => write!($f, "Some({})", v),
            None => write!($f, "None"),
        }
    };
}

impl fmt::Display for DbValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self {
            DbValue::Float(v) => write!(f, "{}", v),
            DbValue::Int(v) => write!(f, "{}", v),
            DbValue::Text(v) => write!(f, "{}", v),
            DbValue::NulFloat(v) => write_opt!(f, v),
            DbValue::NulInt(v) => write_opt!(f, v),
            DbValue::NulText(v) => write_opt!(f, v),
        }
    }
}

db_value_convert! { no_option_type, bool, Int, value != 0 }
db_value_convert! { no_option_type, f32, Float }
db_value_convert! { no_option_type, f64, Float }
db_value_convert! { no_option_type, i32, Int }
db_value_convert! { no_option_type, i64, Int }
db_value_convert! {
    no_option_type,
    NaiveDate,
    Int,
    NaiveDate::from_num_days_from_ce(value as i32),
    other.num_days_from_ce() as i64,
}
db_value_convert! {
    no_option_type,
    NaiveDateTime,
    Int,
    NaiveDateTime::from_timestamp(value, 0),
    other.timestamp(),
}
db_value_convert! {
    no_option_type,
    NaiveTime,
    Int,
    NaiveTime::from_num_seconds_from_midnight(value as u32, 0),
    other.num_seconds_from_midnight() as i64,
}
db_value_convert! { no_option_type, String, Text }
db_value_convert! { no_option_type, u32, Int }
db_value_convert! { no_option_type, u64, Int }
db_value_convert! { no_option_type, usize, Int }

/// An implementation of [`DbConn`](trait.DbConn.html) that does nothing and
/// returns [`Error`](../error/enum.Error.html).
///
#[derive(Clone, Debug)]
pub struct NullConn;

impl DbConn for NullConn {
    fn exec(&self, _sql: &str, _vals: &[DbValue]) -> Result<usize, Error> {
        Err(Error::database("no database"))
    }

    fn query(
        &self,
        _sql: &str,
        _values: &[DbValue],
        _types: &[DbType],
    ) -> Result<Vec<Vec<DbValue>>, Error> {
        Err(Error::database("no database"))
    }
}
