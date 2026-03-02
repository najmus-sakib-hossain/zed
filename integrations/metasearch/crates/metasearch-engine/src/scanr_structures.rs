//! ScanR Structures search engine implementation.
//!
//! Queries the ScanR French research structures JSON API.
//! Website: https://scanr.enseignementsup-recherche.gouv.fr
//! Features: Paging

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

const SCANR_BASE: &str = "https://scanr.enseignementsup-recherche.gouv.fr";

pub struct ScanrStructures {
    metadata: EngineMetadata,
    client: Client,
}

impl ScanrStructures {
    pub fn new(client: Client) -> Self {
        Self {
            metadata: EngineMetadata {
                name: "scanr_structures".to_string().into(),
                display_name: "ScanR Structures".to_string().into(),
                homepage: SCANR_BASE.to_string().into(),
                categories: smallvec![SearchCategory::Science],
                enabled: true,
                timeout_ms: 5000,
                weight: 0.8,
            },
            client,
        }
    }
}

#[async_trait]
impl SearchEngine for ScanrStructures {
    fn metadata(&self) -> EngineMetadata {
        self.metadata.clone()
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>, MetasearchError> {
        let page = query.page.max(1);

        let body = serde_json::json!({
            "query": query.query,
            "searchField": "ALL",
            "sortDirection": "ASC",
            "sortOrder": "RELEVANCY",
            "page": page,
            "pageSize": 20
        });

        let resp = match self
            .client
            .post(format!("{}/api/structures/search", SCANR_BASE))
            .timeout(std::time::Duration::from_secs(6))
            .header("Content-Type", "application/json")
            .header("Accept", "application/json")
            .json(&body)
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

        let mut results = Vec::new();

        let items = data["results"].as_array();
        if let Some(items) = items {
            for (i, item) in items.iter().enumerate() {
                let id = item["id"].as_str().unwrap_or_default();
                if id.is_empty() {
                    continue;
                }

                let item_url = format!("{}/structure/{}", SCANR_BASE, id);

                let title = item["label"]["default"].as_str()
                    .or_else(|| item["label"]["fr"].as_str())
                    .unwrap_or_default();
                if title.is_empty() {
                    continue;
                }

                let content = item["highlights"]
                    .as_array()
                    .and_then(|arr| arr.first())
                    .and_then(|h| h["value"].as_str())
                    .unwrap_or_default();

                let thumbnail = item["logo"].as_str().map(|s| s.to_string());

                let mut r =
                    SearchResult::new(title, &item_url, content, "scanr_structures");
                r.engine_rank = i as u32;
                r.category = SearchCategory::Science.to_string();
                r.thumbnail = thumbnail;
                results.push(r);
            }
        }

        Ok(results)
    }
}
