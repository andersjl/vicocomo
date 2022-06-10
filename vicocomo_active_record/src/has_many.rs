use crate::model::{HasMany, Model};
use ::syn::ItemFn;
use ::vicocomo_derive_utils::*;

pub(crate) fn has_many_impl(
    model: &Model,
    struct_fn: &mut Vec<ItemFn>,
    _trait_fn: &mut Vec<ItemFn>,
) {
    use ::case::CaseExt;
    use ::proc_macro2::Span;
    use ::quote::format_ident;
    use ::syn::{parse_quote, Expr, LitStr};

    let struct_id = &model.struct_id;
    let struct_lit = LitStr::new(&struct_id.to_string(), Span::call_site());
    let pk = model.pk_fields();
    assert!(pk.len() == 1, "HasMany requires exactly one primary key");
    let pk = pk[0];

    for has_many in &model.has_many {
        let HasMany {
            ref assoc_name,
            on_delete: _,
            remote_assoc,
            ref remote_fk_col,
            ref remote_type,
            ref many_to_many,
        } = has_many;
        let assoc_lit = LitStr::new(assoc_name, Span::call_site());
        let mut join_table_name = String::new();
        let mut join_fk_col = String::new();
        let mut remote_pk = format_ident!("dummy");
        let mut remote_pk_mand = false;
        let mut remote_pk_col = String::new();
        match many_to_many {
            Some(mtm) => {
                join_table_name = mtm.join_table_name.clone();
                join_fk_col = mtm.join_fk_col.clone();
                remote_pk = mtm.remote_pk.clone();
                remote_pk_mand = mtm.remote_pk_mand;
                remote_pk_col = mtm.remote_pk_col.clone();
            }
            None => (),
        }
        let filter_assoc = LitStr::new(
            &if many_to_many.is_some() {
                format!(
                    "{} IN (SELECT {} FROM {} WHERE {} = $1)",
                    remote_pk_col,
                    join_fk_col,
                    join_table_name,
                    remote_fk_col,
                )
            } else {
                format!("{} = $1", remote_fk_col)
            },
            Span::call_site(),
        );
        let pk_id = &pk.id;
        let self_pk_none_err_expr =
            Model::field_none_err_expr(&struct_id, pk_id);
        let self_pk_expr: Expr = if pk.opt {
            parse_quote!(
                match self.#pk_id {
                    Some(pk) => pk,
                    None => {
                        return Err(#self_pk_none_err_expr);
                    }
                }
            )
        } else {
            parse_quote!(self.#pk_id)
        };
        let assoc_snake = assoc_name.to_snake();
        let connect_to_fn = format_ident!("connect_to_{}", assoc_snake);
        let disconnect_from_fn =
            format_ident!("disconnect_from_{}", assoc_snake);
        let get_fn = format_ident!("{}s", assoc_snake);
        let save_fn = format_ident!("save_{}s", assoc_snake);

        if many_to_many.is_some() {
            let remote_pk_expr: Expr = if remote_pk_mand {
                parse_quote!(remote.#remote_pk)
            } else {
                let remote_pk_none_err_expr = Model::field_none_err_expr(
                    &type_to_ident(remote_type).unwrap(),
                    &remote_pk,
                );
                parse_quote!(
                    match remote.#remote_pk {
                        Some(ref pk) => pk,
                        None => {
                            return Err(#remote_pk_none_err_expr);
                        }
                    }
                )
            };
            let join_col_vals_expr: Expr = parse_quote!(
                &[#self_pk_expr.clone().into(), #remote_pk_expr.clone().into()]
            );
            let connect_sql = format!(
                "INSERT INTO {} ({}, {}) VALUES ($1, $2)",
                join_table_name, remote_fk_col, join_fk_col,
            );
            let disconnect_sql = format!(
                "DELETE FROM {} WHERE {} = $1 AND {} = $2",
                join_table_name, remote_fk_col, join_fk_col,
            );

            struct_fn.push(parse_quote!(
                pub fn #connect_to_fn(
                    &self,
                    db: ::vicocomo::DatabaseIf,
                    remote: &#remote_type,
                ) -> Result<usize, ::vicocomo::Error> {
                    db.exec(#connect_sql, #join_col_vals_expr)
                }
            ));
            struct_fn.push(parse_quote!(
                pub fn #disconnect_from_fn(
                    &self,
                    db: ::vicocomo::DatabaseIf,
                    remote: &#remote_type,
                ) -> Result<usize, ::vicocomo::Error> {
                    db.exec(#disconnect_sql, #join_col_vals_expr)
                }
            ));
            struct_fn.push(parse_quote!(
                pub fn #save_fn(
                    &self,
                    db: ::vicocomo::DatabaseIf,
                    remotes: &[#remote_type],
                ) -> Result<(), ::vicocomo::Error> {
                    Err(::vicocomo::Error::nyi())
                }
            ));
        } else {
            let remote_set_fn =
                format_ident!("set_{}", remote_assoc.to_snake());
            let remote_all_fn =
                format_ident!("all_belonging_to_{}", remote_assoc.to_snake());
            struct_fn.push(parse_quote!(
                pub fn #save_fn(
                    &self,
                    db: ::vicocomo::DatabaseIf,
                    remotes: &[#remote_type],
                ) -> Result<(), ::vicocomo::Error> {
                    use ::vicocomo::ActiveRecord;

                    let mut to_be =
                        remotes.iter().cloned().collect::<Vec<_>>();
                    for r in &mut to_be {
                        r.#remote_set_fn(&self)?;
                    }
                    let mut existing =
                        #remote_type::#remote_all_fn(db, &self)?;
                    existing.sort_by(|o1, o2| {
                        o1.pk_value().cmp(&o2.pk_value())
                    });
                    to_be.sort_by(|o1, o2| {
                        o1.pk_value().cmp(&o2.pk_value())
                    });
                    // delete all obsolete associated objects before
                    // trying to save new ones, to enable the remote's
                    // before_save() to limit the number of associated
                    // objects.
                    let mut to_delete = Vec::new();
                    let mut to_try = Vec::new();
                    for tb in to_be.drain(..) {
                        while !existing.is_empty()
                            && existing.iter().next().unwrap().pk_value()
                                < tb.pk_value()
                        {
                            to_delete.push(
                                existing.drain(..1).next().unwrap(),
                            );
                        }
                        if !existing.is_empty()
                            && existing.iter().next().unwrap().pk_value()
                                == tb.pk_value()
                        {
                            existing.remove(0);
                        }
                        to_try.push(tb);
                    }
                    to_delete.append(&mut existing);
                    let mut delete_pks = Vec::new();
                    for td in to_delete {
                        let pkv = match td.pk_value() {
                            Some(val) => delete_pks.push(val),
                            None => return Err(
                                ::vicocomo::Error::this_cannot_happen(
                                    "missing-pk-value",
                                )),
                        };
                    }
                    if let Err(e) =
                        <#remote_type as ::vicocomo::ActiveRecord>::delete_batch(
                            db,
                            delete_pks.as_slice(),
                        )
                    {
                        return Err(::vicocomo::Error::Model(
                            ::vicocomo::ModelError {
                                error:
                                    ::vicocomo::ModelErrorKind::
                                        CannotDelete,
                                model: #struct_lit.to_string(),
                                general: None,
                                field_errors: Vec::new(),
                                assoc_errors: vec![(
                                    #assoc_lit.to_string(),
                                    vec![e.to_string()],
                                )],
                            }
                        ));
                    }
                    for ref mut tt in to_try {
                        if let Err(e) =
                            ::vicocomo::ActiveRecord::save(tt, db)
                        {
                            return Err(::vicocomo::Error::Model(
                                ::vicocomo::ModelError {
                                    error:
                                        ::vicocomo::ModelErrorKind::
                                            CannotSave,
                                    model: #struct_lit.to_string(),
                                    general: None,
                                    field_errors: Vec::new(),
                                    assoc_errors: vec![(
                                        #assoc_lit.to_string(),
                                        vec![e.to_string()],
                                    )],
                                }
                            ));
                        }
                    }
                    Ok(())
                }
            ));
        }

        struct_fn.push(parse_quote!(
            pub fn #get_fn(
                &self,
                db: ::vicocomo::DatabaseIf,
                filter: Option<&::vicocomo::Query>,
            ) -> Result<Vec<#remote_type>, ::vicocomo::Error> {
                let mut bld = match filter {
                    Some(f) => f.clone().builder(),
                    None => ::vicocomo::QueryBld::new(),
                };
                <#remote_type as ::vicocomo::ActiveRecord>::query(
                    db,
                    match bld.filter(
                            #filter_assoc,
                            &[Some(#self_pk_expr.clone().into())]
                        )
                        .query()
                        .as_ref()
                    {
                        Some(q) => q,
                        None => return Err(
                            ::vicocomo::Error::this_cannot_happen(
                                #filter_assoc,
                            )),
                    }
                )
            }
        ));
    }
}
