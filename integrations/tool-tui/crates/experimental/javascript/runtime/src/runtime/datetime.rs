//! Date and Time API

use std::time::{SystemTime, UNIX_EPOCH};

pub struct DateTime {
    timestamp: i64,
}

impl DateTime {
    pub fn now() -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0);
        Self { timestamp }
    }

    pub fn from_timestamp(ts: i64) -> Self {
        Self { timestamp: ts }
    }

    pub fn get_time(&self) -> i64 {
        self.timestamp
    }
    pub fn get_year(&self) -> i32 {
        1970 + (self.timestamp / (365 * 24 * 60 * 60 * 1000)) as i32
    }
    pub fn get_month(&self) -> i32 {
        ((self.timestamp / (30 * 24 * 60 * 60 * 1000)) % 12) as i32
    }
    pub fn get_date(&self) -> i32 {
        ((self.timestamp / (24 * 60 * 60 * 1000)) % 30) as i32 + 1
    }
    pub fn get_hours(&self) -> i32 {
        ((self.timestamp / (60 * 60 * 1000)) % 24) as i32
    }
    pub fn get_minutes(&self) -> i32 {
        ((self.timestamp / (60 * 1000)) % 60) as i32
    }
    pub fn get_seconds(&self) -> i32 {
        ((self.timestamp / 1000) % 60) as i32
    }
    pub fn get_milliseconds(&self) -> i32 {
        (self.timestamp % 1000) as i32
    }

    pub fn to_iso_string(&self) -> String {
        format!(
            "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}.{:03}Z",
            self.get_year(),
            self.get_month() + 1,
            self.get_date(),
            self.get_hours(),
            self.get_minutes(),
            self.get_seconds(),
            self.get_milliseconds()
        )
    }

    pub fn value_of(&self) -> i64 {
        self.timestamp
    }
}

impl std::fmt::Display for DateTime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_iso_string())
    }
}

impl Default for DateTime {
    fn default() -> Self {
        Self::now()
    }
}
