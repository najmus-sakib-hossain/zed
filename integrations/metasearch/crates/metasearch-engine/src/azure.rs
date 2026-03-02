//! Azure Cognitive Search — query an Azure Search index.
//!
//! POST JSON to `{base_url}/indexes/{index_name}/docs/search?api-version=2021-04-30-Preview`.
//! Website: https://azure.microsoft.com/products/ai-services/cognitive-search
//! Features: Pagination via skip/top

use async_trait::async_trait;
use metasearch_core::{
    category::SearchCategory,
    engine::{EngineMetadata, SearchEngine},
    error::{MetasearchError, Result},
    query::SearchQuery,
    result::SearchResult,
};
use reqwest::Client;
use tracing::info;
use smallvec::smallvec;

pub struct Azure {
    metadata: EngineMetadata,
    client: Client,
    base_url: String,
    api_key: String,
    index_name: String,
}

impl Azure {
    pub fn new(
        client: Client,
        base_url: &str,
        api_key: Option<String>,
        index_name: Option<String>,
    ) -> Self {
        let base = base_url.trim_end_matches('/').to_string();
        let key = api_key.unwrap_or_default();
        let index = index_name.unwrap_or_default();
        let enabled = !base.is_empty() && !key.is_empty() && !index.is_empty();
        Self {
            metadata: EngineMetadata {
                name: "azure".to_string().into(),
                display_name: "Azure Search".to_string().into(),
                homepage: "https://azure.microsoft.com/products/ai-services/cognitive-search"
                    .to_string().into(),
                categories: smallvec![SearchCategory::General],
                enabled,
                timeout_ms: 5000,
                weight: 1.0,
            },
            client,
            base_url: base,
            api_key: key,
            index_name: index,
        }
    }
}

#[async_trait]
impl SearchEngine for Azure {
    fn metadata(&self) -> EngineMetadata {
        self.metadata.clone()
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>> {
        if self.base_url.is_empty() || self.api_key.is_empty() || self.index_name.is_empty() {
            return Ok(Vec::new());
        }

        let page = query.page.max(1);
        let skip = (page - 1) * 10;

        let url = format!(
            "{}/indexes/{}/docs/search?api-version=2021-04-30-Preview",
            self.base_url, self.index_name
        );

        let body = serde_json::json!({
            "search": query.query,
            "top": 10,
            "skip": skip
        });

        let resp = self
            .client
            .post(&url)
            .header("Content-Type", "application/json")
            .header("api-key", &self.api_key)
            .json(&body)
            .send()
            .await
            .map_err(|e| MetasearchError::HttpError(e.to_string()))?;

        let data: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| MetasearchError::ParseError(format!("Azure JSON error: {}", e)))?;

        let mut results = Vec::new();

        let items = match data.get("value").and_then(|v| v.as_array()) {
            Some(items) => items,
            None => return Ok(results),
        };

        for (i, item) in items.iter().enumerate() {
            let obj = match item.as_object() {
                Some(o) => o,
                None => continue,
            };

            // Try common title fields
            let title = obj
                .get("title")
                .or_else(|| obj.get("@search.highlights"))
                .or_else(|| obj.get("HtmlEncodedTitle"))
                .or_else(|| obj.get("name"))
                .and_then(|v| v.as_str())
                .unwrap_or("Untitled")
                .to_string();

            // Try common URL fields
            let result_url = obj
                .get("url")
                .or_else(|| obj.get("path"))
                .or_else(|| obj.get("uri"))
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();

            // Try common content fields
            let content = obj
                .get("content")
                .or_else(|| obj.get("description"))
                .or_else(|| obj.get("snippet"))
                .or_else(|| obj.get("body"))
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();

            if result_url.is_empty() && title == "Untitled" {
                continue;
            }

            let display_url = if result_url.is_empty() {
                format!("{}/indexes/{}", self.base_url, self.index_name)
            } else {
                result_url
            };

            let mut r = SearchResult::new(&title, &display_url, &content, "azure");
            r.engine_rank = i as u32;
            r.category = SearchCategory::General.to_string();
            results.push(r);
        }

        info!(
            engine = "azure",
            count = results.len(),
            "Search complete"
        );
        Ok(results)
    }
}
