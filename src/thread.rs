use std::{
    sync::{mpsc, Arc, Mutex},
    thread,
};

pub type Job = Box<dyn FnOnce() + Send>;

#[allow(dead_code)]
pub struct Pool {
    sender: Option<mpsc::Sender<Job>>,
    workers: Vec<Worker>,
}

impl Pool {
    #[must_use]
    pub fn new(size: usize) -> Self {
        let (sender, receiver) = mpsc::channel();

        let receiver = Arc::new(Mutex::new(receiver));
        let mut workers = Vec::with_capacity(size);

        for id in 0..size {
            workers.push(Worker::new(id, Arc::clone(&receiver)));
        }

        Self {
            sender: Some(sender),
            workers,
        }
    }

    /// # Errors
    ///
    /// Will error if the corresponding receiver has already been deallocated.
    ///
    /// # Panics
    ///
    /// Panics if the receiver has been deallocated.
    pub fn execute<T>(&self, cb: T)
    where
        T: FnOnce() + Send + 'static,
    {
        let job = Box::new(cb);
        self.sender.as_ref().unwrap().send(job).unwrap();
    }
}

impl Drop for Pool {
    fn drop(&mut self) {
        drop(self.sender.take());

        for worker in &mut self.workers {
            println!("Shutting down worker {}", worker.id);

            if let Some(handle) = worker.handle.take() {
                handle.join().unwrap();
            }
        }
    }
}

#[allow(dead_code)]
pub struct Worker {
    id: usize,
    handle: Option<thread::JoinHandle<()>>,
}

impl Worker {
    /// # Panics
    ///
    /// Panics if another user of this mutex panicked while holding the mutex or the sender has
    /// been deallocated.
    #[must_use]
    pub fn new(id: usize, receiver: Arc<Mutex<mpsc::Receiver<Job>>>) -> Self {
        let handle = thread::spawn(move || loop {
            let message = receiver.lock().unwrap().recv();

            if let Ok(job) = message {
                println!("Worker {id} executing");

                job();
            } else {
                println!("Worker {id} disconnected");
                break;
            }
        });

        Self {
            id,
            handle: Some(handle),
        }
    }
}
