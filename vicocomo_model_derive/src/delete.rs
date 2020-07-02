use crate::model::Model;
use proc_macro::TokenStream;

#[allow(unused_variables)]
pub fn delete_model_impl(model: &Model) -> TokenStream {
    use quote::quote;
    use syn::{export::Span, parse_quote, LitStr};
    let struct_id = &model.struct_id;
    let pk_fields = model.pk_fields();
    let pk_field_names: Vec<String> =
        pk_fields.iter().map(|f| f.id.to_string()).collect();
    let pk_len = pk_fields.len();
    let pk_type = &model.pk_type();
    let batch_expr = model.pk_batch_expr("batch");
    let self_sql = LitStr::new(
        &format!(
            // "DELETE FROM table WHERE pk1 = $1 AND pk2 = $2",
            "DELETE FROM {} WHERE {}",
            &model.table_name,
            &pk_fields
                .iter()
                .enumerate()
                .map(|(ix, pk)| format!("{} = ${}", pk.col.value(), ix + 1))
                .collect::<Vec<_>>()
                .join(" AND ")
        ),
        Span::call_site(),
    );
    let batch_sql_format = LitStr::new(
        format!(
            // "DELETE FROM table WHERE (pk1, pk2) IN (($1, $2), ($3, $4))"
            "DELETE FROM {} WHERE ({}) IN ({{}})",
            &model.table_name,
            &pk_fields
                .iter()
                .map(|pk| pk.col.value())
                .collect::<Vec<_>>()
                .join(", "),
        )
        .as_str(),
        Span::call_site(),
    );
    let delete_err = Model::query_err("delete");
    let batch_placeholders = Model::placeholders_expr(
        parse_quote!(batch.len()),
        parse_quote!(#pk_len),
    );
    let pk_ids = pk_fields.iter().map(|f| &f.id).collect::<Vec<_>>();
    let gen = quote! {
        impl<'a> vicocomo::MdlDelete<'a, #pk_type> for #struct_id {
            fn delete(self, db: &mut impl vicocomo::DbConn<'a>)
                -> Result<usize, vicocomo::Error>
            {
                let deleted = db.exec(
                    #self_sql,
                    &[ #( self.#pk_ids.into() ),* ],
                )?;
                if 1 != deleted {
                    return Err(vicocomo::Error::Database(format!(
                        #delete_err,
                        deleted,
                        1,
                    )));
                }
                Ok(deleted)
            }

            fn delete_batch(
                db: &mut impl vicocomo::DbConn<'a>,
                batch: &[#pk_type],
            ) -> Result<usize, vicocomo::Error> {
                Ok(db.exec(
                    &format!(#batch_sql_format, #batch_placeholders),
                    #batch_expr,
                )?)
            }
        }
    };
    gen.into()
}