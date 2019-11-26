use crossbeam::channel::{self, Receiver, Sender};
use std::{panic, thread};

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
            let receiver = receiver.clone();
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

fn run_task(receiver: Receiver<Box<dyn FnOnce() + Send + 'static>>) {
    let receiver2 = receiver.clone();
    panic::set_hook(Box::new(move |panic_info| {
        error!("Thread panicked: {}", panic_info);
        let receiver = receiver2.clone();
        if let Err(e) = thread::Builder::new().spawn(move || run_task(receiver)) {
            error!("Failed to spawn a thread: {}", e);
        }
    }));

    loop {
        match receiver.recv() {
            Ok(task) => {
                task();
            }
            Err(_) => info!("Thread exits because the thread pool is destroyed."),
        }
    }
}
