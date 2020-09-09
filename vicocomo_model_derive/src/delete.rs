use crate::model::{HasMany, Model};
use proc_macro::TokenStream;

#[allow(unused_variables)]
pub(crate) fn delete_impl(model: &Model) -> TokenStream {
    use crate::model::OnDelete;
    use ::quote::{format_ident, quote};
    use ::syn::{export::Span, parse_quote, Expr, LitStr};

    let Model {
        ref struct_id,
        ref table_name,
        ref has_many,
        delete_errors,
        save_errors,
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
    let row_count_err = Model::row_count_err("delete");
    let batch_placeholders = Model::placeholders_expr(
        parse_quote!(batch.len()),
        parse_quote!(#pk_len),
    );
    let pk_ids = pk_fields.iter().map(|f| &f.id).collect::<Vec<_>>();
    let mut on_delete_expr: Vec<Expr> = Vec::new();
    let mut restrict_expr: Vec<Expr> = Vec::new();
    let delete_errors_expr: Expr = if *delete_errors {
        parse_quote!(self.errors_preventing_delete(db))
    } else {
        parse_quote!(Vec::new())
    };
    for assoc in has_many {
        let HasMany {
            ref assoc_name,
            ref assoc_snake,
            on_delete,
            ref remote_assoc,
            ref remote_fk_col,
            ref remote_type,
            ref many_to_many,
        } = assoc;
        let get_id = format_ident!("{}s", assoc_snake);
        match on_delete {
            OnDelete::Cascade => {
                on_delete_expr.push(parse_quote! {
                    self.#get_id(db, None)
                        .and_then(|objs| {
                            for mut obj in objs {
                                obj.delete(db)?;
                            }
                            Ok(())
                        })?
                });
            }
            OnDelete::Forget => {
                use ::case::CaseExt;
                let forget_id =
                    format_ident!("forget_{}", remote_assoc.to_snake(),);
                on_delete_expr.push(parse_quote! {
                    {
                        use ::vicocomo::Save;
                        self.#get_id(db, None)
                            .and_then(|objs| {
                                for mut obj in objs {
                                    obj.#forget_id()?;
                                    obj.save(db)?;
                                }
                                Ok(())
                            })?
                    }
                });
            }
            OnDelete::Restrict => {
                let assoc_snake_str =
                    LitStr::new(assoc_snake, Span::call_site());
                restrict_expr.push(parse_quote! {
                    if self.#get_id(db, None)
                        .map(|objs| !objs.is_empty())?
                    {
                        errors.push(format!(
                            "the HasMany association {} is not empty",
                            #assoc_snake_str,
                        ));
                    }
                });
                on_delete_expr.push(parse_quote! {
                    self.#get_id(db, None)
                        .and_then(|objs| {
                            if objs.is_empty() {
                                Ok(())
                            } else {
                                Err(::vicocomo::Error::database(
                                    "there are associated objects",
                                ))
                            }
                        })?
                });
            }
        }
    }
    let gen = quote! {
        impl ::vicocomo::Delete<#pk_type> for #struct_id {
            fn delete(
                self,
                db: &impl ::vicocomo::DbConn
            ) -> Result<usize, ::vicocomo::Error> {
                let mut errors: Vec<String> = #delete_errors_expr;
            #(  #restrict_expr; )*
                if !errors.is_empty() {
                    return Err(::vicocomo::Error::delete(&errors.join("\n")));
                }
            #(  #on_delete_expr; )*
                let deleted = db.exec(
                    #self_sql,
                    &[ #( self.#pk_ids.into() ),* ],
                )?;
                if 1 != deleted {
                    return Err(::vicocomo::Error::database(&format!(
                        #row_count_err,
                        deleted,
                        1,
                    )));
                }
                Ok(deleted)
            }

            fn delete_batch(
                db: &impl ::vicocomo::DbConn,
                batch: &[#pk_type],
            ) -> Result<usize, ::vicocomo::Error> {
                /*
                #batch_delete_expr
                check row count
                */
                Ok(db.exec(
                    &format!(#batch_sql_format, #batch_placeholders),
                    #batch_expr,
                )?)
            }
        }
    };
    gen.into()
}
