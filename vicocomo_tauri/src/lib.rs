//! # Tauri application configuration and generation

use std::path::{Path, PathBuf};
use std::sync::Arc;
use tauri::api::dialog::blocking::FileDialogBuilder;
use url::{ParseError, Url};
use vicocomo::{
    t, try_exec_sql, AppConfigVal, DatabaseIf, DbType, Error, HttpHeaderVal,
    HttpParamVals, HttpRespBody, HttpResponse, HttpServer, HttpServerImpl,
};
use vicocomo_sqlite::SqliteConn;
pub use vicocomo_tauri_config::config;

// The boundary used in the body returned from fix_body()
#[doc(hidden)]
pub const BOUNDARY: &'static str = "--__VICOCOMO__boundary--";

// Used by the code generated by config!().
//
// Prevents the UI from reading files and send them to Rust.
//
// Expects the UI to rewrite forms with enctype="multipart/form-data" to
// simply send the POST url with parameters __VICOCOMO__upload_.. as described
// in the config!() doc, the "Simple glue Javascript" section.
//
// Selects the file(s) using a Tauri dialog.
//
// Reads the selected files, and constructs a multipart body from them.
//
// Returns None if the user does not select any file.
//
#[doc(hidden)]
pub fn fix_body(name: &str, parvals: &HttpParamVals) -> Option<Vec<u8>> {
    use std::str::from_utf8;

    let multiple = parvals
        .get("__VICOCOMO__upload_multiple")
        .map(|_| true)
        .unwrap_or(false);
    let dialog =
        FileDialogBuilder::new().set_title(&t!(&format!("upload--{name}")));
    let mut paths: Vec<PathBuf> = Vec::new();
    if multiple {
        if let Some(pths) = dialog.pick_files() {
            paths = pths;
        }
    } else if let Some(path) = dialog.pick_file() {
        paths.push(path);
    }
    if paths.is_empty() {
        return None;
    }
    #[cfg(debug_assertions)]
    eprintln!("{paths:?}");
    let mut bytes = Vec::new(); // no headers!
    for path in paths {
        let filename = path
            .file_name()
            .map(|f| f.to_string_lossy().to_string())
            .unwrap_or(String::new());
        let mut contents = std::fs::read(path).ok()?;
        // Typical file part header from browser:
        // Content-Disposition: form-data; name="file_field"; filename="foo.txt"
        // Content-Type: text/plain; charset=utf-8
        bytes.extend_from_slice(b"--");
        bytes.extend_from_slice(BOUNDARY.as_bytes());
        let content_type = from_utf8(&contents)
            .map(|_| "text/plain; charset=utf-8")
            .unwrap_or("application/octet-stream");
        bytes.append(
            &mut format!(
                "\r\nContent-Disposition: form-data; \
                    name={name}; filename=\"{filename}\"\r\n\
                Content-Type: {content_type}\r\n\r\n"
            )
            .into_bytes(),
        );
        bytes.append(&mut contents);
    }
    bytes.extend_from_slice(b"--");
    bytes.extend_from_slice(BOUNDARY.as_bytes());
    bytes.extend_from_slice(b"--");
    Some(bytes)
}

// Used by the code generated by config!(). Prevents sending file attachments
// to the UI.
//
// For responses with Content-Disposition: attachment:
// - Uses a Tauri dialog to select a directory and file name,
// - saves the file in Rust, and
// - returns None.
//
// Other responses are simply wrapped in `Some(_)`
//
#[doc(hidden)]
pub fn fix_response(response: HttpResponse) -> Option<HttpResponse> {
    if let Some(ref disp) = response.get_header("Content-Disposition") {
        let disp = HttpHeaderVal::from_str(disp);
        if disp.value == "attachment" {
            let mut dialog = FileDialogBuilder::new()
                .set_directory("backup")
                .set_title(&t!("backup--save"));
            if let Some(ref fnam) = disp.get_param("filename") {
                dialog = dialog.set_file_name(fnam);
            }
            if let Some(path) = dialog.save_file() {
                if let HttpRespBody::Bytes(body) = response.get_body() {
                    let _ = std::fs::write(path, body);
                }
            }
            return None;
        }
    }
    Some(response)
}

// Parse the URL parameter to the Tauri command request() to a Rust url::Url.
// If needed, prepend `http://localhost`. Fails only if parsing fails.
//
#[doc(hidden)]
pub fn fix_url(url_str: &str) -> Result<Url, Error> {
    Url::parse(url_str)
        .or_else(|e| {
            if e == ParseError::RelativeUrlWithoutBase {
                Url::parse(&("http://localhost".to_string() + url_str))
            } else {
                Err(e)
            }
        })
        .map_err(|e| Error::invalid_input(&e.to_string()))
}

// Get a path to a resource from server.app_config().
//
// If the configured value is true, return resource_dir/<default>.
// If the configured value is a string, return that.
// Otherwise return None.
#[doc(hidden)]
pub fn get_bool_str_res_path(
    server: &HttpServerImpl,
    key: &str,
    default: &str,
) -> Option<PathBuf> {
    server.app_config(key).and_then(|val| match val {
        AppConfigVal::Bool(flg) if flg => server
            .app_config("resource_dir")
            .and_then(|dir| dir.str())
            .map(|dir| PathBuf::from(&dir).join(default)),
        AppConfigVal::Str(cfg) => Some(PathBuf::from(cfg)),
        _ => None,
    })
}

// used by the code generated by config!().
//
// Tries to open an Sqlite connection to db_path and, if the database has no
// tables and schema is Some(_), executes the SQL in schema.
//
// Errors
//
#[doc(hidden)]
pub fn get_db(
    db_path: &Path,
    schema: Option<&Path>,
) -> Result<DatabaseIf, Error> {
    SqliteConn::new(db_path).and_then(|conn| {
        let db = DatabaseIf::new(Arc::new(conn));
        if let Some(path) = schema {
            db.clone()
                .query("SELECT name FROM sqlite_master", &[], &[DbType::Text])
                .and_then(|rels| {
                    if rels.is_empty() {
                        std::fs::read_to_string(path)
                            .map_err(|e| {
                                Error::invalid_input(&t!(
                                    "file--read",
                                    "path": path.display(),
                                    "error": e,
                                ))
                            })
                            .and_then(|scm| {
                                try_exec_sql(db.clone(), &scm, None)
                            })
                    } else {
                        Ok(())
                    }
                })
        } else {
            Ok(())
        }
        .map(|_| db)
    })
}

// Converts a vicocomo::`HttpResponse to a response from the Tauri command
// request().
//
#[doc(hidden)]
pub fn tauri_response(response: HttpResponse) -> (u32, String) {
    (
        response.get_status() as u32,
        response.get_body().to_string(),
    )
}
