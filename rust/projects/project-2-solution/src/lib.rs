//! # Kvs
//!
//! A simple in-memory key/value store

#![deny(missing_docs)]

mod error;
mod kv;

pub use error::Result;
pub use kv::KvStore;
