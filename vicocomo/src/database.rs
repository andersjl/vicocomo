//! Trait and helper types to abstract an SQL database.
//!
use crate::{db_value_convert, map_error, Error};
use chrono::{
    DateTime, Datelike, NaiveDate, NaiveDateTime, NaiveTime, Timelike, Utc,
};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::sync::Arc;

/// An SQL abstraction for use by other `vicocomo` modules as well as
/// applications.
///
#[derive(Clone)]
pub struct DatabaseIf(Arc<dyn DbConn + Send + Sync>);

impl DatabaseIf {
    /// Create an interface to `client`.
    ///
    pub fn new(client: Arc<dyn DbConn + Send + Sync>) -> Self {
        Self(client.clone())
    }

    /// Begin a transaction.
    ///
    pub fn begin(self) -> Result<(), Error> {
        self.0.begin()
    }

    /// Commit the present transaction.
    ///
    /// On error try to `rollback()` before returning error.
    ///
    pub fn commit(self) -> Result<(), Error> {
        self.0.commit().map_err(|commit_err| {
            if let Err(rollback_err) = self.0.rollback() {
                rollback_err
            } else {
                commit_err
            }
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
    pub fn exec(self, sql: &str, values: &[DbValue]) -> Result<usize, Error> {
        self.0.exec(sql, values)
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
    pub fn query(
        self,
        sql: &str,
        values: &[DbValue],
        types: &[DbType],
    ) -> Result<Vec<Vec<DbValue>>, Error> {
        self.0.query(sql, values, types)
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
    /// <b>Errors</b>
    ///
    /// Forwards any error from the database adapter.
    ///
    pub fn rollback(self) -> Result<(), Error> {
        self.0.rollback()
    }

    /// Wrap code in a database transaction and ensure `ROLLBACK` on any error
    /// -- <b>not only database errors!</b>.
    ///
    /// `action` is a closure that takes the database connection as a
    /// parameter and returns `T` wrapped in a `Result`.
    ///
    /// Returns what `action` returns. Before returning does a `COMMIT` or
    /// `ROLLBACK` depending on whether `action` succeeds.
    ///
    pub fn transaction<T, F>(self, action: F) -> Result<T, Error>
    where
        F: FnOnce(DatabaseIf) -> Result<T, Error>,
    {
        let _ = self.0.begin();
        let result = action(self.clone());
        let _ = match result {
            Ok(_) => self.0.commit(),
            Err(_) => self.0.rollback(),
        };
        result
    }
}

/// An SQL abstraction trait for database adapter developers.
///
pub trait DbConn: Send + Sync {
    /// Begin a transaction.
    ///
    /// The default method simply uses `exec()` to send `BEGIN` to the
    /// database.
    ///
    fn begin(&self) -> Result<(), Error> {
        #[cfg(debug_assertions)]
        eprintln!("BEGIN");
        self.exec("BEGIN", &[]).map(|_| ())
    }

    /// Commit the present transaction.  The default method simply uses
    /// `exec()` to send `COMMIT` to the database.
    ///
    fn commit(&self) -> Result<(), Error> {
        #[cfg(debug_assertions)]
        eprintln!("COMMIT");
        self.exec("COMMIT", &[]).map(|_| ())
    }

    /// See [`DatabaseIf::exec()`](struct.DatabaseIf.html#method.exec)
    ///
    /// Required to return an [`Error::Database`
    /// ](../error/enum.Database.html#variant.Database)) with [`sqlstate`
    /// ](../error/struct.DatabaseError.html#structfield.sqlstate)
    /// [`SQLSTATE_FOREIGN_KEY_VIOLATION`
    /// ](../error/constant.SQLSTATE_FOREIGN_KEY_VIOLATION.html) (or
    /// [`SQLSTATE_UNIQUE_VIOLATION`
    /// ](../error/constant.SQLSTATE_UNIQUE_VIOLATION.html)) if such an error
    /// is returned - *regardless of whether the database driver returns the
    /// corrrect* `SQLSTATE` *code*!
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
        #[cfg(debug_assertions)]
        eprintln!("ROLLBACK");
        self.exec("ROLLBACK", &[]).map(|_| ())
    }
}

/// The possible types as seen by the database.
///
/// See [`DbConn::query()`](trait.DbConn.html#tymethod.query)
#[derive(Clone, Debug, Eq, PartialEq)]
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
    /// ```text
    /// Int(i)          -> Some(Int(i))
    /// NulInt(None)    -> None
    /// NulInt(Some(i)) -> Some(Int(i))
    /// ```
    pub fn to_option(&self) -> Option<Self> {
        match self {
            Self::NulFloat(opt) => opt.map(|f| Self::Float(f)),
            Self::NulInt(opt) => opt.map(|i| Self::Int(i)),
            Self::NulText(opt) => opt.as_ref().map(|s| Self::Text(s.clone())),
            _ => Some(self.clone()),
        }
    }

    /// Write the value as accepted by SQL, e.g.
    /// ```text
    /// Int(42)          -> "42"
    /// NulInt(None)     -> "NULL"
    /// NulInt(Some(42)) -> "42"
    /// Text("foo")      -> "'foo'"
    /// ```
    pub fn sql_value(&self) -> String {
        match self {
            DbValue::Float(v) => v.to_string(),
            DbValue::Int(v) => v.to_string(),
            DbValue::Text(v) => format!("'{}'", v.replace("'", "''")),
            DbValue::NulFloat(v) => match v {
                Some(v) => v.to_string(),
                None => "NULL".to_string(),
            },
            DbValue::NulInt(v) => match v {
                Some(v) => v.to_string(),
                None => "NULL".to_string(),
            },
            DbValue::NulText(v) => match v {
                Some(v) => format!("'{}'", v.replace("'", "''")),
                None => "NULL".to_string(),
            },
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
    DateTime::<Utc>::from_timestamp(value, 0).unwrap().naive_utc(),
    other.and_utc().timestamp(),
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

/// Facilitates conversions between [`DbValue::Text`
/// ](enum.DbValue.html#variant.Text) and any JSON-serializable type.
///
/// There are no conversions between `Option<JsonField>` and
/// [`DbValue::NulText](enum.DbValue.html#variant.NulText)`.
///
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct JsonField<T>(pub T);

impl<T: DeserializeOwned + Serialize> Into<DbValue> for JsonField<T> {
    fn into(self) -> DbValue {
        DbValue::Text(serde_json::to_string(&self.0).expect(&format!(
            "serde_json.to_string() cannot handle {}",
            std::any::type_name::<T>(),
        )))
    }
}

impl<T: ::std::fmt::Debug + DeserializeOwned + Serialize> TryFrom<DbValue>
    for JsonField<T>
{
    type Error = Error;
    fn try_from(db_value: DbValue) -> Result<Self, Self::Error> {
        match db_value {
            DbValue::Text(value) => Ok(Self(map_error!(
                InvalidInput,
                serde_json::from_str(&value),
            )?)),
            _ => Err(Error::invalid_input(&format!(
                "cannot convert {db_value:?} into {}",
                std::any::type_name::<T>(),
            ))),
        }
    }
}

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

/// Try to execute SQL statements read from `source`. The contents of `file`
/// are simply split at `';'` to produce a number of SQL parameters to
/// [`DatabaseIf::exec()`](struct.DatabaseIf.html#method.exec).
///
/// <b>Errors</b>
///
/// If execution fails the `original_error` is returned if `Some(_)`,
/// otherwise the error returned from the failing `exec()`.
///
pub fn try_exec_sql(
    db: DatabaseIf,
    source: &str,
    original_error: Option<Error>,
) -> Result<(), Error> {
    for statement in source.split(';') {
        let statement = statement.trim();
        if statement.is_empty() {
            continue;
        }
        if let Err(e) = db.clone().exec(statement, &[]) {
            return Err(original_error.unwrap_or(e));
        }
    }
    Ok(())
}
