use crate::model::Model;
use ::syn::ItemFn;

pub(crate) fn delete_impl(
    model: &Model,
    _struct_fn: &mut Vec<ItemFn>,
    trait_fn: &mut Vec<ItemFn>,
) {
    use ::syn::parse_quote;
    let pk_fields = model.pk_fields();
    let pk_len = pk_fields.len();

    if pk_len == 0 {
        trait_fn.push(parse_quote!(
            fn delete(
                mut self,
                db: ::vicocomo::DatabaseIf,
            ) -> Result<(), ::vicocomo::Error> {
                Err(::vicocomo::Error::other("not-available"))
            }
        ));
        trait_fn.push(parse_quote!(
            fn delete_batch(
                db: ::vicocomo::DatabaseIf,
                batch: &[Self::PkType],
            ) -> Result<usize, ::vicocomo::Error> {
                Err(::vicocomo::Error::other("not-available"))
            }
        ));
    } else {
        use ::proc_macro2::Span;
        use ::syn::{Expr, LitStr};

        let Model {
            struct_id,
            ref table_name,
            has_many: _,
            before_delete,
            before_save: _,
            fields: _,
            uniques: _,
        } = model;

        let batch_expr = model.pk_batch_expr("batch").unwrap();
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
        let batch_placeholders = Model::placeholders_expr(
            parse_quote!(batch.len()),
            parse_quote!(#pk_len),
        );
        let before_delete_expr: Expr = if *before_delete {
            parse_quote!({
                ::vicocomo::BeforeDelete::before_delete(&mut self, db)?
            })
        } else {
            parse_quote!(())
        };
        let struct_lit =
            LitStr::new(&struct_id.to_string(), Span::call_site());
        trait_fn.push(parse_quote!(
            fn delete(
                mut self,
                db: ::vicocomo::DatabaseIf,
            ) -> Result<(), ::vicocomo::Error> {
                match self.pk_value() {
                    Some(pk) => {
                        #before_delete_expr;
                        Self::delete_batch(db, &[pk]).map(|_| ())
                    }
                    None => Err(Self::__vicocomo__pk_error(
                        ::vicocomo::ModelErrorKind::CannotDelete,
                        None,
                        true,
                    )),
                }
            }
        ));
        trait_fn.push(parse_quote!(
            fn delete_batch(
                db: ::vicocomo::DatabaseIf,
                batch: &[Self::PkType],
            ) -> Result<usize, ::vicocomo::Error> {
                if batch.is_empty() {
                    return Ok(0);
                }
                match db.exec(
                    &format!(#batch_sql_format, #batch_placeholders),
                    #batch_expr,
                ) {
                    Ok(deleted_count) => {
                        if deleted_count == batch.len() {
                            Ok(deleted_count)
                        } else {
                            let mut missing_pk: Option<Self::PkType> = None;
                            for pk in batch {
                                if Self::find(db, pk).is_none() {
                                    missing_pk = Some(pk.clone());
                                    break;
                                }
                            };
                            Err(Self::__vicocomo__pk_error(
                                ::vicocomo::ModelErrorKind::CannotDelete,
                                missing_pk,
                                true,
                            ))
                        }
                    }
                    Err(err) => {
                        if err.is_foreign_key_violation() {
                            for pk in batch {
                                if let Some(assoc) =
                                    Self::__vicocomo__first_that_has_children(
                                        db,
                                        pk.clone(),
                                    )
                                {
                                    return Err(::vicocomo::Error::Model(
                                        ::vicocomo::ModelError {
                                            error: ::vicocomo::ModelErrorKind
                                                ::CannotDelete,
                                            model: #struct_lit.to_string(),
                                            general:
                                                Some(
                                                    "foreign-key-violation"
                                                        .to_string(),
                                                ),
                                            field_errors: Vec::new(),
                                            assoc_errors: vec![(
                                                assoc,
                                                vec!["restricted".to_string()],
                                            )],
                                        }
                                    ));
                                }
                            }
                        }
                        Err(err)
                    }
                }
            }
        ));
    }
}
