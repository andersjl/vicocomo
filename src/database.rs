use crate::Error;
use std::{convert::TryInto, fmt};

pub trait DbConn<'a> {
    fn exec(&mut self, sql: &str, params: &[Value]) -> Result<u64, Error>;

    fn query(
        &mut self,
        sql: &str,
        params: &[Value],
        types: &[Type],
    ) -> Result<Vec<Vec<Value>>, Error>;

    fn transaction(
        &'a mut self,
        statements: fn(Box<dyn DbTrans + 'a>) -> Result<(), Error>,
    ) -> Result<(), Error>;
}

#[allow(unused_variables)]
pub trait DbTrans<'a> {
    fn commit(self: Box<Self>) -> Result<(), Error>;

    fn exec(&mut self, sql: &str, params: &[Value]) -> Result<u64, Error>;

    fn query(
        &mut self,
        sql: &str,
        params: &[Value],
        types: &[Type],
    ) -> Result<Vec<Vec<Value>>, Error>;

    fn rollback(self: Box<Self>) -> Result<(), Error>;

    fn transaction(
        &'a mut self,
        statements: fn(Box<dyn DbTrans + 'a>) -> Result<(), Error>,
    ) -> Result<(), Error> {
        Err(Error::Database(
            "nested transactions not supported by implementation".to_string(),
        ))
    }
}

#[derive(Debug)]
pub enum Type {
    Float,    // f64
    Int,      // i64
    Text,     // String
    NulFloat, // Option<f64>
    NulInt,   // Option<i64>
    NulText,  // Option<String>
}

impl From<Value> for Type {
    fn from(v: Value) -> Self {
        match v {
            Value::Float(_) => Self::Float,
            Value::Int(_) => Self::Int,
            Value::Text(_) => Self::Text,
            Value::NulFloat(_) => Self::NulFloat,
            Value::NulInt(_) => Self::NulInt,
            Value::NulText(_) => Self::NulText,
        }
    }
}

#[derive(Clone, Debug)]
pub enum Value {
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

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self {
            Value::Float(v) => write!(f, "{}", v),
            Value::Int(v) => write!(f, "{}", v),
            Value::Text(v) => write!(f, "{}", v),
            Value::NulFloat(v) => write_opt!(f, v),
            Value::NulInt(v) => write_opt!(f, v),
            Value::NulText(v) => write_opt!(f, v),
        }
    }
}
macro_rules! convert_value {
    // Option<i32>, "i32", NulInt, i64
    (Option<$oth_typ:ty>, $oth_nam:literal, $variant:ident, $as: ty) => {
        impl TryInto<Option<$oth_typ>> for Value {
            type Error = crate::Error;
            fn try_into(self) -> Result<Option<$oth_typ>, Self::Error> {
                match self {
                    Value::$variant(opt) => Ok(match opt {
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

        impl From<Option<$oth_typ>> for Value {
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
        impl TryInto<$oth_typ> for Value {
            type Error = crate::Error;
            fn try_into(self) -> Result<$oth_typ, Self::Error> {
                match self {
                    Value::$variant(val) => Ok(val as $oth_typ),
                    _ => Err(Error::InvalidInput(format!(
                        "cannot convert {:?} into {}",
                        self, $oth_nam
                    ))),
                }
            }
        }

        impl From<$oth_typ> for Value {
            fn from(val: $oth_typ) -> Self {
                Self::$variant(val as $as)
            }
        }
    };
    // i64, "i64", Int
    // Option<i64>, "Option<i64>", NulInt
    ($oth_typ:ty, $oth_nam:literal, $variant:ident) => {
        impl TryInto<$oth_typ> for Value {
            type Error = crate::Error;
            fn try_into(self) -> Result<$oth_typ, Self::Error> {
                match self {
                    Value::$variant(val) => Ok(val),
                    _ => Err(Error::InvalidInput(format!(
                        "cannot convert {:?} into {}",
                        self, $oth_nam
                    ))),
                }
            }
        }

        impl From<$oth_typ> for Value {
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
