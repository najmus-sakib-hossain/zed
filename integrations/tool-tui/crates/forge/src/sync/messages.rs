use serde::{Deserialize, Serialize};

use crate::crdt::Operation;

/// Wire format for sync messages exchanged over WebSockets.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SyncMessage {
    Handshake { actor_id: String, repo_id: String },
    Operation { operation: Operation },
}

impl SyncMessage {
    pub fn handshake(actor_id: String, repo_id: String) -> Self {
        Self::Handshake { actor_id, repo_id }
    }

    pub fn operation(operation: Operation) -> Self {
        Self::Operation { operation }
    }
}
