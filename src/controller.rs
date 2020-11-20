//! W.I.P.  Help to implement the Controller part of the
//! View-Controller-Context-Model pattern.

use crate::{DatabaseIf, HttpServerIf, TemplEngIf};

/*
use ::regex::Regex;
use crate::Error;

pub struct Dispatcher<'a> {
    // registry[n] registers paths with length n
    registry: Vec<Vec<(Regex, &'a dyn NewHandler)>>,
}

impl Dispatcher<'_> {
    pub fn dispatch(
        &self,
        req: &impl Request,
        tmpl: &impl TemplEng,
        db: &impl DatabaseIf,
        sess: Session,
        resp: &mut impl Response,
    ) {
        self.lookup(req.path())?.handle(req, tmpl, db, sess, resp);
    }

    pub fn register(
        &mut self,
        path: &str,
        handler: &dyn NewHandler,
    ) -> Result<(), Error> {
        let len = path.split('/').filter(|s| s.len() > 0).count();
        for _ in self.registry.len()..=len {
            self.registry.push(Vec::new());
        }
        let regex = Regex::new(path);
        self.registry[len].push((regex, handler));
        Ok(())
    }

    pub fn lookup(path: &str) -> Result<&dyn NewHandler, Error> {
    }
}

pub trait NewHandler {
    fn handle(
        &self,
        req: &impl Request,
        tmpl: &impl TemplEng,
        db: &impl DatabaseIf,
        sess: Session,
        resp: &mut impl Response,
    );
}
*/

/// Forward the request to `crate::views::$views::$handler()` with the same
/// signature as the controller method.
///
#[macro_export]
macro_rules! delegate_to_view {
    ( $handler: ident, $views: ident $( , )? ) => {
        fn $handler(
            db: $crate::DatabaseIf,
            srv: $crate::HttpServerIf,
            teng: $crate::TemplEngIf,
        ) {
            crate::views::$views::$handler(db, srv, teng);
        }
    };
}

macro_rules! controller_nyi {
    ( $id:ident , $txt:literal $( , )? ) => {
        fn $id(
            _db: $crate::DatabaseIf,
            srv: $crate::HttpServerIf,
            _teng: $crate::TemplEngIf,
        ) {
            srv.resp_error(Some(&$crate::Error::other(
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
