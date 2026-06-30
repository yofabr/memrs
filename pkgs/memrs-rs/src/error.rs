use std::fmt;
use std::io;

#[derive(Debug)]
pub enum Error {
    Io(io::Error),
    Protocol(String),
    AuthRequired,
    AuthFailed(String),
    NotFound(String),
    ConnectionClosed,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Io(e) => write!(f, "I/O error: {}", e),
            Error::Protocol(msg) => write!(f, "protocol error: {}", msg),
            Error::AuthRequired => write!(f, "authentication required"),
            Error::AuthFailed(msg) => write!(f, "authentication failed: {}", msg),
            Error::NotFound(key) => write!(f, "key not found: {}", key),
            Error::ConnectionClosed => write!(f, "connection closed by server"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Io(e) => Some(e),
            _ => None,
        }
    }
}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self {
        Error::Io(e)
    }
}

impl From<tokio::sync::AcquireError> for Error {
    fn from(_: tokio::sync::AcquireError) -> Self {
        Error::ConnectionClosed
    }
}

pub type Result<T> = std::result::Result<T, Error>;
