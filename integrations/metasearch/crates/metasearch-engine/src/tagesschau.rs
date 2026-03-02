//! Tagesschau — German news search via tagesschau.de JSON API.

use async_trait::async_trait;
use metasearch_core::{
    category::SearchCategory,
    engine::{EngineMetadata, SearchEngine},
    error::MetasearchError,
    query::SearchQuery,
    result::SearchResult,
};
use reqwest::Client;
use serde::Deserialize;
use smallvec::smallvec;

pub struct Tagesschau {
    metadata: EngineMetadata,
    client: Client,
}

impl Tagesschau {
    pub fn new(client: Client) -> Self {
        Self {
            metadata: EngineMetadata {
                name: "tagesschau".to_string().into(),
                display_name: "Tagesschau".to_string().into(),
                homepage: "https://www.tagesschau.de".to_string().into(),
                categories: smallvec![SearchCategory::News],
                enabled: true,
                timeout_ms: 5000,
                weight: 1.0,
            },
            client,
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ApiResponse {
    search_results: Option<Vec<SearchItem>>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SearchItem {
    title: Option<String>,
    sophora_id: Option<String>,
    date: Option<String>,
}

#[async_trait]
impl SearchEngine for Tagesschau {
    fn metadata(&self) -> EngineMetadata {
        self.metadata.clone()
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>, MetasearchError> {
        let page_size = 20;
        let page_index = query.page.saturating_sub(1);
        // Note: do NOT include &resultPage=all - it causes HTTP 400
        let url = format!(
            "https://www.tagesschau.de/api2u/search/?searchText={}&pageSize={}&pageIndex={}",
            urlencoding::encode(&query.query),
            page_size,
            page_index
        );

        let resp = match self
            .client
            .get(&url)
            .timeout(std::time::Duration::from_secs(6))
            .header("Accept", "application/json")
            .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:120.0) Gecko/20100101 Firefox/120.0")
            .send()
            .await
        {
            Ok(r) => r,
            Err(_) => return Ok(Vec::new()),
        };

        if !resp.status().is_success() {
            return Ok(Vec::new());
        }

        let api: ApiResponse = match resp.json().await {
            Ok(v) => v,
            Err(_) => return Ok(Vec::new()),
        };

        let results = api
            .search_results
            .unwrap_or_default()
            .into_iter()
            .enumerate()
            .filter_map(|(i, item)| {
                let title = item.title.filter(|t| !t.is_empty())?;
                let sophora_id = item.sophora_id.filter(|s| !s.is_empty())?;
                // Build URL from sophoraId: https://www.tagesschau.de/{sophoraId}.html
                let result_url = format!("https://www.tagesschau.de/{}.html", sophora_id);
                let snippet = item.date.unwrap_or_default();
                let mut result = SearchResult::new(
                    title,
                    result_url,
                    snippet,
                    "tagesschau",
                );
                result.engine_rank = (i + 1) as u32;
                Some(result)
            })
            .collect();

        Ok(results)
    }
}
