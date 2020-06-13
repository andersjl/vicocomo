use crate::DbConn;
use crate::Error;

#[allow(unused_variables)]
pub trait Delete<'a, PkType> {
    // Return 1 after successfully deleted the corresponding database row.
    //
    fn delete(self, db: &mut impl DbConn<'a>) -> Result<u64, Error>;

    // Return the number of successfully deleted database rows.
    //
    // batch should be a slice of primary key values (or tuples of them if
    // there is more than one primary key field).
    //
    fn delete_batch(
        db: &mut impl DbConn<'a>,
        batch: &[PkType],
    ) -> Result<u64, Error>;
}

#[allow(unused_variables)]
pub trait Find<'a>: Sized {
    // Return a vector with all records in the table in the default order.
    //
    fn load(db: &mut impl DbConn<'a>) -> Result<Vec<Self>, Error>;
}

#[allow(unused_variables)]
pub trait Save<'a>: Sized {
    // Try to INSERT a row in the database from self and update self from the
    // inserted row after insert.
    //
    // The default implementation calls insert_batch().
    //
    // It is an error if self has a primary key that exists in the database.
    //
    fn insert(&mut self, db: &mut impl DbConn<'a>) -> Result<(), Error> {
        *self = Self::insert_batch(db, std::slice::from_ref(self))?
            .pop()
            .unwrap();
        Ok(())
    }

    // Try to INSERT a number of rows in the database from data and return new
    // model structs updated from the inserted rows after insert.
    //
    // The implementation by #[derive(vicocomo::SaveModel)] ensures that any
    // field with the attribute vicocomo_optional will be sent to the database
    // only if it is Some(value).
    //
    // It is an error if any of the data has a primary key that exists in the
    // database.
    //
    fn insert_batch(
        db: &mut impl DbConn<'a>,
        data: &[Self],
    ) -> Result<Vec<Self>, Error>;

    // Save the object's data to the database.
    //
    // If a row with the object's primary key exists in the database, this is
    // equivalent to update().  If not, this is equivalent to insert().
    //
    // The default implementation simply tries first update(), then insert().
    //
    fn save(&mut self, db: &mut impl DbConn<'a>) -> Result<(), Error> {
        self.update(db).or_else(|_e| self.insert(db))
    }

    // Try to UPDATE a row in the database from self and update self from the
    // updated row after insert.
    //
    // It is an error if self lacks a primary key or has one that does not
    // exist in the database.
    //
    fn update(&mut self, db: &mut impl DbConn<'a>) -> Result<(), Error>;
}
