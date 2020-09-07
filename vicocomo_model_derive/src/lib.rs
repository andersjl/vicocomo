//! # Model helper macros
//!
//! ```text
//! #[derive(<one or more of BelongsTo, Delete, Find, HasMany, and Save>)]
//! #[vicocomo_table_name = "example_table"]  // default "examples"
//! // one or more vicocomo_has_many attributes, see HasMany below
//! #[vicocomo_has_many(              // one-to-many or possibly ...
//!     join_table = "tnam",          // ... many-to-many w join table "tnam"
//!     name = "SomeName",            // needed if several impl same Rem
//!     on_delete = "cascade",        // cascade / forget / restrict (default)
//!     remote_type = "super::Rem",   // Remote type, identifier mandatory
//!     remote_fk_col = "fk_self",    // Remote or join key to self, default
//!                                   // "t_id" if the type of Self is T
//!     // ... if many-to-many, i.e. "join_table" table given ----------------
//!     join_fk_col = "fk_rem",       // join tab key to Rem, default "rem_id"
//!     remote_pk_col = "pk")]        // Rem primary col name, default "id",
//! struct Example {
//!     #[vicocomo_optional]          // not sent to DBMS if None
//!     #[vicocomo_primary]           // To find a row to update() or delete()
//!     primary: Option<u32>,         // primary key should be ensured by DBMS
//!     #[vicocomo_column = "db_col"] // different name of DB column
//!     #[vicocomo_unique = "un1"]    // "un1" labels fields w unique comb.
//!     not_null: String,             // VARCHAR NOT NULL
//!     #[vicocomo_order_by(2)]       // precedence 2, see opt_null below
//!     nullable: Option<String>,     // VARCHAR, None -> NULL
//!     #[vicocomo_optional]          // not sent to DBMS if None
//!     #[vicocomo_unique = "un1"]    // UNIQUE(db_col, opt_not_null)
//!     opt_not_null: Option<i32>,    // INTEGER NOT NULL DEFAULT 42
//!     #[vicocomo_order_by(1, "desc")] // ORDER BY opt_null DESC, nullable
//!     #[vicocomo_optional]          // not sent to DBMS if None
//!     opt_null:                     // INTEGER DEFAULT 43
//!         Option<Option<i32>>,      // None -> 43, Some(None) -> NULL
//!     #[vicocomo_belongs_to(        // "many" side of one-to-many
//!         name = "Father",          // needed if several impl same Remote
//!         remote_type =             // remote struct path, default
//!             "crate::x::OlMan",    // crate::models::rem::Rem (if rem_id)
//!         remote_pk = "pk",         // remote PK field, default "id",
//!     )]                            // must be a single primary key field
//!     rem_id: u32,                  // May be nullable, in this case not
//! }
//! ```
extern crate proc_macro;
extern crate syn;

use proc_macro::TokenStream;

mod belongs_to;
mod delete;
mod find;
mod has_many;
mod model;
mod save;

