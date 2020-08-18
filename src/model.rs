//! Traits implemented by model objects.
//!
use crate::Error;
use crate::{DbConn, DbValue};

/// Functions for belongs-to associations.
///
/// `Parent` is the type of the parent model.
///
#[allow(unused_variables)]
pub trait BelongsTo<Parent>: Sized {
    /// Retrive all child objects in the database belonging to a parent.
    ///
    /// `db` is the database connection object.
    ///
    /// `parent` is the parent object.
    ///
    fn belonging_to(
        db: &impl DbConn,
        parent: &Parent,
    ) -> Result<Vec<Self>, Error>;

    /// Retrive the parent object from the database.
    ///
    /// `db` is the database connection object.
    ///
    /// A return value of `None` may be
    /// - because the corresponding field in `self` is `None` (if the field is
    ///   an `Option`),
    /// - because there is no row in the parent table with a primary key
    ///   matching the field value, or
    /// - because of a database error of any kind.
    ///
    fn parent(&self, db: &impl DbConn) -> Option<Parent>;

    /// Set the parent reference.
    ///
    /// `parent` is the parent object.
    ///
    /// The new parent association is not saved to the database.
    ///
    fn set_parent(&mut self, parent: &Parent) -> Result<(), Error>;

    /// Retrive all child objects in the database (including `self`) that
    /// belong to the same parent as `self`.
    ///
    fn siblings(&mut self, db: &impl DbConn) -> Result<Vec<Self>, Error> {
        let parent;
        match self.parent(db) {
            Some(p) => parent = p,
            None => return Err(Error::database("no parent")),
        }
        Self::belonging_to(db, &parent)
    }
}

/// Functions for deleting models from the database.
///
#[allow(unused_variables)]
pub trait Delete<PkType> {
    /// Return 1 after successfully deleted the corresponding database row.
    ///
    fn delete(self, db: &impl DbConn) -> Result<usize, Error>;

    /// Return the number of successfully deleted database rows.
    ///
    /// `batch` should be a slice of primary key values.  If there are more
    /// than one primary key column, `PkType` is a tuple in the order they are
    /// declared in the struct.
    ///
    fn delete_batch(
        db: &impl DbConn,
        batch: &[PkType],
    ) -> Result<usize, Error>;
}

/// Functions for retrieving models from the database.  Some functions are
/// useful also for tables that have no primary key, e.g. views.  In that
/// case `PkType` should be `()`.
///
#[allow(unused_variables)]
pub trait Find<PkType>: Sized {
    /// Find an object in the database by primary key(s).
    ///
    /// `db` is the database connection object.
    ///
    /// `pk` is the primary key.  If there are more than one primary key
    /// column, `PkType` should be a tuple in the order they are declared in
    /// the struct.
    ///
    /// The default implementaion returns `None`.
    ///
    fn find(db: &impl DbConn, pk: &PkType) -> Option<Self> {
        None
    }

    /// Find this object in the database by primary key.
    ///
    /// `db` is the database connection object.
    ///
    /// The default implementaion returns `None`.
    ///
    fn find_equal(&self, db: &impl DbConn) -> Option<Self> {
        None
    }

    /// Return a vector with all records in the table in the default order.
    ///
    /// `db` is the database connection object.
    ///
    /// Must be implemented.
    ///
    fn load(db: &impl DbConn) -> Result<Vec<Self>, Error>;

    /// Return a vector with a possibly limited number of records that satisfy
    /// a condition possibly in a specified order.
    ///
    /// `query` is a [`Query`](struct.Query.html), see that and
    /// [`QueryBld`](struct.QueryBld.html).
    ///
    /// Must be implemented.
    ///
    fn query(db: &impl DbConn, query: &Query) -> Result<Vec<Self>, Error>;

