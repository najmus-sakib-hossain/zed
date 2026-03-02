//! Open Semantic Search — search via a self-hosted Open Semantic instance.
//!
//! Uses the Open Semantic JSON API.
//!
//! Reference: <https://opensemantic.org>

use async_trait::async_trait;
use metasearch_core::{
    category::SearchCategory,
    engine::{EngineMetadata, SearchEngine},
    error::MetasearchError,
    query::SearchQuery,
    result::SearchResult,
};
use reqwest::Client;
use smallvec::smallvec;

pub struct OpenSemantic {
    metadata: EngineMetadata,
    client: Client,
    base_url: String,
}

impl OpenSemantic {
    pub fn new(client: Client, base_url: &str) -> Self {
        let base = base_url.trim_end_matches('/').to_string();
        let enabled = !base.is_empty();
        Self {
            metadata: EngineMetadata {
                name: "opensemantic".to_string().into(),
                display_name: "Open Semantic".to_string().into(),
                homepage: "https://opensemantic.org".to_string().into(),
                categories: smallvec![SearchCategory::General],
                enabled,
                timeout_ms: 5000,
                weight: 0.7,
            },
            client,
            base_url: base,
        }
    }
}

#[async_trait]
impl SearchEngine for OpenSemantic {
    fn metadata(&self) -> EngineMetadata {
        self.metadata.clone()
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>, MetasearchError> {
        if self.base_url.is_empty() {
            return Ok(Vec::new());
        }

        let url = format!(
            "{}/api/search?q={}",
            self.base_url,
            urlencoding::encode(&query.query),
        );

        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| MetasearchError::HttpError(e.to_string()))?;

        let json: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| MetasearchError::ParseError(e.to_string()))?;

        let docs = match json
            .get("response")
            .and_then(|r| r.get("docs"))
            .and_then(|d| d.as_array())
        {
            Some(arr) => arr,
            None => return Ok(Vec::new()),
        };

        let mut results = Vec::new();

        for (i, doc) in docs.iter().enumerate() {
            let id = doc["id"].as_str().unwrap_or_default();
            if id.is_empty() {
                continue;
            }

            let title = doc["title"].as_str().unwrap_or("Untitled").to_string();

            let raw_content = doc["content"].as_str().unwrap_or_default();
            let content = if raw_content.len() > 300 {
                let mut end = 300;
                while !raw_content.is_char_boundary(end) && end > 0 {
                    end -= 1;
                }
                format!("{}…", &raw_content[..end])
            } else {
                raw_content.to_string()
            };

            let mut result = SearchResult::new(&title, id, &content, "opensemantic");
            result.engine_rank = (i + 1) as u32;
            results.push(result);
        }

        Ok(results)
    }
}