/// Derive the [`BelongsTo`](../vicocomo/model/trait.BelongsTo.html) trait for
/// a `struct` with named fields.
///
/// Note that the `Remote` struct must have exactly one `vicocomo_primary`
/// field.  The generated code also requires the `Remote` type to implement
/// [`Find<_>`](derive.Find.html).
///
/// ## Field attributes
///
/// See this [example](../vicocomo_model_derive/index.html).
///
/// `vicocomo_column = "`*column name*`"` - The database column storing the
/// field.  Default the snake cased field name.
///
/// `vicocomo_belongs_to(` ... `)` - The field is a foreign key to a model
/// object on the remote side of the relationship.  The following name-value
/// pairs are optional:
///
/// - `name = "`*a camel case name*`"`:  If there are more than one
///   `BelongsTo` implementation for this type with *the same* `remote_type`,
///   all except one of them must have a `name`.  A unit `struct` *the value
///   of `name`* will be generated, which is only used for this
///   disambiguation.
///
/// - `remote_pk = "`*a field id*`"`: The name of the `Remote` type's primary
///   key *field* - not the column!  `BelongsTo` associations to models with
///   composite primary keys is not possible.  The primary key field is taken
///   to be `vicocomo_optional`.  If it is mandatory, this must be indicated
///   by `remote_pk ="`*a field id* `mandatory"`.
///
///   The default is `id`.
///
/// - `remote_type = "`*a path*`"`:  The `Remote` type.  If the type is a
///   single identifier, `crate::models::`*snake case identifier*`::` is
///   prepended.
///
///   If the field identifier ends in `_id` the default path is
///   `crate::models::`*rem*`::`*rem camel cased*, where *rem* is the field
///   identifier with `_id` stripped.  If not, `remote_type` is mandatory.
///
/// ## Generated code
///
/// Implements [`BelongsTo<Remote, Name = Remote>`
/// ](../vicocomo/model/trait.BelongsTo.html).
///
/// For each `name` given in a `vicocomo_belongs_to` attribute, a unit
/// `struct` with that name is declared.  Make sure it is unique in the
/// context where the macro is expanded.
///
/// Below, "`<name>`" means the `name` value if given, or the last segment of
/// `remote_type` if not, snake cased.
///
/// ### For each `vicocomo_belongs_to` attributed field
///
/// ```text
/// pub fn all_belonging_to_<name>(
///     db: &impl ::vicocomo::DbConn,
///     remote: &Remote,
/// ) -> Result<Vec<Self>, Error>
/// ```
/// Retrive all objects in the database belonging to an instance of
/// `Remote`.
///
/// `db` is the database connection object.
///
/// `remote` is the object on the remote side of the relationship.
///
/// ```text
/// pub fn belongs_to_<name>(&self, db: &impl DbConn) -> Option<Remote>
/// ```
/// Retrive the object on the remote side of the relationship from the
/// database.
///
/// `db` is the database connection object.
///
/// A return value of `None` may be
/// - because the corresponding field in `self` is `None` (if the field is
///   an `Option`),
/// - because there is no row in the remote table with a primary key
///   matching the field value, or
/// - because of some other database error.
///
/// ```text
/// pub fn belong_to_<name>(&mut self, remote: &Remote) -> Result<(), Error>
/// ```
/// Set the reference to an object on the remote side of the relationship.
///
/// `remote` is the object on the remote side of the relationship.
///
/// The new remote association is not saved to the database.
///
/// ```text
/// pub fn belong_to_no_<name>(&mut self) -> Result<(), Error>
/// ```
/// Forget the reference to an object on the remote side of the
/// relationship.
///
/// The old reference is not removed from the database.
///
/// The default function returns an `Error`.
///
/// Should be implemented if the association field is an `Option`.
///
/// ```text
/// pub fn <name>_siblings(&self, db: &impl DbConn) -> Result<Vec<Self>, Error>
/// ```
/// Retrive all owned objects in the database (including `self`) that
/// belong to the same object as `self`.
///
#[proc_macro_derive(
    BelongsTo,
    attributes(vicocomo_column, vicocomo_belongs_to,)
)]
pub fn belongs_to_derive(input: TokenStream) -> TokenStream {
    belongs_to::belongs_to_impl(&model::Model::new(
        input,
        vec![
            model::ExtraInfo::BelongsToData,
            model::ExtraInfo::OrderFields,
            model::ExtraInfo::UniqueFields,
            model::ExtraInfo::DatabaseTypes,
        ],
    ))
}

