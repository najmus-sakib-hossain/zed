//! Matrix Rooms Search — search Matrix chat rooms via MRS API.
//!
//! Reference: <https://matrixrooms.info>

use async_trait::async_trait;
use reqwest::Client;
use smallvec::smallvec;

use metasearch_core::{
    category::SearchCategory,
    engine::{EngineMetadata, SearchEngine},
    error::{MetasearchError, Result},
    query::SearchQuery,
    result::SearchResult,
};

pub struct Mrs {
    metadata: EngineMetadata,
    client: Client,
    base_url: String,
}

impl Mrs {
    pub fn new(client: Client, base_url: &str) -> Self {
        let trimmed = base_url.trim_end_matches('/').to_string();
        Self {
            metadata: EngineMetadata {
                name: "mrs".to_string().into(),
                display_name: "Matrix Rooms Search".to_string().into(),
                homepage: "https://matrixrooms.info".to_string().into(),
                categories: smallvec![SearchCategory::SocialMedia],
                enabled: !trimmed.is_empty(),
                timeout_ms: 5000,
                weight: 1.0,
            },
            client,
            base_url: trimmed,
        }
    }
}

#[async_trait]
impl SearchEngine for Mrs {
    fn metadata(&self) -> EngineMetadata {
        self.metadata.clone()
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>> {
        if self.base_url.is_empty() {
            return Ok(Vec::new());
        }

        let page_size = 20u32;
        let offset = (query.page.saturating_sub(1)) * page_size;

        let url = format!(
            "{}/search/{}/{}/{}",
            self.base_url,
            urlencoding::encode(&query.query),
            page_size,
            offset,
        );

        let resp = self
            .client
            .get(&url)
            .header("User-Agent", "Mozilla/5.0")
            .send()
            .await
            .map_err(|e| MetasearchError::HttpError(e.to_string()))?;

        let data: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| MetasearchError::ParseError(e.to_string()))?;

        let mut results = Vec::new();

        let items = match data.as_array() {
            Some(arr) => arr,
            None => return Ok(results),
        };

        for (i, item) in items.iter().enumerate() {
            let alias = item["alias"].as_str().unwrap_or_default();
            if alias.is_empty() {
                continue;
            }

            let name = item["name"].as_str().unwrap_or(alias);
            let topic = item["topic"].as_str().unwrap_or_default();
            let members = item["members"].as_u64().unwrap_or(0);
            let avatar_url = item["avatar_url"].as_str().unwrap_or_default();

            let room_url = format!("https://matrix.to/#/{}", alias);

            let content = if topic.is_empty() {
                format!("{} members", members)
            } else {
                format!("{} | {} members", topic, members)
            };

            let mut result = SearchResult::new(
                name.to_string(),
                room_url,
                content,
                self.metadata.name.clone(),
            );
            result.engine_rank = (i + 1) as u32;
            result.category = SearchCategory::SocialMedia.to_string();
            if !avatar_url.is_empty() {
                result.thumbnail = Some(avatar_url.to_string());
            }
            results.push(result);
        }

        Ok(results)
    }
}
