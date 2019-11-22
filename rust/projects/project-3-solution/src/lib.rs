//! # Kvs
//!
//! A simple in-memory key/value store

#![deny(missing_docs)]

#[macro_use]
extern crate log;

mod client;
mod engines;
mod error;
mod server;

pub use client::KvsClient;
pub use engines::KvStore;
pub use engines::KvsEngine;
pub use error::{KvsError, Result};
pub use server::KvsServer;
