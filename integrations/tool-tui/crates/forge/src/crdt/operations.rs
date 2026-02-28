use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Operation {
    pub id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub actor_id: String,
    pub file_path: String,
    pub op_type: OperationType,
    pub parent_ops: Vec<Uuid>, // For causality tracking
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OperationType {
    Insert {
        position: Position,
        content: String,
        length: usize,
    },
    Delete {
        position: Position,
        length: usize,
    },
    Replace {
        position: Position,
        old_content: String,
        new_content: String,
    },
    FileCreate {
        content: String,
    },
    FileDelete,
    FileRename {
        old_path: String,
        new_path: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Position {
    /// CRDT-based position that survives transformations
    pub lamport_timestamp: u64,
    pub actor_id: String,
    pub offset: usize,

    /// Human-readable position (may change)
    pub line: usize,
    pub column: usize,
}

impl Position {
    pub fn new(line: usize, column: usize, offset: usize, actor_id: String, lamport: u64) -> Self {
        Self {
            lamport_timestamp: lamport,
            actor_id,
            offset,
            line,
            column,
        }
    }

    /// Create a stable identifier that survives code transformations
    pub fn stable_id(&self) -> String {
        format!("{}:{}:{}", self.actor_id, self.lamport_timestamp, self.offset)
    }
}

impl Operation {
    pub fn new(file_path: String, op_type: OperationType, actor_id: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            actor_id,
            file_path,
            op_type,
            parent_ops: Vec::new(),
        }
    }

    pub fn with_parents(mut self, parents: Vec<Uuid>) -> Self {
        self.parent_ops = parents;
        self
    }

    pub fn lamport(&self) -> Option<u64> {
        match &self.op_type {
            OperationType::Insert { position, .. }
            | OperationType::Delete { position, .. }
            | OperationType::Replace { position, .. } => Some(position.lamport_timestamp),
            _ => None,
        }
    }

    /// Check if operations can be batched together
    pub fn can_batch_with(&self, other: &Operation) -> bool {
        // Operations must be on the same file
        if self.file_path != other.file_path {
            return false;
        }

        // Must have same actor
        if self.actor_id != other.actor_id {
            return false;
        }

        // Check if they are consecutive operations
        matches!(
            (&self.op_type, &other.op_type),
            (OperationType::Insert { .. }, OperationType::Insert { .. })
                | (OperationType::Delete { .. }, OperationType::Delete { .. })
        )
    }
}

/// Batch of operations for efficient processing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationBatch {
    pub operations: Vec<Operation>,
    pub batch_id: Uuid,
    pub created_at: DateTime<Utc>,
}

impl OperationBatch {
    /// Create a new batch
    pub fn new(operations: Vec<Operation>) -> Self {
        Self {
            operations,
            batch_id: Uuid::new_v4(),
            created_at: Utc::now(),
        }
    }

    /// Merge consecutive operations in the batch
    pub fn optimize(&mut self) {
        // Simple optimization: remove redundant operations
        self.operations.dedup_by(|a, b| {
            a.file_path == b.file_path && a.actor_id == b.actor_id && a.timestamp == b.timestamp
        });
    }

    /// Get total size of batch
    pub fn len(&self) -> usize {
        self.operations.len()
    }

    /// Check if batch is empty
    pub fn is_empty(&self) -> bool {
        self.operations.is_empty()
    }
}
