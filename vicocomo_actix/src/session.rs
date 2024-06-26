use vicocomo::{map_error, DatabaseIf, HttpDbSession};

const SESSION_ID_KEY: &'static str = "__vicocomo__session_id";

pub(crate) enum Session {
    Actix(::actix_session::Session),
    Db {
        axs: actix_session::Session,
        dbs: HttpDbSession,
    },
}

impl Session {
    pub(crate) fn new(
        axs: actix_session::Session,
        db: Option<DatabaseIf>,
        prune: i64,
        create_sql: Option<&str>,
    ) -> Option<Self> {
        match db {
            Some(db) => {
                let id = axs.get(SESSION_ID_KEY).ok().and_then(|opt| opt);
                let dbs = match HttpDbSession::new(db, id, prune, create_sql)
                {
                    Ok(d) => d,
                    Err(e) => panic!("{}", e.to_string()),
                };
                if id.is_none()
                    && axs.insert(SESSION_ID_KEY, &dbs.id()).is_err()
                {
                    return None;
                }
                Some(Self::Db { axs, dbs })
            }
            None => Some(Self::Actix(axs)),
        }
    }

    pub(crate) fn clear(&mut self) {
        match self {
            Session::Actix(axs) => axs.clear(),
            Session::Db { axs, dbs } => {
                axs.clear();
                let _ = axs.insert(SESSION_ID_KEY, &dbs.id());
                dbs.clear();
            }
        }
    }

    pub(crate) fn get(&self, key: &str) -> Option<String> {
        match self {
            Session::Actix(axs) => axs.get(key).unwrap_or(None),
            Session::Db { axs: _, dbs } => dbs.get(key),
        }
    }

    pub(crate) fn remove(&mut self, key: &str) {
        match self {
            Session::Actix(axs) => {
                axs.remove(key);
            }
            Session::Db { axs: _, dbs } => dbs.remove(key),
        }
    }

    pub(crate) fn set(
        &mut self,
        key: &str,
        value: &str,
    ) -> Result<(), vicocomo::Error> {
        match self {
            Session::Actix(axs) => map_error!(Other, axs.insert(key, value)),
            Session::Db { axs: _, dbs } => dbs.set(key, value),
        }
    }
}
