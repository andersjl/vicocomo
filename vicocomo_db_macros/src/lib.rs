//! Database helper macros

use proc_macro::TokenStream;

mod db_value_convert;

/// Convert types to and from [`vicocomo::DbValue`
/// ](database/enum.DbValue.html).
///
/// # Usage
///
/// `db_value_convert {` *other-type*`,` *variant*`, [` *from-db-expr*`, [`
/// *into-db-expr* `] ] }`
///
/// Defines a type `pub struct Opt`*other-type*`(Option<`*other-type*`>)` in
/// the module where the macro is invoked.
///
/// Implements `TryFrom<DbValue>` and `Into<DbValue>` for *other-type* and
/// `Opt`*other-type*.
///
/// When converting from `DbValue` we must have the correct variant, hence we
/// implement `TryFrom` rather than `From`.
///
/// *from-db-expr* should use the variable `value` for the value contained in
/// the `DbValue` variant and the type *other_type*.  A missing or empty
/// *from-db-expr* is taken as '`value as `*other-type*'.
///
/// *into-db-expr* should use the variable `other` for the *other_type* value
/// and the type contained in the `DbValue` *variant*.  A missing or empty
/// *into-db-expr* is taken as '`other as `*the type contained in the
/// variant*'.
///
/// # Panics
///
/// The implementations panic if the conversion expression panics.
///
/// # For `vicocomo` maintainers only
///
/// In the module [`::vicocomo::database`](database/index.html), use the macro
/// variant
///
/// `db_value_convert { no_option_type, `*other-type*`,` *variant*`, [
/// `*from-db-expr*`, [ `*into-db-expr* `] ] }`
///
/// to add pre-defined conversions for new types.  This implements
/// `TryFrom<DbValue>` and `Into<DbValue>` directly for
/// `Option<`*other-type*`>` and does *not* define `Opt`*other-type*.
///
#[proc_macro]
pub fn db_value_convert(input: TokenStream) -> TokenStream {
    db_value_convert::db_value_convert_impl(input)
}
