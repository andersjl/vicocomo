use crate::model::{ForKey, Model};
use proc_macro::TokenStream;
//TODO code smells

#[allow(unused_variables)]
pub(crate) fn belongs_to_impl(model: &Model) -> TokenStream {
    use ::vicocomo_derive_utils::*;
    use quote::{format_ident, quote};
    use syn::{export::Span, parse_quote, Expr, LitStr};

    let struct_id = &model.struct_id;
    let table_name = &model.table_name;

    let mut gen = proc_macro2::TokenStream::new();
    for bel_fld in model.belongs_to_fields() {
        let fk = bel_fld.fk.as_ref().unwrap();
        let fk_id = &bel_fld.id;
        let ForKey {
            assoc_name,
            assoc_snake,
            remote_pk,
            remote_pk_mand,
            remote_type,
            trait_types,
        } = fk;
        gen.extend(quote! {
            impl ::vicocomo::BelongsTo<#trait_types> for #struct_id {}
        });
        if assoc_name.is_some() {
            let name_type =
                Model::name_type_item(assoc_name.as_ref().unwrap());
            gen.extend(quote! {
                #name_type
            });
        }
        let fk_is_none = LitStr::new(
            &format!(
                "{}.{} is None",
                struct_id.to_string(),
                fk_id.to_string(),
            ),
            Span::call_site(),
        );
        let pk_is_none = LitStr::new(
            &format!(
                "{}.{} is None",
                tokens_to_string(remote_type),
                remote_pk,
            ),
            Span::call_site(),
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
                    None => return Err(::vicocomo::Error::invalid_input(
                        #pk_is_none
                    )),
                }
            )
        };
        let fk_expr_err: Expr = if bel_fld.dbt.as_ref().unwrap().1 {
            parse_quote!(
                match self.#fk_id {
                    Some(ref fk) => fk.clone().into(),
                    None => return Err(::vicocomo::Error::invalid_input(
                        #fk_is_none
                    )),
                }
            )
        } else {
            parse_quote!(self.#fk_id.clone().into())
        };
        let fk_expr_opt: Expr = if bel_fld.dbt.as_ref().unwrap().1 {
            parse_quote!(
                match self.#fk_id {
                    Some(ref fk) => fk,
                    None => return None,
                }
            )
        } else {
            parse_quote!(&self.#fk_id)
        };
        let set_fk_expr: Expr = if *remote_pk_mand {
            if bel_fld.dbt.as_ref().unwrap().1 {
                parse_quote!({
                    self.#fk_id = Some(remote.#remote_pk.clone());
                })
            } else {
                parse_quote!({
                    self.#fk_id = remote.#remote_pk.clone();
                })
            }
        } else if bel_fld.dbt.as_ref().unwrap().1 {
            parse_quote!(
                match remote.#remote_pk {
                    Some(pk) => self.#fk_id = Some(pk),
                    None => return Err(
                        ::vicocomo::Error::invalid_input(#pk_is_none)
                    ),
                }
            )
        } else {
            parse_quote!(
                match remote.#remote_pk {
                    Some(pk) => self.#fk_id = pk,
                    None => return Err(
                        ::vicocomo::Error::invalid_input(#pk_is_none)
                    ),
                }
            )
        };
        let all_belonging_to_id =
            format_ident!("all_belonging_to_{}", assoc_snake);
        let belongs_to_id = format_ident!("belongs_to_{}", assoc_snake);
        let belong_to_id = format_ident!("belong_to_{}", assoc_snake);
        let siblings_id = format_ident!("{}_siblings", assoc_snake);
        gen.extend(quote! {
            impl #struct_id {
                pub fn #all_belonging_to_id(
                    db: &impl ::vicocomo::DbConn,
                    remote: &#remote_type
                ) -> Result<Vec<Self>, ::vicocomo::Error> {
                    use ::vicocomo::Find;
                    Self::query(
                        db,
                        &::vicocomo::QueryBld::new()
                            .filter(#par_filter, &[Some(#remote_pk_expr)])
                            .query()
                            .unwrap(),
                    )
                }
                pub fn #belongs_to_id(&self, db: &impl ::vicocomo::DbConn)
                    -> Option<#remote_type>
                {
                    use ::vicocomo::Find;
                    #remote_type::find(db, #fk_expr_opt)
                }
                pub fn #belong_to_id(&mut self, remote: &#remote_type)
                    -> Result<(), ::vicocomo::Error>
                {
                    #set_fk_expr
                    Ok(())
                }
                pub fn #siblings_id(
                    &mut self,
                    db: &impl ::vicocomo::DbConn
                ) -> Result<Vec<Self>, ::vicocomo::Error> {
                    use ::vicocomo::Find;
                    Self::query(
                        db,
                        &::vicocomo::QueryBld::new()
                            .filter(#par_filter, &[Some(#fk_expr_err)])
                            .query()
                            .unwrap(),
                    )
                }
            }
        });
        if bel_fld.dbt.as_ref().unwrap().1 {
            let belong_to_no_id =
                format_ident!("belong_to_no_{}", assoc_snake);
            gen.extend(quote! {
                impl #struct_id {
                    pub fn #belong_to_no_id(
                        &mut self
                    ) -> Result<(), ::vicocomo::Error> {
                        self.#fk_id = None;
                        Ok(())
                    }
                }
            });
        }
    }
    //println!("{}", ::vicocomo_derive_utils::tokens_to_string(&gen));
    gen.into()
}
