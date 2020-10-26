//! Traits implemented by model objects and some helper types.
//!
//! All of the traits have [derive macros
//! ](../../vicocomo_active_record/index.html) with the same name.
//!
//! ## Pseudo traits
//!
//! "Pseudo" because there are derive macros, [`BelongsTo`
//! ](../../vicocomo_active_record/derive.BelongsTo.html) and [`HasMany`
//! ](../../vicocomo_active_record/derive.HasMany.html), that actually do not
//! implement traits.
//!
//! ### BelongsTo
//!
//! Functions for an association where this type is on the "many" side of a
//! one-to-many relationship to another (or the same) type.
//!
//! For functions, see the [`BelongsTo`
//! ](../../vicocomo_active_record/derive.BelongsTo.html) derive macro.
//!
//! ### HasMany
//!
//! Functions for a one-to-many or many-to-many relationship between this type
//! and another (or the same) type.
//!
//! For functions, see the [`HasMany`
//! ](../../vicocomo_active_record/derive.HasMany.html) derive macro.
//!
//  ### Motivation for not declaring those traits
//
//  Using BelongsTo as an example, it would have been declared something like
//
//      pub trait BelongsTo<Remote, Name = Remote>: Sized {
//          fn all_belonging_to(
//              db: &impl DbConn,
//              remote: &Remote,
//          ) -> Result<Vec<Self>, Error>;
//
//          fn get(&self, db: &impl DbConn) -> Option<Remote>;
//
//          fn set(&mut self, remote: &Remote) -> Result<(), Error>;
//      }
//
//  The reason for the Name type is to handle the case where there are more
//  than one BelongsTo associations connectiong the implementing Self type to
//  the same Remote type, which is not uncommon.  Name would typically be a
//  unit struct which is only used for this disambiguation.
//
//  Suppose the Remote type is Person, and you have one implementation with
//  Name Mother, and another with Name Employer.  Example code would be:
//
//      children = <Self as BelongsTo<Person, Mother>>::all_belonging_to(
//          db, &a_mother
//      );
//      boss = BelongsTo::<Person, Employer>::get(&me, db);
//
//  This is rather cumbersome.  Therefore the chosen solution is not to
//  declare the trait but having the derive macro define functions with
//  different names:
//
//      children = Self::all_belonging_to_mother(db, &a_mother);
//      boss = me.employer(db);
//
//  Easier to write and (more important) read.

use crate::Error;
use crate::{DbConn, DbValue};

/// Functions for deleting models from the database.
///
#[allow(unused_variables)]
pub trait Delete<PkType> {
    /// Return `Ok(1)` iff the corresponding database row is deleted.
    ///
    /// The implementation should ensure referential integrity if the
    /// implementor derives [`HasMany`](../derive.HasMany.html) as defined by
    /// the `on_delete` value in the attribute `vicocomo_has_many`, e.g.
    /// relying on the database's referential integrity.
    ///
    fn delete(self, db: &impl DbConn) -> Result<usize, Error>;

    /// Return `Ok(batch.len())` iff each key in `batch` identifies a database
    /// row that is deleted.
    ///
    /// `batch` should be a slice of primary key values.  If there are more
    /// than one primary key column, `PkType` is a tuple in the order they are
    /// declared in the struct.
    ///
    /// The implementation should ensure referential integrity if the
    /// implementor derives [`HasMany`](../derive.HasMany.html), see
    /// [`delete()`](#tymethod.delete.html).
    ///
    fn delete_batch(
        db: &impl DbConn,
        batch: &[PkType],
    ) -> Result<usize, Error>;
}

/// A hook called by the `delete()` function implemented by the [`Delete`
/// ](../derive.Delete.html) derive macro.
///
pub trait BeforeDelete {
    /// Do whatever necessary before deleting `self` from the database.
    ///
    /// An `Err` return value means that `self` cannot be deleted, and
    /// [`delete()`](trait.Delete.html#tymethod.delete) should return `Err`
    /// as well.  `before_delete()` should *not* handle objects related to
    /// `self` by [`HasMany`
    /// ](../../vicocomo_model_derive/derive.HasMany.html.html) associations.
    /// Those should be handled by [`delete()`
    /// ](trait.Delete.html#tymethod.delete) directly.
    ///
    fn before_delete(&mut self, db: &impl DbConn) -> Result<(), Error>;
}

