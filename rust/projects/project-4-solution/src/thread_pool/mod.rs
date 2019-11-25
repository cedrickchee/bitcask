//! This module provides various thread pools. All thread pools should implement
//! the `ThreadPool` trait.
//!

use crate::Result;

mod naive;
mod rayon;
mod shared_queue;

pub use naive::NaiveThreadPool;
pub use rayon::RayonThreadPool;
pub use shared_queue::SharedQueueThreadPool;

/// The trait that all thread pools should implement.
pub trait ThreadPool {
    /// Creates a new thread pool.
    fn new(threads: u32) -> Result<Self>
    where
        Self: Sized;

    /// Spawns a function into the threadpool.
    fn spawn<F>(&self, job: F)
    where
        F: FnOnce();
}