    /// Return an error if there is no object in the database whith the given
    /// primary key(s).  See [`find()`](trait.Find.html#tymethod.find).
    ///
    /// The default implementaion uses `find()` in the obvious way.
    ///
    fn validate_exists(
        db: &impl DbConn,
        pk: &PkType,
        msg: &str,
    ) -> Result<(), Error> {
        match Self::find(db, pk) {
            Some(_) => Ok(()),
            None => Err(Error::database(msg)),
        }
    }

    /// Return an error if this object is already stored in the database.  See
    /// [`find_equal()`](trait.Find.html#tymethod.find_equal).
    ///
    /// The default implementaion uses `find_equal()` in the obvious way.
    /// Note that the default `find_equal()` will make the default
    /// `validate_unique()` return `Ok(())`.
    ///
    fn validate_unique(
        &self,
        db: &impl DbConn,
        msg: &str,
    ) -> Result<(), Error> {
        match self.find_equal(db) {
            Some(_) => Err(Error::database(msg)),
            None => Ok(()),
        }
    }
}

/// Functions for has-many associations.
///
/// `Child` is the type of the child struct.
///
#[allow(unused_variables)]
pub trait HasMany<Child>: Sized {
    /// Retrive all child objects from the database.
    ///
    /// `db` is the database connection object.
    ///
    fn children(&self, db: &impl DbConn) -> Result<Vec<Child>, Error>;

    /// Set `self` to the parent of `child`
    ///
    /// `parent` is the parent object.
    ///
    /// The new parent association is not saved to the database.
    ///
    fn add_child(&self, child: &mut Child) -> Result<(), Error>;
}

/// Functions for saving new or old objects to the database.
///
#[allow(unused_variables)]
pub trait Save: Sized {
    /// Try to INSERT a row in the database from `self` and update `self` from
    /// the inserted row after insert.
    ///
    /// The default implementation calls
    /// [`insert_batch()`](trait.Save.html#tymethod.insert_batch).
    ///
    /// It is an error if `self` has a primary key that exists in the
    /// database.
    ///
    fn insert(&mut self, db: &impl DbConn) -> Result<(), Error> {
        *self = Self::insert_batch(db, std::slice::from_ref(self))?
            .pop()
            .unwrap();
        Ok(())
    }

    /// Try to INSERT a number of rows in the database from `data` and return
    /// new model structs updated from the inserted rows after insert.
    ///
    /// The implementation by
    /// [`#[derive(vicocomo::Save)]`](derive.Save.html) ensures that any field
    /// with the attribute `vicocomo_optional` will be sent to the database
    /// only if it is `Some`.
    ///
    /// It is an error if any of the data has a primary key that exists in the
    /// database.
    ///
    fn insert_batch(
        db: &impl DbConn,
        data: &[Self],
    ) -> Result<Vec<Self>, Error>;

    /// Save the object's data to the database.
    ///
    /// If a row with the object's primary key exists in the database, this is
    /// equivalent to [`update()`](trait.Save.html#tymethod.update).  If
    /// not, this is equivalent to [`insert()`
    /// ](trait.Save.html#tymethod.insert).
    ///
    /// The default implementation simply tries first `update()`, then
    /// `insert()`.
    ///
    fn save(&mut self, db: &impl DbConn) -> Result<(), Error> {
        self.update(db).or_else(|_e| self.insert(db))
    }

    /// Try to UPDATE a row in the database from `self` and update self from
    /// the updated row after insert.
    ///
    /// It is an error if `self` lacks a primary key or has one that does not
    /// exist in the database.
    ///
    fn update(&mut self, db: &impl DbConn) -> Result<(), Error>;
}

