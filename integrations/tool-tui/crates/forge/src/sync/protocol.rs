use std::sync::Arc;
use tokio::sync::broadcast;

use crate::crdt::Operation;

/// Lightweight in-process sync manager using a tokio broadcast channel.
/// Components can `publish` operations and other components can `subscribe`
/// to receive live updates. Messages are wrapped in `Arc` to make cloning cheap.
#[derive(Clone)]
pub struct SyncManager {
    tx: broadcast::Sender<Arc<Operation>>,
}

impl Default for SyncManager {
    fn default() -> Self {
        Self::new()
    }
}

impl SyncManager {
    /// Create a new SyncManager with a reasonable buffer size.
    pub fn new() -> Self {
        let (tx, _) = broadcast::channel(256);
        Self { tx }
    }

    /// Subscribe to live operations. The receiver will receive only
    /// messages published after subscription.
    pub fn subscribe(&self) -> broadcast::Receiver<Arc<Operation>> {
        self.tx.subscribe()
    }

    /// Publish an operation to all subscribers. Returns Err if there are
    /// no subscribers or the buffer is full.
    pub fn publish(
        &self,
        op: Arc<Operation>,
    ) -> Result<usize, broadcast::error::SendError<Arc<Operation>>> {
        self.tx.send(op)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn sync_manager_roundtrip() {
        let mgr = SyncManager::new();
        let mut rx = mgr.subscribe();

        let op = Arc::new(Operation::new(
            "/tmp/x".to_string(),
            crate::crdt::OperationType::FileCreate {
                content: "a".into(),
            },
            "actor".into(),
        ));
        mgr.publish(op.clone()).unwrap();

        let got = rx.recv().await.unwrap();
        assert_eq!(got.id, op.id);
    }
}
// Future: WebSocket-based sync protocol for real-time collaboration
