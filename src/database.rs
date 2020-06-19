use crate::{db_value_convert, Error};
use std::{convert::TryInto, fmt};

pub trait DbConn<'a> {
    // The type of the transaction() return value.
    type Transaction: DbConn<'a>;

    // Commit the present transaction.  The default method returns an error.
    fn commit(self: Box<Self>) -> Result<(), Error> {
        Err(Error::database("not in a transaction"))
    }

    // Execute an SQL statement.
    //
    // sql is the statement, which may be parameterized using "$1", "$2", ...
    // to indicate the position of the parameter in values.
    //
    // values are the values for the parameters in sql.
    //
    // Returns the number of affected rows.
    //
    fn exec(&mut self, sql: &str, values: &[DbValue])
        -> Result<usize, Error>;

    // Execute an SQL query and return the result.
    //
    // sql is the query, which may be parameterized using "$1", "$2", ...
    // to indicate the position of the parameter in values.
    //
    // values are the values for the parameters in sql.
    //
    // types indicates how the implementation should convert the result to
    // DbValue vectors.  types.len() must equal the length of each of the
    // returned DbValue vectors.
    //
    // Returns the result as a vector of vectors ov DbValue.
    //
    fn query(
        &mut self,
        sql: &str,
        values: &[DbValue],
        types: &[DbType],
    ) -> Result<Vec<Vec<DbValue>>, Error>;

    // Rollback the present transaction.  The default method returns an error.
    fn rollback(self: Box<Self>) -> Result<(), Error> {
        Err(Error::database("not in a transaction"))
    }

    // Return a transaction object.
    fn transaction(&'a mut self) -> Result<Self::Transaction, Error>;
}

#[derive(Clone, Debug)]
pub enum DbType {
    Float,    // f64
    Int,      // i64
    Text,     // String
    NulFloat, // Option<f64>
    NulInt,   // Option<i64>
    NulText,  // Option<String>
}

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
            None => write!($f, "None)"),
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

db_value_convert! { f32, Float }
db_value_convert! { f64, Float }
db_value_convert! { i32, Int }
db_value_convert! { i64, Int }
db_value_convert! { u32, Int }
db_value_convert! { u64, Int }
db_value_convert! { usize, Int }
db_value_convert! { String, Text }

/*
impl TryInto<Option<NaiveDate>> for DbValue {
    type Error = crate::Error;
    fn try_into(self) -> Result<Option<NaiveDate>, Self::Error> {
        match self {
            DbValue::NulInt(opt) =>
                Ok(opt.map(|val|
                    NaiveDate::from_num_days_from_ce(val as i32)
                )),
            _ => Err(Error::InvalidInput(format!(
                "cannot convert {:?} into Option<NaiveDate>",
                self,
            ))),
    }
}
impl From<Option<NaiveDate>> for DbValue {
    fn from(opt: Option<NaiveDate>) -> Self {
        Self::NulInt(opt.map(|val| val.num_days_from_ce() as i64))
    }
}
impl TryInto<NaiveDate> for DbValue {
    type Error = crate::Error;
    fn try_into(self) -> Result<NaiveDate, Self::Error> {
        match self {
            DbValue::Int(val) =>
                Ok(NaiveDate::from_num_days_from_ce(val as i32)),
            _ => Err(Error::InvalidInput(format!(
                "cannot convert {:?} into NaiveDate",
                self,
            ))),
    }

impl From<NaiveDate> for DbValue {
    fn from(val: NaiveDate) -> Self {
        Self::Int(val.num_days_from_ce() as i64)
    }
}
*/

