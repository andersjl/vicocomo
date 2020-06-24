extern crate proc_macro;
extern crate syn;

use proc_macro::TokenStream;

mod presenter;
mod utils;

// view helper macros --------------------------------------------------------

// Create an actix-web handler that serves a handlebars template with data
// from a vicocomo presenter.  Usage:
//
// presenter! {
//   name: my_name                  // mandatory handler name
// [ , method; my_method         ]  // optional method, default get
// [ , path; "/my/{my_par}/path" ]  // optional path, default "/my_name"
// [ , presenter; my_presenter   ]  // optional presenter, default my_name
// [ , template; "my_template"   ]  // optional template, default "my_name"
// [ ,                           ]  // optional trailing comma
// }
//
// Though they are named for clarity, the optional args must be ordered.
//
#[proc_macro]
pub fn presenter(input: TokenStream) -> TokenStream {
    presenter::presenter_impl(input)
}

