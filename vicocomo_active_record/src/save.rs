use crate::model::{Field, Model};
use ::syn::{parse_quote, Expr, ItemFn};

pub(crate) fn save_impl(
    model: &Model,
    struct_fn: &mut Vec<ItemFn>,
    trait_fn: &mut Vec<ItemFn>,
) {
    use ::quote::format_ident;

    let Model {
        struct_id: _,
        ref table_name,
        has_many: _,
        before_delete: _,
        before_save: _,
        ref fields,
        uniques: _,
    } = model;
    let fields = fields.iter().map(|f| f).collect::<Vec<_>>();

    // --- insert code fragments ---------------------------------------------

    let ins_fmt = format!(
        "INSERT INTO {} ({{}}) VALUES {{}} RETURNING {}",
        table_name,
        &model.cols().join(", "),
    );
    let db_types = model.db_types();

    // use local vars: ins_cols, ins_pars
    let ins_placeholders = Model::placeholders_expr(
        parse_quote!(ins_pars.len()),
        parse_quote!(ins_cols.len()),
    );

    let before_insert_expr =
        model.before_save_expr(format_ident!("data_itm"));

    #[allow(non_snake_case)]
    let push_expr__data_itm__none__insert_cols__itm_pars = push_expr(
        fields.as_slice(),
        parse_quote!(data_itm),
        None,
        parse_quote!(insert_cols),
        parse_quote!(itm_pars),
    );

    #[allow(non_snake_case)]
    let rows_to_models_expr__rows =
        model.rows_to_models_expr(parse_quote!(rows));

    // --- update code fragments ---------------------------------------------

    let before_update_expr = model.before_save_expr(format_ident!("self"));

    let upd_fields = model.upd_fields();
    let upd_fmt = format!(
        "UPDATE {} SET {{}} WHERE {{}} RETURNING {}",
        table_name,
        upd_fields
            .iter()
            .map(|f| f.col.value())
            .collect::<Vec<_>>()
            .join(", "),
    );
    let pk_select = model.pk_select();
    let pk_db_values = model.pk_db_values();

    let return_if_self_has_no_primary_key_expr: Expr =
        parse_quote!(if self.pk_value().is_none() {
            return Err(Self::__vicocomo__pk_error(
                ::vicocomo::ModelErrorKind::CannotSave,
                None,
                true,
            ));
        });

    #[allow(non_snake_case)]
    let push_expr__self__par_ix__upd_cols__upd_pars = push_expr(
        upd_fields.as_slice(),
        parse_quote!(self),
        Some(parse_quote!(par_ix)),
        parse_quote!(upd_cols),
        parse_quote!(upd_pars),
    );

    let upd_db_types = model.upd_db_types();

    // use local vars: output
    let upd_output_expr: Vec<Expr> = upd_fields
        .iter()
        .map(|f| {
            let id = &f.id;
            if f.opt {
                parse_quote!(
                    self.#id = Some(::std::convert::TryInto::try_into(
                        output.drain(..1).next().unwrap()
                    )?)
                )
            } else {
                parse_quote!(
                    self.#id = ::std::convert::TryInto::try_into(
                        output.drain(..1).next().unwrap()
                    )?
                )
            }
        })
        .collect();

    struct_fn.push(parse_quote!(
        fn __vicocomo__handle_update_result(
            &mut self,
            db: ::vicocomo::DatabaseIf,
            result: Result<Vec<Vec<::vicocomo::DbValue>>, ::vicocomo::Error>,
        ) -> Result<(), ::vicocomo::Error> {
            result
                .and_then(|mut updated| {
                    if updated.len() == 1 {
                        let mut output = updated
                            .drain(..1)
                            .next()
                            .unwrap();
                        #( #upd_output_expr; )*
                        Ok(())
                    } else {
                        Err(Self::__vicocomo__pk_error(
                            ::vicocomo::ModelErrorKind::CannotSave,
                            ::vicocomo::ActiveRecord::pk_value(self),
                            true,
                        ))
                    }
                })
        }
    ));

    #[allow(non_snake_case)]
    trait_fn.push(parse_quote!(
        fn insert_batch(
            db: ::vicocomo::DatabaseIf,
            data: &mut [Self],
        ) -> Result<Vec<Self>, ::vicocomo::Error> {
            let mut inserts: std::collections::HashMap<
                Vec<String>,
                Vec<Vec<::vicocomo::DbValue>>,
            > = std::collections::HashMap::new();
            for data_itm in data.iter_mut() {
                let mut insert_cols = Vec::new();
                let mut itm_pars: Vec<::vicocomo::DbValue> = Vec::new();
                #before_insert_expr;
                #( #push_expr__data_itm__none__insert_cols__itm_pars )*
                match inserts.get_mut(&insert_cols) {
                    Some(ins_pars) => ins_pars.push(itm_pars),
                    None => { inserts.insert(insert_cols, vec![itm_pars]); },
                }
            }
            let mut error = None;
            let mut result = Vec::new();
            for (ins_cols, ins_pars) in inserts.iter() {
                let mut db_pars = Vec::new();
                for these_pars in ins_pars.iter() {
                    db_pars.extend(these_pars.clone().drain(..));
                }
                match db.query(
                    &format!(
                        #ins_fmt,
                        &ins_cols.join(", "),
                        #ins_placeholders,
                    ),
                    &db_pars,
                    &[ #( #db_types ),* ],
                ) {
                    Ok(rows) => result.extend(#rows_to_models_expr__rows?),
                    Err(err) => {
                        error = Some(err);
                        break;
                    }
                }
            }
            if let Some(err) = error {
                for data_itm in data {
                    if let Some(mapped) =
                        data_itm.__vicocomo__conv_save_error(db, &err, false)
                    {
                        return Err(mapped);
                    }
                }
                Err(err)
            } else {
                Ok(result)
            }
        }
    ));

    if model.pk_fields().is_empty() {
        trait_fn.push(parse_quote!(
            fn update(
                &mut self,
                _db: ::vicocomo::DatabaseIf,
            ) -> Result<(), ::vicocomo::Error> {
                Err(::vicocomo::Error::other("not-available"))
            }
        ));

        trait_fn.push(parse_quote!(
            fn update_columns(
                &mut self,
                _db: ::vicocomo::DatabaseIf,
                _upd_cols: &[(&str, ::vicocomo::DbValue)],
            ) -> Result<(), ::vicocomo::Error> {
                Err(::vicocomo::Error::other("not-available"))
            }
        ));
    } else {
        #[allow(non_snake_case)]
        trait_fn.push(parse_quote!(
            fn update(&mut self, db: ::vicocomo::DatabaseIf)
                -> Result<(), ::vicocomo::Error>
            {
                use ::std::convert::TryInto;

                #return_if_self_has_no_primary_key_expr
                let mut upd_cols: Vec<String> = Vec::new();
                let mut upd_pars = #pk_db_values;
                let mut par_ix = upd_pars.len();
                #before_update_expr;
                #( #push_expr__self__par_ix__upd_cols__upd_pars )*
                self.__vicocomo__handle_update_result(
                    db,
                    db.query(
                        &format!(
                            #upd_fmt,
                            &upd_cols.join(", "),
                            #pk_select,
                        ),
                        &upd_pars,
                        &[ #( #upd_db_types ),* ],
                    )
                    .map_err(|err| {
                        match self.__vicocomo__conv_save_error(db, &err, true) {
                            Some(mapped) => mapped,
                            None => err,
                        }
                    }),
                )
            }
        ));

        trait_fn.push(parse_quote!(
            fn update_columns(
                &mut self,
                db: ::vicocomo::DatabaseIf,
                upd_cols: &[(&str, ::vicocomo::DbValue)],
            ) -> Result<(), ::vicocomo::Error> {
                use ::std::convert::TryInto;

                #return_if_self_has_no_primary_key_expr
                let mut upd_col_sql: Vec<String> = Vec::new();
                let mut upd_pars = #pk_db_values;
                let mut par_ix = upd_pars.len();
                for (col, dbv) in upd_cols {
                    par_ix += 1;
                    upd_col_sql.push(format!("{} = ${}", col, par_ix));
                    upd_pars.push(dbv.clone());
                };
                self.__vicocomo__handle_update_result(
                    db,
                    db.query(
                        &format!(
                            #upd_fmt,
                            &upd_col_sql.join(", "),
                            #pk_select,
                        ),
                        &upd_pars,
                        &[ #( #upd_db_types ),* ],
                    ),
                )
            }
        ));
    }
}

// Push to cols (String) and vals (DbValue) data for fields.
//
// If a field is vicocomo_optional, noop if None, the contained data is pushed
// if Some.
//
// obj should evaluate to the instance to take values from.
//
// If par_ix is None, the column name is pushed to cols. If it is Some(
// expression evaluating to a mutable integer), the integer is incremented
// before "<column name> = $<the new integer value>" is pushed to cols.
//
// cols should evaluate to a mutable Vec<String>.
//
// vals should evaluate to a mutable Vec<DbValue>.
//
fn push_expr(
    fields: &[&Field],
    obj: Expr,
    par_ix: Option<Expr>,
    cols: Expr,
    vals: Expr,
) -> Vec<Expr> {
    fields
        .iter()
        .map(|f| {
            let fld = &f.id;
            let col = &f.col;
            let (par_ix_expr, col_expr): (Expr, Expr) = match par_ix.as_ref()
            {
                Some(expr) => (
                    parse_quote!(#expr += 1),
                    parse_quote!(format!("{} = ${}", #col, #expr)),
                ),
                None => (parse_quote!(()), parse_quote!(#col.to_string())),
            };
            if f.opt {
                parse_quote!(
                    match #obj.#fld.as_ref() {
                        Some(val) => {
                            #par_ix_expr;
                            #cols.push(#col_expr);
                            #vals.push(val.clone().into());
                        },
                        None => (),
                    }
                )
            } else {
                parse_quote!(
                    {
                        #par_ix_expr;
                        #cols.push(#col_expr);
                        #vals.push(#obj.#fld.clone().into());
                    }
                )
            }
        })
        .collect()
}
