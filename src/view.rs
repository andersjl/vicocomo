//! W.I.P.  Help to implement the View part of the
//! View-Controller-Context-Model pattern.

use crate::{fix_slashes, Error, HttpServerIf, TemplEngIf};
use serde::Serialize;

/// Make a value for a href attribute.
///
/// If the HTTP server's `config` macro's `app_config` attribute
/// [`strip_mtime`](../http/server/struct.Config.html#strip_mtime) is `true`,
/// the return value is `"`*`url_root/fs_path/name-dddddddddd.ext`*`"`. If not,
/// it is `"`*`url_root/fs_path/name.ext`*`"`, where
/// - `url_root` is as defined by the HTTP server's `config` macro's
///   `app_config` attribute [`url_root`
///   ](../http/server/struct.Config.html#url_root),
/// - `path`, `name`, and `ext` are the arguments, and
/// - `dddddddddd` is the file's mtime as unix timestamp, which is inserted to
///   force the browser to reload the file on change.
/// An inserted timestamp will be removed by the server before looking up the
/// file.
///
/// If the file is not found, the error is `Error::Other`, and the error text
/// is *forwarded low level error*`--`*path to file*.
///
/// `srv` is needed to find `app_config` values and the `mtime` of the file.
/// To find the file, any matching `config` macro [`route_static`
/// ](../http/server/struct.Config.html#level-1-route_static) entry as well as
/// the `app_config` attribute [`file_root`
/// ](../http/server/struct.Config.html#file_root) is used.
///
/// `url_path` is the URL path excluding the file name and extension, *not*
/// necessarily the path to the file system directory (see [`route_static`
/// ](../http/server/struct.Config.html#level-1-route_static)).  It may
/// contain slashes. slashes at the beginning or end are ignored.
///
/// `ext` is a file extension without dot. Optional, default the last segment
/// of `url_path`.
///
pub fn make_href(
    srv: HttpServerIf,
    url_path: &str,
    name: &str,
    ext: Option<&str>,
) -> Result<String, Error> {
    use regex::Regex;
    lazy_static::lazy_static! {
        static ref PATH_LAST: Regex = Regex::new(r"(?:/)?([^./]+)$").unwrap();
    }
    let ext = ext.unwrap_or_else(|| {
        PATH_LAST
            .captures(url_path)
            .and_then(|c| c.get(1).map(|m| m.as_str()))
            .unwrap_or("")
    });
    let url_path = fix_slashes(url_path, -1, -1);
    let ext = if ext.is_empty() {
        String::new()
    } else {
        ".".to_string() + ext
    };
    let fs_dir =
        srv.prepend_file_root(&srv.url_path_to_dir(&url_path).ok_or_else(
            || Error::other(&format!("not-a-static-route--{}", &url_path)),
        )?);
    Ok(srv.prepend_url_root(
        &("/".to_string()
            + &url_path
            + "/"
            + name
            + "-"
            + &format!(
                "{}",
                crate::timestamp(&(fs_dir.clone() + name + &ext))?,
            )
            + &ext),
    ))
}

/// Fill the body of `srv` from `template` using `teng` and `data`.
///
/// If `teng.render()` returns an error, call `srv.resp_error()`.
///
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
