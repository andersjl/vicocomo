//! The vicocomo error type.
//!
// TODO i18n, probably using the "fluent" crate family.

#[derive(Clone, Debug)]
pub enum Error {
    Database(String),
    InvalidInput(String),
    Other(String),
    Render(String),
}

impl Error {
    /// Create an `Error::Database`.
    pub fn database(txt: &str) -> Self {
        Self::Database(txt.to_string())
    }
    /// Create an `Error::InvalidInput`.
    pub fn invalid_input(txt: &str) -> Self {
        Self::InvalidInput(txt.to_string())
    }
    #[doc(hidden)]
    pub fn nyi() -> Self {
        Self::other("NYI")
    }
    /// Create an `Error::Other`.
    pub fn other(txt: &str) -> Self {
        Self::Other(txt.to_string())
    }
    /// Create an `Error::Render`.
    pub fn render(txt: &str) -> Self {
        Self::Render(txt.to_string())
    }
}

impl std::error::Error for Error {}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let (kind, txt) = match self {
            Self::Database(s) => ("Database error", s),
            Self::InvalidInput(s) => ("Invalid input", s),
            Self::Other(s) => ("Error", s),
            Self::Render(s) => ("Cannot render", s),
        };
        write!(f, "{}\n{}", kind, txt)
    }
}

/// Create an `Error::Other`.
impl From<&str> for Error {
    fn from(err: &str) -> Self {
        Self::Other(err.to_string())
    }
}
