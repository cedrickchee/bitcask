use sled::{Db, Tree};

use super::KvsEngine;
use crate::{KvsError, Result};

/// Wrapper of `sled::Db`.
#[derive(Clone)]
pub struct SledKvsEngine(Db);

impl SledKvsEngine {
    /// Creates a `SledKvsEngine` from `sled::Db`.
    pub fn new(db: Db) -> Self {
        Self(db)
    }
}

impl KvsEngine for SledKvsEngine {
    fn set(&self, key: String, value: String) -> Result<()> {
        let tree: &Tree = &self.0;
        Ok(tree.insert(key, value.into_bytes()).map(|_| ())?)
    }

    fn get(&self, key: String) -> Result<Option<String>> {
        let tree: &Tree = &self.0;

        Ok(tree
            .get(key)?
            .map(|i_vec| AsRef::<[u8]>::as_ref(&i_vec).to_vec())
            .map(String::from_utf8)
            .transpose()?)
    }

    fn remove(&self, key: String) -> Result<()> {
        let tree: &Tree = &self.0;
        tree.remove(key)?.ok_or(KvsError::KeyNotFound)?;
        tree.flush()?;

        Ok(())
    }
}
