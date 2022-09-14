//! Traits implemented by model objects persisted in an SQL database, and some
//! helper types for querying the objects in the database.
//!
//! The trait [`ActiveRecord`](trait.ActiveRecord.html) has a [derive macro
//! ](../../vicocomo_active_record/index.html) with the same name.

use crate::Error;
use crate::{
    error::{ModelError, ModelErrorKind},
    DatabaseIf, DbValue,
};

/// Help to implement the well-known Acitve Record pattern (see Martin Fowler:
/// [Patterns of Enterprise Application Architecture
/// ](https://martinfowler.com/eaaCatalog/activeRecord.html).
///
/// There is a [derive macro](../../vicocomo_active_record/index.html).
/// The trait should be used as an interface to a *thin* wrapping of the
/// database! Avoid coding elaborate business rules etc in methods of the
/// deriving `struct`. Use [contexts
/// ](https://en.wikipedia.org/wiki/Data,_context_and_interaction) for that.
/// *Especially* if the logic involves more than one `ActiveRecord` class, or
/// classes with no direct relation to the database.
///
/// Note that the derive macro implements a number of functions that are *not*
/// in the trait! These are mainly functions for either side of one-to-many or
/// many-to-many associations between this type and another (or the same)
/// type.
//
// It proved tedious and with no obvious gain execpt possibly machine code
// size to avoid this function name generation using generics.
///
/// For those non-trait functions, see the [derive macro `ActiveRecord`
/// ](../derive.ActiveRecord.html).
///
pub trait ActiveRecord: Sized {
    /// <b>No primary key</b>: `()`. Some functions are useful also for
    /// relations that have no primary key, e.g. views.
    ///
    /// <b>Exactly one primary key</b>: The type of the primary key field or
    /// the held type if it is an `Option` (see the [derive macro attribute
    /// `vicocomo_optional`](../derive.ActiveRecord.html#vicocomo_optional)).
    ///
    /// <b>More than one primary key</b>: A tuple of the types of the primary
    /// keys, again with `Option` stripped.
    ///
    type PkType;

    //- Common functions ---------------------------------------------------//

    /// A clone of the value of the primary key(s) (see [`PkType`
    /// ](#associatedtype.PkType)) or `None` if any of them is `None` or
    /// `PkType` is `()`.
    // TODO: should it return Some(()) if PkType is ()?
    ///
    fn pk_value(&self) -> Option<Self::PkType>;

    //- Functions for deleting models from the database --------------------//

    /// Return `Ok(())` iff the corresponding database row is deleted.
    ///
    /// The implementation should ensure referential integrity, e.g.  relying
    /// on the database's referential integrity.
    ///
    /// <b>Errors</b>
    ///
    /// Return [`Err(Error::Model)` ](../error/enum.Error.html#variant:Model)
    /// if
    /// - self has no primary key, or
    /// - the model implements [`BeforeDelete`](trait.BeforeDelete.html) and
    ///   `before_delete()` returns an error.
    ///
    /// For referential integrity and other error handling, see
    /// [`delete_batch()`](#tymethod.delete_batch).
    ///
    fn delete(self, db: DatabaseIf) -> Result<(), Error>;

    /// Return `Ok(batch.len())` iff each key in `batch` identifies a database
    /// row that is deleted.
    ///
    /// `batch` should be a slice of primary key values, see [`PkType`
    /// ](#associatedtype.PkType).
    ///
    /// The implementation should not call [`before_delete()`
    /// ](trait.BeforeDelete.html#tymethod.before_delete). You have to use
    /// [`delete()`](#tymethod.delete) for that.
    ///
    /// <b>Errors</b>
    ///
    /// Return [`Err(Error::Model)`](../error/enum.Error.html#variant:Model)
    /// for the first of the objects referred to in `batch` that
    /// - does not exist in the database, or
    /// - cannot be deleted because of a database foreign key constraint, or
    /// - cannot be deleted for application specific reasons.
    ///
    /// Return `Err(Error::Other("not-available")' if `PkType` is `()`.
    ///
    /// Forward other database errors as [`Error::Database`
    /// ](../error/enum.Error.html#variant:Database).
    ///
    fn delete_batch(
        db: DatabaseIf,
        batch: &[Self::PkType],
    ) -> Result<usize, Error>;

