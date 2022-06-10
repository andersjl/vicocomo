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

const SESSION_INSERT: &'static str =
    "INSERT INTO __vicocomo__sessions (id, data, time) VALUES ($1, $2, $3)";
const SESSION_PRUNE: &'static str =
    "DELETE FROM __vicocomo__sessions WHERE time < $1";
const SESSION_ROW_COUNT: &'static str =
    "SELECT COUNT(id) FROM __vicocomo__sessions";
const SESSION_SELECT: &'static str =
    "SELECT data FROM __vicocomo__sessions WHERE id = $1";
const SESSION_TOUCH: &'static str =
    "UPDATE __vicocomo__sessions SET time = $2 WHERE id = $1";
const SESSION_UPDATE: &'static str =
    "UPDATE __vicocomo__sessions SET data = $2, time = $3 WHERE id = $1";

/// # For HTTP server adapter developers only
///
/// Intended for implementing a [`Session`](trait.Session.html) that stores
/// all data in a database table `"__vicocomo__sessions"`.
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
    /// On success, the returned object always has a valid [`id`](#method.id)
    /// corresponding to a session stored in the database. If `id` was `Some`
    /// it is never changed. If it was `None` a random one is generated and an
    /// empty session is stored. In that case, the caller is responsible for
    /// persisting the new `id`.
    ///
    /// Returns `Error::Other("cannot-create-db-session")` on failure.
    ///
    pub fn new(
        db: DatabaseIf<'d>,
        id: Option<i64>,
        prune: i64,
    ) -> Result<Self, Error> {
        if prune > 0 {
            let count = db
                .query_column(SESSION_ROW_COUNT, &[], DbType::Int)
                .and_then(|count| i64::try_from(count.clone()).ok())
                .unwrap_or(0);
            // The frequency calling this function is ~ the number of users ~
            // the number of rows in the database. So, to keep the pruning
            // frequency independent of the number of users:
            if count > 0 && thread_rng().gen_range(0..count) == 0 {
                let _ = db.exec(
                    SESSION_PRUNE,
                    &[(now() - Duration::seconds(prune)).into()],
                );
            }
        }
        let mut cache: Option<HashMap<String, String>> = None;
        let id = id
            .map(|old_id| {
                cache = db
                    .query_column(
                        SESSION_SELECT,
                        &[old_id.into()],
                        DbType::Text,
                    )
                    .and_then(|data| String::try_from(data.clone()).ok())
                    .and_then(|map_str| serde_json::from_str(&map_str).ok());
                old_id
            })
            .unwrap_or_else(|| thread_rng().gen());
        let cache = match cache {
            Some(c) => {
                touch(db, id);
                c
            }
            None => {
                match db.exec(
                    SESSION_INSERT,
                    &[id.into(), "{}".to_string().into(), now().into()],
                ) {
                    Ok(count) if count == 1 => HashMap::new(),
                    _ => {
                        return Err(Error::other("cannot-create-db-session"))
                    }
                }
            }
        };
        Ok(Self { db, id, cache })
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
                SESSION_UPDATE,
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

fn touch(db: DatabaseIf, id: i64) {
    let _ = db.exec(SESSION_TOUCH, &[id.into(), now().into()]);
}
