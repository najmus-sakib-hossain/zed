use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::operations::Position;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Anchor {
    pub id: Uuid,
    pub position: Position,
    pub stable_id: String,
    pub file_path: String,
    pub created_at: DateTime<Utc>,
    pub message: Option<String>,
    pub tags: Vec<String>,
}

impl Anchor {
    pub fn new(file_path: String, position: Position, message: Option<String>) -> Self {
        let stable_id = position.stable_id();

        Self {
            id: Uuid::new_v4(),
            position,
            stable_id,
            file_path,
            created_at: Utc::now(),
            message,
            tags: Vec::new(),
        }
    }

    pub fn permalink(&self) -> String {
        format!("forge://{}#{}", self.file_path, self.stable_id)
    }

    #[allow(dead_code)]
    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }
}
