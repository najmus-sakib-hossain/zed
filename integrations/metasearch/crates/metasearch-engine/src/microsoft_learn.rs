//! Microsoft Learn engine — search Microsoft documentation.

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

const PAGE_SIZE: u32 = 10;

pub struct MicrosoftLearn {
    metadata: EngineMetadata,
    client: Client,
}

impl MicrosoftLearn {
    pub fn new(client: Client) -> Self {
        Self {
            metadata: EngineMetadata {
                name: "microsoft_learn".to_string().into(),
                display_name: "Microsoft Learn".to_string().into(),
                homepage: "https://learn.microsoft.com".to_string().into(),
                categories: smallvec![SearchCategory::IT],
                enabled: true,
                timeout_ms: 5000,
                weight: 1.0,
            },
            client,
        }
    }
}

#[async_trait]
impl SearchEngine for MicrosoftLearn {
    fn metadata(&self) -> EngineMetadata {
        self.metadata.clone()
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>, MetasearchError> {
        let skip = (query.page - 1) * PAGE_SIZE;

        let url = format!(
            "https://learn.microsoft.com/api/search?search={}&locale=en-us&$top={}&$skip={}&expandScope=true",
            urlencoding::encode(&query.query),
            PAGE_SIZE,
            skip,
        );

        let resp = self
            .client
            .get(&url)
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
                let title = item["title"].as_str().unwrap_or_default();
                let item_url = item["url"].as_str().unwrap_or_default();
                let description = item["description"].as_str().unwrap_or("");

                if title.is_empty() || item_url.is_empty() {
                    continue;
                }

                let mut result = SearchResult::new(
                    title.to_string(),
                    item_url.to_string(),
                    description.to_string(),
                    "microsoft_learn".to_string(),
                );
                result.engine_rank = (i + 1) as u32;
                result.category = SearchCategory::IT.to_string();
                results.push(result);
            }
        }

        Ok(results)
    }
}
