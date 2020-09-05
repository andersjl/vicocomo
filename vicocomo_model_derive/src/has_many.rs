use crate::model::{HasMany, ManyToMany, Model};
use proc_macro::TokenStream;

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
            ref trait_types,
            ref many_to_many,
        } = has_many;
        gen.extend(quote! {
            impl ::vicocomo::HasMany<#trait_types> for #struct_id {}
        });
        if assoc_name.is_some() {
            let name_type =
                Model::name_type_item(assoc_name.as_ref().unwrap());
            gen.extend(quote! {
                #name_type
            });
        }
        let mut select: String;
        let filter_assoc = LitStr::new(
            &match many_to_many {
                Some(ManyToMany {
                    join_table_name,
                    join_fk_col,
                    remote_pk_col,
                }) => format!(
                    "{} IN (SELECT {} FROM {} WHERE {} = $1)",
                    remote_pk_col,
                    join_fk_col,
                    join_table_name,
                    remote_fk_col,
                ),
                None => format!("{} = $1", remote_fk_col),
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
        let self_pk: Expr = if pk.opt {
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
        let find_remote_id = format_ident!("find_remote_{}", assoc_snake);

        gen.extend(quote! {
            impl #struct_id {
                pub fn #find_remote_id(
                    &self,
                    db: &impl ::vicocomo::DbConn,
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
                            &[Some(#self_pk.clone().into())]
                        )
                            .query()
                            .as_ref()
                            .unwrap(),
                    )
                }
            }
        });
    }
    gen.into()
}
