use anyhow::{Context, Result, anyhow};
use crossbeam::channel::{self, Sender};
use dashmap::DashMap;
use std::sync::Arc;
use std::thread;
use uuid::Uuid;

use super::Database;
use crate::crdt::Operation;

pub struct OperationLog {
    // In-memory cache for fast lookups and deduplication
    cache: DashMap<Uuid, Operation>,
    queue: Sender<Operation>,
}

impl OperationLog {
    pub fn new(db: Arc<Database>) -> Result<Self> {
        let (tx, rx) = channel::unbounded::<Operation>();
        let worker_db = db.clone();
        thread::Builder::new()
            .name("forge-oplog-writer".to_string())
            .spawn(move || {
                while let Ok(op) = rx.recv() {
                    if let Err(err) = worker_db.store_operation(&op) {
                        eprintln!("⚠️  Failed to persist operation {}: {err}", op.id);
                    }
                }
            })
            .context("failed to spawn oplog writer thread")?;

        Ok(Self {
            cache: DashMap::new(),
            queue: tx,
        })
    }

    pub fn append(&self, operation: Operation) -> Result<bool> {
        let is_new = self.cache.insert(operation.id, operation.clone()).is_none();
        if !is_new {
            return Ok(false);
        }

        self.queue
            .send(operation)
            .map_err(|err| anyhow!("failed to enqueue operation for persistence: {err}"))?;

        Ok(true)
    }

    #[allow(dead_code)]
    pub fn get(&self, id: &Uuid) -> Option<Operation> {
        self.cache.get(id).map(|op| op.clone())
    }
}
