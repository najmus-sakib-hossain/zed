//! Niconico video search engine implementation.
//!
//! Uses the public Niconico Snapshot Search API v2.
//! API docs: https://snapshot.search.nicovideo.jp/api/v2/snapshot/version
//! Website: https://www.nicovideo.jp
//! Features: Paging

use async_trait::async_trait;
use metasearch_core::{
    category::SearchCategory,
    engine::{EngineMetadata, SearchEngine},
    error::Result,
    query::SearchQuery,
    result::SearchResult,
};
use reqwest::Client;
use tracing::info;
use smallvec::smallvec;

pub struct Niconico {
    metadata: EngineMetadata,
    client: Client,
}

impl Niconico {
    pub fn new(client: Client) -> Self {
        Self {
            metadata: EngineMetadata {
                name: "niconico".to_string().into(),
                display_name: "Niconico".to_string().into(),
                homepage: "https://www.nicovideo.jp".to_string().into(),
                categories: smallvec![SearchCategory::Videos],
                enabled: true,
                timeout_ms: 5000,
                weight: 1.0,
            },
            client,
        }
    }
}

#[async_trait]
impl SearchEngine for Niconico {
    fn metadata(&self) -> EngineMetadata {
        self.metadata.clone()
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>> {
        let encoded = urlencoding::encode(&query.query);
        let page = query.page.max(1);
        let limit: u32 = 20;
        let offset = (page - 1) * limit;

        // Use the public Niconico Snapshot Search API v2
        let url = format!(
            "https://snapshot.search.nicovideo.jp/api/v2/snapshot/video/contents/search\
             ?q={}&targets=title,description,tags\
             &fields=contentId,title,description,thumbnailUrl\
             &_limit={}&_offset={}&_sort=-viewCounter",
            encoded, limit, offset
        );

        let resp = match self
            .client
            .get(&url)
            .timeout(std::time::Duration::from_secs(6))
            .header(
                "User-Agent",
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
            )
            .send()
            .await
        {
            Ok(r) => r,
            Err(_) => return Ok(Vec::new()),
        };

        if !resp.status().is_success() {
            return Ok(Vec::new());
        }

        let data: serde_json::Value = match resp.json().await {
            Ok(v) => v,
            Err(_) => return Ok(Vec::new()),
        };

        let items = match data["data"].as_array() {
            Some(arr) => arr,
            None => return Ok(Vec::new()),
        };

        let mut results = Vec::new();

        for (i, item) in items.iter().enumerate() {
            let content_id = item["contentId"].as_str().unwrap_or_default();
            if content_id.is_empty() {
                continue;
            }

            let title = item["title"].as_str().unwrap_or_default();
            if title.is_empty() {
                continue;
            }

            let result_url = format!("https://www.nicovideo.jp/watch/{}", content_id);

            let description = item["description"]
                .as_str()
                .unwrap_or("")
                .chars()
                .take(200)
                .collect::<String>();

            let thumbnail = item["thumbnailUrl"]
                .as_str()
                .map(|s| s.to_string());

            let mut r = SearchResult::new(title, &result_url, &description, "niconico");
            r.engine_rank = i as u32;
            r.category = SearchCategory::Videos.to_string();
            r.thumbnail = thumbnail;
            results.push(r);
        }

        info!(
            engine = "niconico",
            count = results.len(),
            "Search complete"
        );
        Ok(results)
    }
}
