//! OpenAlex — scientific papers search
//!
//! OpenAlex is a free, open catalog of the world's scholarly papers.
//! JSON API, no authentication required.
//!
//! Reference: <https://docs.openalex.org/>

use async_trait::async_trait;
use reqwest::Client;
use serde::Deserialize;
use smallvec::smallvec;

use metasearch_core::{
    category::SearchCategory,
    engine::{EngineMetadata, SearchEngine},
    error::MetasearchError,
    query::SearchQuery,
    result::SearchResult,
};

pub struct OpenAlex {
    client: Client,
}

impl OpenAlex {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

#[derive(Debug, Deserialize)]
struct ApiResponse {
    results: Option<Vec<Work>>,
}

#[derive(Debug, Deserialize)]
struct Work {
    title: Option<String>,
    doi: Option<String>,
    id: Option<String>,
    publication_year: Option<u32>,
    abstract_inverted_index: Option<serde_json::Value>,
    authorships: Option<Vec<Authorship>>,
    primary_location: Option<PrimaryLocation>,
    cited_by_count: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct Authorship {
    author: Option<Author>,
}

#[derive(Debug, Deserialize)]
struct Author {
    display_name: Option<String>,
}

#[derive(Debug, Deserialize)]
struct PrimaryLocation {
    landing_page_url: Option<String>,
    source: Option<Source>,
}

#[derive(Debug, Deserialize)]
struct Source {
    display_name: Option<String>,
}

/// Reconstruct abstract text from OpenAlex inverted-index format.
fn reconstruct_abstract(inverted_index: &serde_json::Value) -> String {
    if let Some(obj) = inverted_index.as_object() {
        let mut words: Vec<(u64, String)> = Vec::new();
        for (word, positions) in obj {
            if let Some(arr) = positions.as_array() {
                for pos in arr {
                    if let Some(idx) = pos.as_u64() {
                        words.push((idx, word.clone()));
                    }
                }
            }
        }
        words.sort_by_key(|(idx, _)| *idx);
        let text: Vec<&str> = words.iter().map(|(_, w)| w.as_str()).collect();
        let full = text.join(" ");
        if full.len() > 300 {
            format!("{}…", &full[..300])
        } else {
            full
        }
    } else {
        String::new()
    }
}

#[async_trait]
impl SearchEngine for OpenAlex {
    fn metadata(&self) -> EngineMetadata {
        EngineMetadata {
            name: "OpenAlex".to_string().into(),
            display_name: "OpenAlex".to_string().into(),
            homepage: "https://OpenAlex.com".to_string().into(),
            categories: smallvec![SearchCategory::Science],
            enabled: true,
            timeout_ms: 5000,
            weight: 1.0,
        }
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>, MetasearchError> {
        let url = format!(
            "https://api.openalex.org/works?search={}&page={}&per-page=10&sort=relevance_score:desc",
            urlencoding::encode(&query.query),
            query.page
        );

        let resp = self.client.get(&url).send().await.map_err(|e| MetasearchError::Engine(e.to_string()))?;
        let data: ApiResponse = resp.json().await.map_err(|e| MetasearchError::Engine(e.to_string()))?;

        let results = data
            .results
            .unwrap_or_default()
            .into_iter()
            .enumerate()
            .filter_map(|(i, work)| {
                let title = work.title?;

                // Prefer landing page URL, fall back to DOI, then OpenAlex ID
                let result_url = work
                    .primary_location
                    .as_ref()
                    .and_then(|loc| loc.landing_page_url.clone())
                    .or_else(|| {
                        work.doi.as_ref().map(|d| {
                            if d.starts_with("http") {
                                d.clone()
                            } else {
                                format!("https://doi.org/{}", d)
                            }
                        })
                    })
                    .or_else(|| work.id.clone())?;

                // Build authors string (first 3)
                let authors: String = work
                    .authorships
                    .unwrap_or_default()
                    .iter()
                    .filter_map(|a| a.author.as_ref()?.display_name.clone())
                    .take(3)
                    .collect::<Vec<_>>()
                    .join(", ");

                // Get journal name
                let journal = work
                    .primary_location
                    .and_then(|loc| loc.source)
                    .and_then(|s| s.display_name);

                // Reconstruct abstract if available
                let abstract_text = work
                    .abstract_inverted_index
                    .as_ref()
                    .map(reconstruct_abstract)
                    .unwrap_or_default();

                let mut content_parts = Vec::new();
                if !authors.is_empty() {
                    content_parts.push(authors);
                }
                if let Some(year) = work.publication_year {
                    content_parts.push(format!("({})", year));
                }
                if let Some(j) = journal {
                    content_parts.push(j);
                }
                if let Some(cited) = work.cited_by_count {
                    if cited > 0 {
                        content_parts.push(format!("{} citations", cited));
                    }
                }
                let mut content = content_parts.join(" — ");
                if !abstract_text.is_empty() {
                    if !content.is_empty() {
                        content.push_str(". ");
                    }
                    content.push_str(&abstract_text);
                }

                let mut result = SearchResult::new(
                    title,
                    result_url,
                    content,
                    "openalex",
                );
                result.engine_rank = (i + 1) as u32;
                Some(result)
            })
            .collect();

        Ok(results)
    }
}

