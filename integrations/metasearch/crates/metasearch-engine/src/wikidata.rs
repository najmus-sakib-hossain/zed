//! Wikidata — search entities via the Wikidata wbsearchentities API.
//!
//! Uses the simpler wbsearchentities API instead of full SPARQL.
//!
//! Reference: <https://www.wikidata.org>

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

pub struct Wikidata {
    metadata: EngineMetadata,
    client: Client,
}

impl Wikidata {
    pub fn new(client: Client) -> Self {
        Self {
            metadata: EngineMetadata {
                name: "wikidata".to_string().into(),
                display_name: "Wikidata".to_string().into(),
                homepage: "https://www.wikidata.org".to_string().into(),
                categories: smallvec![SearchCategory::General],
                enabled: true,
                timeout_ms: 5000,
                weight: 0.8,
            },
            client,
        }
    }
}

#[async_trait]
impl SearchEngine for Wikidata {
    fn metadata(&self) -> EngineMetadata {
        self.metadata.clone()
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>, MetasearchError> {
        let url = format!(
            "https://www.wikidata.org/w/api.php?action=wbsearchentities&search={}&language=en&format=json&limit=10",
            urlencoding::encode(&query.query),
        );

        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| MetasearchError::HttpError(e.to_string()))?;

        let json: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| MetasearchError::ParseError(e.to_string()))?;

        let search_results = match json.get("search").and_then(|s| s.as_array()) {
            Some(arr) => arr,
            None => return Ok(Vec::new()),
        };

        let mut results = Vec::new();

        for (i, item) in search_results.iter().enumerate() {
            let id = item["id"].as_str().unwrap_or_default();
            if id.is_empty() {
                continue;
            }

            let label = item["label"].as_str().unwrap_or("Untitled").to_string();
            let description = item["description"].as_str().unwrap_or_default().to_string();
            let entity_url = format!("https://www.wikidata.org/wiki/{}", id);

            let mut result = SearchResult::new(&label, &entity_url, &description, "wikidata");
            result.engine_rank = (i + 1) as u32;
            results.push(result);
        }

        Ok(results)
    }
}
