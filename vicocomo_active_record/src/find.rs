use crate::model::Model;
use ::proc_macro::TokenStream;
use ::proc_macro2::Span;
use ::quote::format_ident;
use ::syn::{Ident, LitStr};

#[allow(unused_variables)]
pub(crate) fn find_impl(model: &Model) -> TokenStream {
    use ::quote::quote;
    use ::syn::{
        parse_quote, punctuated::Punctuated, token::Comma, Expr, ItemFn,
    };
    let struct_id = &model.struct_id;
    let model_name = LitStr::new(&struct_id.to_string(), Span::call_site());
    let table_name = &model.table_name;
    let all_cols = model
        .fields
        .iter()
        .map(|f| f.col.value())
        .collect::<Vec<_>>();
    let db_types = model.db_types();
    let pk_fields = &model.pk_fields();
    let pk_type = &model.pk_type();

    // == general functions ==================================================

    let default_order = model.default_order();
    let all_cols_join = all_cols.join(", ");

    // SELECT <all> FROM <table> [ ORDER BY <default> ]
    let load_sql = format!(
        "SELECT {} FROM {} {}",
        &all_cols_join, table_name, default_order,
    );
    let load_models = model.rows_to_models_expr(
        parse_quote!(db.query(#load_sql, &[], &[ #( #db_types ),* ])?),
    );

    // SELECT <all> FROM <table>
    // [ WHERE ... ] [ ORDER BY ... ] [ LIMIT ... ] [ OFFSET ... ]
    let query_sql = format!(
        "SELECT {} FROM {} {{}} {{}} {{}} {{}}",
        &all_cols_join, &table_name,
    );
    let found_models = model.rows_to_models_expr(parse_quote!(found_rows));
    let pk_len = pk_fields.len();
    let find_model = model.rows_to_models_expr(parse_quote!(outp));
    let load_fn: ItemFn = parse_quote!(
        fn load(
            db: ::vicocomo::DatabaseIf,
        ) -> Result<Vec<Self>, ::vicocomo::Error> {
            #load_models
        }
    );
    let query_fn: ItemFn = parse_quote!(
        fn query(
            db: ::vicocomo::DatabaseIf,
            query: &::vicocomo::Query
        ) -> Result<Vec<Self>, ::vicocomo::Error> {
            let filter = match query.filter.as_ref() {
                Some(f) => format!("WHERE {}", f),
                None => String::new(),
            };
            let limit = match query.limit {
                Some(l) => format!("LIMIT {}", l),
                None => String::new(),
            };
            let offset = match query.offset {
                Some(l) => format!("OFFSET {}", l),
                None => String::new(),
            };
            let order = match &query.order {
                ::vicocomo::Order::Custom(ord) =>
                    format!("ORDER BY {}", ord),
                ::vicocomo::Order::Dflt =>
                    #default_order.to_string(),
                ::vicocomo::Order::NoOrder => String::new(),
            };
            let mut values: Vec<::vicocomo::DbValue> = Vec::new();
            for opt in query.values.as_slice() {
                match opt {
                    Some(v) => values.push(v.clone()),
                    None => return Err(::vicocomo::Error::invalid_input(
                        "database--Query--value-missing",
                    )),
                }
            }
            let sql =
                format!(#query_sql, filter, order, limit, offset);
            let mut found_rows =
                db.query(&sql, &values, &[ #( #db_types ),* ])?;
            #found_models
        }
    );
    let mut gen = proc_macro2::TokenStream::new();
    let self_to_pk_tuple_expr = model.expr_to_pk_tuple(parse_quote!(self));
    if pk_len > 0 {
        let find_pk_sql = model.find_sql(
            model
                .pk_fields()
                .iter()
                .map(|f| f.col.value())
                .collect::<Vec<_>>()
                .as_slice(),
        );
        let mut pk_values: Punctuated<Expr, Comma> = Punctuated::new();
        if pk_len == 1 {
            pk_values.push(parse_quote!(pk.clone().into()));
        } else {
            for ix in (0..pk_len).map(|ix| syn::Index::from(ix)) {
                pk_values.push(parse_quote!(pk.#ix.clone().into()));
            }
        }
        gen.extend(quote! {
            impl ::vicocomo::Find<#pk_type> for #struct_id {
                fn find(db: ::vicocomo::DatabaseIf, pk: &#pk_type)
                    -> Option<Self>
                {
                    match db.query(
                        #find_pk_sql,
                        &[ #pk_values ],
                        &[ #( #db_types ),* ]
                    ) {
                        Ok(mut outp) if 1 == outp.len() => {
                            match #find_model {
                                Ok(mut models) => {
                                    Some(models.drain(..1).next().unwrap())
                                },
                                Err(_) => None,
                            }
                        },
                        _ => None,
                    }
                }

                fn find_equal(&self, db: ::vicocomo::DatabaseIf)
                    -> Option<Self>
                {
                    #self_to_pk_tuple_expr.and_then(|tup| {
                        Self::find(db, &tup)
                    })
                }

                #load_fn

                #query_fn
            }
        });
    } else {
        gen.extend(quote! {
            impl ::vicocomo::Find<#pk_type> for #struct_id {
                #load_fn

                #query_fn
            }
        });
    }

    // == unique field functions =============================================

    for uni_flds in model.unique_fields() {
        use ::syn::FnArg;
        let mut uni_cols = Vec::new();
        let uni_str = &uni_flds
            .iter()
            .map(|f| f.id.to_string())
            .collect::<Vec<_>>()
            .join("_and_");
        let mut find_pars: Punctuated<FnArg, Comma> = Punctuated::new();
        let mut find_args: Punctuated<Expr, Comma> = Punctuated::new();
        let mut par_vals: Punctuated<Expr, Comma> = Punctuated::new();
        let mut self_args: Punctuated<Expr, Comma> = Punctuated::new();
        find_pars.push(parse_quote!(db: ::vicocomo::DatabaseIf));
        find_args.push(parse_quote!(db));
        self_args.push(parse_quote!(db));
        for field in &uni_flds {
            let fld_id = &field.id;
            let par_id = id_to_par(fld_id);
            let par_ty = if field.opt {
                &Model::strip_option(&field.ty)
            } else {
                &field.ty
            };
            find_pars.push(parse_quote!(#par_id: &#par_ty));
            find_args.push(parse_quote!(#par_id));
            par_vals.push(parse_quote!(#par_id.clone().into()));
            self_args.push(if field.opt {
                parse_quote!(self.#fld_id.as_ref().unwrap())
            } else {
                parse_quote!(&self.#fld_id)
            });
            uni_cols.push(field.col.value());
        }

        // -- finding --------------------------------------------------------

        let find_uni_sql = model.find_sql(&uni_cols);
        let find_by_str = format!("find_by_{}", uni_str);
        let find_by_id = Ident::new(find_by_str.as_str(), Span::call_site());
        let find_eq_str = format!("find_equal_{}", uni_str);
        let find_eq_id = Ident::new(find_eq_str.as_str(), Span::call_site());
        gen.extend(quote! {
            impl #struct_id {

                // -- find_by_field1_and_field3(db, v1, v3) ------------------
                pub fn #find_by_id(#find_pars) -> Option<Self> {
                    match db.query(
                        #find_uni_sql,
                        &[#par_vals],
                        &[ #( #db_types ),* ]
                    ) {
                        Ok(mut outp) if 1 == outp.len() => {
                            match #find_model {
                                Ok(mut models) => {
                                    Some(models.drain(..1).next().unwrap())
                                },
                                Err(_) => None,
                            }
                        },
                        _ => None,
                    }
                }

                // -- find_equal_field1_and_field3(db) -----------------------
                pub fn #find_eq_id(&self, db: ::vicocomo::DatabaseIf)
                -> Option<Self> {
                    Self::#find_by_id(#self_args)
                }
            }
        });

        // -- validating -----------------------------------------------------

        let uni_id = format_ident!("validate_unique_{}", uni_str);
        let exi_id = format_ident!("validate_exists_{}", uni_str);
        let uni_dashes = &uni_str.replace("_and_", "--");
        let not_found_error: Expr = if uni_flds.len() > 1 {
            let msg = LitStr::new(
                &format!("{}--not-found", &uni_dashes),
                Span::call_site(),
            );
            parse_quote!(Some(#msg.to_string()))
        } else {
            parse_quote!(None)
        };
        let not_unique_error: Expr = if uni_flds.len() > 1 {
            let msg = LitStr::new(
                &format!("{}--not-unique", &uni_dashes),
                Span::call_site(),
            );
            parse_quote!(Some(#msg.to_string()))
        } else {
            parse_quote!(None)
        };
        let mut uni_name: Vec<LitStr> = Vec::new();
        for field in uni_flds {
            uni_name
                .push(LitStr::new(&field.id.to_string(), Span::call_site()));
        }
        let obj_to_pk_tuple_expr = model.expr_to_pk_tuple(parse_quote!(obj));
        gen.extend(quote! {
            impl #struct_id {

                // -- validate_exists_field1_and_field3(db, v1, v3) ----------
                pub fn #exi_id(#find_pars) -> Result<(), ::vicocomo::Error> {
                    match Self::#find_by_id(#find_args) {
                        Some(_) => Ok(()),
                        None => {
                            Err(::vicocomo::Error::CannotSave(
                                ::vicocomo::ModelError {
                                    general: #not_found_error,
                                    model: #model_name.to_string(),
                                    field_errors: vec![
                                    #(
                                        ::vicocomo::FieldError {
                                            field: #uni_name.to_string(),
                                            texts:
                                                vec!["not-found".to_string()],
                                        },
                                    )*
                                    ],
                                }
                            ))
                        }
                    }
                }

                // -- validate_unique_field1_and_field3(db) ------------------
                pub fn #uni_id(
                    &self,
                    db: ::vicocomo::DatabaseIf,
                ) -> Result<(), ::vicocomo::Error> {
                    match self.#find_eq_id(db) {
                        Some(obj) if #self_to_pk_tuple_expr
                            != #obj_to_pk_tuple_expr =>
                        {
                            Err(::vicocomo::Error::CannotSave(
                                ::vicocomo::ModelError {
                                    general: #not_unique_error,
                                    model: #model_name.to_string(),
                                    field_errors: vec![
                                    #(
                                        ::vicocomo::FieldError {
                                            field: #uni_name.to_string(),
                                            texts: vec![
                                                "not-unique".to_string(),
                                            ],
                                        },
                                    )*
                                    ],
                                }
                            ))
                        }
                        _ => Ok(()),
                    }
                }
            }
        });
    }
    gen.into()
}

fn id_to_par(fld_id: &Ident) -> Ident {
    Ident::new(&format!("{}_par", fld_id), Span::call_site())
}
