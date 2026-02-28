use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct Discussion {
    pub id: Uuid,
    pub anchor_id: Uuid,
    pub messages: Vec<Message>,
    pub participants: Vec<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct Message {
    pub id: Uuid,
    pub author: String,
    pub content: String,
    pub timestamp: DateTime<Utc>,
    pub is_ai: bool,
}
