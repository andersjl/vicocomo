//! W.I.P.  Help to implement the Controller part of the
//! View-Controller-Context-Model pattern.

use crate::{DbConn, Error, Request, Response, Session, TemplEng};

macro_rules! controller_nyi {
    ( $id:ident , $txt:literal $( , )? ) => {
        #[allow(unused_variables)]
        fn $id(
            req: &impl Request,
            tmpl: &impl TemplEng,
            db: &impl DbConn,
            sess: Session,
            resp: &mut impl Response,
        ) {
            resp.internal_server_error(Some(&Error::other(
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
