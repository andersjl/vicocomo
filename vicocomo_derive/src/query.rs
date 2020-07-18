use crate::model::{Model, Order, Param};
use proc_macro::TokenStream;
use syn::Ident;

pub fn generate_query_model_impl(model: &Model) -> TokenStream {
    use quote::quote;
    use syn::{
        export::Span, punctuated::Punctuated, token::Comma, Expr,
        ExprMethodCall, FnArg,
    };
    let table_id = &model.table_id;
    let struct_id = &model.struct_id;
    let order_fields = &model.order_fields;
    let unique_fields = &model.unique_fields;
    let mut load_expr: ExprMethodCall;
    if order_fields.is_empty() {
        load_expr = parse_quote!(#table_id.load::<Self>(db));
    } else {
        let mut order_expr: Option<ExprMethodCall> = None;
        for Order(field_id, descending) in order_fields {
            let field_expr: ExprMethodCall = if *descending {
                parse_quote!(#field_id.desc())
            } else {
                parse_quote!(#field_id.asc())
            };
            order_expr = Some(match order_expr {
                Some(expr) => parse_quote!(#expr.then_order_by(#field_expr)),
                None => parse_quote!(#table_id.order_by(#field_expr)),
            });
        }
        load_expr = order_expr.unwrap();
        load_expr = parse_quote!(#load_expr.load::<Self>(db));
    }
    let mut gen = quote! {
        impl QueryModel<Connection> for #struct_id {
            fn load(
                db: &Connection
            ) -> diesel::result::QueryResult<Vec<Self>> {
                use crate::schema::#table_id::dsl::*;
                use diesel::expression_methods::ExpressionMethods;
                use diesel::query_dsl::{QueryDsl, RunQueryDsl};
                Ok(#load_expr?)
            }
        }
    };
    for fields in unique_fields {
        let unique_str = &fields.iter().enumerate().fold(
            String::new(),
            |acc, (ix, Param(id, _ty))| {
                format!("{}{}_{}", acc, if ix > 0 { "_and" } else { "" }, id)
            },
        );
        let find_by_id =
            Ident::new(&format!("find_by{}", unique_str), Span::call_site());
        let validate_exists_id = Ident::new(
            &format!("validate_exists{}", unique_str),
            Span::call_site(),
        );
        let validate_unique_id = Ident::new(
            &format!("validate_unique{}", unique_str),
            Span::call_site(),
        );
        let validate_error_string = &fields
            .iter()
            .enumerate()
            .fold("{}:".to_string(), |acc, (ix, _)| {
                acc + if ix > 0 { "," } else { "" } + " {}"
            });
        let mut find_params: Punctuated<FnArg, Comma> = Punctuated::new();
        find_params.push(parse_quote!(db: &Connection));
        let mut find_args: Punctuated<Expr, Comma> = Punctuated::new();
        find_args.push(parse_quote!(db));
        let mut filter_expr: Option<ExprMethodCall> = None;
        let mut validate_error_format_args: Punctuated<Expr, Comma> =
            Punctuated::new();
        validate_error_format_args.push(parse_quote!(#validate_error_string));
        validate_error_format_args.push(parse_quote!(msg));
        for Param(field_id, field_ty) in fields {
            let arg_id =
                Ident::new(&format!("{}_arg", &field_id), Span::call_site());
            find_params.push(parse_quote!(#arg_id: #field_ty));
            find_args.push(parse_quote!(#arg_id));
            validate_error_format_args.push(parse_quote!(#arg_id));
            filter_expr = Some(match filter_expr {
                Some(expr) => {
                    parse_quote!(#expr.filter(#field_id.eq(#arg_id)))
                }
                None => parse_quote!(#table_id.filter(#field_id.eq(#arg_id))),
            });
        }
        let mut validate_params = find_params.clone();
        validate_params.push(parse_quote!(msg: &str));
        let filter_expr = filter_expr.unwrap();
        gen.extend(quote! {
            impl #struct_id {
                pub fn #find_by_id(#find_params) -> Option<Box<Self>> {
                    use crate::schema::#table_id::dsl::*;
                    use diesel::expression_methods::ExpressionMethods;
                    use diesel::query_dsl::{QueryDsl, RunQueryDsl};
                    match #filter_expr.first::<Self>(db) {
                        Ok(model) => Some(Box::new(model)),
                        Err(_) => None,
                    }
                }

                pub fn #validate_unique_id(
                    #validate_params
                ) -> diesel::result::QueryResult<()> {
                    use diesel::result::{DatabaseErrorKind::UniqueViolation, Error};
                    match Self::#find_by_id(#find_args) {
                        Some(_) => Err(
                            Error::DatabaseError(
                                UniqueViolation,
                                Box::new(format!(#validate_error_format_args))
                            )
                        ),
                        None => Ok(()),
                    }
                }

                pub fn #validate_exists_id(
                    #validate_params
                ) -> Result<(), String> {
                    match Self::#find_by_id(#find_args) {
                        Some(_) => Ok(()),
                        None => Err(format!(#validate_error_format_args)),
                    }
                }
            }
        });
    }
    gen.into()
}
