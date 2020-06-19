mod database;
mod error;
mod html;
mod model;

pub use database::{DbConn, DbType, DbValue};
pub use error::Error;
pub use html::*;
pub use model::{
    MdlDelete, MdlFind, MdlOrder, MdlQuery, MdlQueryBld, MdlSave,
};
pub use vicocomo_proc_macro::{
    db_value_convert, DeleteModel, FindModel, PathTag, SaveModel,
};
