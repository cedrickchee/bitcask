//! # Kvs
//!
//! A simple in-memory key/value store

#![deny(missing_docs)]

mod engines;
mod error;
mod kv;

pub use engines::KvsEngine;
pub use error::{KvsError, Result};
pub use kv::KvStore;
