//! Web Workers for parallel execution

use crate::error::{DxError, DxResult};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread::{self, JoinHandle};

pub struct Worker {
    /// Worker ID - reserved for worker identification
    #[allow(dead_code)]
    id: usize,
    sender: Sender<WorkerMessage>,
    handle: Option<JoinHandle<()>>,
}

pub enum WorkerMessage {
    Execute(String),
    Terminate,
}

pub enum WorkerResponse {
    Result(String),
    Error(String),
}

impl Worker {
    pub fn new(id: usize) -> DxResult<Self> {
        let (tx, rx) = channel();

        let handle = thread::spawn(move || {
            Self::worker_loop(rx);
        });

        Ok(Self {
            id,
            sender: tx,
            handle: Some(handle),
        })
    }

    fn worker_loop(rx: Receiver<WorkerMessage>) {
        while let Ok(msg) = rx.recv() {
            match msg {
                WorkerMessage::Execute(_code) => {
                    // Execute code in worker context
                }
                WorkerMessage::Terminate => break,
            }
        }
    }

    pub fn post_message(&self, code: String) -> DxResult<()> {
        self.sender
            .send(WorkerMessage::Execute(code))
            .map_err(|e| DxError::RuntimeError(format!("Worker send failed: {}", e)))
    }

    pub fn terminate(mut self) -> DxResult<()> {
        let _ = self.sender.send(WorkerMessage::Terminate);
        if let Some(handle) = self.handle.take() {
            handle
                .join()
                .map_err(|_| DxError::RuntimeError("Worker join failed".to_string()))?;
        }
        Ok(())
    }
}

pub struct WorkerPool {
    workers: Vec<Worker>,
    next_id: usize,
}

impl WorkerPool {
    pub fn new(size: usize) -> DxResult<Self> {
        let mut workers = Vec::with_capacity(size);
        for i in 0..size {
            workers.push(Worker::new(i)?);
        }
        Ok(Self {
            workers,
            next_id: size,
        })
    }

    pub fn execute(&self, worker_id: usize, code: String) -> DxResult<()> {
        self.workers
            .get(worker_id)
            .ok_or_else(|| DxError::RuntimeError("Invalid worker ID".to_string()))?
            .post_message(code)
    }

    pub fn spawn_worker(&mut self) -> DxResult<usize> {
        let id = self.next_id;
        self.workers.push(Worker::new(id)?);
        self.next_id += 1;
        Ok(id)
    }
}
