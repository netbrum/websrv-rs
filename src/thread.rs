use std::{
    sync::{mpsc, Arc, Mutex},
    thread,
};

pub type Job = Box<dyn FnOnce() + Send>;

#[allow(dead_code)]
pub struct Pool {
    sender: mpsc::Sender<Job>,
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

        Self { sender, workers }
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
        self.sender.send(job).unwrap();
    }
}

#[allow(dead_code)]
pub struct Worker {
    id: usize,
    handle: thread::JoinHandle<()>,
}

impl Worker {
    /// # Panics
    ///
    /// Panics if another user of this mutex panicked while holding the mutex or the sender has
    /// been deallocated.
    #[must_use]
    pub fn new(id: usize, receiver: Arc<Mutex<mpsc::Receiver<Job>>>) -> Self {
        let handle = thread::spawn(move || loop {
            let job = receiver.lock().unwrap().recv().unwrap();

            println!("Worker {id} executing");

            job();
        });

        Self { id, handle }
    }
}
