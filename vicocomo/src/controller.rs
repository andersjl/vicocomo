//! W.I.P.  Help to implement the Controller part of the
//! View-Controller-Context-Model pattern.

use crate::{DatabaseIf, HttpResponse, HttpServerIf};

/// Use in your [`Controller`](controller/trait.Controller.html)
/// implementation to forward the request to `crate::views::$view::$handler()`
/// with the same signature as the controller method.
///
/// Use the `pub` variants unless the function is part of a [`Controller`
/// ](trait.Controller.html) implementation.
///
#[macro_export]
macro_rules! delegate_to_view {
    ( pub $handler: ident $( , )? ) => {
        pub fn $handler(
            db: $crate::DatabaseIf,
            srv: $crate::HttpServerIf,
            teng: $crate::TemplEngIf,
        ) -> HttpResponse {
            crate::views::$handler(db, srv, teng)
        }
    };
    ( $handler: ident $( , )? ) => {
        fn $handler(
            db: $crate::DatabaseIf,
            srv: $crate::HttpServerIf,
            teng: $crate::TemplEngIf,
        ) -> HttpResponse {
            crate::views::$handler(db, srv, teng)
        }
    };
    ( pub $handler: ident, $view: ident $( , )? ) => {
        pub fn $handler(
            db: $crate::DatabaseIf,
            srv: $crate::HttpServerIf,
            teng: $crate::TemplEngIf,
        ) -> HttpResponse {
            crate::views::$view::$handler(db, srv, teng)
        }
    };
    ( $handler: ident, $view: ident $( , )? ) => {
        fn $handler(
            db: $crate::DatabaseIf,
            srv: $crate::HttpServerIf,
            teng: $crate::TemplEngIf,
        ) -> HttpResponse {
            crate::views::$view::$handler(db, srv, teng)
        }
    };
}

macro_rules! controller_nyi {
    ( $id:ident , $txt:literal $( , )? ) => {
        fn $id(
            _db: $crate::DatabaseIf,
            srv: $crate::HttpServerIf,
            _teng: $crate::TemplEngIf,
        ) -> HttpResponse {
            srv.resp_error(
                None,
                Some($crate::Error::other(
                    &(String::from($txt) + " not implemented"),
                )),
            )
        }
    };
}

/// Provides default implementations of all the standard route handling
/// methods as defined by [`vicocomo::Config`
/// ](../http/server/struct.HttpServerIf.html#config-macro-input-syntax). They
/// do nothing and return an error.
///
pub trait Controller {
    controller_nyi! { copy_form, "Controller::copy_form" }
    controller_nyi! { create,    "Controller::create"    }
    controller_nyi! { delete,    "Controller::delete"    }
    controller_nyi! { edit_form, "Controller::edit_form" }
    controller_nyi! { ensure,    "Controller::ensure"    }
    controller_nyi! { index,     "Controller::index"     }
    controller_nyi! { new_form,  "Controller::new_form"  }
    controller_nyi! { patch,     "Controller::patch"     }
    controller_nyi! { replace,   "Controller::replace"   }
    controller_nyi! { show,      "Controller::show"      }

    /// Fine-grained access control, see [`vicocomo::Config`
    /// ](../http/server/struct.HttpServerIf.html#filtering-access-control).
    ///
    /// The default method returns `false`, denying access unconditionally.
    ///
    fn filter_access(_db: DatabaseIf, _srv: HttpServerIf) -> bool {
        false
    }
}
