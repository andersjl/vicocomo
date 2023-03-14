//! # Active Record derive macro

use proc_macro::TokenStream;

mod belongs_to;
mod common;
mod delete;
mod find;
mod has_many;
mod model;
mod save;

/// Derive the [`ActiveRecord`
/// ](../vicocomo/active_record/trait.ActiveRecord.html) trait for a `struct`
/// with named fields.
///
/// ## Example
///
/// ```text
/// #[derive(ActiveRecord)]
/// #[vicocomo_table_name = "example_table"]  // default "examples"
/// // one or more vicocomo_has_many attributes
/// #[vicocomo_has_many(              // one-to-many or possibly ...
///     join_table = "tnam",          // ... many-to-many w join table "tnam"
///     name = "SomeName",            // needed if several impl same Rem
///     on_delete = "cascade",        // cascade / forget / restrict (default)
///     remote_type = "super::Rem",   // Remote type, identifier mandatory
///     remote_fk_col = "fk_self",    // Remote or join key to self, default
///                                   // "t_id" if the type of Self is T
///     // ... if many-to-many, i.e. "join_table" table given ----------------
///     join_fk_col = "fk_rem",       // join tab key to Rem, default "rem_id"
///     remote_pk_col = "pk")]        // Rem primary col name, default "id",
/// struct Example {
///     #[vicocomo_optional]          // not sent to DBMS if None
///     #[vicocomo_primary]           // To find a row to update() or delete()
///     primary: Option<u32>,         // primary key should be ensured by DBMS
///     #[vicocomo_column = "db_col"] // different name of DB column
///     #[vicocomo_unique = "un1"]    // "un1" labels fields w unique comb.
///     not_null: String,             // TEXT NOT NULL
///     #[vicocomo_order_by(2)]       // precedence 2, see opt_null below
///     nullable: Option<String>,     // TEXT, None -> NULL
///     #[vicocomo_optional]          // not sent to DBMS if None
///     #[vicocomo_unique = "un1"]    // UNIQUE(db_col, opt_not_null)
///     opt_not_null: Option<i32>,    // BIGINT NOT NULL DEFAULT 42
///     #[vicocomo_order_by(1, "desc")] // ORDER BY opt_null DESC, nullable
///     #[vicocomo_optional]          // not sent to DBMS if None
///     opt_null:                     // BIGINT DEFAULT 43
///         Option<Option<i32>>,      // None -> 43, Some(None) -> NULL
///     #[vicocomo_belongs_to(        // "many" side of one-to-many
///         name = "Father",          // needed if several impl same Remote
///         remote_type =             // remote struct path, default
///             "crate::x::OlMan",    // crate::models::Rem (if rem_id)
///         remote_pk = "pk",         // remote PK field, default "id",
///     )]                            // must be a single primary key field
///     rem_id: u32,                  // May be nullable, in this case not
/// }
/// ```
///
/// ## Struct attributes
///
/// See above [example](#example).
///
/// ### `vicocomo_before_delete`
///
/// See [`BeforeDelete`](../vicocomo/active_record/trait.BeforeDelete.html).
/// If present, the generated [`ActiveRecord::delete()`
/// ](../vicocomo/active_record/trait.ActiveRecord.html#tymethod.delete)
/// requires the model to implement [`BeforeDelete`
/// ](../vicocomo/active_record/trait.BeforeDelete.html) and calls
/// [`before_delete()`
/// ](../vicocomo/active_record/trait.BeforeDelete.html#tymethod.before_delete).
///
/// ### `vicocomo_before_save`
///
/// See [`BeforeSave`](../vicocomo/active_record/trait.BeforeSave.html). If
/// present, the generated [`ActiveRecord::insert()`
/// ](../vicocomo/active_record/trait.ActiveRecord.html#tymethod.insert),
/// [`ActiveRecord::save()`
/// ](../vicocomo/active_record/trait.ActiveRecord.html#tymethod.save), and
/// [`ActiveRecord::update()`
/// ](../vicocomo/active_record/trait.ActiveRecord.html#tymethod.update)
/// methods require the model to implement [`BeforeSave`
/// ](../vicocomo/active_record/trait.BeforeSave.html) and calls
/// [`before_save()`
/// ](../vicocomo/active_record/trait.BeforeSave.html#tymethod.before_save).
///
/// ### `vicocomo_has_many(` ... `)`
///
/// Self has a {one,many}-to-many association. There should be one
/// `vicocomo_has_many` for each -to-many association.
///
/// Note that `Self` must have exactly one `vicocomo_primary` field. The
/// generated code also requires the remote model type to derive
/// `ActiveRecord` and have exactly one `vicocomo_primary` field.
///
/// The following name-value pairs are recognized:
///
/// - `remote_type = "`*the type of the associated object*`"`: Mandatory. If
///   the type given is a single identifier,
///   `"crate::models::`*snake cased identifier*`::"` is prepended.
///
/// - `name = "`*a camel case name*`"`:  If there are more than one
///   `vicocomo_has_many` attributes with *the same* `remote_type`, all except
///   possibly one of them must have a `name`.
///
///   Default `"`*the value of `remote_type`*`"`.
///
/// - `remote_fk_col = "`*a database column name*`"`: The column that refers
///   to `self`. If one-to-many in the remote model's table, if many-to-many
///   in the join table.
///
///   Optional, default `"`*snake cased last identifier in `Self`*`_id"`.
///
/// - <b>Only if one-to-many</b>
///
///   - `on_delete = "`*one of `cascade`, `forget`, or `restrict`*`"`:
///     Actually `derive(ActiveRecord)` relies on the database to handle
///     referential integrity, but nevertheless needs to know the `on_delete`
///     behavior.
///
///     Optional, default `restrict`.
///
/// - <b>Only if many-to-many</b>
///
///   A many-to-many association is realized by way of a "join table", having
///   exactly one row for each associations instance, with foreign keys to the
///   rows representing the associated objects.
///
///   - `join_table = "`*a database table name*`"`: Mandatory if many-to-many.
///
///   - `join_fk_col = "`*a database column name*`"`: The name of the foreign
///     key column in the join table referring to the remote model.
///
///     Optional, default *snake cased last identifier in `remote_type`*`_id`.
///
///   - `remote_pk = "`*a field id*`"`: The name of the `Remote` type's
///     primary key *field* - not the column! The primary key field is taken
///     to be `vicocomo_optional`. If it is mandatory, this must be indicated
///     by `remote_pk ="`*a field id*` mandatory"`.
///
///     Optional, default `id`.
///
///   - `remote_pk_col = "`*a database column name*`"`: The name of the
///     `Remote` type's primary key column.
///
///     Optional, default *value of `remote_pk`* if given or `id`.
///
/// See also the section on [referential integrity](#referential-integrity).
///
/// ### `vicocomo_table_name = "`*some table name*`"`
///
/// The database table storing the struct.
///
/// Optional, default the snake cased struct name with a plural 's'.
///
/// ## Field attributes
///
/// See above [example](#example).
///
/// ### `vicocomo_belongs_to(` ... `)`
///
/// The following name-value pairs are optional:
///
/// - `name = "`*a camel case name*`"`:  If there are more than one
///   `vicocomo_belongs_to` implementation for this type with *the same*
///   `remote_type`, all except one of them must have a `name`.
///
/// - `remote_pk = "`*a field id*`"`: The name of the remote model's primary
///   key *field* - not the column! `vicocomo_belongs_to` associations to
///   models with composite primary keys is not possible. The primary key
///   field is taken to be `vicocomo_optional`. If it is mandatory, this must
///   be indicated by `remote_pk ="`*a field id* `mandatory"`.
///
///   The default is `id`.
///
/// - `remote_type = "`*a path*`"`:  The remote model type. If the value is a
///   single identifier, `crate::models::`*snake cased identifier*`::` is
///   prepended.
///
///   If the field identifier ends in `_id` the default path is
///   `crate::models::`*rem camel cased*, where *rem* is the field identifier
///   with `_id` stripped. If not, `remote_type` is mandatory.
///
/// See also the section on [referential integrity](#referential-integrity).
///
/// ### `vicocomo_column = "`*column name*`"`
///
/// The database column storing the field.
///
/// Optional, default the snake cased field name.
///
/// ### `vicocomo_db_value = "`*DbValue variant as str*`"`
///
/// The field has a locally defined type that has implemented `Into<DbValue>`
/// and `TryFrom<DbValue>`, e.g. using the macro [`db_value_convert`
/// ](../vicocomo_db_macros/macro.db_value_convert.html)
///
/// ### `vicocomo_optional`
///
/// The field should be a Rust `Option`, and a `None` value is never sent to
/// the database. Intended for values that may be generated by the database
/// when missing.
///
/// ### `vicocomo_order_by(`*priority*`[, "`*direction*`"])`
///
/// Defines a default ordering when retrieving model objects. Direction is
/// optional and either `ASC` (default) or `DESC`.
///
/// ### `vicocomo_primary`
///
/// The field corresponds to a primary key in the database.
///
/// ### `vicocomo_required`
///
/// The field must not be nullable (i.e. not an `Option` or, if
/// `vicocomo_optional`, not an `Option<Option>`) and it also has to have a
/// meaningful value:
/// - Fields that convert to [`DbValue::Float`
///   ](../vicocomo/database/enum.DbValue.html#variant.Float) or
///   [`DbValue::Int`](../vicocomo/database/enum.DbValue.html#variant.Int):
///   Non-zero
/// - Fields that convert to [`DbValue::Text`
///   ](../vicocomo/database/enum.DbValue.html#variant.Text): Not only
///   whitespace
///
/// The generated [`ActiveRecord::insert_batch()`
/// ](../vicocomo/active_record/trait.ActiveRecord.html#tymethod.insert_batch)
/// and [`ActiveRecord::update()`
/// ](../vicocomo/active_record/trait.ActiveRecord.html#tymethod.update)
/// methods return an appropriate [`Error::Model`
/// ](../vicocomo/error/enum.Error.html#variant.Model) if this requirement is
/// not met.
///
/// ### `vicocomo_unique = "`*a label*`"`
///
/// The tuple of fields whith the same label should be unique in the database.
/// Primary keys do not need this.
///
/// The generated [`ActiveRecord::insert_batch()`
/// ](../vicocomo/active_record/trait.ActiveRecord.html#tymethod.insert_batch)
/// and [`ActiveRecord::update()`
/// ](../vicocomo/active_record/trait.ActiveRecord.html#tymethod.update)
/// methods return an appropriate [`Error::Model`
/// ](../vicocomo/error/enum.Error.html#variant.Model) if this requirement is
/// not met.
///
/// ### `vicocomo_validate_presence`
///
/// The field should be an `Option` but *not* `vicocomo_optional`. Generates a
/// validation function, see below.
///
/// ## Referential Integrity
///
/// <b>At present, the attriubutes `vicocomo_belongs_to` and
/// `vicocomo_has_many` do not prevent breaking referential integrity when
/// saving or deleting!</b>
///
/// Referential integrity should be handled by the database as follows. If it
/// does, the generated code will transform the database foreign key
/// violation errors to [`Error::Model`
/// ](../vicocomo/error/enum.Error.html#variant.Model).
///
/// - <b>One-to-many associations:</b>  The table storing the remote object
///   should have a foreign key declaration corresponding to the
///   `on_delete = "`*one of `cascade`, `forget`, or `restrict`*`"`
///   name-value pair, in the obvious way.
///
/// - <b>Many-to-many associations:</b>  The join table should have foreign
///   key declarations referring to the primary keys of the tables storing the
///   `Self` and remote types that ensure cascading on-delete behavior.
///
/// The intention is to use the attributes to generate referential integrity
/// tests and/or automatic schema generation in future releases.
///
/// ## Generated code
///
/// Implements [`ActiveRecord`
/// ](../vicocomo/active_record/trait.ActiveRecord.html).
///
/// Note that the implementation of [`delete_batch()`
/// ](../vicocomo/active_record/trait.ActiveRecord.html#tymethod.delete_batch)
/// ignores the attribute `vicocomo_before_delete` and does *not* call
/// [`before_delete()`
/// ](../vicocomo/active_record/trait.BeforeDelete.html#tymethod.before_delete)!
///
/// ### For each `vicocomo_belongs_to` attributed field
///
/// Below, "`*Remote*`" means the `remote_type` value (or the default), and
/// "`*name*`" means the `name` value if given, or the last segment of
/// `remote_type` if not, snake cased.
///
/// ##### `pub fn all_belonging_to_*name*(db: DatabaseIf, remote: &*Remote*) -> Result<Vec<Self>, Error>`
///
/// Retrieve all objects in the database belonging to an instance of
/// `*Remote*`.
///
/// `db` is the [database connection](../vicocomo/struct.DatabaseIf.html).
///
/// `remote` is the object on the remote side of the relationship.
///
/// `Ok(`*empty vector*`)` if there is no row in the database with the
/// `remote` primary key,
///
/// <b>Errors</b>
///
/// [`Error::Database`](../vicocomo/error/enum.Error.html#variant.Database)
/// return if there is some database error.
///
/// ##### `pub fn *name*(&self, db: DatabaseIf) -> Option<*Remote*>`
///
/// Retrive the object on the remote side of the relationship from the
/// database.
///
/// `db` is the [database connection](../vicocomo/struct.DatabaseIf.html).
///
/// A return value of `None` may be
/// - because the corresponding field in `self` is `None` (if the field is
///   an `Option`),
/// - because there is no row in the remote table with a primary key
///   matching the field value, or
/// - because of some other database error.
///
/// ##### `pub fn set_*name*(&mut self, remote: &*Remote*) -> Result<(), Error>`
///
/// Set the reference to an object on the remote side of the relationship.
///
/// `remote` is the object on the remote side of the relationship.
///
/// The new remote association is not saved to the database.
///
/// <b>Errors</b>
///
/// [`Error::Model` with `error: Invalid`
/// ](../vicocomo/error/enum.Error.html#variant.Model) return if the `remote`
/// primary key is not set.
///
/// ##### `pub fn *name*_siblings(&self, db: DatabaseIf) -> Result<Vec<Self>, Error>`
///
/// Retrive all owned objects in the database (including `self`) that
/// belong to the same object as `self`.
///
/// `db` is the [database connection](../vicocomo/struct.DatabaseIf.html).
///
/// ##### `pub fn forget_*name*(&mut self)`
/// *Defined only if the association field is an `Option`.*
///
/// Forget the reference to an object on the remote side of the
/// relationship.
///
/// The old reference is not removed from the database.
///
/// ### For each `vicocomo_has_many` struct attribute
///
/// Below, "`*Remote*`" means the `remote_type` value (or the default), and
/// "`*name*`" means the `name` value if given, or the last segment of
/// `remote_type` if not, snake cased.
///
/// ##### `pub fn *name*s(&self, db: DatabaseIf, filter: Option<&Query>) -> Result<Vec<*Remote*>, Error>`
///
/// Find items related to `self` by the association, filtered by `filter`.
///
/// `db` is the [database connection](../vicocomo/struct.DatabaseIf.html).
///
/// `filter`, see [`QueryBld`](model/struct.QueryBld.html). A condition to
/// select only among the associated objects is automatically added.
///
/// ##### `pub fn save_*name*s(&self, db: DatabaseIf, remotes: &[*Remote*]) -> Result<(), Error>`
///
/// Set and [`save()`
/// ](../vicocomo/active_record/trait.ActiveRecord.html#tymethod.save) the
/// associated model objects to `remotes` and handle errors and cascading.
///
/// - <b>Only if one-to-many</b>
///
///   Before saving, the `remote`s foreign key referring to `self` is set.
///
///   The handling of remote objects existing before the call with a primary
///   key that is not in `remotes` depends on the `on_delete` setting:
///   - `"cascade"`: deleted,
///   - `"forget"`: set to `NULL`,
///   - `"restrict"`: `Error::Model` error return.
///
/// - <b>Only if many-to-many</b>
///
///   NOT YET IMPLEMENTED!
/*
///
///   After saving, a join table row connecting `self` to `remote` is created
///   if it does not exist, and join table rows connecting `self` to
///   `*Remote*`s not in `remotes` are deleted. `*Remote*` objects are never
///   deleted in this case.
*/
///
/// <b>Errors</b>
///
/// Catches database errors depending on `self` not having a primary key or a
/// foreign key restriction and converts them to an `Error::Model`.
///
/// Forwards any other `Err(Error::Database)`.
///
/// #### Functions only for many-to-many associations
///
/// ##### `pub fn connect_to_*name*(&self, db: DatabaseIf, remote: &*Remote*) -> Result<usize, Error>`
///
/// Insert a join table row connecting `self` to `remote`. Returns `Ok(1)` on
/// success. Does *not* check that such a row did not exist previously!  It is
/// strongly recommended to create a unique index in the database to prevent
/// multiple connections between the same objects.
///
/// <b>Errors</b>
///
/// `model_error!(NotUnique, `*model name*`:, *name* `*camel cased*`"])` is
/// returned if there is a unique restriction error.
///
/// ##### `pub fn disconnect_from_*name*(&self, db: DatabaseIf, remote: &*Remote*) -> Result<usize, Error>`
///
/// Delete the join table row connecting `self` to `remote`. *Returns `Ok(0)`
/// if they are not connected*.
///
/// ### For each `vicocomo_unique` label
///
/// Given the struct declaration
/// ```text
/// #[derive(::vicocomo::ActiveRecord)]
/// struct ExampleStruct {
///     #[vicocomo_primary]
///     id: Option<u32>,
///     #[vicocomo_optional]
///     #[vicocomo_unique = "uni-lbl"]
///     un_1: Option<i32>,
///     #[vicocomo_unique = "uni-lbl"]
///     un_2: i32,
/// }
/// ```
/// also the following declarations are generated:
///
/// ##### `pub fn find_by_un1_and_un2(db: DatabaseIf, un_1: &i32, un_2: &i32)) -> Option<Self>`
///
/// Find an object in the database by the unique fields.
///
/// `db` is the [database connection](../vicocomo/struct.DatabaseIf.html).
///
/// `un_1` and `un_2` are the unique parameters. Note that `un_1` is
/// "unwrapped" because it is `vicocomo_optional`.
///
/// ##### `pub fn find_equal_un1_and_un2(&self, db: DatabaseIf) -> Option<Self>`
///
/// Find an object in the database that has the same values for the unique
/// fields as `self`.
///
/// `db` is the [database connection](../vicocomo/struct.DatabaseIf.html).
///
/// ### For each `vicocomo_validate_presence` attributed field
///
/// ##### `pub fn validate_presence_of_`*field name*`(&self) -> Result<Self, Error>`
///
/// Return an [`Error::Model`](../vicocomo/error/enum.Error.html#variant.Model)
/// if the field value is `None`.
///
#[proc_macro_derive(
    ActiveRecord,
    attributes(
        vicocomo_before_delete,
        vicocomo_before_save,
        vicocomo_belongs_to,
        vicocomo_column,
        vicocomo_db_value,
        vicocomo_has_many,
        vicocomo_optional,
        vicocomo_order_by,
        vicocomo_presence_validator,
        vicocomo_primary,
        vicocomo_required,
        vicocomo_table_name,
        vicocomo_unique,
    )
)]
pub fn active_record_derive(input: TokenStream) -> TokenStream {
    use ::quote::quote;
    use ::syn::ItemFn;

    let model = model::Model::new(input);
    let mut struct_fn: Vec<ItemFn> = Vec::new();
    let mut trait_fn: Vec<ItemFn> = Vec::new();

    belongs_to::belongs_to_impl(&model, &mut struct_fn, &mut trait_fn);
    common::common(&model, &mut struct_fn, &mut trait_fn);
    delete::delete_impl(&model, &mut struct_fn, &mut trait_fn);
    find::find_impl(&model, &mut struct_fn, &mut trait_fn);
    if !model.has_many.is_empty() {
        has_many::has_many_impl(&model, &mut struct_fn, &mut trait_fn);
    }
    save::save_impl(&model, &mut struct_fn, &mut trait_fn);

    let struct_id = &model.struct_id;
    let pk_type = &model.pk_type();

    quote!(
        impl #struct_id {
        #( #struct_fn )*
        }

        impl ::vicocomo::ActiveRecord for #struct_id {
            type PkType = #pk_type;

        #( #trait_fn )*
        }
    )
    .into()
}
