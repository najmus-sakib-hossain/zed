//! Elasticsearch search — configurable instance URL, index, and credentials.
//! SearXNG equivalent: `elasticsearch.py`
//!
//! Supports `simple_query_string` queries by default. Configure
//! `base_url`, `index`, and optionally `username`/`password` for auth.

use async_trait::async_trait;
use reqwest::Client;
use serde_json::Value;
use smallvec::smallvec;

use metasearch_core::{
    category::SearchCategory,
    engine::{EngineMetadata, SearchEngine},
    error::MetasearchError,
    query::SearchQuery,
    result::SearchResult,
};

pub struct ElasticsearchEngine {
    client: Client,
    base_url: String,
    index: String,
    username: Option<String>,
    password: Option<String>,
}

impl ElasticsearchEngine {
    pub fn new(
        client: Client,
        base_url: &str,
        index: &str,
        username: Option<String>,
        password: Option<String>,
    ) -> Self {
        Self {
            client,
            base_url: base_url.trim_end_matches('/').to_string(),
            index: index.to_string(),
            username,
            password,
        }
    }
}

#[async_trait]
impl SearchEngine for ElasticsearchEngine {
    fn metadata(&self) -> EngineMetadata {
        EngineMetadata {
            name: "Elasticsearch".to_string().into(),
            display_name: "Elasticsearch".to_string().into(),
            homepage: "https://www.elastic.co".to_string().into(),
            categories: smallvec![SearchCategory::General],
            enabled: !self.base_url.is_empty() && !self.index.is_empty(),
            timeout_ms: 5000,
            weight: 1.0,
        }
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>, MetasearchError> {
        if self.base_url.is_empty() || self.index.is_empty() {
            return Ok(Vec::new());
        }

        let from = (query.page - 1) * 10;
        let body = serde_json::json!({
            "query": {
                "simple_query_string": {
                    "query": query.query
                }
            },
            "from": from,
            "size": 10
        });

        let url = format!("{}/{}/_search", self.base_url, self.index);

        let mut req = self
            .client
            .get(&url)
            .header("Content-Type", "application/json")
            .body(body.to_string());

        if let (Some(user), Some(pass)) = (&self.username, &self.password) {
            req = req.basic_auth(user, Some(pass));
        }

        let resp = req
            .send()
            .await
            .map_err(|e| MetasearchError::Engine(format!("Elasticsearch: {e}")))?;

        let json: Value = resp
            .json()
            .await
            .map_err(|e| MetasearchError::Engine(format!("Elasticsearch JSON: {e}")))?;

        if let Some(error) = json.get("error") {
            return Err(MetasearchError::Engine(format!(
                "Elasticsearch error: {}",
                error
            )));
        }

        let hits = json["hits"]["hits"].as_array().cloned().unwrap_or_default();

        let mut results = Vec::new();
        for (i, hit) in hits.iter().enumerate() {
            let source = &hit["_source"];
            let title = source
                .as_object()
                .and_then(|obj| {
                    obj.get("title")
                        .or_else(|| obj.get("name"))
                        .and_then(|v| v.as_str())
                })
                .unwrap_or("Untitled")
                .to_string();

            let content = source
                .as_object()
                .and_then(|obj| {
                    obj.get("content")
                        .or_else(|| obj.get("description"))
                        .or_else(|| obj.get("text"))
                        .and_then(|v| v.as_str())
                })
                .unwrap_or("")
                .to_string();

            let doc_url = source
                .as_object()
                .and_then(|obj| obj.get("url").and_then(|v| v.as_str()))
                .unwrap_or("")
                .to_string();

            let result_url = if doc_url.is_empty() {
                format!(
                    "{}/{}/_doc/{}",
                    self.base_url,
                    self.index,
                    hit["_id"].as_str().unwrap_or("")
                )
            } else {
                doc_url
            };

            results.push(SearchResult {
                title,
                url: result_url,
                content,
                engine: "Elasticsearch".to_string(),
                engine_rank: (i + 1) as u32,
                score: 0.0,
                thumbnail: None,
                published_date: None,
                category: String::new(),
                metadata: serde_json::Value::Null,
            });
        }
        Ok(results)
    }
}
