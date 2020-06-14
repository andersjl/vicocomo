use crate::{model::Model, utils::*};
use proc_macro::TokenStream;

#[allow(unused_variables)]
pub fn save_model_impl(model: &Model) -> TokenStream {
    use quote::quote;
    //  use syn::{export::Span, LitStr};
    use syn::parse_quote;
    //println!("Save");
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
    let insert_fmt = format!(
        "INSERT INTO {} ({{}}) VALUES {{}} RETURNING {}",
        &model.table_name,
        &all_cols.join(", "),
    );
    let update_fmt = format!(
        "UPDATE {} SET {{}} WHERE {{}} RETURNING {}",
        &model.table_name,
        &all_upd_cols.join(", "),
    );
    let update_err = query_err("update");
    let ins_placeholders = placeholders_expr(
        parse_quote!(ins_pars.len()),
        parse_quote!(ins_cols2.len()),
    );
    let pk_cols_params = pk_cols_params_expr(
        pk_mand_cols,
        pk_mand_fields,
        pk_opt_cols,
        pk_opt_field_names,
        pk_opt_fields,
    );
    let get_models = rows_to_models_expr(
        parse_quote!(
            db.query(
                &format!(
                    #insert_fmt,
                    &ins_cols2.join(", "),
                    #ins_placeholders,
                ),
                &params,
                &[ #( #all_db_types ),* ],
            )?
        ),
        all_mand_fields.as_slice(),
        all_opt_fields.as_slice(),
    );
    let gen = quote! {
                impl<'a> vicocomo::MdlSave<'a> for #struct_id {
                    fn insert_batch(
                        db: &mut impl vicocomo::DbConn<'a>,
                        data: &[Self],
                    ) -> Result<Vec<Self>, vicocomo::Error> {
                        let mut inserts: std::collections::HashMap<
                            Vec<String>,
                            Vec<Vec<vicocomo::DbValue>>,
                        > = std::collections::HashMap::new();
                        for data_itm in data {
                            let mut ins_cols1 = vec![];
                            let mut pars: Vec<vicocomo::DbValue> = vec![];
                            #(
                                ins_cols1.push(#all_mand_cols.to_string());
                                pars.push(data_itm.#all_mand_fields.clone().into());
                            )*
                            #(
                                match &data_itm.#all_opt_fields {
                                    Some(val) => {
                                        ins_cols1.push(#all_opt_cols.to_string());
                                        pars.push(val.clone().into());
                                    },
                                    None => (),
                                }
                            )*
                            match inserts.get_mut(&ins_cols1) {
                                Some(ins_pars) => ins_pars.push(pars),
                                None => { inserts.insert(ins_cols1, vec![pars]); },
                            }
                        }
                        let mut result = vec![];
                        for (ins_cols2, ins_pars) in inserts.iter_mut() {
                            let mut params = vec![];
                            for these_pars in ins_pars.iter_mut() {
                                params.extend(these_pars.drain(..));
                            }
                        /*
                        let mut outputs = db.query(
                            &format!(
                                #insert_fmt,
                                &ins_cols2.join(", "),
                                #ins_placeholders,
                            ),
                            &params,
                            &[ #( #all_db_types ),* ],
                        )?;
    println!(
        "\nquery(\n    {:?},\n    &{:?},    \n    &{:?},\n) -> {:?}",
        &format!(#insert_fmt, &ins_cols2.join(", "), #ins_placeholders),
        &params,
        [ #( #all_db_types ),* ],
        &outputs,
    );
                        */
                            let mut models = #get_models?;
                            result.extend(models);
                        }
                        Ok(result)
                    }

                    fn update(&mut self, db: &mut impl vicocomo::DbConn<'a>)
                        -> Result<(), vicocomo::Error>
                    {
                        let (pk_cols, pk_params, mut par_ix) = #pk_cols_params;
                        let pk_cols = pk_cols.join(" AND ");
                        let mut params = pk_params.clone();
                        let mut upd_cols: Vec<String> = vec![];
                        #(
                            par_ix += 1;
                            upd_cols.push(
                                format!("{} = ${}", #upd_mand_cols, par_ix)
                            );
                            params.push(self.#upd_mand_fields.clone().into());
                        )*
                        #(
                            match &self.#upd_opt_fields {
                                Some(val) => {
                                    par_ix += 1;
                                    upd_cols.push(
                                        format!("{} = ${}", #upd_opt_cols, par_ix)
                                    );
                                    params.push(val.clone().into());
                                }
                                None => (),
                            }
                        )*
    /*
    print!(
        "db.query(\n    {:?},\n    &{:?},\n    &{:?},\n)",
        format!(#update_fmt, &upd_cols.join(", "), &pk_cols),
        params,
        [ #( #all_upd_db_types ),* ],
    );
    */
                        let mut updated = db
                            .query(
                                &format!(#update_fmt, &upd_cols.join(", "), &pk_cols,
                                ),
                                &params,
                                &[ #( #all_upd_db_types ),* ],
                            )?;
                        if updated.is_empty() {
                            return Err(vicocomo::Error::Database(
                                format!(#update_err, 0, 1)
                            ));
                        }
                        let mut output = updated
                            .drain(..1)
                            .next()
                            .unwrap();
    //println!(" -> {:?}", output);
                        #(
                            self.#upd_mand_fields =
                                output.drain(..1).next().unwrap().try_into()?;
                        )*
                        #(
                            self.#upd_opt_fields =
                                Some(output.drain(..1).next().unwrap().try_into()?);
                        )*
                        Ok(())
                    }
                }
            };
    gen.into()
}
