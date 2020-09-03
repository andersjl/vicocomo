//! # Model helper macros
//!
//! ```text
//! #[derive(<one or more of BelongsTo, Delete, Find, and Save>)]
//! #[vicoomo_table_name = "example_table"]  // default "examples"
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
//!     #[vicocomo_belongs_to(        // child referencing parent
//!         name = "father",          // base for generated function names,
//!                                   // default strip "_id" from field name
//!         path = "crate::x::OlMan", // parent struct path, default
//!                                   // crate::models::<name.to_camel()>
//!         parent_pk = "pk",         // parent PK field, default "id", must
//!     )]                            // be a single primary key field
//!     parent_id: u32,               // May be nullable, in this case not
//! }
//! ```
extern crate proc_macro;
extern crate syn;

use proc_macro::TokenStream;

mod belongs_to;
mod delete;
mod find;
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
/// "`<name>`" means the `name` value if given, or the last segment of
/// `remote_path` if not, snake cased.
///
/// ### For each `vicocomo_belongs_to` attributed field
///
/// ```text
/// fn all_belonging_to_<name>(
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
/// fn belongs_to_<name>(&self, db: &impl DbConn) -> Option<Remote>
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
/// fn belong_to_<name>(&mut self, remote: &Remote) -> Result<(), Error>
/// ```
/// Set the reference to an object on the remote side of the relationship.
///
/// `remote` is the object on the remote side of the relationship.
///
/// The new remote association is not saved to the database.
///
/// ```text
/// fn belong_to_no_<name>(&mut self) -> Result<(), Error>
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
/// fn <name>_siblings(&self, db: &impl DbConn) -> Result<Vec<Self>, Error>
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
/// See this [example](../vicocomo_derive/index.html).
///
/// `vicocomo_table_name = "`some table name`"` - The database table storing
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
/// `vicocomo_unique = "`*a label*`"` - The tuple of fields whith the same
/// label should be unique in the database.  Primary keys do not need this.
///
/// ## Generated code
///
/// Implements [`Delete`](../vicocomo/model/trait.Delete.html).
///
#[proc_macro_derive(
    Delete,
    attributes(
        vicocomo_column,
        vicocomo_optional,
        vicocomo_primary,
        vicocomo_table_name,
        vicocomo_unique,
    )
)]
pub fn delete_derive(input: TokenStream) -> TokenStream {
    delete::delete_impl(&model::Model::new(
        input,
        vec![model::ExtraInfo::UniqueFields],
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
        vicocomo_table_name
    )
)]
pub fn save_derive(input: TokenStream) -> TokenStream {
    save::save_impl(&model::Model::new(
        input,
        vec![model::ExtraInfo::DatabaseTypes],
    ))
}