    //- Functions for retrieving models from the database ------------------//

    /// Find an object in the database by primary key(s).
    ///
    /// `db` is the database connection object.
    ///
    /// `pk` is the primary key value(s), see [`PkType`
    /// ](#associatedtype.PkType).
    ///
    /// Return `None` if `PkType` is `()`.
    ///
    fn find(db: DatabaseIf, pk: &Self::PkType) -> Option<Self>;

    /// Find this object in the database by primary key.
    ///
    /// `db` is the database connection object.
    ///
    /// Return `None` if `PkType` is `()`.
    ///
    fn find_equal(&self, db: DatabaseIf) -> Option<Self>;

    /// Return a vector with all records in the table in the default order.
    ///
    /// `db` is the database connection object.
    ///
    /// <b>Errors</b>
    ///
    /// Forwards database errors as [`Error::Database`
    /// ](../error/enum.Error.html#variant:Database).
    ///
    fn load(db: DatabaseIf) -> Result<Vec<Self>, Error>;

    /// Return a vector with a possibly limited number of records that satisfy
    /// a condition possibly in a specified order.
    ///
    /// `query` is a [`Query`](struct.Query.html), see that and
    /// [`QueryBld`](struct.QueryBld.html).
    ///
    /// <b>Errors</b>
    ///
    /// Forwards database errors as [`Error::Database`
    /// ](../error/enum.Error.html#variant:Database).
    ///
    fn query(db: DatabaseIf, query: &Query) -> Result<Vec<Self>, Error>;

    /// Return an [`Error::Model`](../error/enum.Error.html#variant.Model) if
    /// there is no object in the database whith the given primary key(s). See
    /// [`find()`](#tymethod.find).
    ///
    /// The default implementaion uses `find()` in the obvious way. If
    /// `PkType` is `()` this means that it always returns `Err(_)`.
    ///
    fn validate_exists(
        db: DatabaseIf,
        pk: &Self::PkType,
        msg: &str,
    ) -> Result<(), Error> {
        match Self::find(db, pk) {
            Some(_) => Ok(()),
            None => Err(Error::Model(ModelError {
                error: ModelErrorKind::NotFound,
                model: "Self".to_string(),
                general: Some(msg.to_string()),
                field_errors: Vec::new(),
                assoc_errors: Vec::new(),
            })),
        }
    }

    /// Return an [`Error::Model`](../error/enum.Error.html#variant.Model) if
    /// this object is already stored in the database. See [`find_equal()`
    /// ](#tymethod.find_equal).
    ///
    /// The default implementaion uses `find_equal()` in the obvious way. If
    /// `PkType` is `()` this means that it always returns `Ok(())`.
    ///
    fn validate_unique(
        &self,
        db: DatabaseIf,
        msg: &str,
    ) -> Result<(), Error> {
        match self.find_equal(db) {
            Some(_) => Err(Error::Model(ModelError {
                error: ModelErrorKind::NotUnique,
                model: "Self".to_string(),
                general: Some(msg.to_string()),
                field_errors: Vec::new(),
                assoc_errors: Vec::new(),
            })),
            None => Ok(()),
        }
    }

    //- Functions for saving new or old objects to the database ------------//

    /// Try to INSERT a row in the database from `self` and update `self` from
    /// the inserted row after insert.
    ///
    /// The default implementation calls [`insert_batch()`
    /// ](#tymethod.insert_batch).
    ///
    /// <b>Errors</b>
    ///
    /// See [`insert_batch()`](#tymethod.insert_batch).
    ///
    fn insert(&mut self, db: DatabaseIf) -> Result<(), Error> {
        *self = Self::insert_batch(db, std::slice::from_mut(self))?
            .pop()
            .unwrap();
        Ok(())
    }

