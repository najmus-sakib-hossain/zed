//! Springer Nature academic search engine.
//! Translated from SearXNG's `springer.py`.
//! Requires API key from https://dev.springernature.com/subscription/
//! Uses Meta-API v2: https://dev.springernature.com/docs/api-endpoints/meta-api/
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

pub struct Springer {
    metadata: EngineMetadata,
    client: Client,
    api_key: Option<String>,
}

impl Springer {
    pub fn new(client: Client, api_key: Option<String>) -> Self {
        Self {
            metadata: EngineMetadata {
                name: "springer".to_string().into(),
                display_name: "Springer Nature".to_string().into(),
                homepage: "https://www.springernature.com/".to_string().into(),
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
struct SpringerResponse {
    records: Option<Vec<SpringerRecord>>,
}

#[derive(Deserialize)]
#[allow(dead_code)]
struct SpringerRecord {
    title: Option<String>,
    #[serde(rename = "abstract")]
    abstract_text: Option<String>,
    doi: Option<String>,
    url: Option<Vec<SpringerUrl>>,
    creators: Option<Vec<SpringerCreator>>,
    #[serde(rename = "publicationDate")]
    publication_date: Option<String>,
    #[serde(rename = "publicationName")]
    publication_name: Option<String>,
    publisher: Option<String>,
}

#[derive(Deserialize)]
struct SpringerUrl {
    format: Option<String>,
    platform: Option<String>,
    value: Option<String>,
}

#[derive(Deserialize)]
#[allow(dead_code)]
struct SpringerCreator {
    creator: Option<String>,
}

#[async_trait]
impl SearchEngine for Springer {
    fn metadata(&self) -> EngineMetadata {
        self.metadata.clone()
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>> {
        let key = self.api_key.as_deref().unwrap_or("");
        if key.is_empty() {
            return Err(MetasearchError::EngineError {
                engine: "springer".to_string(),
                message:
                    "No API key configured. Get one at https://dev.springernature.com/subscription/"
                        .to_string(),
            });
        }

        let nb_per_page = 10;
        let page = query.page.max(1);
        let start = (page - 1) * nb_per_page;
        let url = format!(
            "https://api.springernature.com/meta/v2/json?api_key={}&q={}&s={}&p={}",
            urlencoding::encode(key),
            urlencoding::encode(&query.query),
            start,
            nb_per_page
        );

        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| MetasearchError::HttpError(e.to_string()))?;

        // Springer returns 403 for premium features — just return empty
        if resp.status().as_u16() == 403 {
            return Ok(Vec::new());
        }

        let data: SpringerResponse = resp
            .json()
            .await
            .map_err(|e| MetasearchError::ParseError(e.to_string()))?;

        let mut results = Vec::new();
        if let Some(records) = data.records {
            for (i, record) in records.iter().enumerate() {
                let title = match &record.title {
                    Some(t) if !t.is_empty() => t.clone(),
                    _ => continue,
                };

                // Find best URL from url array
                let result_url = record
                    .url
                    .as_ref()
                    .and_then(|urls| {
                        // Prefer HTML on web platform
                        urls.iter()
                            .filter(|u| u.platform.as_deref() == Some("web"))
                            .find(|u| u.format.as_deref() == Some("html"))
                            .or_else(|| urls.iter().find(|u| u.platform.as_deref() == Some("web")))
                            .and_then(|u| u.value.as_ref())
                            .map(|v| v.replace("http://", "https://"))
                    })
                    .unwrap_or_else(|| {
                        record
                            .doi
                            .as_ref()
                            .map(|d| format!("https://doi.org/{}", d))
                            .unwrap_or_default()
                    });

                if result_url.is_empty() {
                    continue;
                }

                let content = record.abstract_text.as_deref().unwrap_or("");
                let snippet = if content.len() > 300 {
                    &content[..300]
                } else {
                    content
                };

                let mut r = SearchResult::new(&title, &result_url, snippet, "springer");
                r.engine_rank = (i + 1) as u32;
                r.category = SearchCategory::General.to_string();

                // Parse publication date (YYYY-MM-DD format)
                if let Some(date_str) = &record.publication_date {
                    if let Ok(dt) = chrono::NaiveDate::parse_from_str(date_str, "%Y-%m-%d") {
                        r.published_date = dt.and_hms_opt(0, 0, 0).map(|ndt| {
                            chrono::DateTime::<chrono::Utc>::from_naive_utc_and_offset(
                                ndt,
                                chrono::Utc,
                            )
                        });
                    }
                }

                results.push(r);
            }
        }

        info!(
            engine = "springer",
            count = results.len(),
            "Search complete"
        );
        Ok(results)
    }
}