/// Builds a [`Query`](struct.Query.html) for [`Find::query()`
/// ](trait.Find.html#tymethod.query).
///
/// Example:
///
/// ```text
/// let query =
/// QueryBld::new()              // create the query
/// .col("c1")                   // begin building the first WHERE condition
/// .gt(None)                    // the condition is ">", no value (yet)
/// .and("c2")                   // another WHERE clause condition ...
/// .eq(&Some(DbValue::Text("foo"))) // ... but this time a value is given
/// .order("c2 DESC, c1")        // order is just a string w/o "ORDER BY"
/// .limit(4711)                 // setting a limit ...
/// .offset(50)                  // ... and an offset
/// .query().unwrap()            // create the query, cannot be used ...
/// .value(1, &DbValue::Int(17); // ... w/o setting all values (1-based ix)
///                              // Reuse the query with new values:
/// query.set_values(&[DbValue::Int(42), DbValue::Text("bar")]); // No Some()!
/// query.set_limit(Some(4));    // The limit may be changed ...
/// query.set_limit(None);       // ... or removed (the offset, too)
/// ```
///
/// Function sequences that do not make sense, e.g. `new().and()` or
/// `and().`*any function except a relational operator* will make [`query()`
/// ](struct.QueryBld.html#method.query) return None.
///
/// For more complicated WHERE clauses, use the catch-all [`filter()`
/// ](struct.QueryBld.html#method.filter).
///
#[derive(Clone, Debug)]
pub struct QueryBld(Query, QbState);

#[derive(Clone, Debug)]
enum QbState {
    Valid,
    GotCol,
    Invalid,
}