    /// Try to INSERT a number of rows in the database from `data` and return
    /// new model structs updated from the inserted rows after insert.
    ///
    /// Note that the implementation by the derive macro ensures that any
    /// field with the attribute [`vicocomo_optional`
    /// ](../derive.ActiveRecord.html#vicocomo_optional) will be sent to the
    /// database only if it is `Some`.
    ///
    /// Ensure that either none (on `Err(_)` return) or all of the models in
    /// `data` are inserted.
    ///
    /// <b>Errors</b>
    ///
    /// Return [`Err(Error::Model)`](../error/enum.Error.html#variant:Model)
    /// for the first of the objects in `data` that
    /// - has a given primary key that is invalid or already present in the
    ///   database, or
    /// - has an invalid remote reference (e.g. relying on the database's
    ///   referential integrity), or
    /// - uses [`before_save()`](trait.BeforeSave.html#tymethod.before_save)
    ///   which returns an error, or
    /// - for application specific reasons.
    ///
    /// Forward other database errors as [`Error::Database`
    /// ](../error/enum.Error.html#variant:Database).
    ///
    fn insert_batch(
        db: DatabaseIf,
        data: &mut [Self],
    ) -> Result<Vec<Self>, Error>;

    /// Save the object's data to the database.
    ///
    /// If a row with an object's primary key exists in the database, this
    /// should be equivalent to [`update()`](#tymethod.update).
    /// If not, this should be equivalent to [`insert()`](#method.insert).
    ///
    /// The default implementation uses the [`pk_value()`](#tymethod.pk_value)
    /// and [`find()`](#tymethod.find) methods to decide whether to `update()`
    /// or `insert()`.
    ///
    /// <b>Errors</b>
    ///
    /// See [`update()`](#tymethod.update) and [`insert()`](#method.insert).
    ///
    fn save(&mut self, db: DatabaseIf) -> Result<(), Error> {
        match self.pk_value() {
            Some(pk) if Self::find(db, &pk).is_some() => self.update(db),
            _ => self.insert(db),
        }
    }

    /// Try to UPDATE a row in the database from `self` and update self from
    /// the updated row after update.
    ///
    /// Note that the implementation by the derive macro ensures that any
    /// field with the attribute [`vicocomo_optional`
    /// ](../derive.ActiveRecord.html#vicocomo_optional) will be sent to the
    /// database only if it is `Some`.
    ///
    /// The implementation should ensure referential integrity, see
    /// [`insert_batch()`](#tymethod.insert_batch).
    ///
    /// <b>Errors</b>
    ///
    /// Return [`Err(Error::Model)`](../error/enum.Error.html#variant:Model)
    /// if `self`
    /// - does not have a primary key that exists in the database, or
    /// - has an invalid remote reference (e.g. relying on the database's
    ///   referential integrity), or
    /// - uses [`before_save()`](trait.BeforeSave.html#tymethod.before_save)
    ///   which returns an error, or
    /// - for application specific reasons.
    ///
    /// Return `Err(Error::Other("not-available")' if `PkType` is `()`.
    ///
    /// Return [`Err(Error::Database)`
    /// ](../error/enum.Error.html#variant:Database) if the database update
    /// fails for some other reason.
    ///
    fn update(&mut self, db: DatabaseIf) -> Result<(), Error>;

