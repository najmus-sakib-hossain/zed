//! Yep — privacy-focused search engine.
//!
//! Queries the Yep JSON API for web results.

use async_trait::async_trait;
use metasearch_core::{
    category::SearchCategory,
    engine::{EngineMetadata, SearchEngine},
    error::{MetasearchError, Result},
    query::SearchQuery,
    result::SearchResult,
};
use reqwest::Client;
use smallvec::smallvec;

pub struct Yep {
    metadata: EngineMetadata,
    client: Client,
}

impl Yep {
    pub fn new(client: Client) -> Self {
        Self {
            metadata: EngineMetadata {
                name: "yep".to_string().into(),
                display_name: "Yep".to_string().into(),
                homepage: "https://yep.com".to_string().into(),
                categories: smallvec![SearchCategory::General],
                enabled: true,
                timeout_ms: 5000,
                weight: 1.0,
            },
            client,
        }
    }
}

// Structs for Yep JSON response — ported from metasearch2/src/engines/search/yep.rs
#[allow(dead_code)]
#[derive(serde::Deserialize, Debug)]
struct YepApiResponse {
    pub results: Vec<YepApiResult>,
}

#[derive(serde::Deserialize, Debug)]
struct YepApiResult {
    pub url: String,
    pub title: String,
    pub snippet: String,
}

#[async_trait]
impl SearchEngine for Yep {
    fn metadata(&self) -> EngineMetadata {
        self.metadata.clone()
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>> {
        // URL and JSON parsing ported directly from metasearch2/src/engines/search/yep.rs
        let url = reqwest::Url::parse_with_params(
            "https://api.yep.com/fs/2/search",
            &[
                ("client", "web"),
                ("gl", "all"),
                ("no_correct", "true"),
                ("q", query.query.as_str()),
                ("safeSearch", "off"),
                ("type", "web"),
            ],
        )
        .map_err(|e| MetasearchError::HttpError(e.to_string()))?;

        let resp = self
            .client
            .get(url)
            .send()
            .await
            .map_err(|e| MetasearchError::HttpError(e.to_string()))?;

        let text = resp
            .text()
            .await
            .map_err(|e| MetasearchError::ParseError(e.to_string()))?;

        // Response is a tuple: ["Ok", {results: [...]}]
        let (code, api): (String, YepApiResponse) = match serde_json::from_str(&text) {
            Ok(v) => v,
            Err(_) => return Ok(Vec::new()),
        };

        if code != "Ok" {
            return Ok(Vec::new());
        }

        let results = api
            .results
            .into_iter()
            .enumerate()
            .map(|(i, item)| {
                // Strip HTML from snippet (metasearch2 does the same)
                let snippet_doc = scraper::Html::parse_document(&item.snippet);
                let description: String = snippet_doc.root_element().text().collect();
                let mut r = SearchResult::new(&item.title, &item.url, &description, "yep");
                r.engine_rank = (i + 1) as u32;
                r.category = SearchCategory::General.to_string();
                r
            })
            .collect();

        Ok(results)
    }
}