/// Derive the [`Delete`](../vicocomo/model/trait.Delete.html) trait for a
/// `struct` with named fields.
///
/// ## Struct attributes
///
/// See this [example](../vicocomo_model_derive/index.html).
///
/// `vicocomo_table_name = "`*some table name*`"` - The database table storing
/// the struct.  Default the snake cased struct name with a plural 's'.
///
/// `vicocomo_delete_errors`: - See [`DeleteErrors`
/// ](../vicocomo/model/trait.DeleteErrors.html).  If present, the generated
/// [`Delete::delete()`](../vicocomo/model/trait.Delete.html#tymethod.delete)
/// requires the model to implement [`DeleteErrors`
/// ](../vicocomo/model/trait.DeleteErrors.html) and calls
/// [`errors_preventing_delete()`
/// ](../vicocomo/model/trait.DeleteErrors.html#tymethod.errors_preventing_delete).
///
/// `vicocomo_has_many(` ... `)` - See [`HasMany`](derive.HasMany.html).  For
/// `Delete`, we need this to handle the objects associated to this one by a
/// `HasMany` association.
///
/// - #### One-to-many associations
///
///   Depending on the value of the `on_delete = "`...`"` name-value pair
///   - `cascade`:  The associated objects are deleted when `self` is.  Note
///     that this requires that `Remote`, too, implements [`Delete`
///     ](../vicocomo/model/trait.Delete.html).
///   - `forget`:  The remote references are set to `None`.  Note that this
///     requires that`Remote` object derives [`BelongsTo`
///     ](derive.BelongsTo.html).
///   - `restrict`:  Error return, `self` cannot be deleted as long as there
///     are associatied objects.  This is the default.
///
/// - #### Many-to-many associations
///
///   The remote object is never deleted, and all rows in the join table
///   referring to `self` are always deleted when `self` is.
///
/// ## Field attributes
///
/// See this [example](../vicocomo_model_derive/index.html).
///
/// `vicocomo_column = "`*column name*`"` - The database column storing the
/// field.  Default the snake cased field name.
///
/// `vicocomo_optional` - The field should be a Rust `Option`, and a `None`
/// value is never sent to the database.  Intended for values that may be
/// generated by the database when missing.
///
/// `vicocomo_primary` - The field corresponds to a primary key in the
/// database.
///
/// `vicocomo_unique = "`*a label*`"` - The tuple of fields whith the same
/// label should be unique in the database.  Primary keys do not need this.
///
/// ## Generated code
///
/// Implements [`Delete`](../vicocomo/model/trait.Delete.html).
///
/// Note that the implementation of [`delete_batch()`
/// ](../vicocomo/model/trait.Delete.html#tymethod.delete_batch) ignores the
/// attribute `vicocomo_delete_errors` and does *not* call
/// [`DeleteErrors::errors_preventing_delete()`
/// ](../vicocomo/model/trait.DeleteErrors.html#tymethod.errors_preventing_delete)!
///
#[proc_macro_derive(
    Delete,
    attributes(
        vicocomo_column,
        vicocomo_delete_errors,
        vicocomo_has_many,  // we must know what to do on delete
        vicocomo_optional,
        vicocomo_primary,
        vicocomo_table_name,
        vicocomo_unique,
    )
)]
pub fn delete_derive(input: TokenStream) -> TokenStream {
    delete::delete_impl(&model::Model::new(
        input,
        vec![
            model::ExtraInfo::HasManyData,
            model::ExtraInfo::UniqueFields,
        ],
    ))
}

