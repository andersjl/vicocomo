use crate::utils::*;
use proc_macro::TokenStream;

pub fn generate_path_tag_impl(input: TokenStream) -> TokenStream {
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
                                    if "PathTagData"
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
    so_far.expect("expected struct <name>(PathTagData)");
    let struct_id = struct_tokens.ident;
    let attrs = struct_tokens.attrs;
    let path_tag_data_strings =
        get_strings_from_attr(&attrs, "path_tag_data", Some(2));
    if 1 != path_tag_data_strings.len() {
        panic!("expected #[path_tag_data(\"some_tag_name\", \"some_path_attr_name\")]")
    }
    let tag_str = path_tag_data_strings[0][0].clone();
    let path_attr_name_str = path_tag_data_strings[0][1].clone();
    let mut path_tag_attr_names: Vec<String> = vec![];
    let mut path_tag_attr_values: Vec<String> = vec![];
    for strings in get_strings_from_attr(&attrs, "path_tag_attr", Some(2)) {
        path_tag_attr_names.push(strings[0].clone());
        path_tag_attr_values.push(strings[1].clone());
    }
    let gen = quote! {
        impl #struct_id {
            pub fn new(a_path: Option<&str>) -> Self {
                let mut result =
                    Self(PathTagData::new(#tag_str, #path_attr_name_str));
                match a_path {
                    Some(path) => result.0.set_path(path),
                    None => (),
                };
                #(
                    result.0.attrs.push(HtmlAttr::new(
                        #path_tag_attr_names,
                        Some(#path_tag_attr_values)
                    ));
                )*
                result
            }
        }

        impl PathTag for #struct_id {
            fn set_path(&mut self, a_path: &str) {
                self.0.set_path(a_path);
            }

            fn add_attr(&mut self, attr: &HtmlAttr) {
                self.0.attrs.push(attr.clone());
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
