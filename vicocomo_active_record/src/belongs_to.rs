use crate::model::{ForKey, Model};
use ::syn::ItemFn;

pub(crate) fn belongs_to_impl(
    model: &Model,
    struct_fn: &mut Vec<ItemFn>,
    _trait_fn: &mut Vec<ItemFn>,
) {
    use ::case::CaseExt;
    use ::proc_macro2::Span;
    use ::quote::format_ident;
    use ::syn::{parse_quote, Expr, LitStr};
    use ::vicocomo_derive_utils::*;

    let struct_id = &model.struct_id;
    let struct_lit = LitStr::new(&struct_id.to_string(), Span::call_site());

    for bel_fld in model.belongs_to_fields() {
        let fk = bel_fld.fk.as_ref().unwrap();
        let fk_id = &bel_fld.id;
        let ForKey {
            assoc_name,
            remote_pk,
            remote_pk_mand,
            remote_type,
        } = fk;
        let remote_pk_none_err_expr = Model::field_none_err_expr(
            &type_to_ident(remote_type).unwrap(),
            &remote_pk,
        );
        let par_filter = LitStr::new(
            &format!("{} = $1", bel_fld.col.value()),
            Span::call_site(),
        );
        let remote_pk_expr: Expr = if *remote_pk_mand {
            parse_quote!(remote.#remote_pk.clone().into())
        } else {
            parse_quote!(
                match remote.#remote_pk {
                    Some(ref pk) => pk.clone().into(),
                    None => return Err(#remote_pk_none_err_expr),
                }
            )
        };
        let fk_expr_err: Expr = if bel_fld.dbt.nul() {
            let fk_err_expr = Model::field_none_err_expr(&struct_id, &fk_id);
            parse_quote!(
                match self.#fk_id {
                    Some(ref fk) => fk.clone().into(),
                    None => return Err(#fk_err_expr),
                }
            )
        } else {
            parse_quote!(self.#fk_id.clone().into())
        };
        let fk_expr_opt: Expr = if bel_fld.dbt.nul() {
            parse_quote!(
                match self.#fk_id {
                    Some(ref fk) => fk,
                    None => return None,
                }
            )
        } else {
            parse_quote!(&self.#fk_id)
        };
        let set_fk_expr_err: Expr = {
            let remote_pk_val_expr: Expr = if bel_fld.dbt.nul() {
                parse_quote!(Some(pk))
            } else {
                parse_quote!(pk)
            };
            if *remote_pk_mand {
                parse_quote!({
                    let pk = remote.#remote_pk.clone();
                    self.#fk_id = #remote_pk_val_expr;
                })
            } else {
                let assoc_lit = LitStr::new(assoc_name, Span::call_site());
                parse_quote!(
                    match remote.#remote_pk {
                        Some(pk) => self.#fk_id = #remote_pk_val_expr,
                        None => return Err(::vicocomo::Error::Model(
                            ::vicocomo::ModelError {
                                error: ::vicocomo::ModelErrorKind::Invalid,
                                model: #struct_lit.to_string(),
                                general: None,
                                field_errors: Vec::new(),
                                assoc_errors: vec![(
                                    #assoc_lit.to_string(),
                                    vec!["missing-primary-key".to_string()],
                                )],
                            },
                        )),
                    }
                )
            }
        };
        let assoc_snake = assoc_name.to_snake();
        let all_belonging_to_id =
            format_ident!("all_belonging_to_{}", assoc_snake);
        let get_id = format_ident!("{}", assoc_snake);
        let set_id = format_ident!("set_{}", assoc_snake);
        let siblings_id = format_ident!("{}_siblings", assoc_snake);
        struct_fn.push(parse_quote!(
            pub fn #all_belonging_to_id(
                db: ::vicocomo::DatabaseIf,
                remote: &#remote_type
            ) -> Result<Vec<Self>, ::vicocomo::Error> {
                use ::vicocomo::ActiveRecord;
                Self::query(
                    db,
                    &::vicocomo::QueryBld::new()
                        .filter(#par_filter, &[Some(#remote_pk_expr)])
                        .query()
                        .unwrap(),
                )
            }
        ));
        struct_fn.push(parse_quote!(
            pub fn #get_id(&self, db: ::vicocomo::DatabaseIf)
                -> Option<#remote_type>
            {
                use ::vicocomo::ActiveRecord;
                #remote_type::find(db, #fk_expr_opt)
            }
        ));
        struct_fn.push(parse_quote!(
            pub fn #set_id(&mut self, remote: &#remote_type)
                -> Result<(), ::vicocomo::Error>
            {
                #set_fk_expr_err
                Ok(())
            }
        ));
        struct_fn.push(parse_quote!(
            pub fn #siblings_id(
                &mut self,
                db: ::vicocomo::DatabaseIf
            ) -> Result<Vec<Self>, ::vicocomo::Error> {
                use ::vicocomo::ActiveRecord;
                Self::query(
                    db,
                    &::vicocomo::QueryBld::new()
                        .filter(#par_filter, &[Some(#fk_expr_err)])
                        .query()
                        .unwrap(),
                )
            }
        ));
        if bel_fld.dbt.nul() {
            let forget_id = format_ident!("forget_{}", assoc_snake);
            struct_fn.push(parse_quote!(
                pub fn #forget_id(&mut self) {
                    self.#fk_id = None;
                }
            ));
        }
    }
}
