//! Session history — tracks AI conversation sessions grouped by date.

use serde::{Deserialize, Serialize};
use std::time::SystemTime;

/// A unique session identifier.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SessionId(pub String);

impl SessionId {
    pub fn new() -> Self {
        Self(uuid_v4())
    }
}

impl Default for SessionId {
    fn default() -> Self {
        Self::new()
    }
}

/// A saved AI session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionEntry {
    pub id: SessionId,
    pub title: String,
    pub profile: crate::profile::AiProfile,
    pub created_at: SystemTime,
    pub updated_at: SystemTime,
    pub message_count: usize,
    /// First few words of the conversation for preview.
    pub preview: String,
}

/// Sessions grouped by date for the session history rail.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionGroup {
    pub label: String,
    pub sessions: Vec<SessionEntry>,
}

/// Group sessions by date (Today, Yesterday, This Week, Older).
pub fn group_sessions_by_date(mut sessions: Vec<SessionEntry>) -> Vec<SessionGroup> {
    sessions.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));

    let now = SystemTime::now();
    let day = std::time::Duration::from_secs(86400);
    let week = std::time::Duration::from_secs(7 * 86400);

    let mut today = Vec::new();
    let mut yesterday = Vec::new();
    let mut this_week = Vec::new();
    let mut older = Vec::new();

    for session in sessions {
        let age = now.duration_since(session.updated_at).unwrap_or_default();
        if age < day {
            today.push(session);
        } else if age < day * 2 {
            yesterday.push(session);
        } else if age < week {
            this_week.push(session);
        } else {
            older.push(session);
        }
    }

    let mut groups = Vec::new();
    if !today.is_empty() {
        groups.push(SessionGroup { label: "Today".into(), sessions: today });
    }
    if !yesterday.is_empty() {
        groups.push(SessionGroup { label: "Yesterday".into(), sessions: yesterday });
    }
    if !this_week.is_empty() {
        groups.push(SessionGroup { label: "This Week".into(), sessions: this_week });
    }
    if !older.is_empty() {
        groups.push(SessionGroup { label: "Older".into(), sessions: older });
    }
    groups
}

/// Simple UUID v4 generation (random).
fn uuid_v4() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!("{:032x}", ts)
}
