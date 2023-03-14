//! Trait and helper types to abstract an SQL database.
//!
use crate::{db_value_convert, Error};
use chrono::{Datelike, NaiveDate, NaiveDateTime, NaiveTime, Timelike};
use std::fmt;

/// An SQL abstraction for use by other `vicocomo` modules as well as
/// applications.
///
#[derive(Clone, Copy)]
pub struct DatabaseIf<'a>(&'a dyn DbConn);

impl<'a> DatabaseIf<'a> {
    /// Create an interface to `client`.
    ///
    pub fn new(client: &'a impl DbConn) -> Self {
        Self(client)
    }

    /// Begin a transaction.
    ///
    pub fn begin(self) -> Result<(), Error> {
        self.0.begin()
    }

    /// Commit the present transaction.
    ///
    /// On error rollback() before returning error.
    ///
    pub fn commit(self) -> Result<(), Error> {
        self.0.commit().map_err(|e| {
            let _ = self.0.rollback();
            e
        })
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
    /// On error rollback() before returning error.
    ///
    pub fn exec(self, sql: &str, values: &[DbValue]) -> Result<usize, Error> {
        self.0.exec(sql, values).map_err(|e| {
            let _ = self.0.rollback();
            e
        })
    }

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
    /// On error rollback() before returning error.
    ///
    pub fn query(
        self,
        sql: &str,
        values: &[DbValue],
        types: &[DbType],
    ) -> Result<Vec<Vec<DbValue>>, Error> {
        self.0.query(sql, values, types).map_err(|e| {
            let _ = self.0.rollback();
            e
        })
    }

    /// Query for one single value from one single column. See [`query()`
    /// ](#method.query). Ignores errors.
    ///
    pub fn query_column(
        self,
        sql: &str,
        values: &[DbValue],
        typ: DbType,
    ) -> Option<DbValue> {
        let mut result: Option<DbValue> = None;
        if let Ok(db_result) = self.query(sql, values, &[typ]) {
            if let Some(db_row) = db_result.first() {
                result = db_row.first().map(|db_val| db_val.clone());
            }
        }
        result
    }

    /// Rollback the present transaction.
    ///
    pub fn rollback(self) -> Result<(), Error> {
        self.0.rollback()
    }
}

/// An SQL abstraction trait for database adapter developers.
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

    /// See [`DatabaseIf::exec()`](struct.DatabaseIf.html#method.exec)
    ///
    fn exec(&self, sql: &str, values: &[DbValue]) -> Result<usize, Error>;

    /// See [`DatabaseIf::query()`](struct.DatabaseIf.html#method.query)
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

impl DbValue {
    /// Clone into an `Option` so that e.g.
    ///     Int(i)          -> Some(Int(i))
    ///     NulInt(None)    -> None
    ///     NulInt(Some(i)) -> Some(Int(i))
    ///
    pub fn to_option(&self) -> Option<Self> {
        match self {
            Self::NulFloat(opt) => opt.map(|f| Self::Float(f)),
            Self::NulInt(opt) => opt.map(|i| Self::Int(i)),
            Self::NulText(opt) => opt.as_ref().map(|s| Self::Text(s.clone())),
            _ => Some(self.clone()),
        }
    }
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

db_value_convert! { in_db_value_module, bool, Int, value != 0 }
db_value_convert! { in_db_value_module, f32, Float }
db_value_convert! { in_db_value_module, f64, Float }
db_value_convert! { in_db_value_module, i32, Int }
db_value_convert! { in_db_value_module, i64, Int }
db_value_convert! {
    in_db_value_module,
    NaiveDate,
    Int,
    NaiveDate::from_num_days_from_ce_opt(value as i32).unwrap(),
    other.num_days_from_ce() as i64,
}
db_value_convert! {
    in_db_value_module,
    NaiveDateTime,
    Int,
    NaiveDateTime::from_timestamp_opt(value, 0).unwrap(),
    other.timestamp(),
}
db_value_convert! {
    in_db_value_module,
    NaiveTime,
    Int,
    NaiveTime::from_num_seconds_from_midnight_opt(value as u32, 0).unwrap(),
    other.num_seconds_from_midnight() as i64,
}
db_value_convert! { in_db_value_module, String, Text }
db_value_convert! { in_db_value_module, u32, Int }
db_value_convert! { in_db_value_module, u64, Int }
db_value_convert! { in_db_value_module, usize, Int }

/// An implementation of [`DbConn`](trait.DbConn.html) that does nothing and
/// returns [`Error`](../error/enum.Error.html).
///
#[derive(Clone, Debug)]
pub struct NullConn;

impl DbConn for NullConn {
    fn exec(&self, _sql: &str, _vals: &[DbValue]) -> Result<usize, Error> {
        Err(Error::database(None, "no database"))
    }

    fn query(
        &self,
        _sql: &str,
        _values: &[DbValue],
        _types: &[DbType],
    ) -> Result<Vec<Vec<DbValue>>, Error> {
        Err(Error::database(None, "no database"))
    }
}
