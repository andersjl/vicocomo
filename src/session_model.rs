//! A trait implemented by objects persisted in a web session.

use crate::{Error, HttpServerIf};
use ::serde::{Deserialize, Serialize};
use ::std::{default::Default, marker::Sized};

/// Use the derive macro [`SessionModel`
/// ](../../vicocomo_session_model/derive.SessionModel.html) to implement the
/// trait for your model.
///
pub trait SessionModel:
    Default + for<'de> Deserialize<'de> + Serialize + Sized
{
    /// Change the object and session data to the default.
    ///
    fn clear(&mut self, srv: HttpServerIf) {
        *self = Default::default();
        let _ = self.store(srv);
    }

    /// Return the key to use to store model data in the web session. The key
    /// should be unique.
    ///
    fn key() -> &'static str;

    /// Create an object from the web session if stored, with `Default` values
    /// if not.
    ///
    fn load(srv: HttpServerIf) -> Self {
        srv.session_get(Self::key())
            .unwrap_or_else(|| Self::default())
    }

    /// Store object data in the web session.
    ///
    /// If you implement this trait by
    /// `#[derive(SessionModel)] #[vicocomo_session_model_accessors]`, you do
    /// not have to use `store()` directly. Use the generated
    /// `set_`*field name*`()` functions, they `store()` as a side effect.
    ///
    fn store(&self, srv: HttpServerIf) -> Result<(), Error> {
        srv.session_set(Self::key(), &self)
    }
}
