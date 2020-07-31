//! W.I.P.  Help to implement the View part of the
//! View-Controller-Context-Model pattern.

use crate::{Response, TemplEng};
use serde::Serialize;

pub fn render_template(
    resp: &mut impl Response,
    tmpl: &impl TemplEng,
    template: &str,
    data: &impl Serialize,
) {
    match tmpl.render(template, data) {
        Ok(s) => {
            resp.resp_body(&s);
            resp.ok();
        }
        Err(e) => resp.internal_server_error(Some(&e)),
    }
}
