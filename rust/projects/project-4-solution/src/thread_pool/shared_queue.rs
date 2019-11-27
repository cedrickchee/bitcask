use crossbeam::channel::{self, Receiver, Sender};
use std::thread;

use super::ThreadPool;
use crate::Result;

/// A thread pool using a shared queue inside.
pub struct SharedQueueThreadPool {
    sender: Sender<Box<dyn FnOnce() + Send + 'static>>,
}

impl ThreadPool for SharedQueueThreadPool {
    fn new(threads: u32) -> Result<Self> {
        let (sender, receiver) = channel::unbounded::<Box<dyn FnOnce() + Send + 'static>>();

        for _ in 0..threads {
            let receiver = TaskReceiver(receiver.clone());
            thread::Builder::new().spawn(move || run_task(receiver))?;
        }

        Ok(Self { sender })
    }

    /// Spawns a function into the thread pool.
    ///
    /// # Panics
    ///
    /// Panics if the thread pool has no thread.
    fn spawn<F>(&self, job: F)
    where
        F: FnOnce() + Send + 'static,
    {
        self.sender
            .send(Box::new(job))
            .expect("The thread pool has no thread.");
    }
}

#[derive(Clone)]
struct TaskReceiver(Receiver<Box<dyn FnOnce() + Send + 'static>>);

impl Drop for TaskReceiver {
    fn drop(&mut self) {
        if thread::panicking() {
            let receiver = self.clone();
            if let Err(e) = thread::Builder::new().spawn(move || run_task(receiver)) {
                error!("Failed to spawn a thread: {}", e);
            }
        }
    }
}

fn run_task(receiver: TaskReceiver) {
    loop {
        match receiver.0.recv() {
            Ok(task) => {
                task();
            }
            Err(_) => debug!("Thread exits because the thread pool is destroyed."),
        }
    }
}
