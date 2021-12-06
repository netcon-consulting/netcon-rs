//! An implementation of a thread pool to run code asynchronously in multiple
//! threads
//!
//! This code is initially taken from the
//! [Rust Book](https://doc.rust-lang.org/stable/book/ch20-02-multithreaded.html)
//! and then extended to fit our needs.
//!
//! # Examples
//!
//! Running ten simple tasks in a [`ThreadPool`]:
//!
//! ```rust
//! let threadpool = netcon::threadpool::ThreadPool::new(4).unwrap();
//!
//! for id in 1..10_u32 {
//!     threadpool.execute(move || println!("Hello from task {}", id));
//! }
//! ```
//!
//! Running tasks in a [`ThreadPool`] that produce some result and collecting it:
//!
//! ```
//! use std::sync::{Arc, Mutex};
//!
//! let threadpool = netcon::threadpool::ThreadPool::new(4).unwrap();
//!
//! let n_tasks: u32 = 10;
//! let mut ref_vec = Vec::with_capacity(n_tasks as usize);
//! let result_vec = Arc::new(Mutex::new(Vec::with_capacity(n_tasks as usize)));
//!
//! for i in 1..=n_tasks {
//!     ref_vec.push(i.pow(2));
//!     let result_vec = result_vec.clone();
//!     threadpool.execute(move || {
//!         let result = i.pow(2);
//!         let mut result_vec = result_vec.lock().unwrap();
//!         result_vec.push(result);
//!     });
//! }
//!
//! drop(threadpool);
//!
//! let mut result_vec = result_vec.lock().unwrap();
//! result_vec.sort();
//! assert_eq!(ref_vec, *result_vec);
//! ```
//!
//! To make sure all jobs send to the [`ThreadPool`] were finished before continuing with the
//! execution of the following instructions, the [`ThreadPool`] can be dropped, either by letting it
//! go out of scope or explicitly dropping it by calling `drop(threadpool)`. This sends a
//! termination message to all workers and causes them to stop once all jobs in the queue are
//! finished.
//!
//! The jobs in themselves can't return any values, but in order to collect it, a vector can be
//! used as seen in the above example. It should be noted that doing so can result in having the
//! tasks run in sequence if one isn't careful with locking the [`Mutex`]. The [`Mutex`] is locked
//! while a [`MutexGuard`](std::sync::MutexGuard) still exists, so a [`Mutex`] should only be locked
//! when access to the data stored in [`Mutex`] is actually required. Afterwards the
//! [`MutexGuard`](std::sync::MutexGuard) should immediately be dropped to not block other threads
//! from locking the [`Mutex`].

use log::debug;
use std::{
    fmt,
    sync::{
        mpsc::{self, Receiver, Sender},
        Arc, Mutex, PoisonError,
    },
    thread,
};

/// An enum represent Errors that might occur while using a `ThreadPool`.
#[derive(Debug)]
pub enum ThreadPoolError {
    /// The given size of the `ThreadPool` is below 1
    SizeToLow(usize),
    /// There was en error while sending a job to the workers
    Sender(String),
    /// There was an error while receiving a job by a worker
    Receiver(String),
    /// The channel for sending and receiving jobs was poisoned
    Poison(String),
}

impl std::error::Error for ThreadPoolError {}

impl fmt::Display for ThreadPoolError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::SizeToLow(s) => write!(f, "size of ThreadPool must be at least 1, was {}", s),
            Self::Sender(e) => write!(f, "Sender Error: {}", e),
            Self::Receiver(e) => write!(f, "Receiver Error: {}", e),
            Self::Poison(e) => write!(f, "Poison Error: {}", e),
        }
    }
}

impl<T> From<mpsc::SendError<T>> for ThreadPoolError {
    fn from(error: mpsc::SendError<T>) -> Self {
        Self::Sender(error.to_string())
    }
}

impl From<mpsc::RecvError> for ThreadPoolError {
    fn from(error: mpsc::RecvError) -> Self {
        Self::Receiver(error.to_string())
    }
}

