use crate::utils::*;
use proc_macro::TokenStream;

pub fn path_tag_impl(input: TokenStream) -> TokenStream {
    use quote::quote;
    use syn::{parse, Data::Struct, DeriveInput, Fields, Type};
    let struct_tokens: DeriveInput = parse(input).unwrap();
    let data = struct_tokens.data;
    let mut so_far = Err(());
    match data {
        Struct(data_struct) => match data_struct.fields {
            Fields::Unnamed(fields_unnamed) => {
                if 1 == fields_unnamed.unnamed.len() {
                    match fields_unnamed.unnamed.first() {
                        Some(field) => match &field.ty {
                            Type::Path(type_path) => match type_path
                                .path
                                .segments
                                .last()
                            {
                                Some(segment) => {
                                    if "HtmlTag"
                                        == segment.ident.to_string().as_str()
                                    {
                                        so_far = Ok(());
                                    }
                                }
                                _ => (),
                            },
                            _ => (),
                        },
                        _ => (),
                    }
                }
            }
            _ => (),
        },
        _ => (),
    }
    so_far.expect("expected struct <name>(HtmlTag)");
    let struct_id = struct_tokens.ident;
    let attrs = struct_tokens.attrs;
    let path_tag_data_strings =
        get_strings_from_attr(&attrs, "path_tag_data", Some(2));
//println!("{:?}", path_tag_data_strings);
    let tag_str = path_tag_data_strings[0][0].clone();
    let path_attr_name_str = path_tag_data_strings[0][1].clone();
    let mut path_tag_attr_names: Vec<String> = vec![];
    let mut path_tag_attr_values: Vec<String> = vec![];
    for strings in get_strings_from_attr(&attrs, "path_tag_attr", Some(2)) {
        if path_attr_name_str == strings[0] {
            panic!(
                "#[vicocomo_path_tag_attr(\"{}\", ...)] not allowed, \
                 alredy defined as path attribute name!",
                path_attr_name_str
            );
        }
        path_tag_attr_names.push(strings[0].clone());
        path_tag_attr_values.push(strings[1].clone());
    }
    let gen = quote! {
        impl #struct_id {
            pub fn new(path: Option<&str>) -> Self {
                let mut result = Self(HtmlTag::new(#tag_str));
                result.0.set_attr(#path_attr_name_str, path);
                #(
                    result.0.set_attr(
                        #path_tag_attr_names,
                        Some(#path_tag_attr_values)
                    );
                )*
                result
            }
        }

        impl PathTag for #struct_id {
            fn set_path(&mut self, path: &str) {
                self.0.set_attr(#path_attr_name_str, Some(path));
            }

            fn set_attr(&mut self, attr: &str, values: Option<&str>) {
                self.0.set_attr(attr, values);
            }
        }

        impl fmt::Display for #struct_id {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                self.0.fmt(f)
            }
        }
    };
    gen.into()
}
