use crate::Error;
use crate::{DbConn, DbValue};

#[allow(unused_variables)]
pub trait MdlDelete<'a, PkType> {
    // Return 1 after successfully deleted the corresponding database row.
    //
    fn delete(self, db: &mut impl DbConn<'a>) -> Result<usize, Error>;

    // Return the number of successfully deleted database rows.
    //
    // batch should be a slice of primary key values (or tuples of them if
    // there is more than one primary key field).
    //
    fn delete_batch(
        db: &mut impl DbConn<'a>,
        batch: &[PkType],
    ) -> Result<usize, Error>;
}

#[allow(unused_variables)]
pub trait MdlFind<'a>: Sized {
    // Return a vector with all records in the table in the default order.
    //
    fn load(db: &mut impl DbConn<'a>) -> Result<Vec<Self>, Error>;

    // Return a vector with a possibly limited number of records that satisfy
    // a condition possibly in a specified order.
    //
    // q is a MdlQuery, see that and MdlQueryBld.
    //
    fn query(
        db: &mut impl DbConn<'a>,
        q: &MdlQuery,
    ) -> Result<Vec<Self>, Error>;
}

#[allow(unused_variables)]
pub trait MdlSave<'a>: Sized {
    // Try to INSERT a row in the database from self and update self from the
    // inserted row after insert.
    //
    // The default implementation calls insert_batch().
    //
    // It is an error if self has a primary key that exists in the database.
    //
    fn insert(&mut self, db: &mut impl DbConn<'a>) -> Result<(), Error> {
        *self = Self::insert_batch(db, std::slice::from_ref(self))?
            .pop()
            .unwrap();
        Ok(())
    }

    // Try to INSERT a number of rows in the database from data and return new
    // model structs updated from the inserted rows after insert.
    //
    // The implementation by #[derive(vicocomo::SaveModel)] ensures that any
    // field with the attribute vicocomo_optional will be sent to the database
    // only if it is Some(value).
    //
    // It is an error if any of the data has a primary key that exists in the
    // database.
    //
    fn insert_batch(
        db: &mut impl DbConn<'a>,
        data: &[Self],
    ) -> Result<Vec<Self>, Error>;

    // Save the object's data to the database.
    //
    // If a row with the object's primary key exists in the database, this is
    // equivalent to update().  If not, this is equivalent to insert().
    //
    // The default implementation simply tries first update(), then insert().
    //
    fn save(&mut self, db: &mut impl DbConn<'a>) -> Result<(), Error> {
        self.update(db).or_else(|_e| self.insert(db))
    }

    // Try to UPDATE a row in the database from self and update self from the
    // updated row after insert.
    //
    // It is an error if self lacks a primary key or has one that does not
    // exist in the database.
    //
    fn update(&mut self, db: &mut impl DbConn<'a>) -> Result<(), Error>;
}

// Builds a MdlQuery for MdlFind::query().
//
// Example:
//
// let query =
// MdlQueryBld.new()            // create the query
// .col("c1")                   // begin building the first WHERE condition
// .gt(None)                    // the condition is ">", no value (yet)
// .and("c2")                   // another WHERE clause condition ...
// .eq(&Some(DbValue::Text("foo"))) // ... but this time a value is given
// .order("c2 DESC, c1")        // order is just a string w/o "ORDER BY"
// .limit(4711)                 // setting a limit ...
// .offset(50)                  // ... and an offset
// .query().unwrap()            // create the query, cannot be used ...
// .value(1, &DbValue::Int(17)  // ... w/o setting all values (1-based ix)
//                              // Reuse the query with new values:
// query.set_values(&[DbValue::Int(42), DbValue::Text("bar")]);  // No Some()!
// query.set_limit(Some(4));    // The limit may be changed ...
// query.set_limit(None);       // ... or removed (the offset, too)
//
// Function sequences that do not make sense, e.g. new().and() or
// and().<any function except a relational operator> will make
// MdlQueryBld::query() return None.
//
// For more complicated WHERE clauses, use the catch-all filter().
//
#[derive(Clone, Debug)]
pub struct MdlQueryBld(MdlQuery, QbState);

#[derive(Clone, Debug)]
enum QbState {
    Valid,
    GotCol,
    Invalid,
}

macro_rules! where_rel_op {
    ($op_fn:ident, $op_str:literal) => {
        pub fn $op_fn(mut self, value: Option<&DbValue>) -> Self {
            match self.1 {
                QbState::GotCol => {
                    self.0.filter.as_mut().unwrap().push_str(&format!(
                        concat!(" ", $op_str, " ${}"),
                        self.0.values.len() + 1,
                    ));
                    self.0.values.push(value.map(|v| v.clone()));
                    self.1 = QbState::Valid;
                    self
                },
                _ => self.invalidate(),
            }
        }
    };
}

macro_rules! where_log_op {
    ($op_fn:ident, $op_str:literal) => {
        #[allow(unused_mut)]
        pub fn $op_fn(mut self, db_name: &str) -> Self {
            match self.1 {
                QbState::Valid if self.0.filter.is_some() => {
                    self.0.filter.as_mut().unwrap().push_str(
                        concat!(" ", $op_str, " ")
                    );
                    self.0.filter.as_mut().unwrap().push_str(db_name);
                    self.1 = QbState::GotCol;
                    self
                },
                _ => self.invalidate(),
            }
        }
    };
}

impl MdlQueryBld {
    // public methods w/o receiver - - - - - - - - - - - - - - - - - - - - - -

    // Create a query builder.
    pub fn new() -> Self {
        Self(
            MdlQuery {
                filter: None,
                limit: None,
                offset: None,
                order: MdlOrder::Dflt,
                values: Vec::new(),
            },
            QbState::Valid,
        )
    }