macro_rules! where_rel_op {
    ($( #[$meta:meta] )* $op_fn:ident, $op_str:literal) => {
        $( #[$meta] )*
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
    ($( #[$meta:meta] )* $op_fn:ident, $op_str:literal) => {
        $( #[$meta] )*
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

impl QueryBld {
    // public methods w/o receiver - - - - - - - - - - - - - - - - - - - - - -

    /// Create a query builder.
    pub fn new() -> Self {
        Self(
            Query {
                filter: None,
                limit: None,
                offset: None,
                order: Order::Dflt,
                values: Vec::new(),
            },
            QbState::Valid,
        )
    }

    // public methods with receiver  - - - - - - - - - - - - - - - - - - - - -

    where_log_op! {
        /// Initiate building another WHERE condition AND-ed to the previous.
        ///
        /// `db_name` is the column name in the database.
        ///
        and, "AND"
    }

    /// Initiate building the first WHERE condition.
    ///
    /// `db_name` is the column name in the database.
    ///
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

    where_rel_op! {
    /// Complete building a WHERE condition.
    ///
    /// `value` is the value to use or `None` for a reusable [`Query`
    /// ](struct.Query.html).
    ///
        eq, "="
    }

    /// Build a complete WHERE condition
    ///
    /// `fltr` is the meat of the WHERE clause - no `WHERE`! - or `None` if no
    /// WHERE clause.  It may be parameterized using the notation `$`n for the
    /// n:th parameter, 1 based.
    ///
    /// `values` are the parameter values.
    ///
    pub fn filter(mut self, fltr: Option<&str>, values: &[DbValue]) -> Self {
        match self.1 {
            QbState::Valid if self.0.filter.is_none() => {
                self.0.filter = fltr.map(|s| s.to_string());
                self.0.values.extend(values.iter().map(|v| Some(v.clone())));
                self
            }
            _ => self.invalidate(),
        }
    }

    where_rel_op! {
        /// Complete building a WHERE condition.
        ///
        /// `value` is the value to use or `None` for a reusable [`Query`
        /// ](struct.Query.html).
        ///
        ge, ">="
    }

    where_rel_op! {
        /// Complete building a WHERE condition.
        ///
        /// `value` is the value to use or `None` for a reusable [`Query`
        /// ](struct.Query.html).
        ///
        gt, ">"
    }

    where_rel_op! {
        /// Complete building a WHERE condition.
        ///
        /// `value` is the value to use or `None` for a reusable [`Query`
        /// ](struct.Query.html).
        ///
        le, "<="
    }

    /// Set a limit on the number of returned objects.
    ///
    /// `limit` is the limit to use.
    ///
    pub fn limit(mut self, limit: usize) -> Self {
        self.0.limit = Some(limit);
        self
    }

    where_rel_op! {
        /// Complete building a WHERE condition.
        ///
        /// `value` is the value to use or `None` for a reusable [`Query`
        /// ](struct.Query.html).
        ///
        lt, "<"
    }

    where_rel_op! {
        /// Complete building a WHERE condition.
        ///
        /// `value` is the value to use or `None` for a reusable [`Query`
        /// ](struct.Query.html).
        ///
        ne, "<>"
    }

    /// Set the number of objects to skip.
    ///
    /// `offset` is the offset to use.
    ///
    pub fn offset(mut self, offset: usize) -> Self {
        self.0.offset = Some(offset);
        self
    }

    /// Remove the ORDER clause, e.g. to avoid default ordering.
    ///
    pub fn no_order(mut self) -> Self {
        self.0.order = Order::NoOrder;
        self
    }

    where_log_op! {
        /// Initiate building another WHERE condition OR-ed to the previous.
        ///
        /// `db_name` is the column name in the database.
        ///
        or, "OR"
    }

    /// Define an ORDER clause.
    ///
    /// `order` is the meat of the ORDER clause - no `ORDER BY`!
    ///
    pub fn order(mut self, order: &str) -> Self {
        self.0.order = Order::Custom(order.to_string());
        self
    }

    /// Freeze the query by returning the built
    /// [`Query`](struct.Query.html) struct.
    ///
    /// `None` is returned if there were problems building the query.
    ///
    pub fn query(self) -> Option<Query> {
        match self.1 {
            QbState::Valid => Some(self.0),
            _ => {
                self.invalidate();
                None
            }
        }
    }

    // private - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -

    fn invalidate(mut self) -> Self {
        self.1 = QbState::Invalid;
        self
    }
}

/// A reusable query for
/// [`Find::query()`](trait.Find.html#tymethod.query), see [`QueryBld`
/// ](struct.QueryBld.html) for how to build.
///
/// The fields are public because you need them to implement `Find::query()`.
///
#[derive(Clone, Debug)]
pub struct Query {
    pub filter: Option<String>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
    pub order: Order,
    pub values: Vec<Option<DbValue>>,
}

impl Query {
    /// Set the limit to use.
    ///
    /// `limit` is the new limit or `None` for no limit.
    ///
    pub fn set_limit<'a>(&'a mut self, limit: Option<usize>) -> &'a mut Self {
        self.limit = limit;
        self
    }

    /// Set the offset to use.
    ///
    /// `offset` is the new offset or `None` for no offset.
    ///
    pub fn set_offset<'a>(&'a mut self, offs: Option<usize>) -> &'a mut Self {
        self.offset = offs;
        self
    }

    /// Set a value to use.
    ///
    /// `ix` is the 1 based index.
    ///
    /// `value` is the value.
    ///
    pub fn set_value<'a>(
        &'a mut self,
        ix: usize,
        value: &DbValue,
    ) -> &'a mut Self {
        self.values[ix - 1] = Some(value.clone());
        self
    }

    /// Set all values to use.
    ///
    /// `values` is a slice with the values.
    ///
    pub fn set_values<'a>(&'a mut self, values: &[DbValue]) -> &'a mut Self {
        self.values = values.iter().map(|v| Some(v.clone())).collect();
        self
    }
}

/// Represents the ordering of the objects returned by [`Find::query()`
/// ](trait.Find.html#tymethod.query).
///
/// The variants are public because you need them to implement
/// `Find::query()`.
///
#[derive(Clone, Debug)]
pub enum Order {
    /// The meat of the ORDER clause - no `ORDER BY`!
    ///
    Custom(String),

    /// Use the models default order as defined by the [`vicocomo_order_by`
    /// ](../vicocomo_model_derive/index.html) attribute on one or more model
    /// struct fields.
    Dflt,

    /// No `ORDER BY` sent to the database.
    ///
    NoOrder,
}
