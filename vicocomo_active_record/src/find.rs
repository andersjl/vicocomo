use crate::model::{Model, OnNone};
use ::syn::ItemFn;

pub(crate) fn find_impl(
    model: &Model,
    struct_fn: &mut Vec<ItemFn>,
    trait_fn: &mut Vec<ItemFn>,
) {
    use ::quote::format_ident;
    use ::syn::{parse_quote, punctuated::Punctuated, token::Comma, Expr};

    let table_name = &model.table_name;
    let all_cols = model
        .fields
        .iter()
        .map(|f| f.col.value())
        .collect::<Vec<_>>();
    let db_types = model.db_types();
    let pk_fields = &model.pk_fields();

    // == general functions ==================================================

    let default_order = model.default_order();
    let all_cols_join = all_cols.join(", ");

    // SELECT <all> FROM <table> [ ORDER BY <default> ]
    let load_sql = format!(
        "SELECT {} FROM {} {}",
        &all_cols_join, table_name, default_order,
    );
    let load_models = model.rows_to_models_expr(
        parse_quote!(db.clone().query(#load_sql, &[], &[ #( #db_types ),* ])?),
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
    let pk_value_self_expr = model.pk_value(parse_quote!(self));
    if pk_len == 0 {
        trait_fn.push(parse_quote!(
            fn find(
                db: ::vicocomo::DatabaseIf,
                pk: &Self::PkType,
            ) -> Option<Self> {
                None
            }
        ));
        trait_fn.push(parse_quote!(
            fn find_equal(&self, db: ::vicocomo::DatabaseIf) -> Option<Self> {
                None
            }
        ));
    } else {
        let find_pk_sql = model.find_sql(
            model
                .pk_fields()
                .iter()
                .map(|f| f.col.value())
                .collect::<Vec<_>>()
                .as_slice(),
        );
        let mut pk_db_values: Punctuated<Expr, Comma> = Punctuated::new();
        if pk_len == 1 {
            pk_db_values.push(parse_quote!(pk.clone().into()));
        } else {
            for ix in (0..pk_len).map(|ix| syn::Index::from(ix)) {
                pk_db_values.push(parse_quote!(pk.#ix.clone().into()));
            }
        }

        trait_fn.push(parse_quote!(
            fn find(db: ::vicocomo::DatabaseIf, pk: &Self::PkType)
                -> Option<Self>
            {
                match db.clone().query(
                    #find_pk_sql,
                    &[ #pk_db_values ],
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
        ));
        trait_fn.push(parse_quote!(
            fn find_equal(&self, db: ::vicocomo::DatabaseIf)
                -> Option<Self>
            {
                #pk_value_self_expr.and_then(|tup| {
                    Self::find(db.clone(), &tup)
                })
            }
        ));
    }
    trait_fn.push(parse_quote!(
        fn load(
            db: ::vicocomo::DatabaseIf,
        ) -> Result<Vec<Self>, ::vicocomo::Error> {
            #load_models
        }
    ));
    trait_fn.push(parse_quote!(
        fn query(
            db: ::vicocomo::DatabaseIf,
            query: &::vicocomo::Query
        ) -> Result<Vec<Self>, ::vicocomo::Error> {
            let filter = match query.filter() {
                Some(f) => format!("WHERE {}", f),
                None => String::new(),
            };
            let limit = match query.limit() {
                Some(l) => format!("LIMIT {}", l),
                // E.g. SQLite requires LIMIT if OFFSET
                None if query.offset().is_some() => {
                    "LIMIT 2147483647".to_string()
                },
                _ => String::new(),
            };
            let offset = match query.offset() {
                Some(l) => format!("OFFSET {}", l),
                None => String::new(),
            };
            let order = match query.order() {
                ::vicocomo::Order::Custom(ord) =>
                    format!("ORDER BY {}", ord),
                ::vicocomo::Order::Dflt =>
                    #default_order.to_string(),
                ::vicocomo::Order::NoOrder => String::new(),
            };
            let mut values: Vec<::vicocomo::DbValue> = Vec::new();
            for opt in query.values() {
                match opt {
                    Some(v) => values.push(v.clone()),
                    None => return Err(::vicocomo::Error::invalid_input(
                        "database--Query--value-missing",
                    )),
                }
            }
            let sql = format!(#query_sql, filter, order, limit, offset);
            let mut found_rows =
                db.clone().query(&sql, &values, &[ #( #db_types ),* ])?;
            #found_models
        }
    ));

    // == unique field functions =============================================

    for unique in &model.uniques {
        let uni_flds = &unique.fields;
        let find_by_id = &unique.find_by_id;
        let find_eq_id = &unique.find_eq_id;
        let self_args = &unique.find_self_args;
        use ::syn::FnArg;
        let mut uni_cols = Vec::new();
        let mut find_pars: Punctuated<FnArg, Comma> = Punctuated::new();
        let mut par_vals: Punctuated<Expr, Comma> = Punctuated::new();
        let mut self_test: Vec<Expr> = Vec::new();
        find_pars.push(parse_quote!(db: ::vicocomo::DatabaseIf));
        for field in uni_flds {
            let fld_id = &field.id;
            let par_id = format_ident!("{}_par", fld_id);
            let par_ty = if field.onn == OnNone::Null {
                &field.ty
            } else {
                &Model::strip_option(&field.ty)
            };
            find_pars.push(parse_quote!(#par_id: &#par_ty));
            par_vals.push(parse_quote!(#par_id.clone().into()));
            if field.onn != OnNone::Null {
                self_test.push(parse_quote!(self.#fld_id.is_some()));
            }
            uni_cols.push(field.col.value());
        }

        // -- finding --------------------------------------------------------

        let find_uni_sql = model.find_sql(&uni_cols);

        struct_fn.push(parse_quote!(
            // -- find_by_field1_and_field3(db, v1, v3) ------------------
            pub fn #find_by_id(#find_pars) -> Option<Self> {
                match db.clone().query(
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
        ));
        struct_fn.push(parse_quote!(
            // -- find_equal_field1_and_field3(db) -----------------------
            pub fn #find_eq_id(&self, db: ::vicocomo::DatabaseIf)
            -> Option<Self> {
                if true #( && #self_test )* {
                    Self::#find_by_id( #( #self_args ),* )
                } else {
                    None
                }
            }
        ));
    }
}
