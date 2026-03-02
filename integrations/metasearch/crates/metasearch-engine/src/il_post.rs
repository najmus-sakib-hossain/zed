//! Il Post — Italian news search engine.
//!
//! Uses the Il Post site-search JSON API.

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
use smallvec::smallvec;

pub struct IlPost {
    metadata: EngineMetadata,
    client: Client,
}

impl IlPost {
    pub fn new(client: Client) -> Self {
        Self {
            metadata: EngineMetadata {
                name: "il_post".to_string().into(),
                display_name: "Il Post".to_string().into(),
                homepage: "https://www.ilpost.it".to_string().into(),
                categories: smallvec![SearchCategory::News],
                enabled: true,
                timeout_ms: 5000,
                weight: 1.0,
            },
            client,
        }
    }
}

#[derive(Deserialize)]
struct ApiResponse {
    #[serde(default)]
    docs: Vec<DocItem>,
}

#[derive(Deserialize)]
struct DocItem {
    link: Option<String>,
    title: Option<String>,
    summary: Option<String>,
    image: Option<String>,
}

#[async_trait]
impl SearchEngine for IlPost {
    fn metadata(&self) -> EngineMetadata {
        self.metadata.clone()
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>> {
        let page = query.page;

        let resp = self
            .client
            .get("https://api.ilpost.org/search/api/site_search/")
            .query(&[
                ("qs", query.query.as_str()),
                ("pg", &page.to_string()),
                ("sort", "date_d"),
                ("filters", "ctype:articoli"),
            ])
            .send()
            .await
            .map_err(|e| MetasearchError::HttpError(e.to_string()))?;

        let api: ApiResponse = resp
            .json()
            .await
            .map_err(|e| MetasearchError::ParseError(e.to_string()))?;

        let results = api
            .docs
            .into_iter()
            .enumerate()
            .filter_map(|(i, doc)| {
                let title = doc.title.unwrap_or_default();
                let link = doc.link?;
                if title.is_empty() || link.is_empty() {
                    return None;
                }
                let snippet = doc.summary.unwrap_or_default();
                let mut result =
                    SearchResult::new(title, link, snippet, self.metadata.name.clone());
                result.engine_rank = (i + 1) as u32;
                result.thumbnail = doc.image;
                Some(result)
            })
            .collect();

        Ok(results)
    }
}
