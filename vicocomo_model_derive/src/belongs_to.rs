use crate::model::Model;
use proc_macro::TokenStream;

#[allow(unused_variables)]
pub fn belongs_to_model_impl(model: &Model) -> TokenStream {
    use quote::quote;
    use syn::{export::Span, parse_quote, Expr, LitStr};
    use vicocomo_derive_utils::tokens_to_string;

    let struct_id = &model.struct_id;
    let table_name = &model.table_name;

    let mut gen = proc_macro2::TokenStream::new();
    for bel_fld in model.belongs_to_fields() {
        let fk = bel_fld.fk.as_ref().unwrap();
        let parent = &fk.path;
        let fk_id = &bel_fld.id;
        let pk = &fk.parent_pk;
        let pk_is_none = LitStr::new(
            &format!("{}.{} is None", tokens_to_string(parent), pk),
            Span::call_site(),
        );
        let par_filter = LitStr::new(
            &format!("{} = $1", bel_fld.col.value()),
            Span::call_site(),
        );
        let find_expr: Expr = if fk.parent_pk_mand {
            parse_quote!(parent.#pk.into())
        } else {
            parse_quote!(
                match parent.#pk {
                    Some(pk) => pk.into(),
                    None => return Err(
                        vicocomo::Error::invalid_input(#pk_is_none)
                    ),
                }
            )
        };
        let set_expr: Expr = if fk.parent_pk_mand {
            parse_quote!({
                self.#fk_id = parent.#pk;
            })
        } else {
            parse_quote!(
                match parent.#pk {
                    Some(pk) => self.#fk_id = pk,
                    None => return Err(
                        vicocomo::Error::invalid_input(#pk_is_none)
                    ),
                }
            )
        };
        gen.extend(quote! {
            impl BelongsTo<#parent> for #struct_id {
                fn belonging_to(
                    db: &impl vicocomo::DbConn,
                    parent: &#parent
                ) -> Result<Vec<Self>, vicocomo::Error> {
                    use vicocomo::Find;
                    Self::query(
                        db,
                        &vicocomo::QueryBld::new()
                            .filter(Some(#par_filter), &[#find_expr])
                            .query()
                            .unwrap(),
                    )
                }
                fn parent(&self, db: &impl vicocomo::DbConn)
                    -> Option<#parent>
                {
                    use vicocomo::Find;
                    #parent::find(db, &self.#fk_id)
                }
                fn set_parent(&mut self, parent: &#parent)
                    -> Result<(), vicocomo::Error>
                {
                    #set_expr
                    Ok(())
                }
            }
        });
    }
    gen.into()
}
