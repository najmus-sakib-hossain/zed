//! NASA Astrophysics Data System (ADS) search engine.
//! Requires API key from https://ui.adsabs.harvard.edu/help/api
//! JSON API: https://api.adsabs.harvard.edu/v1/search/query
//! Features: Paging

use async_trait::async_trait;
use chrono::{NaiveDate, TimeZone, Utc};
use metasearch_core::{
    category::SearchCategory,
    engine::{EngineMetadata, SearchEngine},
    error::{MetasearchError, Result},
    query::SearchQuery,
    result::SearchResult,
};
use reqwest::Client;
use tracing::info;
use smallvec::smallvec;

pub struct AstrophysicsDataSystem {
    metadata: EngineMetadata,
    client: Client,
    api_key: Option<String>,
}

impl AstrophysicsDataSystem {
    pub fn new(client: Client, api_key: Option<String>) -> Self {
        Self {
            metadata: EngineMetadata {
                name: "astrophysics_data_system".to_string().into(),
                display_name: "ADS".to_string().into(),
                homepage: "https://ui.adsabs.harvard.edu".to_string().into(),
                categories: smallvec![SearchCategory::Science],
                enabled: api_key.is_some(),
                timeout_ms: 8000,
                weight: 1.0,
            },
            client,
            api_key,
        }
    }
}

#[async_trait]
impl SearchEngine for AstrophysicsDataSystem {
    fn metadata(&self) -> EngineMetadata {
        self.metadata.clone()
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>> {
        let key = self.api_key.as_deref().unwrap_or("");
        if key.is_empty() {
            return Err(MetasearchError::EngineError {
                engine: "astrophysics_data_system".to_string(),
                message:
                    "No API key configured. Get one at https://ui.adsabs.harvard.edu/help/api"
                        .to_string(),
            });
        }

        let rows = 10u32;
        let start = (query.page.max(1) - 1) * rows;
        let url = format!(
            "https://api.adsabs.harvard.edu/v1/search/query\
             ?q={}&fl=bibcode,title,author,abstract,doi,pub,year,date\
             &rows={}&start={}",
            urlencoding::encode(&query.query),
            rows,
            start,
        );

        let resp = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", key))
            .send()
            .await
            .map_err(|e| MetasearchError::HttpError(e.to_string()))?;

        let data: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| MetasearchError::ParseError(format!("JSON error: {}", e)))?;

        // Check for API errors
        if let Some(error_msg) = data
            .get("error")
            .and_then(|e| e.get("msg"))
            .and_then(|m| m.as_str())
        {
            return Err(MetasearchError::EngineError {
                engine: "astrophysics_data_system".to_string(),
                message: error_msg.to_string(),
            });
        }

        let mut results = Vec::new();

        let docs = match data["response"]["docs"].as_array() {
            Some(docs) => docs,
            None => return Ok(results),
        };

        for (i, doc) in docs.iter().enumerate() {
            let title = doc["title"]
                .as_array()
                .and_then(|t| t.first())
                .and_then(|t| t.as_str())
                .unwrap_or_default();

            if title.is_empty() {
                continue;
            }

            let bibcode = doc["bibcode"].as_str().unwrap_or_default();
            let result_url = format!(
                "https://ui.adsabs.harvard.edu/abs/{}/abstract",
                urlencoding::encode(bibcode),
            );

            let abstract_text = doc["abstract"].as_str().unwrap_or_default();
            let authors = doc["author"]
                .as_array()
                .map(|a| {
                    a.iter()
                        .filter_map(|v| v.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                })
                .unwrap_or_default();

            let pub_name = doc["pub"].as_str().unwrap_or_default();
            let year = doc["year"].as_str().unwrap_or_default();

            let mut content_parts = Vec::new();
            if !abstract_text.is_empty() {
                let snippet = if abstract_text.len() > 300 {
                    &abstract_text[..300]
                } else {
                    abstract_text
                };
                content_parts.push(snippet.to_string());
            }
            if !authors.is_empty() {
                content_parts.push(format!("Authors: {}", authors));
            }
            if !pub_name.is_empty() || !year.is_empty() {
                let pub_info = [pub_name, year]
                    .iter()
                    .filter(|s| !s.is_empty())
                    .copied()
                    .collect::<Vec<_>>()
                    .join(", ");
                content_parts.push(pub_info);
            }

            let content = content_parts.join(" | ");

            // Parse date if available (ADS format: YYYY-MM-DD or YYYY-MM-00)
            let published_date = doc["date"]
                .as_str()
                .and_then(|d| {
                    NaiveDate::parse_from_str(d, "%Y-%m-%d")
                        .ok()
                        .and_then(|nd| nd.and_hms_opt(0, 0, 0))
                        .map(|ndt| Utc.from_utc_datetime(&ndt))
                });

            let mut r = SearchResult::new(
                title,
                &result_url,
                &content,
                "astrophysics_data_system",
            );
            r.engine_rank = (i + 1) as u32;
            r.category = SearchCategory::Science.to_string();
            r.published_date = published_date;
            results.push(r);
        }

        info!(
            engine = "astrophysics_data_system",
            count = results.len(),
            "Search complete"
        );
        Ok(results)
    }
}
