use crate::{
    model::{Model, Order},
    utils::*,
};
use proc_macro::TokenStream;
use syn::{export::Span, Ident};

#[allow(unused_variables)]
pub fn find_model_impl(model: &Model) -> TokenStream {
    use quote::quote;
    use syn::parse_quote;
    let Model {
        struct_id,
        table_name,
        fields,
        all_cols,
        all_db_types,
        all_fields,
        all_mand_cols,
        all_mand_fields,
        all_pk_cols,
        all_pk_fields,
        all_opt_cols,
        all_opt_fields,
        all_upd_cols,
        all_upd_db_types,
        pk_mand_fields,
        pk_mand_cols,
        pk_opt_fields,
        pk_opt_field_names,
        pk_opt_cols,
        pk_type,
        upd_mand_fields,
        upd_mand_cols,
        upd_opt_fields,
        upd_opt_cols,
    } = model;

    // == general functions ==================================================
    // -- load(db) -----------------------------------------------------------
    let default_order = if model.order_fields().is_empty() {
        String::new()
    } else {
        format!(
            "ORDER BY {}",
            model
                .order_fields()
                .iter()
                .map(|f| {
                    format!(
                        "{} {}",
                        f.col.value(),
                        match f.ord.as_ref().unwrap() {
                            Order::Asc(_) => "ASC",
                            Order::Desc(_) => "DESC",
                        },
                    )
                })
                .collect::<Vec<_>>()
                .join(", "),
        )
    };
    let all_cols_join = all_cols.join(", ");
    // SELECT col1, col2, col3 FROM table ORDER BY col3, col1
    let load_sql = format!(
        "SELECT {} FROM {} {}",
        &all_cols_join, table_name, default_order,
    );
    let load_models = rows_to_models_expr(
        parse_quote!(db.query(#load_sql, &[], &[ #( #all_db_types ),* ])?),
        all_mand_fields.as_slice(),
        all_opt_fields.as_slice(),
    );
    let query_sql = format!(
        "SELECT {} FROM {} {{}} {{}} {{}} {{}}",
        &all_cols_join, &table_name,
    );
    let found_models = rows_to_models_expr(
        parse_quote!(found_rows),
        all_mand_fields.as_slice(),
        all_opt_fields.as_slice(),
    );
    let mut gen = quote! {
        impl<'a> vicocomo::MdlFind<'a> for #struct_id {
            fn load(
                db: &mut impl vicocomo::DbConn<'a>,
            ) -> Result<Vec<Self>, vicocomo::Error> {
                #load_models
            }
            fn query(
                db: &mut impl vicocomo::DbConn<'a>,
                query: &vicocomo::MdlQuery
            ) -> Result<Vec<Self>, vicocomo::Error> {
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
                    vicocomo::MdlOrder::Custom(ord) =>
                        format!("ORDER BY {}", ord),
                    vicocomo::MdlOrder::Dflt => #default_order.to_string(),
                    vicocomo::MdlOrder::NoOrder => String::new(),
                };
                let mut values: Vec<vicocomo::DbValue> = Vec::new();
                for opt in query.values.as_slice() {
                    match opt {
                        Some(v) => values.push(v.clone()),
                        None => return Err(vicocomo::Error::InvalidInput(
                            "value is None".to_string()
                        )),
                    }
                }
                let sql = format!(#query_sql, filter, order, limit, offset);
                let mut found_rows =
                    db.query(&sql, &values, &[ #( #all_db_types ),* ])?;
                #found_models
            }
        }
    };

    // == unique field functions =============================================
    for uni_flds in model.unique_fields() {
        use syn::{punctuated::Punctuated, token::Comma, Expr, FnArg};
        let mut uni_cols = vec![];
        let uni_str = &uni_flds
            .iter()
            .map(|f| f.id.to_string())
            .collect::<Vec<_>>()
            .join("_");
        let mut find_pars: Punctuated<FnArg, Comma> = Punctuated::new();
        let mut find_args: Punctuated<Expr, Comma> = Punctuated::new();
        let mut par_vals: Punctuated<Expr, Comma> = Punctuated::new();
        let mut self_args: Punctuated<Expr, Comma> = Punctuated::new();
        find_pars.push(parse_quote!(db: &mut impl vicocomo::DbConn<'a>));
        find_args.push(parse_quote!(db));
        self_args.push(parse_quote!(db));
        for field in &uni_flds {
            let fld_id = &field.id;
            let par_id = id_to_par(fld_id);
            let par_ty = &field.ty;
            find_pars.push(parse_quote!(#par_id: #par_ty));
            find_args.push(parse_quote!(#par_id));
            par_vals.push(parse_quote!(#par_id.into()));
            self_args.push(parse_quote!(self.#fld_id));
            uni_cols.push(&field.col);
        }

        // -- finding --------------------------------------------------------
        // SELECT col1, col2, col3 FROM table WHERE col1 = x AND col3 = y
        let find_sql = format!(
            "SELECT {} FROM {} WHERE {}",
            &all_cols.join(", "),
            &table_name,
            &uni_cols
                .iter()
                .enumerate()
                .map(|(ix, col)| format!("{} = ${}", col.value(), ix + 1))
                .collect::<Vec<_>>()
                .join(" AND "),
        );
        let find_by_str = format!("find_by_{}", uni_str);
        let find_by_id = Ident::new(
            if uni_flds[0].pri {
                "find"
            } else {
                find_by_str.as_str()
            },
            Span::call_site(),
        );
        let find_eq_str = format!("find_by_equal_{}", uni_str);
        let find_eq_id = Ident::new(
            if uni_flds[0].pri {
                "find_equal"
            } else {
                find_eq_str.as_str()
            },
            Span::call_site(),
        );
        let find_model = rows_to_models_expr(
            parse_quote!(outp),
            all_mand_fields.as_slice(),
            all_opt_fields.as_slice(),
        );
        gen.extend(quote! {
            impl<'a> #struct_id {

                // -- find_by_field1_and_field3(db, v1, v3) ------------------
                pub fn #find_by_id(#find_pars) -> Option<Self> {
                    match db.query(
                        #find_sql,
                        &[#par_vals],
                        &[ #( #all_db_types ),* ]
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
                pub fn #find_eq_id(&self, db: &mut impl vicocomo::DbConn<'a>)
                -> Option<Self> {
                    Self::#find_by_id(#self_args)
                }
            }
        });

        // -- validating -----------------------------------------------------
        let val_uni_str = format!("validate_unique_{}", uni_str);
        let uni_id = Ident::new(
            if uni_flds[0].pri {
                "validate_unique"
            } else {
                val_uni_str.as_str()
            },
            Span::call_site(),
        );
        let val_exi_str = format!("validate_exists_{}", uni_str);
        let exi_id = Ident::new(
            if uni_flds[0].pri {
                "validate_exists"
            } else {
                val_exi_str.as_str()
            },
            Span::call_site(),
        );
        let mut exi_pars = find_pars.clone();
        exi_pars.push(parse_quote!(msg: &str));
        let validate_error_format = format!(
            "{{}}: {}",
            (0..uni_flds.len())
                .map(|_| "{:?}")
                .collect::<Vec<_>>()
                .join(", "),
        );
        let mut exi_frmt_args: Punctuated<Expr, Comma> = Punctuated::new();
        exi_frmt_args.push(parse_quote!(#validate_error_format));
        exi_frmt_args.push(parse_quote!(msg));
        let mut uni_frmt_args = exi_frmt_args.clone();
        for field in uni_flds {
            let fld_id = &field.id;
            let par_id = id_to_par(fld_id);
            exi_frmt_args.push(parse_quote!(#par_id));
            uni_frmt_args.push(parse_quote!(self.#fld_id));
        }
        gen.extend(quote! {
            impl<'a> #struct_id {

                // -- validate_exists_field1_and_field3(db, v1, v3, msg) -----
                pub fn #exi_id(#exi_pars) -> Result<(), vicocomo::Error> {
                    match Self::#find_by_id(#find_args) {
                        Some(_) => Ok(()),
                        None => Err(vicocomo::Error::Database(
                            format!(#exi_frmt_args)
                        )),
                    }
                }

                // -- validate_unique_field1_and_field3(db, msg) -------------
                pub fn #uni_id(
                    &self,
                    db: &mut impl vicocomo::DbConn<'a>,
                    msg: &str
                ) -> Result<(), vicocomo::Error> {
                    match self.#find_eq_id(db) {
                        Some(_) => Err(vicocomo::Error::Database(
                            format!(#uni_frmt_args)
                        )),
                        None => Ok(()),
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
