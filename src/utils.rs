::lazy_static::lazy_static! {
    static ref WHITESPACE: ::regex::Regex =
        ::regex::Regex::new(r"\s*").unwrap();
}
pub fn blacken(s: &str) -> String {
    WHITESPACE.replace_all(s, "").to_string()
}
