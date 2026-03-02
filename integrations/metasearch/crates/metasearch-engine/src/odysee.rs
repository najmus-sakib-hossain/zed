//! Odysee — decentralized video platform search.
//!
//! Queries the Odysee Lighthouse JSON API.

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

pub struct Odysee {
    metadata: EngineMetadata,
    client: Client,
}

impl Odysee {
    pub fn new(client: Client) -> Self {
        Self {
            metadata: EngineMetadata {
                name: "odysee".to_string().into(),
                display_name: "Odysee".to_string().into(),
                homepage: "https://odysee.com".to_string().into(),
                categories: smallvec![SearchCategory::Videos],
                enabled: true,
                timeout_ms: 5000,
                weight: 1.0,
            },
            client,
        }
    }
}

#[derive(Deserialize)]
struct OdyseeItem {
    name: Option<String>,
    #[serde(rename = "claimId")]
    claim_id: Option<String>,
    title: Option<String>,
    description: Option<String>,
    thumbnail: Option<String>,
}

#[async_trait]
impl SearchEngine for Odysee {
    fn metadata(&self) -> EngineMetadata {
        self.metadata.clone()
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>> {
        let page = query.page;
        let from = (page - 1) * 20;
        let url = format!(
            "https://lighthouse.odysee.tv/search?s={}&size=20&from={}&include=channel,thumbnail,title,description,duration&mediaType=video",
            urlencoding::encode(&query.query),
            from
        );

        let items: Vec<OdyseeItem> = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| MetasearchError::HttpError(e.to_string()))?
            .json()
            .await
            .map_err(|e| MetasearchError::ParseError(e.to_string()))?;

        let results = items
            .into_iter()
            .enumerate()
            .map(|(i, item)| {
                let name = item.name.unwrap_or_default();
                let claim_id = item.claim_id.unwrap_or_default();
                let page_url = format!("https://odysee.com/{}:{}", name, claim_id);
                let title = item.title.unwrap_or_default();
                let snippet = item.description.unwrap_or_default();
                let mut r = SearchResult::new(title, page_url, snippet, self.metadata.name.clone());
                r.engine_rank = (i + 1) as u32;
                r.thumbnail = item.thumbnail;
                r
            })
            .collect();

        Ok(results)
    }
}
