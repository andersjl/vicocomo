use crate::model::{Model, PkParam};
use proc_macro::TokenStream;

pub fn delete_model_impl(model: &Model) -> TokenStream {
    use quote::quote;
    let table_id = &model.table_id;
    let struct_id = &model.struct_id;
    let PkParam(pk_arg, pk_type) = &model.pk_param.clone().unwrap();
    let gen = quote! {
        impl DeleteModel<Connection, #pk_type> for #struct_id {
            fn delete(
                self,
                db: &Connection,
            ) -> diesel::result::QueryResult<usize> {
                use crate::schema::#table_id::dsl::*;
                use diesel::query_dsl::QueryDsl;
                use diesel::query_dsl::RunQueryDsl;
                Ok(diesel::delete(#table_id.find(#pk_arg)).execute(db)?)
            }
            fn delete_batch(
                db: &Connection,
                batch: &[#pk_type],
            ) -> diesel::result::QueryResult<usize> {
                use crate::schema::meals::dsl::*;
                use diesel::expression_methods::ExpressionMethods;
                use diesel::query_dsl::{QueryDsl, RunQueryDsl};
                Ok(diesel::delete(meals.filter(id.eq_any(batch))).execute(db)?)
            }
        }
    };
    gen.into()
}
