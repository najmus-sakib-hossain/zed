//! FindThatMeme image/meme search engine.
//!
//! FindThatMeme is a meme search engine that indexes memes from
//! various sources. This engine queries their JSON API.

use async_trait::async_trait;
use reqwest::Client;
use serde::Deserialize;
use serde_json::json;

use metasearch_core::category::SearchCategory;
use metasearch_core::engine::{EngineMetadata, SearchEngine};
use metasearch_core::error::MetasearchError;
use metasearch_core::query::SearchQuery;
use metasearch_core::result::SearchResult;
use smallvec::smallvec;

const API_URL: &str = "https://findthatmeme.com/api/v1/search";
const RESULTS_PER_PAGE: u32 = 50;

pub struct FindThatMeme {
    metadata: EngineMetadata,
    client: Client,
}

impl FindThatMeme {
    pub fn new(client: Client) -> Self {
        Self {
            metadata: EngineMetadata {
                name: "findthatmeme".to_string().into(),
                display_name: "FindThatMeme".to_string().into(),
                homepage: "https://findthatmeme.com".to_string().into(),
                categories: smallvec![SearchCategory::Images],
                enabled: true,
                timeout_ms: 5000,
                weight: 0.6,
            },
            client,
        }
    }
}

#[derive(Deserialize)]
#[allow(dead_code)]
struct MemeItem {
    image_path: Option<String>,
    source_page_url: Option<String>,
    source_site: Option<String>,
    #[serde(rename = "type")]
    meme_type: Option<String>,
}

#[async_trait]
impl SearchEngine for FindThatMeme {
    fn metadata(&self) -> EngineMetadata {
        self.metadata.clone()
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>, MetasearchError> {
        let page = query.page;
        let offset = (page.saturating_sub(1)) * RESULTS_PER_PAGE;

        let body = json!({
            "search": query.query,
            "offset": offset
        });

        let resp = self
            .client
            .post(API_URL)
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| MetasearchError::HttpError(e.to_string()))?
            .json::<Vec<MemeItem>>()
            .await
            .map_err(|e| MetasearchError::ParseError(e.to_string()))?;

        let mut results = Vec::new();

        for item in resp {
            let source_url = item.source_page_url.unwrap_or_default();
            let title = item.source_site.unwrap_or_else(|| "Meme".to_string());
            let image_path = item.image_path.unwrap_or_default();

            if source_url.is_empty() || image_path.is_empty() {
                continue;
            }

            let img_url = format!("https://s3.thehackerblog.com/findthatmeme/{}", image_path);

            let mut result = SearchResult::new(title, source_url, "", "FindThatMeme");
            result.thumbnail = Some(img_url);
            results.push(result);
        }

        Ok(results)
    }
}