    /// Try to UPDATE the row in the database corresponding to `self`.  Each
    /// pair in `cols` is the name of a database column and the new value.
    ///
    /// On successful return `self` is updated from the database.  On error,
    /// `self` is unchanged.
    ///
    /// <b>Note</b> that this function updates directly to the database and
    /// should
    /// - not call [`before_save()`
    ///   ](trait.BeforeSave.html#tymethod.before_save),
    /// - ignore the visibility of the fields in `self` corresponding to the
    ///   `cols`,
    /// - send data to the database ignoring `vicocomo_optional` attributes,
    ///   and
    /// - forward database errors without conversion.
    ///
    /// <b>Errors</b>
    ///
    /// Return [`Err(Error::Model)`](../error/enum.Error.html#variant:Model)
    /// if `self` does not have a primary key.
    ///
    /// Return `Err(Error::Other("not-available")' if `PkType` is `()`.
    ///
    /// Return [`Err(Error::Database)`
    /// ](../error/enum.Error.html#variant:Database) if the database update
    /// fails for some other reason.
    ///
    fn update_columns(
        &mut self,
        db: DatabaseIf,
        cols: &[(&str, DbValue)],
    ) -> Result<(), Error>;
}

/// A hook that may be called by the [`delete()` function
/// ](trait.ActiveRecord.html#tymethod.delete), e.g. if implemented by the
/// [`ActiveRecord` derive macro](../derive.ActiveRecord.html).
///
pub trait BeforeDelete {
    /// Do whatever necessary before deleting `self` from the database.
    ///
    /// An `Err` return value means that `self` cannot be deleted, and
    /// [`delete()`](trait.ActiveRecord.html#tymethod.delete) should return
    /// `Err` as well. `before_delete()` should *not* handle referential
    /// integrity for one-to-many associations. Those should be handled by
    /// [`delete()`](trait.ActiveRecord.html#tymethod.delete) directly.
    ///
    /// <b>Errors</b>
    ///
    /// On error return, the variant should be a [`Model`
    /// ](../error/enum.Error.html#variant:Model) indicating the offending
    /// field(s) or associations(s) and what the problem is.
    ///
    fn before_delete(&mut self, db: DatabaseIf) -> Result<(), Error>;
}

/// A hook called by the [`insert()`](trait.ActiveRecord.html#method.insert)
/// and [`update()`](trait.ActiveRecord.html#tymethod.update) functions as
/// implemented by the [`ActiveRecord` derive macro
/// ](../derive.ActiveRecord.html).
///
pub trait BeforeSave {
    /// Do whatever necessary before saving `self` to the database.
    ///
    /// An `Err` return value means that `self` cannot be saved, and
    /// [`insert()`](trait.ActiveRecord.html#tymethod.insert),
    /// [`save()`](trait.ActiveRecord.html#tymethod.save), and
    /// [`update()`](trait.ActiveRecord.html#tymethod.update) should return
    /// `Err` as well. `before_save()` should *not* handle referential
    /// integrity for many-to-one associations.  Those should be handled by
    /// [`insert()`](trait.ActiveRecord.html#tymethod.insert), [`save()`
    /// ](trait.ActiveRecord.html#tymethod.save), and [`update()`
    /// ](trait.ActiveRecord.html#tymethod.update) directly.
    ///
    /// <b>Errors</b>
    ///
    /// On error return, the variant should be an [`Error::Model`
    /// ](../error/enum.Error.html#variant:Model) indicating the offending
    /// field(s) or associations(s) and what the problem is.
    ///
    fn before_save(&mut self, db: DatabaseIf) -> Result<(), Error>;
}

