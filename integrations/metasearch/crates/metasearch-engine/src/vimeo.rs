//! Vimeo engine — search videos via Vimeo's internal API using a per-request JWT.
//!
//! Vimeo now uses a SPA architecture; the old `var data = {...};` pattern no longer works.
//! Instead, we perform a 2-step request:
//!   1. GET the search page to extract an embedded public JWT token.
//!   2. Call api.vimeo.com/videos with that JWT in the Authorization header.

use async_trait::async_trait;
use metasearch_core::{
    category::SearchCategory,
    engine::{EngineMetadata, SearchEngine},
    error::Result,
    query::SearchQuery,
    result::SearchResult,
};
use reqwest::Client;
use smallvec::smallvec;

pub struct Vimeo {
    metadata: EngineMetadata,
    client: Client,
}

impl Vimeo {
    pub fn new(client: Client) -> Self {
        Self {
            metadata: EngineMetadata {
                name: "vimeo".to_string().into(),
                display_name: "Vimeo".to_string().into(),
                homepage: "https://vimeo.com".to_string().into(),
                categories: smallvec![SearchCategory::Videos],
                enabled: true,
                timeout_ms: 8000,
                weight: 1.0,
            },
            client,
        }
    }
}

/// Extract JWT from Vimeo's embedded page config.
/// Vimeo embeds config in `<script id="viewer-bootstrap" type="application/json">`
/// with content like `{"user":null,"jwt":"...","apiUrl":"api.vimeo.com",...}`.
fn extract_jwt(html: &str) -> Option<String> {
    // Directly search for the jwt key anywhere in the page (most reliable)
    let jwt_key = "\"jwt\":\"";
    let start = html.find(jwt_key)? + jwt_key.len();
    let remaining = &html[start..];
    let end = remaining.find('"')?;
    let jwt = &remaining[..end];
    if !jwt.is_empty() && jwt.len() > 20 {
        Some(jwt.to_string())
    } else {
        None
    }
}

#[async_trait]
impl SearchEngine for Vimeo {
    fn metadata(&self) -> EngineMetadata {
        self.metadata.clone()
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>> {
        let page = query.page.max(1);
        let per_page = 20u32;

        // Step 1: GET search page to extract public JWT
        let page_url = format!(
            "https://vimeo.com/search?q={}",
            urlencoding::encode(&query.query),
        );

        let page_resp = match self
            .client
            .get(&page_url)
            .header(
                "User-Agent",
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
            )
            .timeout(std::time::Duration::from_secs(3))
            .send()
            .await
        {
            Ok(r) => r,
            Err(_) => return Ok(Vec::new()),
        };

        if !page_resp.status().is_success() {
            return Ok(Vec::new());
        }

        let html = match page_resp.text().await {
            Ok(t) => t,
            Err(_) => return Ok(Vec::new()),
        };

        let jwt = match extract_jwt(&html) {
            Some(j) => j,
            None => return Ok(Vec::new()),
        };

        // Step 2: Call Vimeo API with JWT
        let api_url = format!(
            "https://api.vimeo.com/videos?query={}&per_page={}&page={}&direction=relevance",
            urlencoding::encode(&query.query),
            per_page,
            page,
        );

        let api_resp = match self
            .client
            .get(&api_url)
            .header("Authorization", format!("jwt {}", jwt))
            .header("Accept", "application/vnd.vimeo.*+json;version=3.4")
            .header(
                "User-Agent",
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
            )
            .timeout(std::time::Duration::from_secs(4))
            .send()
            .await
        {
            Ok(r) => r,
            Err(_) => return Ok(Vec::new()),
        };

        if !api_resp.status().is_success() {
            return Ok(Vec::new());
        }

        let data: serde_json::Value = match api_resp.json().await {
            Ok(v) => v,
            Err(_) => return Ok(Vec::new()),
        };

        let mut results = Vec::new();

        if let Some(items) = data["data"].as_array() {
            for (i, item) in items.iter().enumerate() {
                let uri = item["uri"].as_str().unwrap_or_default();
                let video_id = uri.rsplit('/').next().unwrap_or("");
                if video_id.is_empty() {
                    continue;
                }

                let video_url = format!("https://vimeo.com/{}", video_id);
                let title = item["name"].as_str().unwrap_or("Untitled").to_string();

                let description = item["description"]
                    .as_str()
                    .unwrap_or("")
                    .chars()
                    .take(200)
                    .collect::<String>();

                let thumbnail = item["pictures"]["sizes"]
                    .as_array()
                    .and_then(|sizes| sizes.last())
                    .and_then(|s| s["link"].as_str())
                    .map(|s| s.to_string());

                let mut sr = SearchResult::new(title, video_url, description, "vimeo".to_string());
                sr.engine_rank = (i + 1) as u32;
                sr.category = SearchCategory::Videos.to_string();
                sr.thumbnail = thumbnail;
                results.push(sr);
            }
        }

        Ok(results)
    }
}
