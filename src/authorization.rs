//! Traits to help implement very simple authentication and role based access
//! control.
//!
//! For context see [`Config`
//! ](../http/server/struct.Config.html#level-1-authorize).

use crate::{DatabaseIf, Error, HttpServerIf};
use ::std::fmt::Debug;

/// An abstracted password hash.
///
/// Used by web application developers.
///
/// Should generally not be implemented by web application developers. There
/// is a default implementation using [`bcrypt`
/// ](https://docs.rs/bcrypt/0.10.1/bcrypt/index.html) in the crate
/// [`vicocomo_bcrypt`](../../vicocomo_bcrypt/index.html).
///
/// See the example in `examples/authorization/password_digest/`.
///
pub trait PasswordDigest: Debug + Sized {
    /// Override to implement a password hash algorithm.
    ///
    fn digest(password: &str) -> Result<Self, Error>;

    /// A convenience wrapping of [`digest()`](#tymethod.digest) for password
    /// validation.
    ///
    /// <b>Errors</b>
    ///
    /// Forwards errors from the `validator` and from [`digest()`
    /// ](#tymethod.digest).
    ///
    /// If `password` is validated but not equal to `pwd_conf`,
    /// `Err(Error::InvalidInput("password--differ")`.
    ///
    fn set<F>(
        password: &str,
        pwd_conf: &str,
        validator: F,
    ) -> Result<Self, Error>
    where
        F: Fn(&str) -> Result<(), Error>,
    {
        validator(password).and_then(|_| {
            if password != pwd_conf {
                return Err(Error::invalid_input("password--differ"));
            }
            Self::digest(password)
        })
    }

    /// Override to implement password hash verification.
    ///
    fn authenticate(&self, password: &str) -> bool;
}

/// Implemented by web application developers, used by HTTP server adapter
/// developers.
///
/// <b>Web application developers:</b> Use the HTTP server adapter's [`config`
/// ](../http/server/struct.Config.html#level-1-app_config) macro to define
/// a role `enum` type, and implement this trait for that type.
///
/// <b>HTTP server adapter developers:</b> Use [`is_authorized()`
/// ](#method.is_authorized).  Do not use [`is_authenticated()`
/// ](#tymethod.is_authenticated) directly.
///
pub trait UserRole: Debug + Eq + Sized {
    /// Return `true` iff there is an authenticated user with the role `self`.
    ///
    fn is_authenticated(&self, db: DatabaseIf, srv: HttpServerIf) -> bool;

    /// Used by an HTTP server adapter to authorize, see [`Config`
    /// ](../http/server/struct.Config.html#level-1-authorize).
    ///
    /// `authorized` are the authorized roles.
    ///
    /// `db` and `srv` are forwarded to [`is_authenticated()`
    /// ](#tymethod.is_authenticated).
    ///
    /// `disabled` is the optional `Disabled` role if configured.
    ///
    /// `superuser` is the `Superuser` role.
    ///
    fn is_authorized(
        authorized: &[Self],
        db: DatabaseIf,
        srv: HttpServerIf,
        disabled: Option<Self>,
        superuser: Self,
    ) -> bool {
        match disabled {
            Some(d)
                if d.is_authenticated(db, srv)
                    && !authorized.contains(&d) =>
            {
                return false
            }
            _ => (),
        }
        if superuser.is_authenticated(db, srv) {
            return true;
        }
        for r in authorized {
            if r.is_authenticated(db, srv) {
                return true;
            }
        }
        return false;
    }
}
