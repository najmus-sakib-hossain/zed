//! StackExchange engine — search Q&A via StackExchange API v2.3.
//! Translated from SearXNG `searx/engines/stackexchange.py`.
//!
//! Works with any StackExchange site (stackoverflow, serverfault,
//! superuser, askubuntu, etc.) by changing the `api_site` parameter.
//! Default: `stackoverflow`.

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

pub struct StackExchange {
    metadata: EngineMetadata,
    client: Client,
    api_site: String,
}

impl StackExchange {
    pub fn new(client: Client) -> Self {
        Self {
            metadata: EngineMetadata {
                name: "stackexchange".to_string().into(),
                display_name: "StackExchange".to_string().into(),
                homepage: "https://stackoverflow.com".to_string().into(),
                categories: smallvec![SearchCategory::IT],
                enabled: true,
                timeout_ms: 5000,
                weight: 1.2,
            },
            client,
            api_site: "stackoverflow".to_string(),
        }
    }

    pub fn with_site(client: Client, site: &str) -> Self {
        Self {
            metadata: EngineMetadata {
                name: "stackexchange".to_string().into(),
                display_name: "StackExchange".to_string().into(),
                homepage: format!("https://{}.com", site).into(),
                categories: smallvec![SearchCategory::IT],
                enabled: true,
                timeout_ms: 5000,
                weight: 1.2,
            },
            client,
            api_site: site.to_string(),
        }
    }
}

#[async_trait]
impl SearchEngine for StackExchange {
    fn metadata(&self) -> EngineMetadata {
        self.metadata.clone()
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>, MetasearchError> {
        let page = query.page;
        let pagesize: u32 = 10;

        // StackExchange API v2.3 advanced search
        let url = format!(
            "https://api.stackexchange.com/2.3/search/advanced?q={}&page={}&pagesize={}&site={}&sort=activity&order=desc",
            urlencoding::encode(&query.query),
            page,
            pagesize,
            self.api_site,
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

        if let Some(items) = data["items"].as_array() {
            for (i, item) in items.iter().enumerate() {
                let question_id = item["question_id"].as_u64().unwrap_or(0);
                if question_id == 0 {
                    continue;
                }

                // Title — HTML entities are returned encoded by the API
                let raw_title = item["title"].as_str().unwrap_or("Untitled");
                let title = html_escape::decode_html_entities(raw_title).to_string();

                let question_url = format!("https://{}.com/q/{}", self.api_site, question_id,);

                // Tags
                let tags: Vec<String> = item["tags"]
                    .as_array()
                    .unwrap_or(&Vec::new())
                    .iter()
                    .filter_map(|t| t.as_str().map(|s| s.to_string()))
                    .collect();

                let owner = item["owner"]["display_name"].as_str().unwrap_or("");
                let owner_clean = html_escape::decode_html_entities(owner).to_string();
                let is_answered = item["is_answered"].as_bool().unwrap_or(false);
                let score = item["score"].as_i64().unwrap_or(0);
                let answer_count = item["answer_count"].as_u64().unwrap_or(0);
                let view_count = item["view_count"].as_u64().unwrap_or(0);

                // Build content string matching the Python format
                let mut content_parts = Vec::new();
                if !tags.is_empty() {
                    content_parts.push(format!("[{}]", tags.join(", ")));
                }
                content_parts.push(owner_clean);
                if is_answered {
                    content_parts.push("✅ answered".to_string());
                }
                content_parts.push(format!("score: {}", score));
                content_parts.push(format!("{} answers", answer_count));
                content_parts.push(format!("{} views", view_count));

                let snippet = content_parts.join(" // ");

                let mut sr =
                    SearchResult::new(title, question_url, snippet, "stackexchange".to_string());
                sr.engine_rank = (i + 1) as u32;
                sr.category = SearchCategory::IT.to_string();
                results.push(sr);
            }
        }

        Ok(results)
    }
}
