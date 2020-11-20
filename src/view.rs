//! W.I.P.  Help to implement the View part of the
//! View-Controller-Context-Model pattern.

use crate::{HttpServerIf, TemplEngIf};
use serde::Serialize;

pub fn render_template(
    srv: HttpServerIf,
    teng: TemplEngIf,
    template: &str,
    data: &impl Serialize,
) {
    match teng.render(template, data) {
        Ok(s) => {
            srv.resp_body(&s);
            srv.resp_ok();
        }
        Err(e) => srv.resp_error(Some(&e)),
    }
}