/// Functions for retrieving models from the database.
///
/// Some functions are useful also for tables that have no primary key, e.g.
/// views.  In that case `PkType` should be `()`.
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
    fn load(db: &impl DbConn) -> Result<Vec<Self>, Error>;

    /// Return a vector with a possibly limited number of records that satisfy
    /// a condition possibly in a specified order.
    ///
    /// `query` is a [`Query`](struct.Query.html), see that and
    /// [`QueryBld`](struct.QueryBld.html).
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
    /// The implementation should return `Err` if the implementor derives
    /// [`BelongsTo`](../derive.BelongsTo.html) and there is an invalid remote
    /// reference, e.g. relying on the database's referential integrity.
    ///
    fn insert(&mut self, db: &impl DbConn) -> Result<(), Error> {
        *self = Self::insert_batch(db, std::slice::from_mut(self))?
            .pop()
            .unwrap();
        Ok(())
    }

    /// Try to INSERT a number of rows in the database from `data` and return
    /// new model structs updated from the inserted rows after insert.
    ///
    /// The implementation by [`#[derive(vicocomo::Save)]`
    /// ](../derive.Save.html) ensures that any field with the attribute
    /// `vicocomo_optional` will be sent to the database only if it is `Some`.
    ///
    /// It is an error if any of the data has a primary key that exists in the
    /// database.
    ///
    /// The implementation should ensure referential integrity if the
    /// implementor derives [`BelongsTo`](../derive.BelongsTo.html), see
    /// [`insert()`](#method.insert.html).
    ///
    fn insert_batch(
        db: &impl DbConn,
        data: &mut [Self],
    ) -> Result<Vec<Self>, Error>;

    /// Save the object's data to the database.
    ///
    /// If a row with the object's primary key exists in the database, this
    /// should be equivalent to [`update()`](trait.Save.html#tymethod.update).
    /// If not, this should be equivalent to [`insert()`
    /// ](trait.Save.html#method.insert).
    ///
    /// The default implementation tries first `update()`, then `insert()`.
    ///
    fn save(&mut self, db: &impl DbConn) -> Result<(), Error> {
        self.update(db).or_else(|_e| self.insert(db))
    }

    /// Try to UPDATE a row in the database from `self` and update self from
    /// the updated row after update.
    ///
    /// It is an error if `self` lacks a primary key or has one that does not
    /// exist in the database.
    ///
    /// The implementation should ensure referential integrity if the
    /// implementor derives [`BelongsTo`](../derive.BelongsTo.html), see
    /// [`insert()`](#method.insert.html).
    ///
    fn update(&mut self, db: &impl DbConn) -> Result<(), Error>;

    /// Try to UPDATE the row in the database corresponding to `self`.  Each
    /// pair in `cols` is the name of a database column and the new value.
    ///
    /// On successful return `self` is updated from the database.  On error,
    /// `self` is unchanged.
    ///
    /// <b>Note</b> that this function updates directly to the database and
    /// should ignore the visibility of the fields in `self` corresponding to
    /// the `cols`.
    ///
    fn update_columns(
        &mut self,
        db: &impl DbConn,
        cols: &[(&str, DbValue)],
    ) -> Result<(), Error>;
}

/// A hook called by functions implemented by the [`Save`
/// ](../derive.Save.html) derive macro.
///
pub trait BeforeSave {
    /// Do whatever necessary before saving `self` to the database.
    ///
    /// An `Err` return value means that `self` cannot be saved, and
    /// [`insert()`](trait.Save.html#method.insert),
    /// [`save()`](trait.Save.html#method.save), and
    /// [`update()`](trait.Save.html#tymethod.update) should return `Err` as
    /// well.  `before_save()` should *not* handle referential integrity for
    /// [`BelongsTo`](../derive.BelongsTo.html) associations.  Those should be
    /// handled by [`insert()`](trait.Save.html#method.insert), [`save()`
    /// ](trait.Save.html#method.save), and [`update()`
    /// ](trait.Save.html#tymethod.update) directly.
    ///
    fn before_save(&mut self, db: &impl DbConn) -> Result<(), Error>;
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
/// [`Find::query()`](trait.Find.html#tymethod.query), see [`QueryBld`
/// ](struct.QueryBld.html) for how to build.
///
/// The fields are public because you need them to implement `Find::query()`.
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
    /// ](../vicocomo_active_record/index.html) attribute on one or more model
    /// struct fields.
    Dflt,

    /// No `ORDER BY` sent to the database.
    ///
    NoOrder,
}
