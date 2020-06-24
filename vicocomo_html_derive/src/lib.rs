extern crate proc_macro;
extern crate syn;

use proc_macro::TokenStream;

mod path_tag;
//mod utils;

// Implement the PathTag and Display traits and a new(path: Option<&str>)
// function for a struct MyPathTag(vicocomo::html::HtmlTag).
//
#[proc_macro_derive(
    PathTag,
    attributes(vicocomo_path_tag_data, vicocomo_path_tag_attr)
)]
pub fn path_tag_derive(input: TokenStream) -> TokenStream {
    path_tag::path_tag_impl(input)
}
