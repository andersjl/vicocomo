//! A trait used for role based access control.
//!
//! For context see [`Config`
//! ](../http_server/struct.Config.html#level-1-authorize).

use crate::{DatabaseIf, HttpServerIf};
use ::std::fmt::Debug;

/// Implemented by web application developers, used by HTTP server adapter
/// developers.
///
/// <b>Web application developers:</b> Use the HTTP server adapter's [`config`
/// ](../http_server/struct.Config.html#level-1-app_config) macro to define
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
    /// ](../http_server/struct.Config.html#level-1-authorize).
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
        if disabled.is_some() {
            let d = disabled.unwrap();
            if d.is_authenticated(db, srv) && !authorized.contains(&d) {
                return false;
            }
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
