extern crate proc_macro;
extern crate syn;

use proc_macro::TokenStream;

mod configure;
mod create;
mod delete;
mod model;
mod path_tag;
//mod presenter;
mod query;
mod utils;

// actix-web helper macros ---------------------------------------------------
//
//  Configure the application. Usage:
//  configure! {
//  // Route config, see below for the meaning of "Control" in route(Control)
//                           // HTTP | Actix URL            | ctrl | method
//                           // =====+======================+======+==========
//    route(Rsrc) {          // CRUD requests, only those given are generated
//    // Create request         -----+----------------------+------+----------
//      new_form,            // get  | "/rsrc/new"          | Rsrc | new_form
//      copy_form,           // get  | "/rsrc/{id}/copy"    | Rsrc | copy_form
//      create,              // post | "/rsrc"              | Rsrc | create
//      ensure,              // post | "/rsrc/ensure"       | Rsrc | ensure
//    // Read request           -----+----------------------+------+----------
//      index,               // get  | "/rsrc"              | Rsrc | index
//      show,                // get  | "/rsrc/{id}"         | Rsrc | show
//    // Update request         -----+----------------------+------+----------
//      edit_form,           // get  | "/rsrc/{id}/edit"    | Rsrc | edit_form
//      patch,               // post | "/rsrc/{id}"         | Rsrc | patch
//      replace,             // post | "/rsrc/{id}/replace" | Rsrc | replace
//    // Delete request         -----+----------------------+------+----------
//      delete,              // post | "/rsrc/{id}/delete"  | Rsrc | delete
//    },                     // =====+======================+======+==========
//    route(Cust) {          //   Methods may be customized |      |
//      custom {             // -----+----------------------+------+----------
//        http_method: get,  //   Order matters, omitted default if defined
//        path: "path",      // get  | "/path"              | Cust | custom
//    }},                    // =====+======================+======+==========
//    route(Sing) {          //   Example: configure a singleton resource
//      new_form,            // get  | "/sing/new"          | Sing | new_form
//      create,              // post | "/sing"              | Sing | create
//      ensure,              // post | "/sing/ensure"       | Sing | ensure
//      show                 //   full path must be given if leading slash
//      { path: "/sing" },   // get  | "/sing"              | Sing | show
//      edit_form            //   resource snake prepended if no leading slash
//      { path: "edit" },    // get  | "/sing/edit"         | Sing | edit_form
//      patch { path: "" },  // post | "/sing"              | Sing | patch
//      replace              //      |                      |      |
//      { path: "replace" }, // post | "/sing/replace"      | Sing | replace
//      delete               //      |                      |      |
//      { path: "delete" },  // post | "/sing/delete"       | Sing | delete
//    },                     // =====+======================+======+==========
//    route(Othr) {          //   Customized path parameters are given as
//      parm_req { path:     // {type} rather than {id} (those above are i32)
//        "some/{String}",   // get  | "/some/{p0}"         | Othr | parm_req
//      },                   // -----+----------------------+------+----------
//      post_req {           //   Except for the standardized CRUD requests
//        http_method: post, // above get is the default HTTP method
//        path: "postpth",   // post | "/postpth"           | Othr | post_req
//    }},                    // =====+======================+======+==========
//  // Not Found handler     //      |                      |      |
//    notfnd(Hand) { func }, // all not handled elsewhere   | Hand | func
//  }                        // default a simple 404 with text body
//
//  Definition of "Controller" in route(Controller) and notfnd(Controller):
//  The controller is given as some::path::to::controller.  If the path has
//  only one segment, as in the examples, crate::controllers:: is prepended.
//
//  The handling method is called as some::path::to::controller::handler(...).
//  So the controller may be a module, struct, or enum as long as the handling
//  method does not have a receiver.  In the struct/enum case it would
//  probably be a constructor.
//
//  route handling method signature: (
//    &crate::Connection,       // database connection
//    actix_session::Session,   // session object
//    std::sync::Arc<handlebars::Handlebars>,   // the template engine
//    String,                   // request body
//    path parameter type, ...  // as many as there are path parameters
//  ) -> actix-web::HttpResponse
//
//  notfnd handling method signature: (
//    method: &actix_web::http::Method,
//    uri: &actix_web::http::uri::Uri,
//  ) -> actix-web::HttpResponse
//
#[proc_macro]
pub fn configure(input: TokenStream) -> TokenStream {
    configure::configure_impl(input)
}

// view helper macros --------------------------------------------------------

/*
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
*/

// Implement the PathTag and Display traits and a new(path: Option<&str>)
// function for a struct MyPathTag(vicocomo::html::HtmlTag).
//
#[proc_macro_derive(PathTag, attributes(path_tag_data, path_tag_attr))]
pub fn path_tag_derive(input: TokenStream) -> TokenStream {
    path_tag::path_tag_impl(input)
}

// model helper macros -------------------------------------------------------

#[proc_macro_derive(CreateModel, attributes(new_struct))]
pub fn create_model_derive(input: TokenStream) -> TokenStream {
    create::create_model_impl(&model::Model::new(
        input,
        vec![model::ModelField::NewStruct],
    ))
}

#[proc_macro_derive(DeleteModel)]
pub fn delete_model_derive(input: TokenStream) -> TokenStream {
    delete::delete_model_impl(&model::Model::new(
        input,
        vec![model::ModelField::PkParam],
    ))
}

#[proc_macro_derive(QueryModel, attributes(order_by, table_name, unique))]
pub fn query_model_derive(input: TokenStream) -> TokenStream {
    query::query_model_impl(&model::Model::new(
        input,
        vec![
            model::ModelField::OrderFields,
            model::ModelField::UniqueFields,
        ],
    ))
}
