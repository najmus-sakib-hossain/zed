//! BPB engine — search Bundeszentrale für politische Bildung (German political education).
//! Translated from SearXNG `searx/engines/bpb.py`.

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

pub struct Bpb {
    metadata: EngineMetadata,
    client: Client,
}

impl Bpb {
    pub fn new(client: Client) -> Self {
        Self {
            metadata: EngineMetadata {
                name: "bpb".to_string().into(),
                display_name: "BPB".to_string().into(),
                homepage: "https://www.bpb.de".to_string().into(),
                categories: smallvec![SearchCategory::General],
                enabled: true,
                timeout_ms: 5000,
                weight: 0.5,
            },
            client,
        }
    }
}

#[async_trait]
impl SearchEngine for Bpb {
    fn metadata(&self) -> EngineMetadata {
        self.metadata.clone()
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>, MetasearchError> {
        let page = query.page;
        let page_index = (page as i32) - 1;

        let url = format!(
            "https://www.bpb.de/bpbapi/filter/search?query%5Bterm%5D={}&page={}&sort%5Bdirection%5D=descending&payload%5Bnid%5D=65350",
            urlencoding::encode(&query.query),
            page_index,
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

        if let Some(teasers) = data["teaser"].as_array() {
            for (i, item) in teasers.iter().enumerate() {
                let title = item["teaser"]["title"].as_str().unwrap_or_default();
                let content = item["teaser"]["text"].as_str().unwrap_or_default();
                let link_path = item["teaser"]["link"]["url"].as_str().unwrap_or_default();
                let link = format!("https://www.bpb.de{}", link_path);

                let overline = item["extension"]["overline"].as_str().unwrap_or_default();

                let snippet = if content.is_empty() {
                    overline.to_string()
                } else if overline.is_empty() {
                    content.to_string()
                } else {
                    format!("{} — {}", overline, content)
                };

                let thumbnail = item["teaser"]["image"]["sources"]
                    .as_array()
                    .and_then(|sources| sources.last())
                    .and_then(|s| s["url"].as_str())
                    .map(|url_path| format!("https://www.bpb.de{}", url_path));

                let mut result =
                    SearchResult::new(title.to_string(), link, snippet, "bpb".to_string());
                result.engine_rank = (i + 1) as u32;
                result.category = SearchCategory::General.to_string();
                result.thumbnail = thumbnail;
                results.push(result);
            }
        }

        Ok(results)
    }
}