impl<T> From<PoisonError<T>> for ThreadPoolError {
    fn from(error: PoisonError<T>) -> Self {
        Self::Poison(error.to_string())
    }
}

enum Message {
    NewJob(Job),
    Terminate,
}

type Job = Box<dyn FnOnce() + Send + 'static>;

/// A struct to limit the number of threads a multithreaded code can spawn. It works as a drop in
/// replacement for [`std::thread::spawn`].
pub struct ThreadPool {
    workers: Vec<Worker>,
    sender: Sender<Message>,
}

impl ThreadPool {
    /// Create a new `ThreadPool` with `size` number of workers.
    ///
    /// # Errors
    ///
    /// When `size` is below 1, `ThreadPool::new` returns an [`ThreadPoolError::SizeToLow`]
    /// containing the given `size`.
    pub fn new(size: usize) -> Result<Self, ThreadPoolError> {
        if size < 1 {
            return Err(ThreadPoolError::SizeToLow(size));
        }

        debug!("Initializing a ThreadPool of size {}", size);

        let (sender, receiver) = mpsc::channel();
        let receiver = Arc::new(Mutex::new(receiver));
        let mut workers = Vec::with_capacity(size);
        for id in 0..size {
            workers.push(Worker::new(id, Arc::clone(&receiver))?);
        }
        Ok(Self { workers, sender })
    }
}

impl ThreadPool {
    /// Send a task to be run by a worker, once one is available.
    ///
    /// # Errors
    ///
    /// When there is a problem while sending task, this function will return a
    /// [`ThreadPoolError::Sender`] with further information encapsulated within it.
    pub fn execute<F>(&self, f: F) -> Result<(), ThreadPoolError>
    where
        F: FnOnce() + Send + 'static,
    {
        let job = Box::new(f);
        self.sender.send(Message::NewJob(job))?;

        Ok(())
    }
}

impl Drop for ThreadPool {
    fn drop(&mut self) {
        debug!("Sending terminate messages to all workers");
        for _ in &self.workers {
            self.sender.send(Message::Terminate).unwrap();
        }

        debug!("Shutting down all workers");
        for worker in &mut self.workers {
            debug!("Shutting down Worker {}", worker.id);
            if let Some(thread) = worker.thread.take() {
                thread.join().unwrap().unwrap();
            }
        }
    }
}

struct Worker {
    id: usize,
    thread: Option<thread::JoinHandle<Result<(), ThreadPoolError>>>,
}

impl Worker {
    fn new(id: usize, receiver: Arc<Mutex<Receiver<Message>>>) -> Result<Self, ThreadPoolError> {
        debug!("Worker {} initializing", id);
        let thread = thread::spawn(move || -> Result<(), ThreadPoolError> {
            loop {
                let message = receiver.lock()?.recv()?;
                match message {
                    Message::NewJob(job) => {
                        debug!("Worker {} got a job; executing.", id);
                        job();
                    }
                    Message::Terminate => {
                        debug!("Worker {} was told to terminate.", id);
                        break;
                    }
                }
            }
            Ok(())
        });
        debug!("Worker {} initialized", id);
        Ok(Self {
            id,
            thread: Some(thread),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn basic_thread_pool() {
        let threadpool = ThreadPool::new(2).unwrap();
        let n_tasks: usize = 100;
        let mut ref_vec = Vec::with_capacity(n_tasks);
        let result_vec = Arc::new(Mutex::new(Vec::with_capacity(n_tasks)));
        for i in 1..n_tasks {
            ref_vec.push(i.pow(2));
            let result_vec = result_vec.clone();
            threadpool
                .execute(move || {
                    let mut result_vec = result_vec.lock().unwrap();
                    result_vec.push(i.pow(2));
                })
                .unwrap();
        }
        drop(threadpool);

        let mut result_vec = result_vec.lock().unwrap();
        result_vec.sort();
        assert_eq!(ref_vec, *result_vec);
    }
}
