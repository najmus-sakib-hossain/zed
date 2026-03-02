//! ArtStation engine — search art projects via JSON API.
//! Translated from SearXNG `searx/engines/artstation.py`.
//!
//! ArtStation requires CSRF tokens for API calls. We fetch them
//! from the token endpoint before each search request.

use async_trait::async_trait;
use metasearch_core::{
    category::SearchCategory,
    engine::{EngineMetadata, SearchEngine},
    error::MetasearchError,
    query::SearchQuery,
    result::SearchResult,
};
use regex::Regex;
use reqwest::Client;
use smallvec::smallvec;

pub struct ArtStation {
    metadata: EngineMetadata,
    client: Client,
}

impl ArtStation {
    pub fn new(client: Client) -> Self {
        Self {
            metadata: EngineMetadata {
                name: "artstation".to_string().into(),
                display_name: "ArtStation".to_string().into(),
                homepage: "https://www.artstation.com".to_string().into(),
                categories: smallvec![SearchCategory::Images],
                enabled: true,
                timeout_ms: 5000,
                weight: 0.8,
            },
            client,
        }
    }
}

#[async_trait]
impl SearchEngine for ArtStation {
    fn metadata(&self) -> EngineMetadata {
        self.metadata.clone()
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>, MetasearchError> {
        let page = query.page;

        // Step 1: Fetch CSRF tokens
        let token_resp = self
            .client
            .post("https://www.artstation.com/api/v2/csrf_protection/token.json")
            .send()
            .await
            .map_err(|e| MetasearchError::HttpError(e.to_string()))?;

        let private_token = token_resp
            .headers()
            .get_all("set-cookie")
            .iter()
            .filter_map(|v| v.to_str().ok())
            .find(|s| s.starts_with("PRIVATE-CSRF-TOKEN="))
            .and_then(|s| s.split('=').nth(1))
            .and_then(|s| s.split(';').next())
            .unwrap_or_default()
            .to_string();

        let token_data: serde_json::Value = token_resp
            .json()
            .await
            .map_err(|e| MetasearchError::ParseError(e.to_string()))?;

        let public_token = token_data["public_csrf_token"]
            .as_str()
            .unwrap_or_default()
            .to_string();

        // Step 2: Perform the search
        let form_data = serde_json::json!({
            "query": query.query,
            "page": page,
            "per_page": 20,
            "sorting": "relevance",
            "pro_first": 1,
        });

        let resp = self
            .client
            .post("https://www.artstation.com/api/v2/search/projects.json")
            .header("Content-Type", "application/json")
            .header("PUBLIC-CSRF-TOKEN", &public_token)
            .header("Cookie", format!("PRIVATE-CSRF-TOKEN={}", private_token))
            .json(&form_data)
            .send()
            .await
            .map_err(|e| MetasearchError::HttpError(e.to_string()))?;

        let data: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| MetasearchError::ParseError(e.to_string()))?;

        let mut results = Vec::new();
        let size_re = Regex::new(r"/\d{6,}/").unwrap();

        if let Some(items) = data["data"].as_array() {
            for (i, item) in items.iter().enumerate() {
                let title = item["title"].as_str().unwrap_or("Untitled");
                let item_url = item["url"].as_str().unwrap_or_default();
                let thumb = item["smaller_square_cover_url"]
                    .as_str()
                    .unwrap_or_default();

                if item_url.is_empty() {
                    continue;
                }

                // Derive full-size image from thumbnail
                let fullsize = size_re
                    .replace(thumb, "/")
                    .replace("smaller_square", "large");

                let username = item["user"]["username"].as_str().unwrap_or("");
                let full_name = item["user"]["full_name"].as_str().unwrap_or("");
                let author = if !username.is_empty() && !full_name.is_empty() {
                    format!("{} ({})", username, full_name)
                } else {
                    username.to_string()
                };

                let snippet = if author.is_empty() {
                    "ArtStation project".to_string()
                } else {
                    format!("By {}", author)
                };

                let mut result = SearchResult::new(
                    title.to_string(),
                    item_url.to_string(),
                    snippet,
                    "artstation".to_string(),
                );
                result.engine_rank = (i + 1) as u32;
                result.category = SearchCategory::Images.to_string();
                result.thumbnail = Some(fullsize.to_string());
                results.push(result);
            }
        }

        Ok(results)
    }
}
