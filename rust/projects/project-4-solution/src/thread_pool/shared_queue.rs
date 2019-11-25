use super::ThreadPool;
use crate::Result;

/// A thread pool using a shared queue inside.
pub struct SharedQueueThreadPool;

impl ThreadPool for SharedQueueThreadPool {
    /// New ...
    fn new(threads: u32) -> Result<Self> {
        println!("num. of threads: {}", threads);
        Ok(Self)
    }

    fn spawn<F>(&self, _job: F)
    where
        F: FnOnce(),
    {
        unimplemented!();
    }
}
