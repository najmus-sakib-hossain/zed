//! Calendar integrations

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalendarEvent {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
    pub location: Option<String>,
    pub attendees: Vec<String>,
}

pub trait CalendarProvider {
    async fn list_events(&self, start: DateTime<Utc>, end: DateTime<Utc>) -> Result<Vec<CalendarEvent>>;
    async fn create_event(&self, event: CalendarEvent) -> Result<String>;
    async fn update_event(&self, id: &str, event: CalendarEvent) -> Result<()>;
    async fn delete_event(&self, id: &str) -> Result<()>;
}

pub struct GoogleCalendar {
    api_key: String,
}

impl GoogleCalendar {
    pub fn new(api_key: String) -> Self {
        Self { api_key }
    }
}

pub struct OutlookCalendar {
    access_token: String,
}

impl OutlookCalendar {
    pub fn new(access_token: String) -> Self {
        Self { access_token }
    }
}
