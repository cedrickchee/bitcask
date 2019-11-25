use super::ThreadPool;
use crate::Result;

/// It is actually not a thread pool. It spawns a new thread every time
/// the `spawn` method is called.
pub struct NaiveThreadPool;

impl ThreadPool for NaiveThreadPool {
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