    // public methods with receiver  - - - - - - - - - - - - - - - - - - - - -

    // Initiate building another WHERE condition AND-ed to the previous.
    //
    // fn and(&mut self, db_name: &str) -> Self
    //
    // db_name is the column name in the database.
    //
    where_log_op! {and, "AND"}

    // Initiate building the first WHERE condition.
    //
    // db_name is the column name in the database.
    //
    pub fn col(mut self, db_name: &str) -> Self {
        match self.1 {
            QbState::Valid if self.0.filter.is_none() => {
                self.0.filter = Some(db_name.to_string());
                self.1 = QbState::GotCol;
                self
            }
            _ => self.invalidate(),
        }
    }

    // Complete building a WHERE condition.
    //
    // fn eq(&mut self, value: Option<&DbValue>) -> Self
    //
    // value is the value to use or None for a reusable MdlQuery.
    //
    where_rel_op! {eq, "="}

    // Build a complete WHERE condition
    //
    // filter is the meat of the WHERE clause - no "WHERE"! - or None if no
    // WHERE clause.  It may be parameterized using the notation $<n> for the
    // n:th parameter, 1 based.
    //
    // values are the parameter values.
    //
    pub fn filter(mut self, fltr: Option<&str>, values: &[DbValue]) -> Self {
        match self.1 {
            QbState::Valid if self.0.filter.is_none() => {
                if fltr.is_some() {
                    self.0.filter = Some(format!("WHERE {}", fltr.unwrap()));
                }
                self.0.values.extend(values.iter().map(|v| Some(v.clone())));
                self
            }
            _ => self.invalidate(),
        }
    }

    // Complete building a WHERE condition.
    //
    // fn ge(&mut self, value: Option<&DbValue>) -> Self
    //
    // value is the value to use or None for a reusable MdlQuery.
    //
    where_rel_op! {ge, ">="}

    // Complete building a WHERE condition.
    //
    // fn gt(&mut self, value: Option<&DbValue>) -> Self
    //
    // value is the value to use or None for a reusable MdlQuery.
    //
    where_rel_op! {gt, ">"}

    // Complete building a WHERE condition.
    //
    // fn le(&mut self, value: Option<&DbValue>) -> Self
    //
    // value is the value to use or None for a reusable MdlQuery.
    //
    where_rel_op! {le, "<="}

    // Set a limit on the number of returned objects.
    //
    // limit is the limit to use.
    //
    pub fn limit(mut self, limit: usize) -> Self {
        self.0.limit = Some(limit);
        self
    }

    // Complete building a WHERE condition.
    //
    // fn lt(&mut self, value: Option<&DbValue>) -> Self
    //
    // value is the value to use or None for a reusable MdlQuery.
    //
    where_rel_op! {lt, "<"}

    // Complete building a WHERE condition.
    //
    // fn ne(&mut self, value: Option<&DbValue>) -> Self
    //
    // value is the value to use or None for a reusable MdlQuery.
    //
    where_rel_op! {ne, "<>"}

    // Set the number of objects to skip.
    //
    // offset is the offset to use.
    //
    pub fn offset(mut self, offset: usize) -> Self {
        self.0.offset = Some(offset);
        self
    }

    // Remove the ORDER clause, e.g. to avoid default ordering.
    //
    pub fn no_order(mut self) -> Self {
        self.0.order = MdlOrder::NoOrder;
        self
    }

    // Initiate building another WHERE condition OR-ed to the previous.
    //
    // fn or(&mut self, db_name: &str) -> Self
    //
    // db_name is the column name in the database.
    //
    where_log_op! {or, "OR"}

    // Define an ORDER clause.
    //
    // order is the meat of the ORDER clause - no "ORDER BY"!
    //
    pub fn order(mut self, order: &str) -> Self {
        self.0.order = MdlOrder::Custom(order.to_string());
        self
    }

    // Freeze the query by returning the built MdlQuery struct.
    //
    // None is returned if there were problems building the query.
    //
    pub fn query(self) -> Option<MdlQuery> {
        match self.1 {
            QbState::Valid => Some(self.0),
            _ => None,
        }
    }

    // private - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -

    fn invalidate(mut self) -> Self {
        self.1 = QbState::Invalid;
        self
    }
}

// A reusable query for MdlFind::query(), see MdlQueryBld for how to build.
//
// The fields are public because you need them to implement MdlFind::query().
//
#[derive(Clone, Debug)]
pub struct MdlQuery {
    pub filter: Option<String>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
    pub order: MdlOrder,
    pub values: Vec<Option<DbValue>>,
}

impl MdlQuery {
    // Set the limit to use.
    //
    // limit is the new limit or None for no limit.
    //
    pub fn set_limit<'a>(&'a mut self, limit: Option<usize>) -> &'a mut Self {
        self.limit = limit;
        self
    }

    // Set the offset to use.
    //
    // offset is the new offset or None for no offset.
    //
    pub fn set_offset<'a>(&'a mut self, offs: Option<usize>) -> &'a mut Self {
        self.offset = offs;
        self
    }

    // Set a value to use.
    //
    // ix is the 1 based index.
    //
    // value is the value.
    //
    pub fn set_value<'a>(
        &'a mut self,
        ix: usize,
        value: &DbValue,
    ) -> &'a mut Self {
        self.values[ix - 1] = Some(value.clone());
        self
    }

    // Set all values to use.
    //
    // values is a slice with the values.
    //
    pub fn set_values<'a>(&'a mut self, values: &[DbValue]) -> &'a mut Self {
        self.values = values.iter().map(|v| Some(v.clone())).collect();
        self
    }
}

#[derive(Clone, Debug)]
pub enum MdlOrder {
    Custom(String),
    Dflt,
    NoOrder,
}