/// Derive the [`Find`](../vicocomo/model/trait.Find.html) trait for a
/// `struct` with named fields.
///
/// ## Struct attributes
///
/// See this [example](../vicocomo_model_derive/index.html).
///
/// `vicocomo_table_name = "`*some table name*`"` - The database table storing
/// the struct.  Default the snake cased struct name with a plural 's'.
///
/// ## Field attributes
///
/// See this [example](../vicocomo_model_derive/index.html).
///
/// `vicocomo_column = "`*column name*`"` - The database column storing the
/// field.  Default the snake cased field name.
///
/// `vicocomo_optional` - The field should be a Rust `Option`, and a `None`
/// value is never sent to the database.  Intended for values that may be
/// generated by the database when missing.
///
/// `vicocomo_order_by(`priority`[`, "`*direction*`"`]`)` - Defines a default
/// ordering when retrieving model objects.  Direction is optional and either
/// `ASC` or `DESC`.
///
/// `vicocomo_primary` - The field corresponds to a primary key in the
/// database.
///
/// `vicocomo_unique = "`*a label*`"` - The tuple of fields whith the same
/// label should be unique in the database.  Primary keys do not need this.
///
/// ## Generated code
///
/// Implements [`Find`](../vicocomo/model/trait.Find.html).
///
/// ### For each `vicocomo_unique` label
///
/// Given the struct declaration
/// ```text
/// #[derive(::vicocomo::Find)]
/// struct Example {
///     #[vicocomo_primary]
///     id: Option<u32>,
///     #[vicocomo_optional]
///     #[vicocomo_unique = "uni-lbl"]
///     un1: Option<i32>,
///     #[vicocomo_unique = "uni-lbl"]
///     un2: i32,
/// }
/// ```
/// also the following methods are generated:
///
/// ```text
/// pub fn find_by_un1_and_un2(
///     db: &mut impl ::vicocomo::DbConn,
///     un1: i32,
///     un2: i32,
/// ) -> Option<Self>
/// ```
///
/// Find an object in the database by the unique fields.
///
/// `db` is the database connection object.
///
/// `un1` and `un2` are the unique parameters.  Note that `un1` is "unwrapped"
/// even though it is declared `vicocomo_optional`.
///
/// ```text
/// pub fn find_equal_un1_and_un2(
///     &self,
///     db: &mut impl ::vicocomo::DbConn
/// ) -> Option<Self>
/// ```
///
/// Find an object in the database that has the same values for the unique
/// fields as `self`.  If a unique field in `self` is `vicocomo_optional` and
/// `None`, error return.
///
/// `db` is the database connection object.
///
/// ```text
/// pub fn validate_exists_un1_and_un2(
///     db: &mut impl ::vicocomo::DbConn,
///     un1: i32,
///     un2: i32,
///     msg: &str,
/// ) -> Result<(), ::vicocomo::Error> {
///
#[proc_macro_derive(
    Find,
    attributes(
        vicocomo_column,
        vicocomo_optional,
        vicocomo_order_by,
        vicocomo_primary,
        vicocomo_table_name,
        vicocomo_unique
    )
)]
pub fn find_derive(input: TokenStream) -> TokenStream {
    find::find_impl(&model::Model::new(
        input,
        vec![
            model::ExtraInfo::OrderFields,
            model::ExtraInfo::UniqueFields,
            model::ExtraInfo::DatabaseTypes,
        ],
    ))
}

