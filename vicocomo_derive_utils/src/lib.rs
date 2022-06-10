//! Only for use by the `vicocomo_`... derive macros.

use ::proc_macro2::Span;
use ::quote::ToTokens;
use ::syn::{
    punctuated::Punctuated, token::Comma, Attribute, Data, DeriveInput,
    Field, Fields, Ident, LitStr, Type,
};

#[doc(hidden)]
pub fn tokens_to_string<T: ToTokens>(obj: &T) -> String {
    let mut ts = proc_macro2::TokenStream::new();
    obj.to_tokens(&mut ts);
    ts.to_string()
}

#[doc(hidden)]
pub fn type_to_ident(typ: &Type) -> Option<Ident> {
    match typ {
        Type::Path(tp) => tp.path.segments.last().map(|p| p.ident.clone()),
        _ => None,
    }
}

#[doc(hidden)]
pub fn get_string_from_attr<F>(
    attrs: &[Attribute],
    attr_name: &str,
    id: &Ident,
    default: F,
) -> String
where
    F: Fn(&Ident) -> String,
{
    use ::syn::{Lit, Meta};
    let vicocomo_name = vicocomo_attr(attr_name);
    let error_msg = format!("expected #[{} = \"some_name\"]", vicocomo_name);
    match attrs
        .iter()
        .filter(|a| a.path.is_ident(&vicocomo_name))
        .last()
    {
        Some(attr) => match attr.parse_meta().expect(&error_msg) {
            Meta::NameValue(value) => match value.lit {
                Lit::Str(name) => name.value(),
                _ => panic!("{}", error_msg),
            },
            _ => panic!("{}", error_msg),
        },
        None => default(id),
    }
}

#[doc(hidden)]
pub fn get_strings_from_attr(
    attrs: &[Attribute],
    attr_name: &str,
    count: Option<usize>,
) -> Vec<Vec<String>> {
    use ::syn::{Lit, Meta, NestedMeta};
    let vicocomo_name = vicocomo_attr(attr_name);
    let error_msg = format!(
        "expected #[{}(\"val\", ...)]{}",
        vicocomo_name,
        match count {
            Some(c) => format!(" with exactly {} args", c),
            None => String::new(),
        }
    );
    attrs
        .iter()
        .filter(|a| a.path.is_ident(&vicocomo_name))
        .map(|attr| match attr.parse_meta().expect(&error_msg) {
            Meta::List(list)
                if None == count || list.nested.len() == count.unwrap() =>
            {
                list.nested
                    .iter()
                    .map(|meta| match meta {
                        NestedMeta::Lit(lit) => match lit {
                            Lit::Str(a_string) => a_string.value(),
                            _ => panic!("{}", error_msg),
                        },
                        _ => panic!("{}", error_msg),
                    })
                    .collect::<Vec<_>>()
            }
            _ => panic!("{}", error_msg),
        })
        .collect::<Vec<_>>()
}

#[doc(hidden)]
pub fn get_id_from_attr<F>(
    attrs: &[Attribute],
    attr_name: &str,
    struct_id: &Ident,
    default: F,
) -> Ident
where
    F: Fn(&Ident) -> String,
{
    Ident::new(
        &get_string_from_attr(attrs, attr_name, struct_id, default),
        Span::call_site(),
    )
}

#[doc(hidden)]
pub fn id_to_litstr(id: &Ident) -> LitStr {
    LitStr::new(&id.to_string(), Span::call_site())
}

#[doc(hidden)]
pub fn named_fields(
    struct_tokens: &DeriveInput,
) -> Result<Punctuated<Field, Comma>, String> {
    if let Data::Struct(data_struct) = &struct_tokens.data {
        if let Fields::Named(fields_named) = &data_struct.fields {
            Ok(fields_named.named.clone())
        } else {
            Err("fields must be named".to_string())
        }
    } else {
        Err("must be a struct".to_string())
    }
}

#[doc(hidden)]
pub fn tmplog(data: &str) {
    ::std::fs::write("tmp/log.txt", data).expect("tmplog failed");
}

fn vicocomo_attr(attr_name: &str) -> String {
    format!("vicocomo_{}", attr_name)
}
