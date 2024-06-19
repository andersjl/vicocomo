use crate::model::{Model, OnDelete, OnNone, UniqueFieldSet};
use ::syn::{Ident, ItemFn, LitInt};

#[allow(non_snake_case)]
pub(crate) fn common(
    model: &Model,
    struct_fn: &mut Vec<ItemFn>,
    trait_fn: &mut Vec<ItemFn>,
) {
    use ::proc_macro2::Span;
    use ::syn::{parse_quote, LitStr};

    if model.pk_fields().is_empty() {
        trait_fn.push(parse_quote!(
            fn pk_value(&self) -> Option<Self::PkType> {
                None
            }
        ));
    } else {
        let pk_value_expr__self = model.pk_value(parse_quote!(self));
        trait_fn.push(parse_quote!(
            fn pk_value(&self) -> Option<Self::PkType> {
                #pk_value_expr__self
            }
        ));
    }

    // --- private ----------------------------------------------------------

    let struct_lit =
        LitStr::new(&model.struct_id.to_string(), Span::call_site());

    // --- __vicocomo__first_that_has_children

    let mut assoc_lit = Vec::new();
    let mut child_type = Vec::new();
    let mut filter = Vec::new();
    for restr in model.has_many.as_slice() {
        if restr.on_delete == OnDelete::Restrict {
            assoc_lit.push(LitStr::new(&restr.assoc_name, Span::call_site()));
            child_type.push(&restr.remote_type);
            filter.push(LitStr::new(
                &format!("{} = $1", &restr.remote_fk_col),
                Span::call_site(),
            ));
        }
    }

    struct_fn.push(parse_quote!(
        fn __vicocomo__first_that_has_children(
            db: ::vicocomo::DatabaseIf,
            pk: <Self as ::vicocomo::ActiveRecord>::PkType,
        ) -> Option<String> {
            use ::vicocomo::ActiveRecord;

            let mut result = None;
        #(
            if result.is_none() {
                if let Ok(found) = #child_type::query(
                    db.clone(),
                    ::vicocomo::QueryBld::new()
                        .filter(#filter, &[Some(pk.clone().into())])
                        .query()
                        .as_ref()
                        .unwrap(),
                ) {
                    if !found.is_empty() {
                        result = Some(#assoc_lit.to_string());
                    }
                }
            }
        )*
            result
        }
    ));

    // --- __vicocomo__conv_save_error

    use ::syn::Expr;

    let foreign_key_violation_conversion: Expr = if model
        .belongs_to_fields()
        .is_empty()
    {
        parse_quote!(())
    } else {
        let mut fk_expr: Vec<Expr> = Vec::new();
        let mut fk_lit = Vec::new();
        let mut rem_type = Vec::new();
        for bel_fld in model.belongs_to_fields() {
            let fk_id = &bel_fld.id;
            let value = parse_quote!(self.#fk_id);
            fk_expr.push(model.field_value_expr(bel_fld, value));
            fk_lit.push(LitStr::new(&fk_id.to_string(), Span::call_site()));
            rem_type.push(bel_fld.fk.as_ref().unwrap().remote_type.clone());
        }
        parse_quote!(
            if err.is_foreign_key_violation() {
            #(
                if let Some(fk) = #fk_expr {
                    if #rem_type::find(db.clone(), fk).is_none() {
                        return Some(::vicocomo::Error::Model(
                            ::vicocomo::ModelError {
                                error: ::vicocomo::ModelErrorKind::CannotSave,
                                model: #struct_lit.to_string(),
                                general: Some("foreign-key-violation".to_string()),
                                field_errors: vec![(#fk_lit.to_string(), Vec::new())],
                                assoc_errors: Vec::new(),
                            }
                        ));
                    }
                }
            )*
            }
        )
    };

    let pk_exists_conversion: Expr = if model.pk_fields().is_empty() {
        parse_quote!(())
    } else {
        parse_quote!(if !update {
            if let Some(pk_val) = self.pk_value() {
                if Self::find(db.clone(), &pk_val).is_some() {
                    return Some(Self::__vicocomo__pk_error(
                        ::vicocomo::ModelErrorKind::CannotSave,
                        self.pk_value(),
                        update,
                    ));
                }
            }
        })
    };

    let unique_fields_conversion: Expr = if model.uniques.is_empty() {
        parse_quote!(())
    } else {
        let mut find_by: Vec<&Ident> = Vec::new();
        let mut find_args: Vec<Vec<Expr>> = Vec::new();
        let mut opt_id = Vec::new();
        let mut opt_lit = Vec::new();
        let mut uni_lit = Vec::new();
        for UniqueFieldSet {
            fields,
            find_by_id,
            find_eq_id: _,
            find_self_args,
        } in &model.uniques
        {
            find_args.push(find_self_args.clone());
            find_by.push(&find_by_id);
            let mut these_opt_ids = Vec::new();
            let mut these_opt_lits = Vec::new();
            let mut these_uni_lits = Vec::new();
            for field in fields {
                let lit =
                    LitStr::new(&field.id.to_string(), Span::call_site());
                these_uni_lits.push(lit.clone());
                if field.onn != OnNone::Null {
                    these_opt_ids.push(field.id.clone());
                    these_opt_lits.push(lit);
                }
            }
            opt_id.push(these_opt_ids);
            opt_lit.push(these_opt_lits);
            uni_lit.push(these_uni_lits);
        }
        parse_quote!({
            let mut missing_opt_fields: Vec<&'static str> = Vec::new();
            let mut opt_none: Vec<&'static str> = Vec::new();
        #(
          #(
              if self.#opt_id.is_none() {
                  opt_none.push(#opt_lit);
              }
          )*
            if opt_none.is_empty() {
                if let Some(exist) = Self::#find_by( #( #find_args) ,* ) {
                    if !update || exist.pk_value() != self.pk_value() {
                        return Some(::vicocomo::Error::Model(
                            ::vicocomo::ModelError {
                                error: ::vicocomo::ModelErrorKind::CannotSave,
                                model: #struct_lit.to_string(),
                                general:
                                    Some("unique-violation".to_string()),
                                field_errors: vec![
                                #( (#uni_lit.to_string(), Vec::new()) ),*
                                ],
                                assoc_errors: Vec::new(),
                            }
                        ));
                    }
                }
            } else {
                missing_opt_fields.extend(opt_none.drain(..));
            }
        )*
            if !missing_opt_fields.is_empty() {
                return Some(::vicocomo::Error::Model(
                    ::vicocomo::ModelError {
                        error: ::vicocomo::ModelErrorKind::CannotSave,
                        model: #struct_lit.to_string(),
                        general: Some("unique-violation".to_string()),
                        field_errors: missing_opt_fields
                            .drain(..)
                            .map(|f| (f.to_string(), Vec::new()))
                            .collect(),
                        assoc_errors: Vec::new(),
                    }
                ));
            }
        })
    };

    let unique_violation_conversion: Expr =
        if model.pk_fields().is_empty() && model.uniques.is_empty() {
            parse_quote!(())
        } else {
            parse_quote!(
                if err.is_unique_violation() {
                    #pk_exists_conversion;
                    #unique_fields_conversion;
                }
            )
        };

    struct_fn.push(parse_quote!(
        #[doc(hidden)]
        fn __vicocomo__conv_save_error(
            &self,
            db: ::vicocomo::DatabaseIf,
            err: &::vicocomo::Error,
            update: bool,
        ) -> Option<::vicocomo::Error> {
            use ::vicocomo::ActiveRecord;
            #foreign_key_violation_conversion;
            #unique_violation_conversion;
            None
        }
    ));

    // --- __vicocomo__pk_error

    let mut pk_fld_lit = Vec::new();
    let mut pk_val_str_expr: Vec<Expr> = Vec::new();
    let tuple = model.pk_fields().len() > 1;
    for (ix, fld) in model.pk_fields().iter().enumerate() {
        pk_fld_lit.push(LitStr::new(&fld.id.to_string(), Span::call_site()));
        pk_val_str_expr.push(if tuple {
            let ix_lit = LitInt::new(&ix.to_string(), Span::call_site());
            parse_quote!(vals.#ix_lit.to_string())
        } else {
            parse_quote!(vals.to_string())
        });
    }

    // Return an Error::Model` with
    // - if `pk` is `None`: general text "missing-primary-key" and no field
    //   errors,
    // - if `pk` is `Some(_)`: One field error for each primary key field
    //   with the stringified value as error text, and
    //   - if `should_exist` is `true`: general text `"not-found"`,
    //   - if `should_exist` is `false`: general text `"unique-violation"`.
    struct_fn.push(parse_quote!(
        #[doc(hidden)]
        fn __vicocomo__pk_error(
            kind: ::vicocomo::ModelErrorKind,
            pk: Option<<Self as ::vicocomo::ActiveRecord>::PkType>,
            should_exist: bool,
        ) -> ::vicocomo::Error {
            ::vicocomo::Error::Model(::vicocomo::ModelError {
                error: kind,
                model: #struct_lit.to_string(),
                general: Some(
                    match pk {
                        Some(_) => {
                            if should_exist {
                                "not-found"
                            } else {
                                "unique-violation"
                            }
                        }
                        None => "missing-primary-key",
                    }
                    .to_string()
                ),
                field_errors: match pk {
                    Some(vals) => {
                        vec![ #( (#pk_fld_lit.to_string(), Vec::new()) ),* ]
                    }
                    None => Vec::new(),
                },
                assoc_errors: Vec::new(),
            })
        }
    ));
}
