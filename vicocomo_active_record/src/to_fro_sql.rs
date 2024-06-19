use crate::model::{Model, OnNone};
use proc_macro2::Span;
use syn::{parse_quote, Expr, ItemFn, LitBool, LitStr};

pub(crate) fn to_fro_sql_impl(model: &Model, trait_fn: &mut Vec<ItemFn>) {
    let model_name =
        LitStr::new(&model.struct_id.to_string(), Span::call_site());
    let table = LitStr::new(&model.table_name, Span::call_site());
    let readonly = LitBool::new(model.readonly, Span::call_site());
    let mut col_name = Vec::new();
    let mut db_type = Vec::new();
    let mut field_value: Vec<Expr> = Vec::new();
    for fld in &model.fields {
        col_name.push(fld.col.clone());
        db_type.push(fld.dbt.path());
        field_value.push({
            let id = &fld.id;
            let id_name = LitStr::new(&id.to_string(), Span::call_site());
            let val: Expr = if fld.onn == OnNone::Null {
                parse_quote!(self.#id)
            } else {
                // strip (one) option, error if None
                parse_quote!(
                    match self.#id.as_ref() {
                        Some(val) => val,
                        None => {
                            return Err(::vicocomo::model_error!(
                                Invalid,
                                #model_name: "",
                                #id_name: ["optional-value-required"],
                            ));
                        }
                    }
                )
            };
            if fld.ser {
                parse_quote!(::vicocomo::JsonField(#val.clone()).into())
            } else {
                parse_quote!(#val.clone().into())
            }
        });
    }

    trait_fn.push(parse_quote!(
        fn col_type(col: &str) -> Option<::vicocomo::DbType> {
            match col {
            #(  #col_name => Some(#db_type), )*
                _ => None,
            }
        }
    ));

    trait_fn.push(parse_quote!(
        fn columns() -> Vec<String> {
            let mut result = Vec::new();
        #(  result.push(#col_name.to_string()); )*
            result
        }
    ));

    trait_fn.push(parse_quote!(
        fn readonly() -> bool {
            #readonly
        }
    ));

    trait_fn.push(parse_quote!(
        fn table() -> String {
            #table.to_string()
        }
    ));

    trait_fn.push(parse_quote!(
        fn values(
            &self,
        ) -> Result<Vec<::vicocomo::DbValue>, ::vicocomo::Error> {

            let mut result = Vec::new();
        #(  result.push(#field_value); )*
            Ok(result)
        }
    ));
}
