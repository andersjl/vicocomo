pub mod database;
pub mod error;
mod html;
mod model;

pub use database::DbConn;
pub use error::Error;
pub use html::*;
pub use model::{Delete, Find, Save};
pub use vicocomo_proc_macro::{
    configure, DeleteModel, FindModel, PathTag, SaveModel,
};
