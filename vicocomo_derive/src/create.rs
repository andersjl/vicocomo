use crate::model::Model;
use proc_macro::TokenStream;

pub fn create_model_impl(model: &Model) -> TokenStream {
    use quote::quote;
    let table_id = &model.table_id;
    let struct_id = &model.struct_id;
    let new_struct_id = &model.new_struct_id.clone().unwrap();
    let gen = quote! {
        impl CreateModel<Connection, #new_struct_id> for #struct_id {
            fn create(
               db: &Connection,
               data: &#new_struct_id,
            ) -> diesel::result::QueryResult<Box<Self>> {
               use crate::schema::#table_id::dsl::*;
               use diesel::dsl::insert_into;
               use diesel::query_dsl::RunQueryDsl;
               Ok(Box::new(
                   insert_into(#table_id).values(data).get_result::<Self>(db)?,
               ))
            }
            fn create_batch(
               db: &Connection,
               data: &[#new_struct_id],
            ) -> diesel::result::QueryResult<usize> {
               use crate::schema::#table_id::dsl::*;
               use diesel::dsl::insert_into;
               use diesel::query_dsl::RunQueryDsl;
               Ok(insert_into(#table_id).values(data).execute(db)?)
            }
        }
    };
    gen.into()
}
