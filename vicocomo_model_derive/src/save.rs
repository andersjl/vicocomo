use crate::model::Model;
use proc_macro::TokenStream;

#[allow(unused_variables)]
pub(crate) fn save_impl(model: &Model) -> TokenStream {
    use quote::quote;
    use syn::{parse_quote, Expr};
    let struct_id = &model.struct_id;
    let table_name = &model.table_name;
    let db_types = model.db_types();
    let insert_fmt = format!(
        "INSERT INTO {} ({{}}) VALUES {{}} RETURNING {}",
        &model.table_name,
        &model.cols().join(", "),
    );
    let upd_fields = model.upd_fields();
    let update_fmt = format!(
        "UPDATE {} SET {{}} WHERE {{}} RETURNING {}",
        &model.table_name,
        upd_fields
            .iter()
            .map(|f| f.col.value())
            .collect::<Vec<_>>()
            .join(", "),
    );
    let ins_placeholders = Model::placeholders_expr(
        parse_quote!(ins_pars.len()),
        parse_quote!(insert_cols.len()),
    );
    let pk_select = model.pk_select();
    let pk_values = model.pk_values();
    let insert_push_expr: Vec<Expr> = model
        .fields
        .iter()
        .map(|f| {
            let id = &f.id;
            let col = &f.col;
            if f.opt {
                parse_quote!(
                    match &data_itm.#id {
                        Some(val) => {
                            insert_cols.push(#col.to_string());
                            pars.push(val.clone().into());
                        },
                        None => (),
                    }
                )
            } else {
                parse_quote!(
                    {
                        insert_cols.push(#col.to_string());
                        pars.push(data_itm.#id.clone().into());
                    }
                )
            }
        })
        .collect();
    let insert_do_it = model.rows_to_models_expr(parse_quote!(
        db.query(
            &format!(
                #insert_fmt,
                &insert_cols.join(", "),
                #ins_placeholders,
            ),
            &params,
            &[ #( #db_types ),* ],
        )?
    ));
    let update_input_expr: Vec<Expr> = upd_fields
        .iter()
        .map(|f| {
            let id = &f.id;
            let col = &f.col;
            if f.opt {
                parse_quote!(
                    match &self.#id {
                        Some(val) => {
                            par_ix += 1;
                            update_cols.push(
                                format!("{} = ${}", #col, par_ix)
                            );
                            params.push(val.clone().into());
                        }
                        None => (),
                    }
                )
            } else {
                parse_quote!(
                    {
                        par_ix += 1;
                        update_cols.push(format!("{} = ${}", #col, par_ix));
                        params.push(self.#id.clone().into());
                    }
                )
            }
        })
        .collect();
    let upd_db_types = model.upd_db_types();
    let update_output_expr: Vec<Expr> = upd_fields
        .iter()
        .map(|f| {
            let id = &f.id;
            if f.opt {
                parse_quote!(
                    self.#id =
                        Some(output.drain(..1).next().unwrap().try_into()?)
                )
            } else {
                parse_quote!(
                    self.#id =
                        output.drain(..1).next().unwrap().try_into()?
                )
            }
        })
        .collect();
    let update_err = Model::query_err("update");

    let gen = quote! {
        impl ::vicocomo::Save for #struct_id {
            fn insert_batch(
                db: &impl ::vicocomo::DbConn,
                data: &[Self],
            ) -> Result<Vec<Self>, ::vicocomo::Error> {
                let mut inserts: std::collections::HashMap<
                    Vec<String>,
                    Vec<Vec<::vicocomo::DbValue>>,
                > = std::collections::HashMap::new();
                for data_itm in data {
                    let mut insert_cols = Vec::new();
                    let mut pars: Vec<::vicocomo::DbValue> = Vec::new();
                    #( #insert_push_expr )*
                    match inserts.get_mut(&insert_cols) {
                        Some(ins_pars) => ins_pars.push(pars),
                        None => { inserts.insert(insert_cols, vec![pars]); },
                    }
                }
                let mut result = Vec::new();
                for (insert_cols, ins_pars) in inserts.iter_mut() {
                    let mut params = Vec::new();
                    for these_pars in ins_pars.iter_mut() {
                        params.extend(these_pars.drain(..));
                    }
                    let mut models = #insert_do_it?;
                    result.extend(models);
                }
                Ok(result)
            }

            fn update(&mut self, db: &impl ::vicocomo::DbConn)
                -> Result<(), ::vicocomo::Error>
            {
                use std::convert::TryInto;
                let mut params = #pk_values;
                let mut par_ix = params.len();
                let mut update_cols: Vec<String> = Vec::new();
                #( #update_input_expr )*
                let mut updated = db
                    .query(
                        &format!(
                            #update_fmt,
                            &update_cols.join(", "),
                            #pk_select,
                        ),
                        &params,
                        &[ #( #upd_db_types ),* ],
                    )?;
                if updated.is_empty() {
                   return Err(::vicocomo::Error::Database(
                        format!(#update_err, 0, 1)
                    ));
                }
                let mut output = updated
                    .drain(..1)
                    .next()
                    .unwrap();
                #( #update_output_expr; )*
                Ok(())
            }
        }
    };
    gen.into()
}
