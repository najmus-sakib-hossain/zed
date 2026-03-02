//! Baidu search engine implementation.
//!
//! Translated from SearXNG's `baidu.py` (186 lines, JSON API).
//! Baidu is the largest Chinese search engine.
//! Website: https://www.baidu.com
//! Features: Paging, Time Range

use async_trait::async_trait;
use metasearch_core::{
    category::SearchCategory,
    engine::{EngineMetadata, SearchEngine},
    error::Result,
    query::SearchQuery,
    result::SearchResult,
};
use reqwest::Client;
use serde::Deserialize;
use tracing::{info, warn};
use smallvec::smallvec;

pub struct Baidu {
    metadata: EngineMetadata,
    client: Client,
}

impl Baidu {
    pub fn new(client: Client) -> Self {
        Self {
            metadata: EngineMetadata {
                name: "baidu".to_string().into(),
                display_name: "Baidu".to_string().into(),
                homepage: "https://www.baidu.com".to_string().into(),
                categories: smallvec![SearchCategory::General],
                enabled: true,
                timeout_ms: 5000,
                weight: 1.0,
            },
            client,
        }
    }

    /// Map time_range to Baidu's gpc parameter (unix timestamps).
    fn time_range_secs(time_range: Option<&str>) -> Option<u64> {
        match time_range {
            Some("day") => Some(86400),
            Some("week") => Some(604800),
            Some("month") => Some(2592000),
            Some("year") => Some(31536000),
            _ => None,
        }
    }
}

/// Baidu JSON feed response structure.
#[derive(Deserialize, Debug)]
struct BaiduResponse {
    feed: Option<BaiduFeed>,
}

#[derive(Deserialize, Debug)]
struct BaiduFeed {
    entry: Option<Vec<BaiduEntry>>,
}

#[derive(Deserialize, Debug)]
struct BaiduEntry {
    title: Option<String>,
    url: Option<String>,
    #[serde(rename = "abs")]
    abs_text: Option<String>,
    time: Option<i64>,
}

#[async_trait]
impl SearchEngine for Baidu {
    fn metadata(&self) -> EngineMetadata {
        self.metadata.clone()
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>> {
        let results_per_page = 10;
        let page = query.page.max(1);
        let offset = (page - 1) * results_per_page;
        let encoded = urlencoding::encode(&query.query);

        let mut url = format!(
            "https://www.baidu.com/s?wd={}&rn={}&pn={}&tn=json",
            encoded, results_per_page, offset
        );

        // Add time range filter
        if let Some(secs) = Self::time_range_secs(query.time_range.as_deref()) {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            let past = now - secs;
            url.push_str(&format!("&gpc=stf%3D{}%2C{}%7Cstftype%3D1", past, now));
        }

        let resp = match self
            .client
            .get(&url)
            .timeout(std::time::Duration::from_secs(6))
            .header(
                "User-Agent",
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
            )
            .header("Accept-Language", "zh-CN,zh;q=0.9,en;q=0.8")
            .send()
            .await
        {
            Ok(r) => r,
            Err(_) => return Ok(Vec::new()),
        };

        // Check for captcha redirects
        if let Some(location) = resp.headers().get("location") {
            if let Ok(loc_str) = location.to_str() {
                if loc_str.contains("wappass.baidu.com") {
                    warn!(engine = "baidu", "Captcha detected");
                    return Ok(Vec::new());
                }
            }
        }

        let body = match resp.text().await {
            Ok(t) => t,
            Err(_) => return Ok(Vec::new()),
        };

        // Baidu's JSON sometimes contains raw control characters (like Python's strict=False).
        // Sanitize by replacing ASCII control chars (except tab/newline/carriage-return) with space.
        let sanitized: String = body
            .chars()
            .map(|c| {
                if c.is_ascii_control() && c != '\t' && c != '\n' && c != '\r' {
                    ' '
                } else {
                    c
                }
            })
            .collect();

        if sanitized.trim().is_empty() || !sanitized.trim_start().starts_with('{') {
            return Ok(Vec::new());
        }
        let data: BaiduResponse = match serde_json::from_str(&sanitized) {
            Ok(v) => v,
            Err(_) => return Ok(Vec::new()),
        };

        let mut results = Vec::new();

        if let Some(feed) = data.feed {
            if let Some(entries) = feed.entry {
                for (i, entry) in entries.iter().enumerate() {
                    let title = match &entry.title {
                        Some(t) if !t.is_empty() => html_unescape(t),
                        _ => continue,
                    };
                    let entry_url = match &entry.url {
                        Some(u) if !u.is_empty() => u.clone(),
                        _ => continue,
                    };
                    let snippet = entry
                        .abs_text
                        .as_deref()
                        .map(html_unescape)
                        .unwrap_or_default();

                    let mut r = SearchResult::new(&title, &entry_url, &snippet, "baidu");
                    r.engine_rank = i as u32;
                    r.category = SearchCategory::General.to_string();

                    // Parse published date from unix timestamp
                    if let Some(ts) = entry.time {
                        if ts > 0 {
                            if let Some(dt) = chrono::DateTime::from_timestamp(ts, 0) {
                                r.published_date = Some(dt);
                            }
                        }
                    }

                    results.push(r);
                }
            }
        }

        info!(engine = "baidu", count = results.len(), "Search complete");
        Ok(results)
    }
}

/// Simple HTML entity unescaping.
fn html_unescape(s: &str) -> String {
    s.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&nbsp;", " ")
}
