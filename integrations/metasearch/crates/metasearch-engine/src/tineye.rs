//! TinEye — reverse image search (JSON API)
//!
//! Searches by image URL via TinEye's internal JSON API:
//! `https://tineye.com/api/v1/result_json/?page={page}&url={image_url}`
//!
//! Reference: <https://github.com/searxng/searxng/blob/master/searx/engines/tineye.py>

use async_trait::async_trait;
use reqwest::Client;
use smallvec::smallvec;

use metasearch_core::{
    category::SearchCategory,
    engine::{EngineMetadata, SearchEngine},
    error::MetasearchError,
    query::SearchQuery,
    result::SearchResult,
};

pub struct TinEye {
    client: Client,
}

impl TinEye {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl SearchEngine for TinEye {
    fn metadata(&self) -> EngineMetadata {
        EngineMetadata {
            name: "TinEye".to_string().into(),
            display_name: "TinEye".to_string().into(),
            homepage: "https://tineye.com".to_string().into(),
            categories: smallvec![SearchCategory::Images, SearchCategory::General],
            enabled: true,
            timeout_ms: 5000,
            weight: 1.0,
        }
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>, MetasearchError> {
        // TinEye expects an image URL as the query
        let image_url = &query.query;

        let api_url = format!(
            "https://tineye.com/api/v1/result_json/?page={}&url={}",
            query.page,
            urlencoding::encode(image_url)
        );

        let resp = self
            .client
            .get(&api_url)
            .header("Host", "tineye.com")
            .header("DNT", "1")
            .header("Connection", "keep-alive")
            .send()
            .await
            .map_err(|e| MetasearchError::Engine(format!("TinEye request error: {e}")))?;

        // Handle 400/422 gracefully
        if resp.status().as_u16() == 400 || resp.status().as_u16() == 422 {
            return Ok(vec![]);
        }

        let json: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| MetasearchError::Engine(format!("TinEye parse error: {e}")))?;

        let mut results = Vec::new();

        if let Some(matches) = json.get("matches").and_then(|v| v.as_array()) {
            for (i, m) in matches.iter().enumerate() {
                let backlinks = m.get("backlinks").and_then(|v| v.as_array());
                let backlink = backlinks.and_then(|bl| bl.first());

                let bl = match backlink {
                    Some(b) => b,
                    None => continue,
                };

                let page_url = bl
                    .get("backlink")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();
                let image_name = bl
                    .get("image_name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Image");
                let domain = m.get("domain").and_then(|v| v.as_str()).unwrap_or_default();
                let width = m.get("width").and_then(|v| v.as_u64()).unwrap_or(0);
                let height = m.get("height").and_then(|v| v.as_u64()).unwrap_or(0);

                let mut result = SearchResult::new(
                    image_name,
                    page_url,
                    format!("Found on {} — {}×{}", domain, width, height),
                    "tineye",
                );
                result.engine_rank = (i + 1) as u32;
                results.push(result);

                if results.len() >= 20 {
                    break;
                }
            }
        }

        Ok(results)
    }
}
