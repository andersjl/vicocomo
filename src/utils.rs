use crate::Error;
use lazy_static::lazy_static;

pub fn blacken(s: &str) -> String {
    lazy_static! {
        static ref WHITE: regex::Regex = regex::Regex::new(r"\s*").unwrap();
    }
    WHITE.replace_all(s, "").to_string()
}

/// Change `"+"` to `"%20"`, then [`urlencoding::decode()`
/// ](https://docs.rs/urlencoding/latest/urlencoding/fn.decode.html).
///
pub fn decode_url_parameter(par: &str) -> Result<String, Error> {
    lazy_static! {
        static ref PLUS: regex::Regex = regex::Regex::new(r"\+").unwrap();
    }
    urlencoding::decode(&PLUS.replace_all(par, "%20"))
        .map(|s| s.to_string())
        .map_err(|e| Error::invalid_input(&e.to_string()))
}

/// - leave an empty string alone
/// - if lead is negative remove leading slash if present
/// - if lead is positive add leading slash if missing
/// - if trail is negative remove trailing slash if present
/// - if trail is positive add trailing slash if missing
///
pub fn fix_slashes(s: &str, lead: i32, trail: i32) -> String {
    if s.is_empty() {
        return String::new();
    }
    let mut result = s.to_string();
    if lead < 0 && result.starts_with('/') {
        result.remove(0);
    }
    if lead > 0 && !result.starts_with('/') {
        result.insert(0, '/');
    }
    if trail < 0 && result.ends_with('/') {
        result.remove(result.len() - 1);
    }
    if trail > 0 && !result.ends_with('/') {
        result = result + "/";
    }
    result
}

/// Get a unix timestamp from a file path.
///
/// If the file is not found, the error is `Error::Other`, and the error text
/// is *forwarded low level error*`--`*path to file*.
///
pub fn timestamp(file: &str) -> Result<u64, Error> {
    match std::fs::metadata(file) {
        Ok(data) => match data.modified() {
            Ok(system_time) => match system_time
                .duration_since(std::time::SystemTime::UNIX_EPOCH)
            {
                Ok(duration) => Ok(duration.as_secs()),
                Err(e) => Err(Error::other(&(e.to_string() + "--" + file))),
            },
            Err(e) => Err(Error::other(&(e.to_string() + "--" + file))),
        },
        Err(e) => Err(Error::other(&(e.to_string() + "--" + file))),
    }
}
