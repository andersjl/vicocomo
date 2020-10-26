//! W.I.P.  Help to implement the Controller part of the
//! View-Controller-Context-Model pattern.

/// Forward the request to `crate::views::$views::$handler()` with the same
/// signature as the controller method.
///
#[macro_export]
macro_rules! delegate_to_view {
    ( $handler: ident, $views: ident $( , )? ) => {
        fn $handler(
            req: &impl $crate::Request,
            teng: &impl $crate::TemplEng,
            db: &impl $crate::DbConn,
            sess: $crate::Session,
            resp: &mut impl $crate::Response,
        ) {
            crate::views::$views::$handler(req, teng, db, sess, resp);
        }
    };
}

macro_rules! controller_nyi {
    ( $id:ident , $txt:literal $( , )? ) => {
        #[allow(unused_variables)]
        fn $id(
            req: &impl $crate::Request,
            tmpl: &impl $crate::TemplEng,
            db: &impl $crate::DbConn,
            sess: $crate::Session,
            resp: &mut impl $crate::Response,
        ) {
            resp.internal_server_error(Some(&$crate::Error::other(
                &(String::from($txt) + " not implemented"),
            )))
        }
    };
}

/// Provides default implementations of all the standard route handling
/// methods as defined by
/// [`vicocomo::Config`](../../vicocomo/http_server/struct.Config.html).  They
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
}
