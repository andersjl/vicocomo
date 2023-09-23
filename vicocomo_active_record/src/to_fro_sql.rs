use crate::model::Model;
use proc_macro2::Span;
use syn::{parse_quote, Expr, ItemFn, LitStr};

pub(crate) fn to_fro_sql_impl(model: &Model, trait_fn: &mut Vec<ItemFn>) {
    let cols = model.cols().join(", ");
    let insert_fmt = LitStr::new(
        &format!(
            "INSERT INTO {} ({}) VALUES {{}};",
            &model.table_name, &cols
        ),
        Span::call_site(),
    );
    let delete_sql = LitStr::new(
        &format!("DELETE FROM {};", &model.table_name),
        Span::call_site(),
    );
    let fld_val: Vec<Expr> = model
        .fields
        .iter()
        .map(|fld| {
            let id = &fld.id;
            if fld.opt {
                // strip (one) option, it cannot be None after load()
                parse_quote!({
                    let db_val: ::vicocomo::DbValue =
                        obj.#id.as_ref().unwrap().clone().into();
                    db_val.sql_value()
                })
            } else {
                parse_quote!({
                    let db_val: ::vicocomo::DbValue = obj.#id.clone().into();
                    db_val.sql_value()
                })
            }
        })
        .collect();

    trait_fn.push(parse_quote!(
        fn to_sql(
            db: ::vicocomo::DatabaseIf,
        ) -> Result<String, ::vicocomo::Error> {
            //Ok(String::new())
            Ok(format!(
                #insert_fmt,
                {
                    let mut obj_vals = Vec::new();
                    for obj in Self::load(db)?.iter() {
                        obj_vals.push(format!(
                            "({})",
                            {
                                let mut fld_vals = Vec::new();
                            #(  fld_vals.push(#fld_val);)*
                                fld_vals.join(", ")
                            },
                        ));
                    }
                    obj_vals.join(", ")
                },
            ))
        }
    ));

    if !model.readonly {
        trait_fn.push(parse_quote!(
            fn try_from_sql(
                db: ::vicocomo::DatabaseIf,
                sql: &str,
            ) -> Result<(), ::vicocomo::Error> {
                db.clone().exec(#delete_sql, &[])?;
                db.exec(sql, &[])?;
                Ok(())
            }
        ));
    }
}
