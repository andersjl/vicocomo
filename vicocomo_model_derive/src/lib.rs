extern crate proc_macro;
extern crate syn;

use proc_macro::TokenStream;

mod delete;
mod find;
mod model;
mod save;

// model helper macros -------------------------------------------------------
//
//     #[derive(<any combination of the below>)]
//     #[vicoomo_table_name = "example_table"]  // default "examples"
//     struct Example {
//         #[vicocomo_optional]       // not sent to DBMS if None
//         #[vicocomo_primary]        // To find a row to update() or delete()
//         primary: Option<u32>,      // primary key should be ensured by DBMS
//         #[vicocomo_column = "db_col"]  // different name of DB column
//         #[vicocomo_unique = "un1"] // "un1" labels fields w unique comb.
//         not_null: String,          // VARCHAR NOT NULL
//         #[vicocomo_order_by(2)]    // precedence 2, see opt_null below
//         nullable: Option<String>,  // VARCHAR, None -> NULL
//         #[vicocomo_optional]       // not sent to DBMS if None
//         #[vicocomo_unique = "un1"] // UNIQUE(db_col, opt_not_null)
//         opt_not_null: Option<i32>  // INTEGER NOT NULL DEFAULT 42
//         #[vicocomo_order_by(1, "desc")] // ORDER BY opt_null DESC, nullable
//         #[vicocomo_optional]       // not sent to DBMS if None
//         opt_null: Option<Option<i32>>  // INTEGER DEFAULT 43
//                                    // None -> 43, Some(None) -> NULL
//     }
//
#[proc_macro_derive(
    DeleteModel,
    attributes(
        vicocomo_column,
        vicocomo_optional,
        vicocomo_table_name,
        vicocomo_unique,
    )
)]
pub fn delete_model_derive(input: TokenStream) -> TokenStream {
    delete::delete_model_impl(&model::Model::new(
        input,
        vec![model::ExtraInfo::UniqueFields],
    ))
}

#[proc_macro_derive(
    FindModel,
    attributes(
        vicocomo_column,
        vicocomo_optional,
        vicocomo_order_by,
        vicocomo_primary,
        vicocomo_table_name,
        vicocomo_unique
    )
)]
pub fn find_model_derive(input: TokenStream) -> TokenStream {
    find::find_model_impl(&model::Model::new(
        input,
        vec![
            model::ExtraInfo::OrderFields,
            model::ExtraInfo::UniqueFields,
            model::ExtraInfo::DatabaseTypes,
        ],
    ))
}

#[proc_macro_derive(
    SaveModel,
    attributes(
        vicocomo_column,
        vicocomo_optional,
        vicocomo_primary,
        vicocomo_table_name
    )
)]
pub fn save_model_derive(input: TokenStream) -> TokenStream {
    save::save_model_impl(&model::Model::new(
        input,
        vec![model::ExtraInfo::DatabaseTypes],
    ))
}
