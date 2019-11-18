use std::fmt;

#[derive(Debug, Eq, PartialEq)]
pub enum Error {
    Auth,
    App(String, u16),
    Status(u16),
    Empty,
    Timeout,
    Other(String),
}

impl From<http::Error> for Error {
    fn from(err: http::Error) -> Self {
        Error::Other(err.to_string())
    }
}

impl From<isahc::Error> for Error {
    fn from(err: isahc::Error) -> Self {
        Error::Other(err.to_string())
    }
}

impl From<serde_json::Error> for Error {
    fn from(err: serde_json::Error) -> Self {
        Error::Other(err.to_string())
    }
}

impl std::error::Error for Error {}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "{:#?}", self)
    }
}
