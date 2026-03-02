//! iQiyi — Chinese video search engine.
//!
//! Queries the iQiyi JSON API for video results.

use async_trait::async_trait;
use metasearch_core::{
    category::SearchCategory,
    engine::{EngineMetadata, SearchEngine},
    error::{MetasearchError, Result},
    query::SearchQuery,
    result::SearchResult,
};
use reqwest::Client;
use smallvec::smallvec;

pub struct Iqiyi {
    metadata: EngineMetadata,
    client: Client,
}

impl Iqiyi {
    pub fn new(client: Client) -> Self {
        Self {
            metadata: EngineMetadata {
                name: "iqiyi".to_string().into(),
                display_name: "iQiyi".to_string().into(),
                homepage: "https://www.iqiyi.com".to_string().into(),
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
impl SearchEngine for Iqiyi {
    fn metadata(&self) -> EngineMetadata {
        self.metadata.clone()
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>> {
        let page = query.page;
        let url = format!(
            "https://mesh.if.iqiyi.com/portal/lw/search/homePageV3?key={}&pageNum={}&pageSize=25",
            urlencoding::encode(&query.query),
            page
        );

        let resp: serde_json::Value = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| MetasearchError::HttpError(e.to_string()))?
            .json()
            .await
            .map_err(|e| MetasearchError::ParseError(e.to_string()))?;

        let mut results = Vec::new();
        let mut rank = 0u32;

        let templates = resp
            .pointer("/data/templates")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();

        for entry in &templates {
            let album_info = &entry["albumInfo"];
            let brief = album_info
                .pointer("/brief/value")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            let thumbnail = album_info
                .get("img")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();

            let videos = album_info.get("videos").and_then(|v| v.as_array());

            let items: Vec<&serde_json::Value> = match videos {
                Some(vids) => vids.iter().collect(),
                None => vec![album_info],
            };

            for video in items {
                rank += 1;
                let title = video
                    .get("title")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string();
                let page_url = video
                    .get("pageUrl")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .replace("http://", "https://");

                let mut r =
                    SearchResult::new(title, page_url, brief.clone(), self.metadata.name.clone());
                r.engine_rank = rank;
                r.thumbnail = if thumbnail.is_empty() {
                    None
                } else {
                    Some(thumbnail.clone())
                };
                results.push(r);
            }
        }

        Ok(results)
    }
}
