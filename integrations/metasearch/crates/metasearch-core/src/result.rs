//! Search result types.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// A single search result from an engine.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    /// Title of the result.
    pub title: String,

    /// URL of the result.
    pub url: String,

    /// Text snippet / description.
    pub content: String,

    /// Which engine produced this result.
    pub engine: String,

    /// The position this result appeared in the engine's response.
    pub engine_rank: u32,

    /// Aggregated score after ranking.
    #[serde(default)]
    pub score: f64,

    /// Optional thumbnail URL (for image/video results).
    pub thumbnail: Option<String>,

    /// Optional publication date.
    pub published_date: Option<DateTime<Utc>>,

    /// Result category.
    #[serde(default)]
    pub category: String,

    /// Additional metadata (engine-specific).
    #[serde(default)]
    pub metadata: serde_json::Value,
}

impl SearchResult {
    pub fn new(
        title: impl Into<String>,
        url: impl Into<String>,
        content: impl Into<String>,
        engine: impl Into<String>,
    ) -> Self {
        Self {
            title: title.into(),
            url: url.into(),
            content: content.into(),
            engine: engine.into(),
            engine_rank: 0,
            score: 0.0,
            thumbnail: None,
            published_date: None,
            category: "general".to_string(),
            metadata: serde_json::Value::Null,
        }
    }
}

/// The aggregated response returned to the user.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResponse {
    /// The original query string.
    pub query: String,

    /// Deduplicated and ranked results.
    pub results: Vec<SearchResult>,

    /// Number of results.
    pub number_of_results: usize,

    /// Which engines responded.
    pub engines_used: Vec<String>,

    /// Which engines failed.
    pub engines_failed: Vec<String>,

    /// Total search time in milliseconds.
    pub search_time_ms: u64,
}
