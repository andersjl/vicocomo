#![allow(dead_code)]

use ::proc_macro::TokenStream;
use ::syn::{
    parse::{self, Parse, ParseStream},
    Expr, Ident, Type,
};
use ::vicocomo_derive_utils::*;

pub fn db_value_convert_impl(input: TokenStream) -> TokenStream {
    use ::proc_macro2::Span;
    use ::quote::{format_ident, quote};
    use ::syn::{parse_macro_input, parse_quote, LitStr};
    let ConvertDef {
        no_option_type,
        other,
        variant,
        from_db,
        into_db,
    } = parse_macro_input!(input as ConvertDef);
    let other_str = tokens_to_string(&other);
    let option_type_str: String;
    let option_type: Type;
    if no_option_type {
        option_type_str = format!("Option<{}>", other_str);
        option_type = parse_quote!(Option<#other>);
    } else {
        option_type_str = format!("Opt{}", other_str);
        let option_type_id = format_ident!("{}", option_type_str);
        option_type = parse_quote!(#option_type_id);
    }
    let contained_type: Type = match variant.to_string().as_str() {
        "Float" => parse_quote!(f64),
        "Int" => parse_quote!(i64),
        "Text" => parse_quote!(String),
        _ => panic!("db_value_convert cannot handle variant {}", variant),
    };
    let conversion = contained_type != other;
    let value_to_other = if conversion {
        from_db.unwrap_or(parse_quote!(value as #other))
    } else {
        parse_quote!(value)
    };
    let other_to_value = if conversion {
        into_db.unwrap_or(parse_quote!(other as #contained_type))
    } else {
        parse_quote!(other)
    };
    let option_to_other: Expr = {
        let conv: Expr = if conversion {
            // We avoid using option.map() here to enable using the ? operator
            // in value_to_other
            parse_quote!(
                match option {
                    Some(value) => Some(#value_to_other),
                    None => None,
                }
            )
        } else {
            parse_quote!(option)
        };
        if no_option_type {
            conv
        } else {
            parse_quote!(Self(#conv))
        }
    };
    let other_opt: Expr = if no_option_type {
        parse_quote!(self)
    } else {
        parse_quote!(self.0)
    };
    let other_to_option: Expr = if conversion {
        parse_quote!(#other_opt.map(|other| #other_to_value))
    } else {
        parse_quote!(#other_opt)
    };
    let wrong_variant = LitStr::new(
        &format!("cannot convert {{:?}} into {}", other_str),
        Span::call_site(),
    );
    let wrong_option = LitStr::new(
        &format!("cannot convert {{:?}} into {}", option_type_str),
        Span::call_site(),
    );
    let nul_variant = format_ident!("Nul{}", variant);
    let mut gen = proc_macro2::TokenStream::new();
    if !no_option_type {
        gen.extend(quote! {
            pub struct #option_type(Option<#other>);
        });
    }
    gen.extend(quote! {
        impl ::std::convert::Into<DbValue> for #option_type {
            fn into(self) -> DbValue {
                DbValue::#nul_variant(#other_to_option)
            }
        }
        impl ::std::convert::TryFrom<DbValue> for #option_type {
            type Error = Error;
            fn try_from(db_value: DbValue) -> Result<Self, Self::Error> {
                match db_value {
                    DbValue::#nul_variant(option) => Ok(#option_to_other),
                    _ => Err(Error::invalid_input(
                        &format!(#wrong_option, db_value),
                    )),
                }
            }
        }
        impl ::std::convert::TryFrom<DbValue> for #other {
            type Error = Error;
            fn try_from(db_value: DbValue) -> Result<Self, Self::Error> {
                match db_value {
                    DbValue::#variant(value) => Ok(#value_to_other),
                    _ => Err(Error::invalid_input(
                        &format!(#wrong_variant, db_value),
                    )),
                }
            }
        }
        impl ::std::convert::Into<DbValue> for #other {
            fn into(self) -> DbValue {
                let other = self;
                DbValue::#variant(#other_to_value)
            }
        }
    });
    gen.into()
}

struct ConvertDef {
    no_option_type: bool,
    other: Type,
    variant: Ident,
    from_db: Option<Expr>,
    into_db: Option<Expr>,
}

impl Parse for ConvertDef {
    fn parse(input: ParseStream) -> parse::Result<Self> {
        use ::syn::token;
        let no_option_type = input
            .fork()
            .parse::<Ident>()
            .map(|id| &id.to_string() == "no_option_type")
            .unwrap_or(false);
        if no_option_type {
            input.parse::<Ident>().and(input.parse::<token::Comma>())?;
        }
        let result = Self {
            no_option_type,
            other: input.parse()?,
            variant: input.parse::<token::Comma>().and(input.parse())?,
            from_db: input
                .parse::<token::Comma>()
                .ok()
                .and(input.parse().ok()),
            into_db: input
                .parse::<token::Comma>()
                .ok()
                .and(input.parse().ok()),
        };
        match input.parse::<token::Comma>() {
            _ => (),
        }
        Ok(result)
    }
}
