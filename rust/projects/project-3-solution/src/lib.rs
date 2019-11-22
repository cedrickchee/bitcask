//! # Kvs
//!
//! A simple in-memory key/value store

#![deny(missing_docs)]

#[macro_use]
extern crate log;

mod engines;
mod error;
mod server;

pub use engines::KvStore;
pub use engines::KvsEngine;
pub use error::{KvsError, Result};
pub use server::KvsServer;
