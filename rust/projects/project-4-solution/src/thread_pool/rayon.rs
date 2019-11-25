use super::ThreadPool;
use crate::Result;

/// Wrapper of rayon::ThreadPool
pub struct RayonThreadPool;

impl ThreadPool for RayonThreadPool {
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
