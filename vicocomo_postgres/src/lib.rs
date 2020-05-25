use postgres;
use postgres_types;
use vicocomo::{
    database::{DbConn, DbTrans, Type, Value},
    error::Error,
};

pub struct PgConn(postgres::Client);

macro_rules! from_values {
    ($params:expr) => {
        &$params
            .iter()
            .map(|val| match val {
                Value::Float(v) => v as &(dyn postgres_types::ToSql + Sync),
                Value::Int(v) => v as &(dyn postgres_types::ToSql + Sync),
                Value::Text(v) => v as &(dyn postgres_types::ToSql + Sync),
                Value::NulFloat(v) => {
                    v as &(dyn postgres_types::ToSql + Sync)
                }
                Value::NulInt(v) => v as &(dyn postgres_types::ToSql + Sync),
                Value::NulText(v) => v as &(dyn postgres_types::ToSql + Sync),
            })
            .collect::<Vec<_>>()[..]
    };
}

impl PgConn {
    pub fn connect(conn_str: &str) -> Result<Self, Error> {
        Ok(Self(
            postgres::Client::connect(conn_str, postgres::NoTls)
                .map_err(|e| Error::Database(e.to_string()))?,
        ))
    }

    pub fn connection(&mut self) -> &mut postgres::Client {
        &mut self.0
    }
}

impl<'a> DbConn<'a> for PgConn {
    fn exec(&mut self, sql: &str, params: &[Value]) -> Result<u64, Error> {
/*
print!("PgConn.0.exec(\n    {:?},\n    {:?},\n)", sql, from_values!(params));
let result =
*/
        self.0
            .execute(sql, from_values!(params))
            .map_err(|e| Error::Database(e.to_string()))
/*
; println!(" -> {:?}", result); result
*/
    }

    fn query(
        &mut self,
        sql: &str,
        params: &[Value],
        types: &[Type],
    ) -> Result<Vec<Vec<Value>>, Error> {
/*
print!("PgConn.0.query(\n    {:?},\n    {:?},\n)", sql, from_values!(params));
let result =
*/
        Ok(do_query(self.0.query(sql, from_values!(params)), types)?)
/*
; println!(" -> {:?}", result); result
*/
    }

    fn transaction(
        &'a mut self,
        statements: fn(Box<dyn DbTrans + 'a>) -> Result<(), Error>,
    ) -> Result<(), Error> {
        statements(Box::new(PgTrans(
            self.0
                .transaction()
                .map_err(|e| Error::Database(e.to_string()))?,
        )))
    }
}

struct PgTrans<'a>(postgres::Transaction<'a>);

impl<'a> DbTrans<'a> for PgTrans<'a> {
    fn commit(self: Box<Self>) -> Result<(), Error> {
        self.0.commit().map_err(|e| Error::Database(e.to_string()))
    }

    fn exec(&mut self, sql: &str, params: &[Value]) -> Result<u64, Error> {
        self.0
            .execute(sql, from_values!(params))
            .map_err(|e| Error::Database(e.to_string()))
    }

    fn query(
        &mut self,
        sql: &str,
        params: &[Value],
        types: &[Type],
    ) -> Result<Vec<Vec<Value>>, Error> {
        Ok(do_query(self.0.query(sql, from_values!(params)), types)?)
    }

    fn rollback(self: Box<Self>) -> Result<(), Error> {
        self.0
            .rollback()
            .map_err(|e| Error::Database(e.to_string()))
    }

    fn transaction(
        &'a mut self,
        statements: fn(Box<dyn DbTrans + 'a>) -> Result<(), Error>,
    ) -> Result<(), Error> {
        statements(Box::new(PgTrans(
            self.0
                .transaction()
                .map_err(|e| Error::Database(e.to_string()))?,
        )))
    }
}

fn do_query(
    pg_rows: Result<Vec<postgres::Row>, postgres::Error>,
    types: &[Type],
) -> Result<Vec<Vec<Value>>, Error> {
    //print!("vicocomo_postgres::do_query(\n    {:?},\n    {:?},\n)", pg_rows, types);
    let mut vicocomo_rows = vec![];
    for postgres_row in
        pg_rows.map_err(|e| Error::Database(e.to_string()))?.iter()
    {
        let mut vicocomo_row = vec![];
        for (ix, typ) in types.iter().enumerate() {
            vicocomo_row.push(match typ {
                Type::Float => Value::Float(
                    postgres_row
                        .try_get::<_, f64>(ix)
                        .map_err(|e| Error::Database(e.to_string()))?,
                ),
                Type::Int => Value::Int(
                    postgres_row
                        .try_get::<_, i64>(ix)
                        .map_err(|e| Error::Database(e.to_string()))?,
                ),
                Type::Text => Value::Text(
                    postgres_row
                        .try_get::<_, String>(ix)
                        .map_err(|e| Error::Database(e.to_string()))?,
                ),
                Type::NulFloat => Value::NulFloat(
                    postgres_row
                        .try_get::<_, Option<f64>>(ix)
                        .map_err(|e| Error::Database(e.to_string()))?,
                ),
                Type::NulInt => Value::NulInt(
                    postgres_row
                        .try_get::<_, Option<i64>>(ix)
                        .map_err(|e| Error::Database(e.to_string()))?,
                ),
                Type::NulText => Value::NulText(
                    postgres_row
                        .try_get::<_, Option<String>>(ix)
                        .map_err(|e| Error::Database(e.to_string()))?,
                ),
            });
        }
        vicocomo_rows.push(vicocomo_row);
    }
    //println!( " -> {:?}", vicocomo_rows);
    Ok(vicocomo_rows)
}
