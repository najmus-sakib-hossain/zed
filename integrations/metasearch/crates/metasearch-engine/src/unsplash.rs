//! Unsplash engine — search free photos via Unsplash internal API.
//! Translated from SearXNG `searx/engines/unsplash.py`.
//!
//! Uses the undocumented `napi/search/photos` endpoint (no API key needed).
//! Strips the `ixid` tracking parameter from all returned URLs.

use async_trait::async_trait;
use metasearch_core::{
    category::SearchCategory,
    engine::{EngineMetadata, SearchEngine},
    error::MetasearchError,
    query::SearchQuery,
    result::SearchResult,
};
use reqwest::Client;
use url::Url;
use smallvec::smallvec;

pub struct Unsplash {
    metadata: EngineMetadata,
    client: Client,
}

impl Unsplash {
    pub fn new(client: Client) -> Self {
        Self {
            metadata: EngineMetadata {
                name: "unsplash".to_string().into(),
                display_name: "Unsplash".to_string().into(),
                homepage: "https://unsplash.com".to_string().into(),
                categories: smallvec![SearchCategory::Images],
                enabled: true,
                timeout_ms: 5000,
                weight: 1.0,
            },
            client,
        }
    }
}

/// Remove the `ixid` tracking parameter from an Unsplash URL.
fn clean_url(raw_url: &str) -> String {
    match Url::parse(raw_url) {
        Ok(mut parsed) => {
            let cleaned_query: Vec<(String, String)> = parsed
                .query_pairs()
                .filter(|(key, _)| key != "ixid")
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect();

            if cleaned_query.is_empty() {
                parsed.set_query(None);
            } else {
                let qs: String = cleaned_query
                    .iter()
                    .map(|(k, v)| format!("{}={}", k, v))
                    .collect::<Vec<_>>()
                    .join("&");
                parsed.set_query(Some(&qs));
            }
            parsed.to_string()
        }
        Err(_) => raw_url.to_string(),
    }
}

#[async_trait]
impl SearchEngine for Unsplash {
    fn metadata(&self) -> EngineMetadata {
        self.metadata.clone()
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>, MetasearchError> {
        let page = query.page;
        let per_page: u32 = 20;

        let url = format!(
            "https://unsplash.com/napi/search/photos?query={}&page={}&per_page={}",
            urlencoding::encode(&query.query),
            page,
            per_page,
        );

        let resp = self
            .client
            .get(&url)
            .header(
                "User-Agent",
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
            )
            .send()
            .await
            .map_err(|e| MetasearchError::HttpError(e.to_string()))?;

        let data: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| MetasearchError::ParseError(e.to_string()))?;

        let mut results = Vec::new();

        if let Some(items) = data["results"].as_array() {
            for (i, item) in items.iter().enumerate() {
                let page_url = item["links"]["html"].as_str().unwrap_or_default();
                let thumb_url = item["urls"]["thumb"].as_str().unwrap_or_default();
                let img_url = item["urls"]["regular"].as_str().unwrap_or_default();

                let title = item["alt_description"].as_str().unwrap_or("Unknown photo");
                let description = item["description"].as_str().unwrap_or("");

                if page_url.is_empty() {
                    continue;
                }

                let snippet = if description.is_empty() {
                    format!("Photo on Unsplash — {}", img_url)
                } else {
                    description.to_string()
                };

                let mut sr = SearchResult::new(
                    title.to_string(),
                    clean_url(page_url),
                    snippet,
                    "unsplash".to_string(),
                );
                sr.engine_rank = (i + 1) as u32;
                sr.category = SearchCategory::Images.to_string();
                sr.thumbnail = if thumb_url.is_empty() {
                    None
                } else {
                    Some(clean_url(thumb_url))
                };
                results.push(sr);
            }
        }

        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clean_url_removes_ixid() {
        let url = "https://images.unsplash.com/photo-123?ixlib=rb-4.0.3&ixid=abc123&w=400";
        let cleaned = clean_url(url);
        assert!(!cleaned.contains("ixid"));
        assert!(cleaned.contains("ixlib"));
        assert!(cleaned.contains("w=400"));
    }

    #[test]
    fn test_clean_url_no_ixid() {
        let url = "https://images.unsplash.com/photo-123?w=400";
        assert_eq!(clean_url(url), url);
    }
}
