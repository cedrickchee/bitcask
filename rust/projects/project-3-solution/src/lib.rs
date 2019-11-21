//! # Kvs
//!
//! A simple in-memory key/value store

#![deny(missing_docs)]

mod engines;
mod error;

pub use engines::KvStore;
pub use engines::KvsEngine;
pub use error::{KvsError, Result};
