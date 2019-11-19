use failure::Fail;
use std::io;

/// Error type. It represents the ways a kvs could be invalid.
#[derive(Fail, Debug)]
pub enum KvsError {
    /// An IO error. Wraps a `std::io::Error`.
    #[fail(display = "{}", _0)]
    Io(#[fail(cause)] io::Error),
    /// Serialization or deserialization error.
    #[fail(display = "{}", _0)]
    Serde(#[fail(cause)] serde_json::Error),
}

impl From<io::Error> for KvsError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<serde_json::Error> for KvsError {
    fn from(error: serde_json::Error) -> Self {
        Self::Serde(error)
    }
}

/// Result type.
pub type Result<T> = std::result::Result<T, KvsError>;
