//! CORE (COnnecting REpositories) academic search engine.
//! Translated from SearXNG's `core.py`.
//! Requires API key from https://core.ac.uk/services/api
//! Uses CORE API v3: https://api.core.ac.uk/docs/v3
//! Features: Paging

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
use tracing::info;
use smallvec::smallvec;

pub struct CoreEngine {
    metadata: EngineMetadata,
    client: Client,
    api_key: Option<String>,
}

impl CoreEngine {
    pub fn new(client: Client, api_key: Option<String>) -> Self {
        Self {
            metadata: EngineMetadata {
                name: "core".to_string().into(),
                display_name: "CORE".to_string().into(),
                homepage: "https://core.ac.uk".to_string().into(),
                categories: smallvec![SearchCategory::General],
                enabled: true,
                timeout_ms: 8000,
                weight: 1.0,
            },
            client,
            api_key,
        }
    }
}

#[derive(Deserialize)]
struct CoreResponse {
    results: Option<Vec<CoreResult>>,
}

#[derive(Deserialize)]
struct CoreResult {
    title: Option<String>,
    doi: Option<String>,
    id: Option<u64>,
    #[serde(rename = "downloadUrl")]
    download_url: Option<String>,
    #[serde(rename = "sourceFulltextUrls")]
    source_urls: Option<String>,
    #[serde(rename = "fullText")]
    full_text: Option<String>,
    #[allow(dead_code)]
    authors: Option<Vec<CoreAuthor>>,
    #[serde(rename = "publishedDate")]
    published_date: Option<String>,
}

#[derive(Deserialize)]
#[allow(dead_code)]
struct CoreAuthor {
    name: Option<String>,
}

#[async_trait]
impl SearchEngine for CoreEngine {
    fn metadata(&self) -> EngineMetadata {
        self.metadata.clone()
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>> {
        let key = self.api_key.as_deref().unwrap_or("");
        if key.is_empty() {
            return Err(MetasearchError::EngineError {
                engine: "core".to_string(),
                message: "No API key configured. Register at https://core.ac.uk/services/api"
                    .to_string(),
            });
        }

        let nb_per_page = 10;
        let page = query.page.max(1);
        let offset = (page - 1) * nb_per_page;
        let url = format!(
            "https://api.core.ac.uk/v3/search/works/?q={}&offset={}&limit={}&sort=relevance",
            urlencoding::encode(&query.query),
            offset,
            nb_per_page
        );

        let resp = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", key))
            .send()
            .await
            .map_err(|e| MetasearchError::HttpError(e.to_string()))?;

        let data: CoreResponse = resp
            .json()
            .await
            .map_err(|e| MetasearchError::ParseError(e.to_string()))?;

        let mut results = Vec::new();
        if let Some(items) = data.results {
            for (i, item) in items.iter().enumerate() {
                let title = match &item.title {
                    Some(t) if !t.is_empty() => t.clone(),
                    _ => continue,
                };

                // Build URL: prefer DOI, then CORE link, then download URL
                let result_url = if let Some(doi) = &item.doi {
                    if !doi.is_empty() {
                        format!("https://doi.org/{}", doi)
                    } else {
                        String::new()
                    }
                } else if let Some(id) = item.id {
                    format!("https://core.ac.uk/works/{}", id)
                } else if let Some(dl) = &item.download_url {
                    dl.clone()
                } else if let Some(src) = &item.source_urls {
                    src.clone()
                } else {
                    continue;
                };

                if result_url.is_empty() {
                    continue;
                }

                let content = item.full_text.as_deref().unwrap_or("");
                let snippet = if content.len() > 300 {
                    &content[..300]
                } else {
                    content
                };

                let mut r = SearchResult::new(&title, &result_url, snippet, "core");
                r.engine_rank = (i + 1) as u32;
                r.category = SearchCategory::General.to_string();

                // Parse published date
                if let Some(date_str) = &item.published_date {
                    let cleaned = date_str.replace('Z', "+00:00");
                    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(&cleaned) {
                        r.published_date = Some(dt.with_timezone(&chrono::Utc));
                    }
                }

                results.push(r);
            }
        }

        info!(engine = "core", count = results.len(), "Search complete");
        Ok(results)
    }
}
