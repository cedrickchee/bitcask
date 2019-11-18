use failure::Fail;
use std::io;

/// Error type. It represents the ways a kvs could be invalid.
#[derive(Fail, Debug)]
pub enum KvsError {
    /// An IO error.
    #[fail(display = "{}", _0)]
    Io(#[fail(cause)] io::Error),
}

impl From<io::Error> for KvsError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

/// Result type.
pub type Result<T> = std::result::Result<T, KvsError>;
