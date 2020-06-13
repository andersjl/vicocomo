use quote::ToTokens;
use syn::{export::Span, parse_quote, Attribute, Expr, Ident, LitStr};

pub fn debug_to_tokens<T: ToTokens>(obj: &T) -> String {
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

pub fn pk_cols_params_expr(
    pk_mand_cols: &[LitStr],
    pk_mand_fields: &[Ident],
    pk_opt_cols: &[LitStr],
    pk_opt_field_names: &[LitStr],
    pk_opt_fields: &[Ident],
) -> Expr {
    parse_quote!(
        {
            let mut pk_cols: Vec<String> = vec![];
            let mut values: Vec<vicocomo::DbValue> = vec![];
            let mut par_ix = 0;
            #(
                par_ix += 1;
                pk_cols.push(format!("{} = ${}", #pk_mand_cols, par_ix));
                values.push(self.#pk_mand_fields.clone().into());
            )*
            #(
                match &self.#pk_opt_fields {
                    Some(val) => {
                        par_ix += 1;
                        pk_cols.push(
                            format!("{} = ${}", #pk_opt_cols, par_ix)
                        );
                        values.push(val.clone().into());
                    }
                    None => return Err(vicocomo::Error::Database(format!(
                        "missing primary key {}",
                        #pk_opt_field_names,
                    ))),
                }
            )*
            (pk_cols, values, par_ix)
        }
    )
}

pub fn placeholders_expr(row_cnt: Expr, col_cnt: Expr) -> Expr {
    parse_quote!(
        (0..#row_cnt)
            .map(|row_ix| {
                format!(
                    "({})",
                    (0..#col_cnt)
                        .map(|col_ix| {
                            format!( "${}", 1 + #col_cnt * row_ix + col_ix,)
                        })
                        .collect::<Vec<_>>()
                        .join(", "),
                )
            }).collect::<Vec<_>>()
            .join(", ")
    )
}

pub fn query_err(query: &str) -> LitStr {
    LitStr::new(
        format!("{} {{}} records, expected {{}}", query).as_str(),
        Span::call_site(),
    )
}

pub fn rows_to_models_expr(rows: Expr, man: &[Ident], opt: &[Ident]) -> Expr {
    parse_quote!(
        {
            let mut error: Option<vicocomo::Error> = None;
            let mut models = vec![];
            for mut row in #rows.drain(..) {
                #(
                    let #man;
                    match row.drain(..1).next().unwrap().try_into() {
                        Ok(val) => #man = val,
                        Err(err) => {
                            error = Some(err);
                            break;
                        },
                    }
                )*
                #(
                    let #opt;
                    match row.drain(..1).next().unwrap().try_into() {
                        Ok(val) => #opt = Some(val),
                        Err(err) => {
                            error = Some(err);
                            break;
                        },
                    }
                )*
                models.push(Self {
                    #( #man , )*
                    #( #opt , )*
                });
            }
            match error {
                Some(err) => Err(err),
                None => Ok(models),
            }
        }
    )
}

pub fn row_to_self_expr(row: Expr, man: &[Ident], opt: &[Ident]) -> Expr {
    parse_quote!(
        {
            #(
                self.#man =
                    #row.drain(..1).next().unwrap().try_into()?;
            )*
            #(
                self.#opt = Some(
                    #row.drain(..1).next().unwrap().try_into()?
                );
            )*
        }
    )
}

fn vicocomo_attr(attr_name: &str) -> String {
    format!("vicocomo_{}", attr_name)
}
