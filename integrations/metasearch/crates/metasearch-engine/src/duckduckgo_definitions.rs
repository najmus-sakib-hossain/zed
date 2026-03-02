//! DuckDuckGo Instant Answer API engine.
//! Translated from SearXNG's `duckduckgo_definitions.py`.
//! No API key required.
//! API docs: https://duckduckgo.com/api

use async_trait::async_trait;
use metasearch_core::{
    category::SearchCategory,
    engine::{EngineMetadata, SearchEngine},
    error::Result,
    query::SearchQuery,
    result::SearchResult,
};
use reqwest::Client;
use serde::Deserialize;
use tracing::info;
use smallvec::smallvec;

pub struct DuckDuckGoDefinitions {
    metadata: EngineMetadata,
    client: Client,
}

impl DuckDuckGoDefinitions {
    pub fn new(client: Client) -> Self {
        Self {
            metadata: EngineMetadata {
                name: "duckduckgo_definitions".to_string().into(),
                display_name: "DuckDuckGo Definitions".to_string().into(),
                homepage: "https://duckduckgo.com/".to_string().into(),
                categories: smallvec![SearchCategory::General],
                enabled: true,
                timeout_ms: 5000,
                weight: 1.0,
            },
            client,
        }
    }
}

#[derive(Deserialize)]
#[allow(dead_code)]
struct DdgResponse {
    #[serde(rename = "Heading")]
    heading: Option<String>,
    #[serde(rename = "Abstract")]
    abstract_text: Option<String>,
    #[serde(rename = "AbstractURL")]
    abstract_url: Option<String>,
    #[serde(rename = "Answer")]
    answer: Option<String>,
    #[serde(rename = "Definition")]
    definition: Option<String>,
    #[serde(rename = "DefinitionURL")]
    definition_url: Option<String>,
    #[serde(rename = "Results")]
    direct_results: Option<Vec<DdgDirectResult>>,
    #[serde(rename = "RelatedTopics")]
    related_topics: Option<Vec<DdgRelatedTopic>>,
}

#[derive(Deserialize)]
#[allow(dead_code)]
struct DdgDirectResult {
    #[serde(rename = "FirstURL")]
    first_url: Option<String>,
    #[serde(rename = "Text")]
    text: Option<String>,
}

#[derive(Deserialize)]
struct DdgRelatedTopic {
    #[serde(rename = "FirstURL")]
    first_url: Option<String>,
    #[serde(rename = "Text")]
    text: Option<String>,
}

#[async_trait]
impl SearchEngine for DuckDuckGoDefinitions {
    fn metadata(&self) -> EngineMetadata {
        self.metadata.clone()
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>> {
        let url = format!(
            "https://api.duckduckgo.com/?q={}&format=json&pretty=0&no_redirect=1&d=1",
            urlencoding::encode(&query.query)
        );

        let resp = match self
            .client
            .get(&url)
            .timeout(std::time::Duration::from_secs(7))
            .send()
            .await
        {
            Ok(r) => r,
            Err(_) => return Ok(Vec::new()),
        };

        if !resp.status().is_success() {
            return Ok(Vec::new());
        }

        let data: DdgResponse = match resp.json().await {
            Ok(v) => v,
            Err(_) => return Ok(Vec::new()),
        };

        let mut results = Vec::new();
        let mut rank = 1u32;

        // Main abstract result
        if let (Some(heading), Some(abstract_url)) = (&data.heading, &data.abstract_url) {
            if !heading.is_empty() && !abstract_url.is_empty() {
                let mut content_parts = Vec::new();
                if let Some(abs) = &data.abstract_text {
                    if !abs.is_empty() {
                        content_parts.push(abs.as_str());
                    }
                }
                if let Some(def) = &data.definition {
                    if !def.is_empty() {
                        content_parts.push(def.as_str());
                    }
                }
                let content = content_parts.join(" ");

                let mut r =
                    SearchResult::new(heading, abstract_url, &content, "duckduckgo_definitions");
                r.engine_rank = rank;
                rank += 1;
                r.category = SearchCategory::General.to_string();
                results.push(r);
            }
        }

        // Direct results (official website links)
        if let Some(direct) = &data.direct_results {
            for item in direct {
                if let (Some(url), Some(text)) = (&item.first_url, &item.text) {
                    if !url.is_empty() && !text.is_empty() {
                        let title = data.heading.as_deref().unwrap_or(text);
                        let mut r = SearchResult::new(title, url, text, "duckduckgo_definitions");
                        r.engine_rank = rank;
                        rank += 1;
                        r.category = SearchCategory::General.to_string();
                        results.push(r);
                    }
                }
            }
        }

        // Related topics
        if let Some(topics) = &data.related_topics {
            for topic in topics {
                if let (Some(url), Some(text)) = (&topic.first_url, &topic.text) {
                    if !url.is_empty() && !text.is_empty() {
                        // Skip broken links (text starting with http and containing spaces)
                        if text.starts_with("http") && text.contains(' ') {
                            continue;
                        }
                        let mut r = SearchResult::new(text, url, "", "duckduckgo_definitions");
                        r.engine_rank = rank;
                        rank += 1;
                        r.category = SearchCategory::General.to_string();
                        results.push(r);
                    }
                }
            }
        }

        info!(
            engine = "duckduckgo_definitions",
            count = results.len(),
            "Search complete"
        );
        Ok(results)
    }
}
