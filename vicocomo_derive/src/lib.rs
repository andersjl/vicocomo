extern crate proc_macro;
#[macro_use]
extern crate syn;

use proc_macro::TokenStream;

mod create;
mod delete;
mod model;
mod query;

#[proc_macro_derive(CreateModel, attributes(new_struct))]
pub fn create_model_derive(input: TokenStream) -> TokenStream {
    create::generate_create_model_impl(&model::Model::new(
        input,
        vec![model::ModelField::NewStruct],
    ))
}

#[proc_macro_derive(DeleteModel)]
pub fn delete_model_derive(input: TokenStream) -> TokenStream {
    delete::generate_delete_model_impl(&model::Model::new(
        input,
        vec![model::ModelField::PkParam],
    ))
}

#[proc_macro_derive(QueryModel, attributes(order_by, table_name, unique))]
pub fn query_model_derive(input: TokenStream) -> TokenStream {
    query::generate_query_model_impl(&model::Model::new(
        input,
        vec![
            model::ModelField::OrderFields,
            model::ModelField::UniqueFields,
        ],
    ))
}
