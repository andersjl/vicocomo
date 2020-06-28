use crate::model::Model;
use proc_macro::TokenStream;

#[allow(unused_variables)]
pub fn delete_model_impl(model: &Model) -> TokenStream {
    use quote::quote;
    use syn::{export::Span, parse_quote, LitStr};
    let struct_id = &model.struct_id;
    let all_pk_fields = &model.all_pk_fields;
    let pk_mand_fields = &model.pk_mand_fields;
    let pk_mand_cols = &model.pk_mand_cols;
    let pk_opt_fields = &model.pk_opt_fields;
    let pk_opt_field_names = &model.pk_opt_field_names;
    let pk_opt_cols = &model.pk_opt_cols;
    let pk_len = all_pk_fields.len();
    let pk_type = &model.pk_type;
    let batch_expr = model.pk_batch_expr("batch");
    let self_sql = LitStr::new(
        format!(
            // "DELETE FROM table WHERE pk1 = $1 AND pk2 = $2",
            "DELETE FROM {} WHERE {}",
            &model.table_name,
            &model
                .all_pk_cols
                .iter()
                .enumerate()
                .map(|(ix, col)| format!("{} = ${}", col, ix + 1))
                .collect::<Vec<_>>()
                .join(" AND ")
        )
        .as_str(),
        Span::call_site(),
    );
    let batch_sql_format = LitStr::new(
        format!(
            // "DELETE FROM table WHERE (pk1, pk2) IN (($1, $2), ($3, $4))"
            "DELETE FROM {} WHERE ({}) IN ({{}})",
            &model.table_name,
            &model.all_pk_cols.join(", "),
        )
        .as_str(),
        Span::call_site(),
    );
    let delete_err = Model::query_err("delete");
    let batch_placeholders = Model::placeholders_expr(
        parse_quote!(batch.len()),
        parse_quote!(#pk_len),
    );
    let gen = quote! {
        impl<'a> vicocomo::MdlDelete<'a, #pk_type> for #struct_id {
            fn delete(self, db: &mut impl vicocomo::DbConn<'a>)
                -> Result<usize, vicocomo::Error>
            {
                let deleted = db.exec(
                    #self_sql,
                    &[ #( self.#all_pk_fields.into() ),* ],
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
