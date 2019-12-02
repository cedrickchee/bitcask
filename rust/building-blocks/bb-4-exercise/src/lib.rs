//! Simple threadpool
//!
//! A thread pool runs jobs (functions) on a set of reusable threads, which can be more efficient
//! than spawning a new thread for every job. The implementation was inspired by the
//! threadpool design for multithreaded server from TRPL book.

use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread;

/// Modified ThreadPool that hold Worker instances instead of holding threads directly.
pub struct ThreadPool {
    workers: Vec<Worker>,
    sender: mpsc::Sender<Message>,
}

impl ThreadPool {
    /// Create a new ThreadPool.
    ///
    /// The size is the number of threads in the pool.
    ///
    /// # Panics
    ///
    /// The `new` function will panic if the size is zero.
    pub fn new(size: usize) -> ThreadPool {
        assert!(size > 0);

        // 1. The ThreadPool will create a channel and hold on to the sending side of the channel.
        // Modified `ThreadPool` that store the sending end of a channel that sends `Job` instances.
        let (sender, receiver) = mpsc::channel();

        let receiver = Arc::new(Mutex::new(receiver));

        // Because we know we need to store size elements in the vector, doing this
        // allocation up front is slightly more efficient than using Vec::new, which
        // resizes itself as elements are inserted.
        let mut workers = Vec::with_capacity(size);

        for id in 0..size {
            // create some workers (thread & id) and store them in the vector
            workers.push(Worker::new(id, Arc::clone(&receiver)));
        }

        ThreadPool { workers, sender }
    }

    pub fn execute<F>(&self, f: F)
    where
        F: FnOnce() + Send + 'static,
    {
        // A `Box` that holds each closure (`f`) and then sending the job down the channel
        let job = Box::new(f);
        self.sender.send(Message::NewJob(job)).unwrap();
    }
}

impl Drop for ThreadPool {
    fn drop(&mut self) {
        println!("Sending terminate message to all workers.");

        for _ in &mut self.workers {
            self.sender.send(Message::Terminate).unwrap();
        }

        println!("Shutting down all workers.");

        for worker in &mut self.workers {
            println!("Shutting down worker {}", worker.id);

            // Move the thread out of the Worker instance that owns thread so join can consume the thread.
            if let Some(thread) = worker.thread.take() {
                thread.join().unwrap();
            }
        }
    }
}

trait FnBox {
    fn call_box(self: Box<Self>);
}

// Defining FnBox for any type that implements FnOnce()
impl<F: FnOnce()> FnBox for F {
    fn call_box(self: Box<Self>) {
        // The method takes `self: Box<Self>` to take ownership of self and move the value out of the `Box<T>`.
        (*self)() // move the closure out of the Box<T> and call the closure.
    }
}

/// A `Job` type alias for a `Box` that holds each closure
// type Job = Box<dyn FnOnce() + Send + 'static>; // Change our type alias to use the new FnBox trait
type Job = Box<dyn FnBox + Send + 'static>; // Job type alias is a Box of anything that implements trait FnBox
                                            // This will allow us to use `call_box` in Worker when we get a `Job` value
                                            // instead of invoking the closure directly.

struct Worker {
    id: usize,
    thread: Option<thread::JoinHandle<()>>,
}

impl Worker {
    fn new(id: usize, receiver: Arc<Mutex<mpsc::Receiver<Message>>>) -> Worker {
        // Receiving and executing the jobs in the worker’s thread
        let thread = thread::spawn(move || {
            // We need the closure to loop forever, asking the receiving end of the channel for a job and running the job when it gets one.
            loop {
                let message = receiver.lock().unwrap().recv().unwrap();

                match message {
                    Message::NewJob(job) => {
                        println!("Worker {} got a job; executing.", id);

                        // Compiler error: cannot move a value of type FnOnce() + Send: the size of FnOnce() + Send cannot be statically determined
                        // Rust is still a work in progress with places where the compiler could be improved,
                        // but in the future, the code like this should work just fine. People just like you are working to fix this and other issues!
                        //
                        // But for now, let’s work around this problem using a handy trick. We can tell Rust explicitly that in this case we can
                        // take ownership of the value inside the `Box<T>` using `self: Box<Self>`; then, once we have ownership of the closure, we can call it.
                        // (*job)();

                        // Change `Worker` to use the `call_box` method
                        job.call_box();
                    }
                    Message::Terminate => {
                        println!("Worker {} was told to terminate.", id);

                        break;
                    }
                }
            }
        });

        Worker {
            id,
            thread: Some(thread),
        }
    }
}

enum Message {
    NewJob(Job),
    Terminate,
}
