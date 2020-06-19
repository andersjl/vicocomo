use crate::Error;
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
macro_rules! convert_value {
    // Option<i32>, "i32", NulInt, i64
    (Option<$oth_typ:ty>, $oth_nam:literal, $variant:ident, $as: ty) => {
        impl TryInto<Option<$oth_typ>> for DbValue {
            type Error = crate::Error;
            fn try_into(self) -> Result<Option<$oth_typ>, Self::Error> {
                match self {
                    DbValue::$variant(opt) => Ok(match opt {
                        Some(val) => Some(val as $oth_typ),
                        None => None,
                    }),
                    _ => Err(Error::InvalidInput(format!(
                        "cannot convert {:?} into Option<{}>",
                        self, $oth_nam
                    ))),
                }
            }
        }

        impl From<Option<$oth_typ>> for DbValue {
            fn from(opt: Option<$oth_typ>) -> Self {
                Self::$variant(match opt {
                    Some(val) => Some(val as $as),
                    None => None,
                })
            }
        }
    };
    // i32, "i32", Int, i64
    ($oth_typ:ty, $oth_nam:literal, $variant:ident, $as: ty) => {
        impl TryInto<$oth_typ> for DbValue {
            type Error = crate::Error;
            fn try_into(self) -> Result<$oth_typ, Self::Error> {
                match self {
                    DbValue::$variant(val) => Ok(val as $oth_typ),
                    _ => Err(Error::InvalidInput(format!(
                        "cannot convert {:?} into {}",
                        self, $oth_nam
                    ))),
                }
            }
        }

        impl From<$oth_typ> for DbValue {
            fn from(val: $oth_typ) -> Self {
                Self::$variant(val as $as)
            }
        }
    };
    // i64, "i64", Int
    // Option<i64>, "Option<i64>", NulInt
    ($oth_typ:ty, $oth_nam:literal, $variant:ident) => {
        impl TryInto<$oth_typ> for DbValue {
            type Error = crate::Error;
            fn try_into(self) -> Result<$oth_typ, Self::Error> {
                match self {
                    DbValue::$variant(val) => Ok(val),
                    _ => Err(Error::InvalidInput(format!(
                        "cannot convert {:?} into {}",
                        self, $oth_nam
                    ))),
                }
            }
        }

        impl From<$oth_typ> for DbValue {
            fn from(val: $oth_typ) -> Self {
                Self::$variant(val)
            }
        }
    };
}
convert_value! { f64, "f64", Float }
convert_value! { f32, "f32", Float, f64 }
convert_value! { i64, "i64", Int }
convert_value! { u64, "u64", Int, i64 }
convert_value! { i32, "i32", Int, i64 }
convert_value! { u32, "u32", Int, i64 }
convert_value! { String, "String", Text }
convert_value! { Option<f64>, "Option<f64>", NulFloat }
convert_value! { Option<f32>, "f32", NulFloat, f64 }
convert_value! { Option<i64>, "Option<i64>", NulInt }
convert_value! { Option<u64>, "u64", NulInt, i64 }
convert_value! { Option<i32>, "i32", NulInt, i64 }
convert_value! { Option<u32>, "u32", NulInt, i64 }
convert_value! { Option<String>, "Option<String>", NulText }

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

