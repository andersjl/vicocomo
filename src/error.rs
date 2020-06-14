#[derive(Clone, Debug)]
pub enum Error {
    Database(String),
    InvalidInput(String),
    Other(String),
}

impl std::error::Error for Error {}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let (kind, txt) = match self {
            Self::Database(s) => ("Databasfel", s),
            Self::InvalidInput(s) => ("Felaktiga indata", s),
            Self::Other(s) => ("Fel", s),
        };
        write!(f, "{}\n{}", kind, txt)
    }
}

impl From<&str> for Error {
    fn from(err: &str) -> Self {
        Self::Other(err.to_string())
    }
}
