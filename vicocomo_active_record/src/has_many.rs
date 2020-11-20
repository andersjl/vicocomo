use crate::model::{HasMany, Model};
use ::proc_macro::TokenStream;
use ::vicocomo_derive_utils::*;

#[allow(unused_variables)]
pub(crate) fn has_many_impl(model: &Model) -> TokenStream {
    use ::quote::{format_ident, quote};
    use ::syn::{export::Span, parse_quote, Expr, LitStr};

    let struct_id = &model.struct_id;
    let table_name = &model.table_name;
    let pk = model.pk_fields();
    assert!(pk.len() == 1, "HasMany requires exactly one primary key");
    let pk = pk[0];

    let mut gen = proc_macro2::TokenStream::new();
    for has_many in &model.has_many {
        let HasMany {
            ref assoc_name,
            ref assoc_snake,
            on_delete,
            ref remote_assoc,
            ref remote_fk_col,
            ref remote_type,
            ref many_to_many,
        } = has_many;
        let mut join_table_name = String::new();
        let mut join_fk_col = String::new();
        let mut remote_pk = format_ident!("dummy");
        let mut remote_pk_mand = false;
        let mut remote_pk_col = String::new();
        &match many_to_many {
            Some(mtm) => {
                join_table_name = mtm.join_table_name.clone();
                join_fk_col = mtm.join_fk_col.clone();
                remote_pk = mtm.remote_pk.clone();
                remote_pk_mand = mtm.remote_pk_mand;
                remote_pk_col = mtm.remote_pk_col.clone();
            }
            None => (),
        };
        let mut select: String;
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
        let pk_is_none = LitStr::new(
            &format!(
                "{}.{} is None",
                struct_id.to_string(),
                pk_id.to_string()
            ),
            Span::call_site(),
        );
        let self_pk_expr: Expr = if pk.opt {
            parse_quote!(
                match self.#pk_id {
                    Some(pk) => pk,
                    None => return Err(
                        vicocomo::Error::invalid_input(#pk_is_none)
                    ),
                }
            )
        } else {
            parse_quote!(self.#pk_id)
        };
        let remote_pk_expr: Expr = if many_to_many.is_some() {
            if remote_pk_mand {
                parse_quote!(remote.#remote_pk)
            } else {
                let remote_pk_is_none = LitStr::new(
                    &format!(
                        "{}.{} is None",
                        tokens_to_string(&remote_type),
                        remote_pk.to_string()
                    ),
                    Span::call_site(),
                );
                parse_quote!(
                    match remote.#remote_pk {
                        Some(ref pk) => pk,
                        None => return Err(::vicocomo::Error::invalid_input(
                            #remote_pk_is_none
                        )),
                    }
                )
            }
        } else {
            parse_quote!(())
        };
        let connect_sql = if many_to_many.is_some() {
            format!(
                "INSERT INTO {} ({}, {}) VALUES ($1, $2)",
                join_table_name, remote_fk_col, join_fk_col,
            )
        } else {
            String::new()
        };
        let disconnect_sql = if many_to_many.is_some() {
            format!(
                "DELETE FROM {} WHERE {} = $1 AND {} = $2",
                join_table_name, remote_fk_col, join_fk_col,
            )
        } else {
            String::new()
        };
        let join_col_vals_expr: Expr = parse_quote!(
            &[#self_pk_expr.clone().into(), #remote_pk_expr.clone().into()]
        );
        let connect_to_id = format_ident!("connect_to_{}", assoc_snake);
        let disconnect_from_id =
            format_ident!("disconnect_from_{}", assoc_snake);
        let get_id = format_ident!("{}s", assoc_snake);

        if many_to_many.is_some() {
            gen.extend(quote! {
                impl #struct_id {
                    pub fn #connect_to_id(
                        &self,
                        db: ::vicocomo::DatabaseIf,
                        remote: &#remote_type,
                    ) -> Result<usize, ::vicocomo::Error> {
                        db.exec(#connect_sql, #join_col_vals_expr)
                    }

                    pub fn #disconnect_from_id(
                        &self,
                        db: ::vicocomo::DatabaseIf,
                        remote: &#remote_type,
                    ) -> Result<usize, ::vicocomo::Error> {
                        db.exec(#disconnect_sql, #join_col_vals_expr)
                    }
                }
            });
        }

        gen.extend(quote! {
            impl #struct_id {
                pub fn #get_id(
                    &self,
                    db: ::vicocomo::DatabaseIf,
                    filter: Option<&::vicocomo::Query>,
                ) -> Result<Vec<#remote_type>, ::vicocomo::Error> {
                    use ::vicocomo::Find;
                    /*
                    Ok(Vec::new())
                    */
                    let mut bld = match filter {
                        Some(f) => f.clone().builder(),
                        None => ::vicocomo::QueryBld::new(),
                    };
                    #remote_type::query(
                        db,
                        bld.filter(
                            #filter_assoc,
                            &[Some(#self_pk_expr.clone().into())]
                        )
                            .query()
                            .as_ref()
                            .unwrap(),
                    )
                }
            }
        });
    }
    //println!("{}", ::vicocomo_derive_utils::tokens_to_string(&gen));
    gen.into()
}
