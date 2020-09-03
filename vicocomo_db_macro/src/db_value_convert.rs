#![allow(dead_code)]

use ::vicocomo_derive_utils::*;
use proc_macro::TokenStream;
use syn::{
    parse::{self, Parse, ParseStream},
    Expr, Ident, Type,
};

pub fn db_value_convert_impl(input: TokenStream) -> TokenStream {
    use quote::{format_ident, quote};
    use syn::{export::Span, parse_macro_input, parse_quote, LitStr};
    let ConvertDef {
        other,
        variant,
        into,
        from,
    } = parse_macro_input!(input as ConvertDef);
    let contained_type: Type = match variant.to_string().as_str() {
        "Float" => parse_quote!(f64),
        "Int" => parse_quote!(i64),
        "Text" => parse_quote!(String),
        _ => panic!("db_value_convert cannot handle variant {}", variant),
    };
    let conversion = contained_type != other;
    let value_to_other = if conversion {
        into.unwrap_or(parse_quote!(value as #other))
    } else {
        parse_quote!(value)
    };
    let other_to_value = if conversion {
        from.unwrap_or(parse_quote!(other as #contained_type))
    } else {
        parse_quote!(other)
    };
    let into_option: Expr = if conversion {
        parse_quote!(option.map(|value| #value_to_other))
    } else {
        parse_quote!(option)
    };
    let from_option: Expr = if conversion {
        parse_quote!(option.map(|other| #other_to_value))
    } else {
        parse_quote!(option)
    };
    let wrong_variant = LitStr::new(
        &format!("cannot convert {{:?}} into {}", tokens_to_string(&other)),
        Span::call_site(),
    );
    let wrong_option = LitStr::new(
        &format!(
            "cannot convert {{:?}} into Option<{}>",
            tokens_to_string(&other)
        ),
        Span::call_site(),
    );
    let nul_variant = format_ident!("Nul{}", variant);
    TokenStream::from(quote! {
        impl std::convert::TryInto<#other> for DbValue {
            type Error = crate::Error;
            fn try_into(self) -> Result<#other, Self::Error> {
                match self {
                    DbValue::#variant(value) => Ok(#value_to_other),
                    _ => Err(Error::invalid_input(
                        &format!(#wrong_variant, self),
                    )),
                }
            }
        }
        impl From<#other> for DbValue {
            fn from(other: #other) -> Self {
                Self::#variant(#other_to_value)
            }
        }
        impl std::convert::TryInto<Option<#other>> for DbValue {
            type Error = crate::Error;
            fn try_into(self) -> Result<Option<#other>, Self::Error> {
                match self {
                    DbValue::#nul_variant(option) => Ok(#into_option),
                    _ => Err(Error::invalid_input(
                        &format!(#wrong_option, self),
                    )),
                }
            }
        }
        impl From<Option<#other>> for DbValue {
            fn from(option: Option<#other>) -> Self {
                Self::#nul_variant(#from_option)
            }
        }
    })
}

struct ConvertDef {
    other: Type,
    variant: Ident,
    into: Option<Expr>,
    from: Option<Expr>,
}

impl Parse for ConvertDef {
    fn parse(input: ParseStream) -> parse::Result<Self> {
        use syn::token;
        let result = Self {
            other: input.parse()?,
            variant: input.parse::<token::Comma>().and(input.parse())?,
            into: input.parse::<token::Comma>().ok().and(input.parse().ok()),
            from: input.parse::<token::Comma>().ok().and(input.parse().ok()),
        };
        match input.parse::<token::Comma>() {
            _ => (),
        }
        Ok(result)
    }
}
