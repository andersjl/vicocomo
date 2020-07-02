use crate::model::Model;
use proc_macro::TokenStream;
use syn::{export::Span, Ident};

#[allow(unused_variables)]
pub fn find_model_impl(model: &Model) -> TokenStream {
    use quote::quote;
    use syn::{parse_quote, punctuated::Punctuated, token::Comma, Expr};
    let struct_id = &model.struct_id;
    let table_name = &model.table_name;
    let all_cols =
        model.fields.iter().map(|f| f.col.value()).collect::<Vec<_>>();
    let db_types = model.db_types();
    let pk_fields = &model.pk_fields();
    let pk_type = &model.pk_type();

    // == general functions ==================================================
    // -- load(db) -----------------------------------------------------------
    let default_order = model.default_order();
    let all_cols_join = all_cols.join(", ");
    // SELECT col1, col2, col3 FROM table ORDER BY col3, col1
    let load_sql = format!(
        "SELECT {} FROM {} {}",
        &all_cols_join, table_name, default_order,
    );
    let load_models = model.rows_to_models_expr(
        parse_quote!(db.query(#load_sql, &[], &[ #( #db_types ),* ])?),
    );
    let query_sql = format!(
        "SELECT {} FROM {} {{}} {{}} {{}} {{}}",
        &all_cols_join, &table_name,
    );
    let found_models =
        model.rows_to_models_expr(parse_quote!(found_rows)/*, None*/);
    let pk_self_to_tuple = model.pk_self_to_tuple();
    let find_pk_sql = model.find_sql(
        model
            .pk_fields()
            .iter()
            .map(|f| f.col.value())
            .collect::<Vec<_>>()
            .as_slice(),
    );
    let pk_len = pk_fields.len();
    let pk_iter = (0..pk_len).map(|ix| syn::Index::from(ix));
    let mut pk_values: Punctuated<Expr, Comma> = Punctuated::new();
    if pk_len == 1 {
        pk_values.push(parse_quote!((*pk).into()));
    } else {
        for ix in (0..pk_len).map(|ix| syn::Index::from(ix)) {
            pk_values.push(parse_quote!(pk.#ix.into()));
        }
    }
    let find_model = model.rows_to_models_expr(parse_quote!(outp)/*, None*/);
    let mut gen = quote! {
        impl<'a> vicocomo::MdlFind<'a, #pk_type> for #struct_id {
            fn find(db: &mut impl DbConn<'a>, pk: &#pk_type) -> Option<Self> {
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

            fn find_equal(&self, db: &mut impl DbConn<'a>) -> Option<Self> {
                #pk_self_to_tuple.and_then(|tup| Self::find(db, &tup))
            }

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
                    db.query(&sql, &values, &[ #( #db_types ),* ])?;
                #found_models
            }
        }
    };

    // == unique field functions =============================================
    for uni_flds in model.unique_fields() {
        if uni_flds[0].pri {
            continue;
        }
        use syn::FnArg;
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
            uni_cols.push(field.col.value());
        }

        // -- finding --------------------------------------------------------
        let find_uni_sql = model.find_sql(&uni_cols);
        let find_by_str = format!("find_by_{}", uni_str);
        let find_by_id = Ident::new(find_by_str.as_str(), Span::call_site());
        let find_eq_str = format!("find_equal_{}", uni_str);
        let find_eq_id = Ident::new(find_eq_str.as_str(), Span::call_site());
        gen.extend(quote! {
            impl<'a> #struct_id {

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
                pub fn #find_eq_id(&self, db: &mut impl vicocomo::DbConn<'a>)
                -> Option<Self> {
                    Self::#find_by_id(#self_args)
                }
            }
        });

        // -- validating -----------------------------------------------------
        let val_uni_str = format!("validate_unique_{}", uni_str);
        let uni_id = Ident::new(val_uni_str.as_str(), Span::call_site());
        let val_exi_str = format!("validate_exists_{}", uni_str);
        let exi_id = Ident::new(val_exi_str.as_str(), Span::call_site());
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