/// Derive the [`HasMany`](../vicocomo/model/trait.HasMany.html) trait for a
/// `struct` with named fields.
///
/// Note that `Self` must have exactly one `vicocomo_primary` field.  The
/// generated code also requires `Remote` to implement [`Find<_>`
/// ](derive.Find.html).
///
/// ## Struct attributes
///
/// See this [example](../vicocomo_model_derive/index.html).
///
/// `vicocomo_has_many(` ... `)` - Self has a {one,many}-to-many association.
/// There should be one `vicocomo_has_many` for each -to-many association.
/// The following name-value pairs are recognized:
///
/// - `remote_type = "`*the type of the associated object*`"`:  Mandatory.  If
///   the type given is a single identifier,
///   `crate::models::`*snake case identifier*`::` is prepended.
///
/// - `name = "`*a camel case name*`"`:  If there are more than one
///   `HasMany` implementation for this type with *the same* `remote_type`,
///   all except one of them must have a `name`.  A unit `struct` *the value
///   of `name`* will be generated, which is only used for this
///   disambiguation.
///
/// - `remote_fk_col = "`*a database column name*`"`:  If one-to-many, the
///   column in the remote model, if many-to-many in the join table, that
///   refers to `self`.  Default *snake case last identifier in `Self`*`_id`.
///
///   Note that a model with a composite primary key cannot have any `HasMany`
///   associations.
///
/// - <b>Only if one-to-many</b>
///
///   - `on_delete = "`*one of `cascade`, `forget`, or `restrict`*`"`:  See
///     [`Delete`](derive.Delete.html).  Defines the beavior when `self` is
///     deleted.  Optional, with default `restrict`.
///
///   - <b>Only if `on_delete = "forget"`</b>
///
///     - `remote_assoc = "`*a `BelongsTo` association name*`"`: To call
///       *remote object*`.belongs_to_no_`*snaked value of `remote_assoc`*.
///       Optional, default *last identifier in `Self`*.
///
/// - <b>Only if many-to-many</b>
///
///   A many-to-many association is realized by way of a "join table", having
///   exactly one row for each associations instance, with foreign keys to the
///   rows representing the associated objects.
///
///   When `self` is deleted, all join table rows associated to `self` are
///   deleted, but no rows representing `Remote` objects.
///
///   - `join_table = "`*a database table name*`"`:  The name of a join table,
///     making the association many-to-many.
///
///   - `join_fk_col = "`*a database column name*`"`:  Optional name of the
///     foreign key column in the join table referring to the remote model.
///     The default is *snake case last identifier in `remote_type`*`_id`.
///
///   - `remote_pk = "`*a field id*`"`: The name of the `Remote` type's
///     primary key *field* - not the column!  Many-to-many associations to
///     models with composite primary keys is not possible.  The primary key
///     field is taken to be `vicocomo_optional`.  If it is mandatory, this
///     must be indicated by `remote_pk ="`*a field id* `mandatory"`.
///
///     The default is `id`.
///
///   - `remote_pk_col = "`*a database column name*`"`:  Optional name of the
///     `Remote` type's primary key column.  Many-to-many associations to
///     models with composite primary keys is not possible.
///
///     The default is *value of `remote_pk`* if given or `id`.
///
/// ## Generated code
///
/// Implements [`HasMany<Remote, Name = Remote>`
/// ](../vicocomo/model/trait.BelongsTo.html).
///
/// For each `name` given in a `vicocomo_has_many` attribute, a unit `struct`
/// with that name is declared.  Make sure it is unique in the context where
/// the macro is expanded.
///
/// Below, "`<name>`" means the snake cased version of the `name` value if
/// present or of the last segment of `remote_type` if not.
///
/// ### For each `vicocomo_has_many` struct attribute
///
/// ```text
/// pub fn find_remote_<name>(
///     &self,
///     db: &impl DbConn,
///     filter: Option<&Query>,
/// ) -> Result<Vec<Remote>, Error>;
///```
/// Find items related to `self` by the association, filtered by `filter`.
///
/// `filter`, see [`QueryBld`](struct.QueryBld.html).  A condition to select
/// only among the associated objects is automatically added.
///
/// #### Functions only for many-to-many associations
///
/// ```text
/// pub fn connect_to_<name>(
///     &self,
///     db: &impl DbConn,
///     remote: &Remote,
/// ) -> Result<usize, Error>;
/// ```
/// Insert a join table row connecting `self` to `remote`.  Returns `Ok(1)` on
/// success.  Does *not* check that such a row did not exist previously!  It
/// is strongly recommended to create a unique index in the database to
/// prevent multiple connections between the same objects.
///
/// ```text
/// pub fn disconnect_from_<name>(
///     &self,
///     db: &impl DbConn,
///     remote: &Remote,
/// ) -> Result<usize, Error>;
/// ```
/// Delete the join table row connecting `self` to `remote`.  *Returns `Ok(0)`
/// if they are not connected*.
///
#[proc_macro_derive(HasMany, attributes(vicocomo_hasmany))]
pub fn has_many_derive(input: TokenStream) -> TokenStream {
    has_many::has_many_impl(&model::Model::new(
        input,
        vec![
            model::ExtraInfo::DatabaseTypes,
            model::ExtraInfo::HasManyData,
        ],
    ))
}

/// Derive the [`Save`](../vicocomo/model/trait.Save.html) trait for a
/// `struct` with named fields.
///
/// ## Struct attributes
///
/// See this [example](../vicocomo_model_derive/index.html).
///
/// `vicocomo_table_name = "`*some table name*`"` - The database table storing
/// the struct.  Default the snake cased struct name with a plural 's'.
///
/// ## Field attributes
///
/// See this [example](../vicocomo_model_derive/index.html).
///
/// `vicocomo_column = "`*column name*`"` - The database column storing the
/// field.  Default the snake cased field name.
///
/// `vicocomo_optional` - The field should be a Rust `Option`, and a `None`
/// value is never sent to the database.  Intended for values that may be
/// generated by the database when missing.
///
/// `vicocomo_primary` - The field corresponds to a primary key in the
/// database.
///
/// ## Generated code
///
/// Implements [`Save`](../vicocomo/model/trait.Save.html).
///
#[proc_macro_derive(
    Save,
    attributes(
        vicocomo_column,
        vicocomo_optional,
        vicocomo_primary,
        vicocomo_save_errors,
        vicocomo_table_name
    )
)]
pub fn save_derive(input: TokenStream) -> TokenStream {
    save::save_impl(&model::Model::new(
        input,
        vec![model::ExtraInfo::DatabaseTypes],
    ))
}
