// TODO: test optional unique field without value

pub mod belongs_to;
pub use belongs_to::test_belongs_to;
pub mod delete;
pub use delete::test_delete;
pub mod many_to_many;
pub use many_to_many::test_many_to_many;
pub mod models;
pub use models::setup;
pub mod multi_pk;
pub use multi_pk::test_multi_pk;
pub mod no_pk;
pub use no_pk::test_no_pk;
pub mod nonstandard_parent;
pub use nonstandard_parent::test_nonstandard_parent;
pub mod one_to_many;
pub use one_to_many::test_one_to_many;
pub mod single_pk;
pub use single_pk::test_single_pk;
