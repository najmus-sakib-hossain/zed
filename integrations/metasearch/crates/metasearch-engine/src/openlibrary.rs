//! Open Library — open, editable library catalog (JSON API)
//!
//! Searches openlibrary.org for books using their search.json endpoint.

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

const SEARCH_API: &str = "https://openlibrary.org/search.json";
const BASE_URL: &str = "https://openlibrary.org";
const RESULTS_PER_PAGE: u32 = 10;

pub struct OpenLibrary {
    client: Client,
}

impl OpenLibrary {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

#[derive(Debug, Deserialize)]
struct OpenLibraryResponse {
    docs: Option<Vec<OpenLibraryDoc>>,
}

#[derive(Debug, Deserialize)]
struct OpenLibraryDoc {
    key: Option<String>,
    title: Option<String>,
    author_name: Option<Vec<String>>,
    first_publish_year: Option<u32>,
    first_sentence: Option<Vec<String>>,
    lending_identifier_s: Option<String>,
    #[allow(dead_code)]
    subject: Option<Vec<String>>,
}

#[async_trait]
impl SearchEngine for OpenLibrary {
    fn metadata(&self) -> EngineMetadata {
        EngineMetadata {
            name: "openlibrary".to_string().into(),
            display_name: "openlibrary".to_string().into(),
            homepage: "https://openlibrary.com".to_string().into(),
            categories: smallvec![SearchCategory::General],
            enabled: true,
            timeout_ms: 5000,
            weight: 1.0,
        }
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>, MetasearchError> {
        let url = format!(
            "{}?q={}&page={}&limit={}&fields=key,title,author_name,first_publish_year,first_sentence,lending_identifier_s,subject",
            SEARCH_API,
            urlencoding::encode(&query.query),
            query.page,
            RESULTS_PER_PAGE
        );

        let resp =
            self.client.get(&url).send().await.map_err(|e| {
                MetasearchError::Engine(format!("OpenLibrary request failed: {}", e))
            })?;

        let data: OpenLibraryResponse = resp
            .json()
            .await
            .map_err(|e| MetasearchError::Engine(format!("OpenLibrary parse failed: {}", e)))?;

        let results = data
            .docs
            .unwrap_or_default()
            .into_iter()
            .enumerate()
            .filter_map(|(i, doc)| {
                let title = doc.title?;
                let key = doc.key?;
                let page_url = format!("{}{}", BASE_URL, key);

                let mut snippet_parts = Vec::new();
                if let Some(authors) = &doc.author_name {
                    if !authors.is_empty() {
                        snippet_parts.push(format!("by {}", authors.join(", ")));
                    }
                }
                if let Some(year) = doc.first_publish_year {
                    snippet_parts.push(format!("({})", year));
                }
                if let Some(sentences) = &doc.first_sentence {
                    if let Some(first) = sentences.first() {
                        snippet_parts.push(first.clone());
                    }
                }
                let snippet = snippet_parts.join(" ");

                let thumbnail = doc
                    .lending_identifier_s
                    .map(|id| format!("https://archive.org/services/img/{}", id));

                let mut result = SearchResult::new(&title, &page_url, &snippet, "openlibrary");
                result.engine_rank = (i + 1) as u32;
                result.category = SearchCategory::General.to_string();
                result.thumbnail = thumbnail;
                Some(result)
            })
            .collect();

        Ok(results)
    }
}
