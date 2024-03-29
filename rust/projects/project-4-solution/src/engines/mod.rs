use crate::Result;

/// Trait for a key value storage engine.
pub trait KvsEngine: Clone + Send + 'static {
    /// Set the value of a string key to a string.
    ///
    /// Returns an error if the value is not written successfully.
    /// If the key already exists, the previous value will be overwritten.
    fn set(&self, key: String, value: String) -> Result<()>;

    /// Get the string value of a string key.
    ///
    /// If the key does not exist, return `None`.
    /// Returns an error if the value is not read successfully.
    fn get(&self, key: String) -> Result<Option<String>>;

    /// Remove a given string key.
    ///
    /// Returns `KvsError::KeyNotFound` error if the given key does not exit
    /// or value is not read successfully.
    fn remove(&self, key: String) -> Result<()>;
}

mod kvs;
mod sled;

pub use self::kvs::KvStore;
pub use self::sled::SledKvsEngine;
