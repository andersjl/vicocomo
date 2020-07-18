use diesel::result::QueryResult;
pub use vicocomo_derive::{CreateModel, DeleteModel, QueryModel};

pub trait QueryModel<Connection>: Sized {
    // Return a vector with all records in the table.
    fn load(db: &Connection) -> QueryResult<Vec<Self>>;
}

pub trait CreateModel<Connection, NewStruct> {
    // Return a struct with data from an inserted database row.
    fn create(db: &Connection, data: &NewStruct) -> QueryResult<Box<Self>>;

    // Return the number of inserted database rows.
    fn create_batch(
        db: &Connection,
        data: &[NewStruct],
    ) -> QueryResult<usize>;
}

pub trait DeleteModel<Connection, PkType> {
    // Return 1 after successfully deleted the corresponding database row.
    fn delete(self, db: &Connection) -> QueryResult<usize>;

    // Return the numbor of successfulle deleted database rows.
    fn delete_batch(db: &Connection, batch: &[PkType]) -> QueryResult<usize>;
}
