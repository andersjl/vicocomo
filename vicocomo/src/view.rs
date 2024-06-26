//! W.I.P.  Help to implement the View part of the
//! View-Controller-Context-Model pattern.

// TODO: generalise get_on and set_on from andersjlindeberg

use crate::{Error, HttpServerIf};
use ljumvall_utils::fix_slashes;

/// Make a value for a href attribute.
///
/// If the HTTP server's `config` macro's `app_config` attribute
/// [`strip_mtime`](../http/server/struct.Config.html#strip_mtime) is `true`,
/// the return value is `"`*`url_root/url_path/name-dddddddddd.ext`*`"`. If
/// not, it is `"`*`url_root/url_path/name.ext`*`"`, where
/// - `url_root` is as defined by the HTTP server's `config` macro's
///   `app_config` attribute [`url_root`
///   ](../http/server/struct.Config.html#url_root),
/// - `path`, `name`, and `ext` are the arguments, and
/// - `dddddddddd` is the file's mtime as unix [timestamp
///   ](../../ljumvall_utils/fn.timestamp.html), which is inserted to force
///   the browser to reload the file on change.
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
    use std::sync::OnceLock;

    static PATH_LAST: OnceLock<Regex> = OnceLock::new();
    let path_last =
        PATH_LAST.get_or_init(|| Regex::new(r"(?:/)?([^./]+)$").unwrap());

    let add_timestamp = srv
        .app_config("strip_mtime")
        .map(|val| {
            if let crate::AppConfigVal::Bool(b) = val {
                b
            } else {
                false
            }
        })
        .unwrap_or(false);
    let ext = ext.unwrap_or_else(|| {
        path_last
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
    let timestamp = if add_timestamp {
        format!(
            "-{}",
            ljumvall_utils::timestamp(
                &(srv.prepend_file_root(
                    &srv.url_path_to_dir(&url_path).ok_or_else(|| {
                        Error::other(&format!(
                            "not-a-static-route--{}",
                            &url_path
                        ))
                    })?
                ) + name
                    + &ext)
            )?,
        )
    } else {
        String::new()
    };
    Ok(srv.prepend_url_root(
        &("/".to_string() + &url_path + "/" + name + &timestamp + &ext),
    ))
}

/*
/// Fill the body of `srv` from `template` using `teng` and `data`, set the
/// response status to 200 `OK` and the `Content-Type` to
/// `text/<content_type>; charset=utf-8`
///
/// If `teng.render()` returns an error, call `srv.resp_error()`.
///
pub fn render_template(
    srv: HttpServerIf,
    teng: TemplEngIf,
    template: &str,
    data: &impl Serialize,
    content_type: &str,
) -> HttpResponse {
    match teng.render(template, data) {
        Ok(s) => HttpResponse::utf8(None, Some(content_type), s),
        Err(e) => srv.resp_error(None, Some(e)),
    }
}
*/
