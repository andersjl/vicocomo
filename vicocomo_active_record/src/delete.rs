use crate::model::Model;
use proc_macro::TokenStream;

#[allow(unused_variables)]
pub(crate) fn delete_impl(model: &Model) -> TokenStream {
    use ::quote::quote;
    use ::syn::{export::Span, parse_quote, Expr, LitStr};

    let Model {
        ref struct_id,
        ref table_name,
        has_many,
        before_delete,
        before_save,
        ref fields,
    } = model;
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
            table_name,
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
            table_name,
            &pk_fields
                .iter()
                .map(|pk| pk.col.value())
                .collect::<Vec<_>>()
                .join(", "),
        )
        .as_str(),
        Span::call_site(),
    );
    let check_delete_count = Model::check_row_count_expr(
        "delete()",
        parse_quote!(deleted_count),
        parse_quote!(1),
    );
    let check_batch_count = Model::check_row_count_expr(
        "delete_batch()",
        parse_quote!(deleted_count),
        parse_quote!(batch.len()),
    );
    let batch_placeholders = Model::placeholders_expr(
        parse_quote!(batch.len()),
        parse_quote!(#pk_len),
    );
    let pk_ids = pk_fields.iter().map(|f| &f.id).collect::<Vec<_>>();
    let before_delete_expr: Expr = if *before_delete {
        parse_quote!(self.before_delete(db)?)
    } else {
        parse_quote!(())
    };
    let gen = quote! {
        impl ::vicocomo::Delete<#pk_type> for #struct_id {
            fn delete(
                self,
                db: &impl ::vicocomo::DbConn
            ) -> Result<usize, ::vicocomo::Error> {
                #before_delete_expr;
                let deleted_count = db.exec(
                    #self_sql,
                    &[ #( self.#pk_ids.into() ),* ],
                )?;
                #check_delete_count
                Ok(deleted_count)
            }

            fn delete_batch(
                db: &impl ::vicocomo::DbConn,
                batch: &[#pk_type],
            ) -> Result<usize, ::vicocomo::Error> {
                let deleted_count = db.exec(
                    &format!(#batch_sql_format, #batch_placeholders),
                    #batch_expr,
                )?;
                #check_batch_count
                Ok(deleted_count)
            }
        }
    };
    gen.into()
}
