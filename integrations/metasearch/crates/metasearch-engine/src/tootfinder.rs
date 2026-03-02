//! Tootfinder — Mastodon / fediverse post search
//!
//! Tootfinder is a search engine for public Mastodon posts.
//! Simple JSON API, no authentication required.
//!
//! Reference: <https://wiki.tootfinder.ch/index.php?name=the-tootfinder-rest-api>

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

pub struct Tootfinder {
    client: Client,
}

impl Tootfinder {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

#[derive(Debug, Deserialize)]
struct TootResult {
    url: Option<String>,
    content: Option<String>,
    #[allow(dead_code)]
    created_at: Option<String>,
    card: Option<TootCard>,
}

#[derive(Debug, Deserialize)]
struct TootCard {
    title: Option<String>,
}

#[async_trait]
impl SearchEngine for Tootfinder {
    fn metadata(&self) -> EngineMetadata {
        EngineMetadata {
            name: "Tootfinder".to_string().into(),
            display_name: "Tootfinder".to_string().into(),
            homepage: "https://Tootfinder.com".to_string().into(),
            categories: smallvec![SearchCategory::SocialMedia],
            enabled: true,
            timeout_ms: 5000,
            weight: 1.0,
        }
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>, MetasearchError> {
        let url = format!(
            "https://www.tootfinder.ch/rest/api/search/{}",
            urlencoding::encode(&query.query)
        );

        let resp = self.client.get(&url).send().await.map_err(|e| MetasearchError::Engine(e.to_string()))?;
        let text = resp.text().await.map_err(|e| MetasearchError::Engine(e.to_string()))?;

        // Tootfinder sometimes appends HTML error messages to the JSON;
        // only parse lines that start with '[{' (the actual JSON data).
        let json_str = text
            .lines()
            .find(|line| line.starts_with("[{"))
            .unwrap_or("[]");

        let data: Vec<TootResult> = serde_json::from_str(json_str).unwrap_or_default();

        let results = data
            .into_iter()
            .enumerate()
            .filter_map(|(i, item)| {
                let url = item.url?;
                let raw_content = item.content.unwrap_or_default();
                // Strip HTML tags for plain-text content
                let content = html_escape::decode_html_entities(
                    &regex::Regex::new(r"<[^>]+>")
                        .unwrap()
                        .replace_all(&raw_content, " "),
                )
                .trim()
                .to_string();

                let title = item.card.and_then(|c| c.title).unwrap_or_else(|| {
                    let truncated: String = content.chars().take(75).collect();
                    truncated
                });

                let mut result = SearchResult::new(
                    title,
                    url,
                    content,
                    "tootfinder",
                );
                result.engine_rank = (i + 1) as u32;
                Some(result)
            })
            .collect();

        Ok(results)
    }
}

