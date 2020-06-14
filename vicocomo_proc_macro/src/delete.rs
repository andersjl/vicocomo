use crate::model::Model;
use proc_macro::TokenStream;

#[allow(unused_variables)]
pub fn delete_model_impl(model: &Model) -> TokenStream {
    use crate::utils::*;
    use quote::quote;
    use syn::{export::Span, parse_quote, Expr, LitStr};
    //println!("Delete");
    let struct_id = &model.struct_id;
    let all_pk_fields = &model.all_pk_fields;
    let pk_mand_fields = &model.pk_mand_fields;
    let pk_mand_cols = &model.pk_mand_cols;
    let pk_opt_fields = &model.pk_opt_fields;
    let pk_opt_field_names = &model.pk_opt_field_names;
    let pk_opt_cols = &model.pk_opt_cols;
    let pk_len = all_pk_fields.len();
    let pk_type = &model.pk_type;
    let (self_expr, batch_expr): (Expr, Expr) = match pk_len {
        0 => panic!("missing primary key field"),
        1 => {
            let pk_field = &all_pk_fields[0];
            (
                parse_quote!(std::slice::from_ref(&self.#pk_field.into())),
                parse_quote!(
                    &batch.iter().map(|v| (*v).into()).collect::<Vec<_>>()[..]
                ),
            )
        }
        _ => {
            let ixs = (0..pk_len).map(|i| {
                syn::LitInt::new(i.to_string().as_str(), Span::call_site())
            });
            (
                parse_quote!( &[ #( self.#all_pk_fields.into()),* ] ),
                parse_quote!(
                    &batch
                        .iter()
                        .fold(
                            vec![],
                            |mut all_vals, pk| {
                                #( all_vals.push((*pk).#ixs.into()); )*
                                all_vals
                            }
                        )[..]
                ),
            )
        }
    };
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
    let delete_err = query_err("delete");
    let batch_placeholders =
        placeholders_expr(parse_quote!(batch.len()), parse_quote!(#pk_len));
    let pk_cols_params = pk_cols_params_expr(
        pk_mand_cols,
        pk_mand_fields,
        pk_opt_cols,
        pk_opt_field_names,
        pk_opt_fields,
    );
    /*
    let debug: Expr = parse_quote!(#pk_cols_params);
    println!("pk_cols_params: {}", debug_to_tokens(&debug));
    */
    let gen = quote! {
        impl<'a> vicocomo::MdlDelete<'a, #pk_type> for #struct_id {
            fn delete(self, db: &mut impl vicocomo::DbConn<'a>)
                -> Result<usize, vicocomo::Error>
            {
                let deleted = db.exec(#self_sql, #self_expr)?;
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
