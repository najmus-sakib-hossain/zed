//! Search query model.

use crate::category::SearchCategory;
use serde::{Deserialize, Serialize};

/// Represents a user's search request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchQuery {
    /// The raw query string from the user.
    pub query: String,

    /// Which categories to search (web, images, news, etc.).
    pub categories: Vec<SearchCategory>,

    /// Language preference (e.g., "en", "de", "fr").
    pub language: Option<String>,

    /// Safe search level: 0 = off, 1 = moderate, 2 = strict.
    pub safe_search: u8,

    /// Pagination: which page of results.
    pub page: u32,

    /// Time range filter (e.g., "day", "week", "month", "year").
    pub time_range: Option<String>,

    /// Specific engines to query (overrides category defaults).
    pub engines: Vec<String>,
}

impl SearchQuery {
    pub fn new(query: impl Into<String>) -> Self {
        Self {
            query: query.into(),
            categories: vec![SearchCategory::General],
            language: None,
            safe_search: 1,
            page: 1,
            time_range: None,
            engines: Vec::new(),
        }
    }
}
