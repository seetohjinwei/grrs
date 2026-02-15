use std::thread;

use crossbeam::channel::{Sender, bounded};
use log::debug;

const DEFAULT_THREADS: usize = 8;
const CHANNEL_MULITPLIER: usize = 4;

pub trait Function: FnOnce() + Send + 'static {}
impl<T: FnOnce() + Send + 'static> Function for T {}
type FunctionBox = Box<dyn Function>;

pub struct ThreadPool {
    tx: Sender<FunctionBox>,
    handles: Vec<thread::JoinHandle<()>>,
}

/// A pool of threads for executing functions.
///
/// The API is deceptively simple even though it only supports functions that take in 0 arguments
/// and 0 returns.
/// If an argument is required, it can simply be captured in the function itself.
/// If a return value is required, the caller can create a channel and push results from the function.
impl ThreadPool {
    /// Creates a new ThreadPool with `num_threads`.
    pub fn new(num_threads: usize) -> Self {
        if num_threads <= 0 {
            panic!(
                "ThreadPool expects a positive num_threads, but {} was provided",
                num_threads
            );
        }

        let (tx, rx) = bounded::<FunctionBox>(num_threads * CHANNEL_MULITPLIER);

        let mut handles = Vec::new();

        for _ in 0..num_threads {
            let rx = rx.clone();

            let handle = thread::spawn(move || {
                while let Ok(function) = rx.recv() {
                    // TODO: Have some way to handle errors...?
                    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        function();
                    }));
                    if let Err(_) = result {
                        eprintln!("Worker thread caught a panic in a task!");
                    }
                }
            });
            handles.push(handle);
        }

        Self {
            tx: tx,
            handles: handles,
        }
    }

    pub fn all_cores() -> Self {
        let Ok(num_cores) = thread::available_parallelism() else {
            return Self::default();
        };

        debug!("found {} cores", num_cores);

        Self::new(num_cores.get())
    }

    /// Executes a function.
    ///
    /// This blocks until one of the threads actually start executing the function.
    pub fn execute<F: Function>(&self, function: F) -> () {
        self.tx.send(Box::new(function)).unwrap();
    }

    /// Waits for all threads to complete.
    ///
    /// The ThreadPool will no longer be usable.
    pub fn wait(self) {
        drop(self.tx); // Close the channel

        for handle in self.handles {
            // TODO: Handle this properly
            // https://doc.rust-lang.org/std/thread/struct.JoinHandle.html#method.join
            handle.join().unwrap();
        }
    }
}

impl Default for ThreadPool {
    fn default() -> Self {
        Self::new(DEFAULT_THREADS)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "it's a manual smoke test"]
    fn test_thread_pool_smoke() {
        use std::thread::sleep;
        use std::time::Duration;

        let thread_pool = ThreadPool::new(2);

        thread_pool.execute(|| {
            println!("A: Sending something!!");
            sleep(Duration::from_secs(2));
        });
        thread_pool.execute(|| {
            println!("B: Sending something!!");
            sleep(Duration::from_secs(2));
        });
        thread_pool.execute(|| {
            println!("C: Sending something!!");
            sleep(Duration::from_secs(2));
        });

        thread_pool.wait();
    }
}
