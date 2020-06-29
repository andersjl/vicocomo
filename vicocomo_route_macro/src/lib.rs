//! # actix-web helper macros

extern crate proc_macro;
extern crate syn;

use proc_macro::TokenStream;

mod configure;

/// Configure the application.
///
/// # Usage:
///
/// ```text
/// configure! {
/// // Route config, see below for the meaning of "Control" in route(Control)
///                          // HTTP | Actix URL            | ctrl | method
///                          // =====+======================+======+==========
///   route(Rsrc) {          // CRUD requests, only those given are generated
///   // Create request         -----+----------------------+------+----------
///     new_form,            // get  | "/rsrc/new"          | Rsrc | new_form
///     copy_form,           // get  | "/rsrc/{id}/copy"    | Rsrc | copy_form
///     create,              // post | "/rsrc"              | Rsrc | create
///     ensure,              // post | "/rsrc/ensure"       | Rsrc | ensure
///   // Read request           -----+----------------------+------+----------
///     index,               // get  | "/rsrc"              | Rsrc | index
///     show,                // get  | "/rsrc/{id}"         | Rsrc | show
///   // Update request         -----+----------------------+------+----------
///     edit_form,           // get  | "/rsrc/{id}/edit"    | Rsrc | edit_form
///     patch,               // post | "/rsrc/{id}"         | Rsrc | patch
///     replace,             // post | "/rsrc/{id}/replace" | Rsrc | replace
///   // Delete request         -----+----------------------+------+----------
///     delete,              // post | "/rsrc/{id}/delete"  | Rsrc | delete
///   },                     // =====+======================+======+==========
///   route(Cust) {          //   Methods may be customized |      |
///     custom {             // -----+----------------------+------+----------
///       http_method: get,  //   Order matters, omitted default if defined
///       path: "path",      // get  | "/path"              | Cust | custom
///   }},                    // =====+======================+======+==========
///   route(Sing) {          //   Example: configure a singleton resource
///     new_form,            // get  | "/sing/new"          | Sing | new_form
///     create,              // post | "/sing"              | Sing | create
///     ensure,              // post | "/sing/ensure"       | Sing | ensure
///     show                 //   full path must be given if leading slash
///     { path: "/sing" },   // get  | "/sing"              | Sing | show
///     edit_form            //   resource snake prepended if no leading slash
///     { path: "edit" },    // get  | "/sing/edit"         | Sing | edit_form
///     patch { path: "" },  // post | "/sing"              | Sing | patch
///     replace              //      |                      |      |
///     { path: "replace" }, // post | "/sing/replace"      | Sing | replace
///     delete               //      |                      |      |
///     { path: "delete" },  // post | "/sing/delete"       | Sing | delete
///   },                     // =====+======================+======+==========
///   route(Othr) {          //   Customized path parameters are given as
///     parm_req { path:     // {type} rather than {id} (those above are i32)
///       "some/{String}",   // get  | "/some/{p0}"         | Othr | parm_req
///     },                   // -----+----------------------+------+----------
///     post_req {           //   Except for the standardized CRUD requests
///       http_method: post, // above get is the default HTTP method
///       path: "postpth",   // post | "/postpth"           | Othr | post_req
///   }},                    // =====+======================+======+==========
/// // Not Found handler     //      |                      |      |
///   notfnd(Hand) { func }, // all not handled elsewhere   | Hand | func
/// }                        // default a simple 404 with text body
/// ```
///
///  Definition of "Controller" in `route(Controller)` and
///  `notfnd(Controller)`:
///
///  The controller is given as `some::path::to::controller`.  If the path has
///  only one segment, as in the examples, `crate::controllers::` is
///  prepended.
///
///  The handling method is called as
///  `some::path::to::controller::handler(...)`.  So the controller may be a
///  module, struct, or enum as long as the handling method does not have a
///  receiver.  In the struct/enum case it would probably be a constructor.
///
///  route handling method signature:
///  ```text
///  (
///    &mut impl vicocomo::DbConn<'a>,  // database connection
///    actix_session::Session,          // session object
///    std::sync::Arc<handlebars::Handlebars>,  // the template engine
///    String,                          // request body
///    path parameter type, ...         // as many as there are path parameters
///  ) -> actix-web::HttpResponse
/// ```
///
///  `notfnd` handling method signature:
///
///  ```text
///  (
///    method: &actix_web::http::Method,
///    uri: &actix_web::http::uri::Uri,
///  ) -> actix-web::HttpResponse
/// ```
///
#[proc_macro]
pub fn configure(input: TokenStream) -> TokenStream {
    configure::configure_impl(input)
}
