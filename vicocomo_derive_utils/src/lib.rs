//! Utilities for use by the `vicocomo` derive macros.
//!
use quote::ToTokens;
use syn::{export::Span, Attribute, Ident};

pub fn tokens_to_string<T: ToTokens>(obj: &T) -> String {
    let mut ts = proc_macro2::TokenStream::new();
    obj.to_tokens(&mut ts);
    ts.to_string()
}

pub fn get_string_from_attr<F>(
    attrs: &[Attribute],
    attr_name: &str,
    struct_id: &Ident,
    default: F,
) -> String
where
    F: Fn(&Ident) -> String,
{
    use syn::{Lit, Meta};
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
        None => default(struct_id),
    }
}

pub fn get_strings_from_attr(
    attrs: &[Attribute],
    attr_name: &str,
    count: Option<usize>,
) -> Vec<Vec<String>> {
    use syn::{Lit, Meta, NestedMeta};
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

fn vicocomo_attr(attr_name: &str) -> String {
    format!("vicocomo_{}", attr_name)
}