use sled::{Db, Tree};

use super::KvsEngine;
use crate::thread_pool::ThreadPool;
use crate::{KvsError, Result};

/// Wrapper of `sled::Db`.
#[derive(Clone)]
pub struct SledKvsEngine<P: ThreadPool> {
    db: Db,
    thread_pool: P,
}

impl<P: ThreadPool> SledKvsEngine<P> {
    /// Creates a `SledKvsEngine` from `sled::Db`.
    ///
    /// Operations are run in the given thread pool. `concurrency` specifies the number of
    /// threads in the thread pool.
    pub fn new(db: Db, concurrency: u32) -> Result<Self> {
        let thread_pool = P::new(concurrency)?;
        Ok(Self { db, thread_pool })
    }
}

impl<P: ThreadPool> KvsEngine for SledKvsEngine<P> {
    fn set(&self, key: String, value: String) -> Result<()> {
        let tree: &Tree = &self.db;
        Ok(tree.insert(key, value.into_bytes()).map(|_| ())?)
    }

    fn get(&self, key: String) -> Result<Option<String>> {
        let tree: &Tree = &self.db;

        Ok(tree
            .get(key)?
            .map(|i_vec| AsRef::<[u8]>::as_ref(&i_vec).to_vec())
            .map(String::from_utf8)
            .transpose()?)
    }

    fn remove(&self, key: String) -> Result<()> {
        let tree: &Tree = &self.db;
        tree.remove(key)?.ok_or(KvsError::KeyNotFound)?;
        tree.flush()?;

        Ok(())
    }
}
