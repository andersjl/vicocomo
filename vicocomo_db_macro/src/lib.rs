//! Database helper macros

extern crate proc_macro;
extern crate syn;

use proc_macro::TokenStream;

mod db_value_convert;


/// Convert types to and from vicocomo::DbValue.
///
/// # Usage
///
/// `db_value_convert { other_type, variant, [ into_expr, [ from_expr ] ] }`
///
/// Implements `TryInto<`*other_type*`>`, `From<`*other_type*`>` for
/// `DbValue::`*variant*, and `TryInto<Option<`*other_type*`>>`,
/// `From<Option<`*other_type*`>>` for `DbValue::Nul`*variant*.
///
/// When converting from `DbValue` we must have the correct variant, hence we
/// implement `TryInto` rather than `Into`.
///
/// *into_expr* should use the variable `value` for the value contained in the
/// `DbValue` variant and the type *other_type*.  A missing or empty into_expr
/// is taken as '`value as `*other type*'.
///
/// *from_expr* should use the variable `other` for the *other_type* value and
/// the type contained in the `DbValue` *variant*.  A missing or empty
/// *from_expr* is taken as '`other as `the type contained in the *variant*'.
///
/// # Panics
///
/// The implementations panic if the conversion expression panics.
///
#[proc_macro]
pub fn db_value_convert(input: TokenStream) -> TokenStream {
    db_value_convert::db_value_convert_impl(input)
}
