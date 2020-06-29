//! # Model helper macros
//!
//! ```text
//! #[derive(<any combination of the below>)]
//! #[vicoomo_table_name = "example_table"]  // default "examples"
//! struct Example {
//!     #[vicocomo_optional]       // not sent to DBMS if None
//!     #[vicocomo_primary]        // To find a row to update() or delete()
//!     primary: Option<u32>,      // primary key should be ensured by DBMS
//!     #[vicocomo_column = "db_col"]  // different name of DB column
//!     #[vicocomo_unique = "un1"] // "un1" labels fields w unique comb.
//!     not_null: String,          // VARCHAR NOT NULL
//!     #[vicocomo_order_by(2)]    // precedence 2, see opt_null below
//!     nullable: Option<String>,  // VARCHAR, None -> NULL
//!     #[vicocomo_optional]       // not sent to DBMS if None
//!     #[vicocomo_unique = "un1"] // UNIQUE(db_col, opt_not_null)
//!     opt_not_null: Option<i32>  // INTEGER NOT NULL DEFAULT 42
//!     #[vicocomo_order_by(1, "desc")] // ORDER BY opt_null DESC, nullable
//!     #[vicocomo_optional]       // not sent to DBMS if None
//!     opt_null: Option<Option<i32>>  // INTEGER DEFAULT 43
//!                                // None -> 43, Some(None) -> NULL
//! }
//! ```
extern crate proc_macro;
extern crate syn;

use proc_macro::TokenStream;

mod delete;
mod find;
mod model;
mod save;

/// Derive the [`MdlDelete`](../vicocomo/trait.MdlDelete.html) trait for a
/// `struct` with named fields.
///
/// ## Struct attributes
///
/// See this [example](../vicocomo_model_derive/index.html).
///
/// `vicocomo_table_name = "`some table name`"` - The database table storing
/// the struct.  Default the snake cased struct name with a plural 's'.
///
/// ## Field attributes
///
/// See this [example](../vicocomo_model_derive/index.html).
///
/// `vicocomo_column = "`column name`"` - The database column storing the
/// field.  Default the snake cased field name.
///
/// `vicocomo_optional` - The field should be a Rust `Option`, and a `None`
/// value is never sent to the database.  Intended for values that may be
/// generated by the database when missing.
///
/// `vicocomo_primary` - The field corresponds to a primary key in the
/// database.
///
/// `vicocomo_unique = "`a label`"` - The tuple of fields whith the same label
/// should be unique in the database.  Primary keys do not need this.
///
/// ## Generated code
///
/// Implements [`MdlDelete`](../vicocomo/trait.MdlDelete.html).
///
#[proc_macro_derive(
    DeleteModel,
    attributes(
        vicocomo_column,
        vicocomo_optional,
        vicocomo_primary,
        vicocomo_table_name,
        vicocomo_unique,
    )
)]
pub fn delete_model_derive(input: TokenStream) -> TokenStream {
    delete::delete_model_impl(&model::Model::new(
        input,
        vec![model::ExtraInfo::UniqueFields],
    ))
}

/// Derive the [`MdlFind`](../vicocomo/trait.MdlFind.html) trait for a
/// `struct` with named fields.
///
/// ## Struct attributes
///
/// See this [example](../vicocomo_model_derive/index.html).
///
/// `vicocomo_table_name = "`some table name`"` - The database table storing
/// the struct.  Default the snake cased struct name with a plural 's'.
///
/// ## Field attributes
///
/// See this [example](../vicocomo_model_derive/index.html).
///
/// `vicocomo_column = "`column name`"` - The database column storing the
/// field.  Default the snake cased field name.
///
/// `vicocomo_optional` - The field should be a Rust `Option`, and a `None`
/// value is never sent to the database.  Intended for values that may be
/// generated by the database when missing.
///
/// `vicocomo_order_by(`priority`[`, "`direction`"`]`)` - Defines a default
/// ordering when retrieving model objects.  Direction is optional and either
/// `ASC` or `DESC`.
///
/// `vicocomo_primary` - The field corresponds to a primary key in the
/// database.
///
/// `vicocomo_unique = "`a label`"` - The tuple of fields whith the same label
/// should be unique in the database.  Primary keys do not need this.
///
/// ## Generated code
///
/// Implements [`MdlFind`](../vicocomo/derive.MdlFind.html).
///
/// ### For each `vicocomo_unique` label
///
/// Given the struct declaration
/// ```text
/// #[derive(vicocomo::FindModel)]
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
///     db: &mut impl vicocomo::DbConn<'a>,
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
///     db: &mut impl vicocomo::DbConn<'a>
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
///     db: &mut impl vicocomo::DbConn<'a>,
///     un1: Option<i32>,
///     un2: i32,
///     msg: &str,
/// ) -> Result<(), vicocomo::Error> {
///
#[proc_macro_derive(
    FindModel,
    attributes(
        vicocomo_column,
        vicocomo_optional,
        vicocomo_order_by,
        vicocomo_primary,
        vicocomo_table_name,
        vicocomo_unique
    )
)]
pub fn find_model_derive(input: TokenStream) -> TokenStream {
    find::find_model_impl(&model::Model::new(
        input,
        vec![
            model::ExtraInfo::OrderFields,
            model::ExtraInfo::UniqueFields,
            model::ExtraInfo::DatabaseTypes,
        ],
    ))
}

/// Derive the [`MdlSave`](../vicocomo/trait.MdlSave.html) trait for a
/// `struct` with named fields.
///
/// ## Struct attributes
///
/// See this [example](../vicocomo_model_derive/index.html).
///
/// `vicocomo_table_name = "`some table name`"` - The database table storing
/// the struct.  Default the snake cased struct name with a plural 's'.
///
/// ## Field attributes
///
/// See this [example](../vicocomo_model_derive/index.html).
///
/// `vicocomo_column = "`column name`"` - The database column storing the
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
#[proc_macro_derive(
    SaveModel,
    attributes(
        vicocomo_column,
        vicocomo_optional,
        vicocomo_primary,
        vicocomo_table_name
    )
)]
pub fn save_model_derive(input: TokenStream) -> TokenStream {
    save::save_model_impl(&model::Model::new(
        input,
        vec![model::ExtraInfo::DatabaseTypes],
    ))
}