/// Builds a [`Query`](struct.Query.html) for [`ActiveRecord::query()`
/// ](trait.ActiveRecord.html#tymethod.query).
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
        pub fn $op_fn(mut self, db_col: &str) -> Self {
            match self.1 {
                QbState::Valid if self.0.filter.is_some() => {
                    self.0.filter.as_mut().unwrap().push_str(
                        concat!(" ", $op_str, " ")
                    );
                    self.0.filter.as_mut().unwrap().push_str(db_col);
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
    ///
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
        /// `db_col` is the column name in the database.
        ///
        and, "AND"
    }

    /// Initiate building the first WHERE condition.
    ///
    /// `db_col` is the column name in the database.
    ///
    pub fn col(mut self, db_col: &str) -> Self {
        match self.1 {
            QbState::Valid if self.0.filter.is_none() => {
                self.0.filter = Some(db_col.to_string());
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

    /// Add a WHERE condition.
    ///
    /// `fltr` is the meat of a WHERE clause - no `WHERE`!  It may be
    /// parameterized using the notation `$`*n* for the n:th parameter, 1
    /// based.
    ///
    /// If there is an existing WHERE clause, *n* in the `$`*n* in the new one
    /// are increased with the number of previously existing parameters and
    /// the new clause will be `"(`*old clause*`) AND `*new condition*`"`.
    ///
    /// `values` are the new parameter values, appended to any existing.
    ///
    pub fn filter(mut self, fltr: &str, values: &[Option<DbValue>]) -> Self {
        match self.1 {
            QbState::Valid => {
                self.0.filter = Some(match self.0.filter {
                    Some(old_filter) => {
                        // add old parameter count to new parameter indexes
                        use lazy_static::lazy_static;
                        use regex::Regex;
                        lazy_static! {
                            static ref PARAMS: Regex =
                                Regex::new(r"\$([0-9]+)").unwrap();
                        }
                        let old_par_count = self.0.values.len();
                        let mut new_filter = String::new();
                        let mut last = 0;
                        for cap in PARAMS.captures_iter(fltr) {
                            let nr = cap.get(1).unwrap();
                            new_filter.extend(fltr[last..nr.start()].chars());
                            new_filter +=
                                &(nr.as_str().parse::<usize>().unwrap()
                                    + old_par_count)
                                    .to_string();
                            last = nr.end();
                        }
                        if last < fltr.len() {
                            new_filter.extend(fltr[last..].chars());
                        }
                        format!("({}) AND {}", old_filter, &new_filter)
                    }
                    None => fltr.to_string(),
                });
                self.0.values.extend(values.iter().map(|v| v.clone()));
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

    /// Remove the ORDER clause, e.g. to avoid default ordering.
    ///
    pub fn no_order(mut self) -> Self {
        self.0.order = Order::NoOrder;
        self
    }

    /// Set the number of objects to skip.
    ///
    /// `offset` is the offset to use.
    ///
    pub fn offset(mut self, offset: usize) -> Self {
        self.0.offset = Some(offset);
        self
    }

    where_log_op! {
        /// Initiate building another WHERE condition OR-ed to the previous.
        ///
        /// `db_col` is the column name in the database.
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
/// [`ActiveRecord::query()`](trait.ActiveRecord.html#tymethod.query), see
/// [`QueryBld`](struct.QueryBld.html) for how to build.
///
/// The fields are public because you need them to implement
/// `ActiveRecord::query()`.
///
#[derive(Clone, Debug)]
pub struct Query {
    /// The meat of a WHERE clause - no `WHERE`!
    pub filter: Option<String>,
    /// The limit to send to the database.
    pub limit: Option<usize>,
    /// The offset to send to the database.
    pub offset: Option<usize>,
    /// See [`Order`](enum.Order.html).
    pub order: Order,
    /// The values to put in the database query.
    pub values: Vec<Option<DbValue>>,
}

impl Query {
    /// Create a [query builder](struct.QueryBld.html) to extend `self`.
    pub fn builder(self) -> QueryBld {
        QueryBld(self, QbState::Valid)
    }

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

/// Represents the ordering of the objects returned by
/// [`ActiveRecord::query()`](trait.ActiveRecord.html#tymethod.query).
///
/// The variants are public because you need them to implement
/// `ActiveRecord::query()`.
///
#[derive(Clone, Debug)]
pub enum Order {
    /// The meat of the ORDER clause - no `ORDER BY`!
    ///
    Custom(String),

    /// Use the models default order as defined by the [`vicocomo_order_by`
    /// ](../vicocomo_active_record/index.html) attribute on one or more model
    /// struct fields.
    Dflt,

    /// No `ORDER BY` sent to the database.
    ///
    NoOrder,
}
