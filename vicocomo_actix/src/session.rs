use ::chrono::{Duration, Local, NaiveDateTime};
use ::rand::{thread_rng, Rng};
use ::vicocomo::{map_error, DatabaseIf, DbType, Error};
pub use ::vicocomo_actix_config::config;
use std::{cell::RefCell, collections::HashMap, convert::TryFrom};

const SESSION_DELETE: &'static str =
    "DELETE FROM __vicocomo__sessions WHERE id = $1";
const SESSION_ID_KEY: &'static str = "__vicocomo__session_id";
const SESSION_INSERT: &'static str =
    "INSERT INTO __vicocomo__sessions (id, data, time) VALUES ($1, $2, $3)";
const SESSION_PRUNE: &'static str =
    "DELETE FROM __vicocomo__sessions WHERE time < $1";
const SESSION_PRUNE_ODDS: i64 = 3; // the odds that we will prune on read
const SESSION_SELECT: &'static str =
    "SELECT data FROM __vicocomo__sessions WHERE id = $1";
const SESSION_TOUCH: &'static str =
    "UPDATE __vicocomo__sessions SET time = $2 WHERE id = $1";
const SESSION_UPDATE: &'static str =
    "UPDATE __vicocomo__sessions SET data = $2, time = $3 WHERE id = $1";

pub(crate) enum Session<'d> {
    Actix(::actix_session::Session),
    // The compiler fails to see that Db is actually constructed in new()???
    #[allow(dead_code)]
    Db {
        axs: ::actix_session::Session,
        db: DatabaseIf<'d>,
        id: i64,
        dbs: HashMap<String, String>,
    },
}

impl<'d> Session<'d> {
    pub(crate) fn new(
        axs: ::actix_session::Session,
        db: Option<DatabaseIf<'d>>,
        prune: i64,
    ) -> Option<RefCell<Self>> {
        match db {
            Some(db) => {
                let id: i64 = match axs.get(SESSION_ID_KEY).ok() {
                    Some(opt) => match opt {
                        Some(old_id) => old_id,
                        None => {
                            let new_id: i64 = thread_rng().gen();
                            if axs.set(SESSION_ID_KEY, &new_id).is_err() {
                                return None;
                            }
                            new_id
                        }
                    },
                    None => return None,
                };
                let dbs: HashMap<String, String> = match db.query(
                    SESSION_SELECT,
                    &[id.into()],
                    &[DbType::Text],
                ) {
                    Ok(db_result) => {
                        match db_result
                            .first()
                            .and_then(|db_row| db_row.first())
                            .and_then(|data| {
                                String::try_from(data.clone()).ok()
                            })
                            .and_then(|dbs_str| {
                                serde_json::from_str(&dbs_str).ok()
                            }) {
                            Some(old_dbs) => {
                                let _ = db.exec(
                                    SESSION_TOUCH,
                                    &[id.into(), now().into()],
                                );
                                if thread_rng()
                                    .gen_range(0..SESSION_PRUNE_ODDS)
                                    == 0
                                {
                                    let _ = db.exec(
                                        SESSION_PRUNE,
                                        &[(now() - Duration::seconds(prune))
                                            .into()],
                                    );
                                }
                                old_dbs
                            }
                            None => {
                                let new_dbs: HashMap<String, String> =
                                    HashMap::new();
                                match db.exec(
                                    SESSION_INSERT,
                                    &[
                                        id.into(),
                                        "{}".to_string().into(),
                                        now().into(),
                                    ],
                                ) {
                                    Ok(count) if count == 1 => new_dbs,
                                    _ => return None,
                                }
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("{}", e);
                        return None;
                    }
                };
                Some(RefCell::new(Self::Db { axs, db, id, dbs }))
            }
            None => Some(RefCell::new(Self::Actix(axs))),
        }
    }

    pub(crate) fn clear(&mut self) {
        let _ = match self {
            Session::Actix(axs) => axs.clear(),
            Session::Db { axs, db, id, dbs } => {
                let _ = db.exec(SESSION_DELETE, &[(*id).into()]);
                *id = thread_rng().gen();
                *dbs = HashMap::new();
                let _ = db.exec(
                    SESSION_INSERT,
                    &[(*id).into(), "{}".to_string().into(), now().into()],
                );
                axs.clear();
            }
        };
    }

    pub(crate) fn get(&self, key: &str) -> Option<String> {
        match self {
            Session::Actix(axs) => axs.get(key).unwrap_or(None),
            Session::Db {
                axs: _,
                db: _,
                id: _,
                dbs,
            } => dbs.get(key).map(|r| r.to_string()),
        }
    }

    pub(crate) fn remove(&mut self, key: &str) {
        let _ = match self {
            Session::Actix(axs) => axs.remove(key),
            Session::Db {
                axs: _,
                db: _,
                id: _,
                dbs,
            } => {
                dbs.remove(key);
            }
        };
    }

    pub(crate) fn set(
        &mut self,
        key: &str,
        value: &str,
    ) -> Result<(), ::vicocomo::Error> {
        match self {
            Session::Actix(axs) => map_error!(Other, axs.set(key, value)),
            Session::Db {
                axs: _,
                db,
                id,
                dbs,
            } => {
                dbs.insert(key.to_string(), value.to_string());
                if db.exec(
                    SESSION_UPDATE,
                    &[
                        (*id).into(),
                        map_error!(Other, ::serde_json::to_string(dbs))?
                            .into(),
                        now().into(),
                    ],
                )? == 1
                {
                    Ok(())
                } else {
                    Err(Error::other("actix-db-session--cannot-update"))
                }
            }
        }
    }
}

fn now() -> NaiveDateTime {
    Local::now().naive_local()
}
