//! MetaCPAN — search for Perl modules.
//!
//! Uses the MetaCPAN Elasticsearch-backed JSON API (POST).

use async_trait::async_trait;
use metasearch_core::{
    category::SearchCategory,
    engine::{EngineMetadata, SearchEngine},
    error::{MetasearchError, Result},
    query::SearchQuery,
    result::SearchResult,
};
use reqwest::Client;
use serde::Deserialize;
use serde_json::json;
use smallvec::smallvec;

pub struct MetaCpan {
    metadata: EngineMetadata,
    client: Client,
}

impl MetaCpan {
    pub fn new(client: Client) -> Self {
        Self {
            metadata: EngineMetadata {
                name: "metacpan".to_string().into(),
                display_name: "MetaCPAN".to_string().into(),
                homepage: "https://metacpan.org".to_string().into(),
                categories: smallvec![SearchCategory::IT],
                enabled: true,
                timeout_ms: 5000,
                weight: 1.0,
            },
            client,
        }
    }
}

#[derive(Deserialize)]
struct EsResponse {
    hits: Option<EsHits>,
}

#[derive(Deserialize)]
struct EsHits {
    #[serde(default)]
    hits: Vec<EsHit>,
}

#[derive(Deserialize)]
struct EsHit {
    #[serde(rename = "_source")]
    source: Option<EsSource>,
}

#[derive(Deserialize)]
struct EsSource {
    documentation: Option<String>,
    #[serde(rename = "abstract")]
    abstract_text: Option<String>,
    module: Option<Vec<ModuleInfo>>,
}

#[derive(Deserialize)]
struct ModuleInfo {
    name: Option<String>,
}

#[async_trait]
impl SearchEngine for MetaCpan {
    fn metadata(&self) -> EngineMetadata {
        self.metadata.clone()
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>> {
        let page = query.page;
        let from = (page.saturating_sub(1)) * 10;

        let body = json!({
            "query": {
                "multi_match": {
                    "query": query.query,
                    "fields": ["documentation", "abstract", "module.name"]
                }
            },
            "from": from,
            "size": 10
        });

        let resp = self
            .client
            .post("https://fastapi.metacpan.org/v1/file/_search")
            .json(&body)
            .send()
            .await
            .map_err(|e| MetasearchError::HttpError(e.to_string()))?;

        let es: EsResponse = resp
            .json()
            .await
            .map_err(|e| MetasearchError::ParseError(e.to_string()))?;

        let hits = match es.hits {
            Some(h) => h.hits,
            None => return Ok(Vec::new()),
        };

        let results = hits
            .into_iter()
            .enumerate()
            .filter_map(|(i, hit)| {
                let src = hit.source?;
                let doc = src.documentation.unwrap_or_default();
                if doc.is_empty() {
                    return None;
                }

                let title = src
                    .module
                    .and_then(|m| m.into_iter().next())
                    .and_then(|m| m.name)
                    .unwrap_or_else(|| doc.clone());

                let url = format!("https://metacpan.org/pod/{doc}");
                let snippet = src.abstract_text.unwrap_or_default();

                let mut result = SearchResult::new(title, url, snippet, self.metadata.name.clone());
                result.engine_rank = (i + 1) as u32;
                Some(result)
            })
            .collect();

        Ok(results)
    }
}
