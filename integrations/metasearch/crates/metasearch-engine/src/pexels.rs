//! Pexels image search engine.
//! Uses the Pexels API v3 to search for photos.
//! Requires an API key from <https://www.pexels.com/api/>.

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

pub struct Pexels {
    metadata: EngineMetadata,
    client: Client,
    api_key: Option<String>,
}

impl Pexels {
    pub fn new(client: Client, api_key: Option<String>) -> Self {
        let enabled = api_key.as_ref().is_some_and(|k| !k.is_empty());
        Self {
            metadata: EngineMetadata {
                name: "pexels".to_string().into(),
                display_name: "Pexels".to_string().into(),
                homepage: "https://www.pexels.com".to_string().into(),
                categories: smallvec![SearchCategory::Images],
                enabled,
                timeout_ms: 5000,
                weight: 1.0,
            },
            client,
            api_key,
        }
    }
}

#[async_trait]
impl SearchEngine for Pexels {
    fn metadata(&self) -> EngineMetadata {
        self.metadata.clone()
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>, MetasearchError> {
        let key = match self.api_key.as_deref() {
            Some(k) if !k.is_empty() => k,
            _ => {
                return Err(MetasearchError::EngineError {
                    engine: "pexels".to_string(),
                    message: "No API key configured".to_string(),
                });
            }
        };

        let page = query.page.max(1);
        let url = format!(
            "https://www.pexels.com/en-us/api/v3/search/photos?query={}&page={}&per_page=20",
            urlencoding::encode(&query.query),
            page,
        );

        let resp = self
            .client
            .get(&url)
            .header("Authorization", key)
            .send()
            .await
            .map_err(|e| MetasearchError::HttpError(e.to_string()))?;

        let json: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| MetasearchError::ParseError(e.to_string()))?;

        // The API may return photos under "data" or "photos"
        let photos = json
            .get("data")
            .or_else(|| json.get("photos"))
            .and_then(|v| v.as_array());

        let photos = match photos {
            Some(arr) => arr,
            None => return Ok(Vec::new()),
        };

        let mut results = Vec::new();

        for (i, photo) in photos.iter().enumerate() {
            let url = photo["url"].as_str().unwrap_or_default();
            if url.is_empty() {
                continue;
            }

            let title = photo["alt"]
                .as_str()
                .or_else(|| photo["photographer"].as_str())
                .unwrap_or("Untitled")
                .to_string();

            let photographer = photo["photographer"].as_str().unwrap_or("Unknown");
            let width = photo["width"].as_u64().unwrap_or(0);
            let height = photo["height"].as_u64().unwrap_or(0);
            let content = format!("By {} — {}x{}", photographer, width, height);

            let thumbnail = photo["src"]["medium"].as_str().unwrap_or_default();
            let img_src = photo["src"]["large2x"]
                .as_str()
                .or_else(|| photo["src"]["original"].as_str())
                .unwrap_or_default();

            let mut result = SearchResult::new(&title, url, &content, "pexels");
            result.engine_rank = (i + 1) as u32;
            result.category = SearchCategory::Images.to_string();
            if !thumbnail.is_empty() {
                result.thumbnail = Some(thumbnail.to_string());
            }
            if !img_src.is_empty() {
                result.metadata = serde_json::json!({ "img_src": img_src });
            }
            results.push(result);
        }

        Ok(results)
    }
}
