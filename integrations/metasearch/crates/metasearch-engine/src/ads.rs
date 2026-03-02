//! NASA Astrophysics Data System (ADS) search engine.
//! Translated from SearXNG's `astrophysics_data_system.py`.
//! Requires API key from https://ui.adsabs.harvard.edu/help/api
//! Uses ADS search/query API endpoint.
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

pub struct Ads {
    metadata: EngineMetadata,
    client: Client,
    api_key: Option<String>,
}

impl Ads {
    pub fn new(client: Client, api_key: Option<String>) -> Self {
        Self {
            metadata: EngineMetadata {
                name: "astrophysics_data_system".to_string().into(),
                display_name: "NASA ADS".to_string().into(),
                homepage: "https://ui.adsabs.harvard.edu/".to_string().into(),
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
struct AdsApiResponse {
    response: Option<AdsResponseBody>,
    error: Option<AdsError>,
}

#[derive(Deserialize)]
struct AdsResponseBody {
    docs: Option<Vec<AdsDoc>>,
}

#[derive(Deserialize)]
struct AdsError {
    msg: Option<String>,
}

#[derive(Deserialize)]
#[allow(dead_code)]
struct AdsDoc {
    title: Option<Vec<String>>,
    author: Option<Vec<String>>,
    bibcode: Option<String>,
    #[serde(rename = "abstract")]
    abstract_text: Option<String>,
    doi: Option<Vec<String>>,
    date: Option<String>,
    year: Option<String>,
    read_count: Option<u64>,
}

#[async_trait]
impl SearchEngine for Ads {
    fn metadata(&self) -> EngineMetadata {
        self.metadata.clone()
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>> {
        let key = self.api_key.as_deref().unwrap_or("");
        if key.is_empty() {
            return Err(MetasearchError::EngineError {
                engine: "astrophysics_data_system".to_string(),
                message: "No API key configured. Get one at https://ui.adsabs.harvard.edu/help/api"
                    .to_string(),
            });
        }

        let rows = 10u32;
        let start = (query.page.max(1) - 1) * rows;
        let fields = "abstract,author,bibcode,doi,date,title,volume,year,read_count";
        let url = format!(
            "https://api.adsabs.harvard.edu/v1/search/query?q={}&fl={}&rows={}&start={}&sort=read_count+desc",
            urlencoding::encode(&query.query),
            fields,
            rows,
            start
        );

        let resp = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", key))
            .send()
            .await
            .map_err(|e| MetasearchError::HttpError(e.to_string()))?;

        let data: AdsApiResponse = resp
            .json()
            .await
            .map_err(|e| MetasearchError::ParseError(e.to_string()))?;

        if let Some(err) = data.error {
            return Err(MetasearchError::EngineError {
                engine: "astrophysics_data_system".to_string(),
                message: err
                    .msg
                    .unwrap_or_else(|| "Unknown ADS API error".to_string()),
            });
        }

        let mut results = Vec::new();
        if let Some(response) = data.response {
            if let Some(docs) = response.docs {
                for (i, doc) in docs.iter().enumerate() {
                    let title = doc
                        .title
                        .as_ref()
                        .and_then(|t| t.first())
                        .cloned()
                        .unwrap_or_default();
                    if title.is_empty() {
                        continue;
                    }

                    let bibcode = doc.bibcode.as_deref().unwrap_or("");
                    let result_url = format!("https://ui.adsabs.harvard.edu/abs/{}/", bibcode);

                    let content = doc.abstract_text.as_deref().unwrap_or("");
                    let snippet = if content.len() > 300 {
                        &content[..300]
                    } else {
                        content
                    };

                    let mut r =
                        SearchResult::new(&title, &result_url, snippet, "astrophysics_data_system");
                    r.engine_rank = (i + 1) as u32;
                    r.category = SearchCategory::General.to_string();

                    // Parse date — ADS returns ISO format
                    if let Some(date_str) = &doc.date {
                        let cleaned = date_str.replace('Z', "+00:00");
                        if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(&cleaned) {
                            r.published_date = Some(dt.with_timezone(&chrono::Utc));
                        } else if let Ok(dt) =
                            chrono::NaiveDate::parse_from_str(date_str, "%Y-%m-%d")
                        {
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
        }

        info!(
            engine = "astrophysics_data_system",
            count = results.len(),
            "Search complete"
        );
        Ok(results)
    }
}
