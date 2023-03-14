//! Html-related derive macros for `vicocomo` presenters.

use proc_macro::TokenStream;

mod html_form;
mod path_tag;

/// Derives [`vicocomo::HtmlForm`
/// ](../vicocomo/html/input/trait.HtmlForm.html).
///
/// ## Generated Code
///
/// ### Implementation of the trait
///
/// The implementation is straight-forward. `update()` simply calls the
/// `update()` method in each `HtmlInput` field.
///
/// See the [examples](../vicocomo/html/input/trait.HtmlForm.html#examples)
/// for details.
///
/// ### Constructors
///
/// ```text
/// pub fn new() -> Self
/// ```
/// Initialize the `HtmlInput` fields to have `InputType` variants as defined
/// by the derived struct's field types and attributes (see the [examples
/// ](../vicocomo/html/input/trait.HtmlForm.html#examples)), with value `None`
/// (or `vec![]` for multiple selection variants).
///
/// Initialize other fields to `None`.
///
/// ```text
/// pub fn with_labels(prepend: Option<&str>) -> Self
/// ```
/// Initialize like `new()`, and also generate labels. The generated labels
/// are *prepend*`--`*form id*`--`*field name*.
///
/// Note that [`to_json`
/// ](../vicocomo/html/input/trait.HtmlForm.html#tymethod.to_json) localizes
/// the labels.
///
/// ### Restrictions imposed by the derive macro
///
/// The item that derives `HtmlForm` must be a `struct` with named fields.
///
/// It must have one field declared exactly (modulo visibility) as
/// ```
/// errors: Vec<String>,
/// ```
/// The fields in the struct that correspond to HTML form input elements
/// should have type [`HtmlInput`
/// ](../vicocomo/html/input/struct.HtmlInput.html). Use the `HtmlInput`
/// methods to configure the inputs and set and read the values.
///
/// <small>It is technically possible, using the `HtmlInput` constructors, to
/// change the `InputType` variant of a field by reinitializing it. It is hard
/// to see a reasonable use case for that.</small>
///
/// Other fields must have the type `Option<_>`, see [above](#constructors).
///
#[proc_macro_derive(HtmlForm, attributes(vicocomo_html_input_type))]
pub fn html_form(input: TokenStream) -> TokenStream {
    html_form::html_form_impl(input)
}

/// Implement the [`PathTag`](../vicocomo/html/utils/trait.PathTag.html)
/// and `Display` traits and a constructor for a struct.
///
#[proc_macro_derive(
    PathTag,
    attributes(vicocomo_path_tag_data, vicocomo_path_tag_attr)
)]
pub fn path_tag_derive(input: TokenStream) -> TokenStream {
    path_tag::path_tag_impl(input)
}
