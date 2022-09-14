//! # For HTTP server adapter developers only
//!

/// # For HTTP server adapter developers only
///
/// Methods to store a session. Use in a server specific [`config`
/// ](struct.Config.html) macro to enable session store plugin options.
///
pub trait Session {
    /// Clear the entire session.
    ///
    fn clear(&self);

    /// Retreive the value for `key` or `None` if not present.
    ///
    fn get(&self, key: &str) -> Option<String>;

    /// Remove the `key`-value pair.
    ///
    fn remove(&self, key: &str);

    /// Set a `value` for `key`.
    ///
    fn set(&self, key: &str, value: &str) -> Result<(), Error>;
}

/// # For HTTP server adapter developers only
///
/// An implementation of [`Session`](trait.Session.html) that does
/// nothing and returns `()`, `None`, or [`Error`](../error/enum.Error.html).
///
#[derive(Clone, Copy, Debug)]
pub struct NullSession;

impl Session for NullSession {
    fn clear(&self) {
        ()
    }
    fn get(&self, _key: &str) -> Option<String> {
        None
    }
    fn remove(&self, _key: &str) {
        ()
    }
    fn set(&self, _key: &str, _value: &str) -> Result<(), Error> {
        Err(Error::other("no session store defined"))
    }
}

use crate::{map_error, DatabaseIf, DbType, Error};
use ::chrono::{Duration, Local, NaiveDateTime};
use ::rand::{thread_rng, Rng};
use std::{collections::HashMap, convert::TryFrom};

const INSERT: &'static str =
    "INSERT INTO __vicocomo__sessions (id, data, time) VALUES ($1, $2, $3)";
const PRUNE: &'static str =
    "DELETE FROM __vicocomo__sessions WHERE time < $1";
const ROW_COUNT: &'static str = "SELECT COUNT(id) FROM __vicocomo__sessions";
const SELECT: &'static str =
    "SELECT data FROM __vicocomo__sessions WHERE id = $1";
const TOUCH: &'static str =
    "UPDATE __vicocomo__sessions SET time = $2 WHERE id = $1";
const UPDATE: &'static str =
    "UPDATE __vicocomo__sessions SET data = $2, time = $3 WHERE id = $1";

/// # For HTTP server adapter developers only
///
/// Intended for implementing a [`Session`](trait.Session.html) that stores
/// all data in a database table `"__vicocomo__sessions"`.  The table has
/// three columns, `id` storing a 64 bit integer primary key, `data` storing
/// the serialized session data as an unlimited UTF-8 text, and `time` storing
/// the last access time as a 64 bit integer.
///
pub struct DbSession<'d> {
    db: DatabaseIf<'d>,
    id: i64,
    cache: HashMap<String, String>,
}

impl<'d> DbSession<'d> {
    /// Try to create.
    ///
    /// `db` is remembered for use by other methods.
    ///
    /// `id` is a key to the database row for this session, typically
    /// retrieved from a cookie session. If given, we try to retrieve session
    /// data from the database to a cache in the returned object.
    ///
    /// `prune`, if positive, removes all session data older than that many
    /// seconds from the database, possibly including the one with `id`.
    ///
    /// `create_sql`, if `Some(_)`, is an SQL string used on error to try to
    /// create a table for storing session data in the database. E.g., for
    /// Postgres the following should work:
    /// `CREATE TABLE __vicocomo__sessions(id BIGINT, data TEXT, time BIGINT)`.
    ///
    /// On success, the returned object always has a valid [`id`](#method.id)
    /// corresponding to a session stored in the database. If `id` was `Some`
    /// it is never changed. If it was `None` a random one is generated and an
    /// empty session is stored. In that case, the caller is responsible for
    /// persisting the new `id`.
    ///
    /// Returns `Error::Other("cannot-create-db-session")` on failure,
    /// translated with one [`parameter`](../../texts/index.html), the error
    /// reported from the database.
    ///
    pub fn new(
        db: DatabaseIf<'d>,
        id: Option<i64>,
        prune: i64,
        create_sql: Option<&str>,
    ) -> Result<Self, Error> {
        use crate::t;
        if prune > 0 {
            let count = db
                .query_column(ROW_COUNT, &[], DbType::Int)
                .and_then(|count| i64::try_from(count.clone()).ok())
                .unwrap_or(0);
            // The frequency calling this function is ~ the number of users ~
            // the number of rows in the database. So, to keep the pruning
            // frequency independent of the number of users:
            if count > 0 && thread_rng().gen_range(0..count) == 0 {
                let _ = db.exec(
                    PRUNE,
                    &[(now() - Duration::seconds(prune)).into()],
                );
            }
        }
        let mut cache: Option<HashMap<String, String>> = None;
        let id = id
            .map(|old_id| {
                cache = db
                    .query_column(SELECT, &[old_id.into()], DbType::Text)
                    .and_then(|data| String::try_from(data.clone()).ok())
                    .and_then(|map_str| serde_json::from_str(&map_str).ok());
                old_id
            })
            .unwrap_or_else(|| thread_rng().gen());
        if cache.is_some() {
            let _ = db.exec(TOUCH, &[id.into(), now().into()]);
        } else {
            let mut tried_create = false;
            loop {
                match db.exec(
                    INSERT,
                    &[id.into(), "{}".to_string().into(), now().into()],
                ) {
                    Ok(count) => {
                        if count == 1 {
                            cache = Some(HashMap::new());
                            break;
                        } else {
                            return Err(Error::this_cannot_happen(""));
                        }
                    }
                    Err(e) if create_sql.is_none() || tried_create => {
                        return Err(Error::other(&t!(
                            "cannot-create-db-session",
                            "p1": &e.to_string(),
                        )));
                    }
                    _ => {
                        let _ = db.exec(create_sql.unwrap(), &[]);
                        tried_create = true;
                        continue;
                    }
                }
            }
        }
        Ok(Self { db, id, cache: cache.unwrap() })
    }

    /// Clear session data with [`id`](#method.id) from the database and the
    /// cached data.
    ///
    pub fn clear(&mut self) {
        self.cache = HashMap::new();
        let _ = self.update();
    }

    /// Get the current `id` of the session in the database.
    ///
    pub fn id(&self) -> i64 {
        self.id
    }

    /// Get value from cache.
    ///
    pub fn get(&self, key: &str) -> Option<String> {
        self.cache.get(key).map(|s| s.to_string())
    }

    /// Remove value from cache and database.
    ///
    pub fn remove(&mut self, key: &str) {
        self.cache.remove(key);
        let _ = self.update();
    }

    pub fn set(&mut self, key: &str, value: &str) -> Result<(), Error> {
        self.cache.insert(key.to_string(), value.to_string());
        self.update()
    }

    pub fn update(&self) -> Result<(), Error> {
        self.db
            .exec(
                UPDATE,
                &[
                    self.id.into(),
                    map_error!(Other, ::serde_json::to_string(&self.cache))?
                        .into(),
                    now().into(),
                ],
            )
            .and_then(|count| {
                if count == 1 {
                    Ok(())
                } else {
                    Err(Error::other("actix-db-session--cannot-update"))
                }
            })
    }
}

fn now() -> NaiveDateTime {
    Local::now().naive_utc()
}
