use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

fn main() {
    run_multithreading();
    run_threadpool();
}

// Exercise: Basic multithreading.
//
// This is a simple multithreading exercise from the rustlings project.
//
// threads1.rs
// Make this compile! Execute `rustlings hint threads1` for hints :)
// The idea is the thread spawned on line 21 is completing jobs while the main thread is
// monitoring progress until 10 jobs are completed. If you see 6 lines
// of "waiting..." and the program ends without timing out when running,
// you've got it :)
struct JobStatus {
    jobs_completed: u32,
}

fn run_multithreading() {
    println!("********** Multithreading example **********");

    let status = Arc::new(Mutex::new(JobStatus { jobs_completed: 0 }));
    let status_shared = status.clone();
    thread::spawn(move || {
        for _ in 0..10 {
            thread::sleep(Duration::from_millis(250));
            status_shared.lock().unwrap().jobs_completed += 1;
        }
    });
    while status.lock().unwrap().jobs_completed < 10 {
        println!("waiting... ");
        thread::sleep(Duration::from_millis(500));
    }
}

// Exercise: Write a thread pool.
//
// A thread pool runs jobs (functions) on a set of reusable threads, which can be more efficient
// than spawning a new thread for every job.
// The threadpool crate and Rayon's ThreadPool may provide inspiration.
use bb_4_exercise::ThreadPool;
use std::sync::mpsc;

fn run_threadpool() {
    println!("********** Threadpool example **********");

    let n_workers = 4;
    let n_jobs = 10;
    let pool = ThreadPool::new(n_workers);

    // Synchronized with a channel.
    let (tx, rx) = mpsc::channel();

    for _ in 0..n_jobs {
        let tx = tx.clone();

        pool.execute(move || {
            tx.send(1)
                .expect("channel will be there waiting for the pool");
        });
    }

    let sum = rx.iter().take(n_jobs).fold(0, |a, b| a + b);
    assert_eq!(sum, 10);
    println!("Done summing {} of 1s: {}", n_jobs, sum);
}
