//! Derive [`SessionModel`
//! ](../vicocomo/session_model/trait.SessionModel.html).

use ::proc_macro::TokenStream;

/// Derive `SessionModel` for a `struct` or `enum`.
///
/// ## Struct attributes
///
/// `vicocomo_session_key = "`*some session key*`"` - The key to use to store
/// the model in the web session. Should be unique. Optional, default the
/// `struct` identifier (*not* the Rust path, beware!).
///
/// ## Generated code
///
/// Implements [`SessionModel`
/// ](../vicocomo/session_model/trait.SessionModel.html#tymethod.key).
///
/// ### If `#[vicocomo_session_model_accessors]`
///
/// The object must be a `struct` with named fields. For each field:
///
/// `pub fn `*field id*`(&self) -> `*field type*
///
/// Get a clone of the current field value in the session
///
/// `pub fn set_`*field id*`(&mut self, srv: `[`HttpServerIf`
/// ](../vicocomo/http/server/struct.HttpServerIf.html)`, val: &`*field
/// type*`) -> Result<(), `[`Error`](../vicocomo/error/enum.Error.html)`>`
///
/// Set the field value in the session to a clone of `val`.
///
#[proc_macro_derive(
    SessionModel,
    attributes(vicocomo_session_key, vicocomo_session_model_accessors)
)]
pub fn session_model_derive(input: TokenStream) -> TokenStream {
    use ::proc_macro2::Span;
    use ::quote::{format_ident, quote};
    use ::syn::{
        parse, Data::Struct, DeriveInput, Fields::Named, FieldsNamed, LitStr,
    };
    use ::vicocomo_derive_utils::get_string_from_attr;

    let tokens: DeriveInput = parse(input).unwrap();
    let obj_id = tokens.ident;
    let session_key: LitStr = LitStr::new(
        get_string_from_attr(&tokens.attrs, "session_key", &obj_id, |id| {
            id.to_string()
        })
        .as_str(),
        Span::call_site(),
    );
    let mut gen = ::proc_macro2::TokenStream::new();
    if tokens
        .attrs
        .iter()
        .find(|a| a.path.is_ident("vicocomo_session_model_accessors"))
        .is_some()
    {
        let mut named_fields: Option<FieldsNamed> = None;
        match tokens.data {
            Struct(data_struct) => match data_struct.fields {
                Named(fields_named) => {
                    named_fields = Some(fields_named);
                }
                _ => (),
            },
            _ => panic!("must be a struct"),
        }
        let fields = named_fields.expect("fields must be named").named;
        let mut field_id_vec = Vec::new();
        let mut field_ty_vec = Vec::new();
        let mut set_field_id_vec = Vec::new();
        for field in &fields {
            let id = field.ident.as_ref().unwrap();
            set_field_id_vec.push(format_ident!("set_{}", id));
            field_id_vec.push(id);
            field_ty_vec.push(field.ty.clone());
        }
        gen.extend(quote! {
            impl #obj_id {
              #(
                pub fn #field_id_vec(&self) -> #field_ty_vec {
                    self.#field_id_vec.clone()
                }
                pub fn #set_field_id_vec(
                    &mut self,
                    srv: ::vicocomo::HttpServerIf,
                    val: &#field_ty_vec,
                ) -> Result<(), ::vicocomo::Error> {
                    self.#field_id_vec = val.clone();
                    self.store(srv)
                }
              )*
            }
        });
    }
    gen.extend(quote!(
        impl ::vicocomo::SessionModel for #obj_id {
            fn key() -> &'static str {
                #session_key
            }
        }
    ));
    gen.into()
}
