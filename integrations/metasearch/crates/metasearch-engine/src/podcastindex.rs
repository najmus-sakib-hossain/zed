//! Podcast Index — podcast search via podcastindex.org JSON API.

use async_trait::async_trait;
use metasearch_core::{
    category::SearchCategory,
    engine::{EngineMetadata, SearchEngine},
    error::MetasearchError,
    query::SearchQuery,
    result::SearchResult,
};
use reqwest::Client;
use serde::Deserialize;
use smallvec::smallvec;

pub struct PodcastIndex {
    metadata: EngineMetadata,
    client: Client,
}

impl PodcastIndex {
    pub fn new(client: Client) -> Self {
        Self {
            metadata: EngineMetadata {
                name: "podcastindex".to_string().into(),
                display_name: "Podcast Index".to_string().into(),
                homepage: "https://podcastindex.org".to_string().into(),
                categories: smallvec![SearchCategory::Music],
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
    feeds: Option<Vec<PodcastFeed>>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PodcastFeed {
    title: Option<String>,
    url: Option<String>,
    link: Option<String>,
    description: Option<String>,
    author: Option<String>,
    image: Option<String>,
    id: Option<u64>,
}

#[async_trait]
impl SearchEngine for PodcastIndex {
    fn metadata(&self) -> EngineMetadata {
        self.metadata.clone()
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>, MetasearchError> {
        // Use the public (no-auth) podcastindex.org API endpoint
        let url = format!(
            "https://podcastindex.org/api/search/byterm?q={}",
            urlencoding::encode(&query.query)
        );

        let resp = self
            .client
            .get(&url)
            .header("Accept", "application/json")
            .header("User-Agent", "metasearch/1.0")
            .send()
            .await
            .map_err(|e| MetasearchError::Engine(format!("PodcastIndex request failed: {e}")))?;

        let api: ApiResponse = resp
            .json()
            .await
            .map_err(|e| MetasearchError::Engine(format!("PodcastIndex parse failed: {e}")))?;

        let results = api
            .feeds
            .unwrap_or_default()
            .into_iter()
            .enumerate()
            .filter_map(|(i, feed)| {
                let title = feed.title.filter(|t| !t.is_empty())?;
                let result_url = feed
                    .link
                    .filter(|u| !u.is_empty())
                    .or_else(|| feed.url.filter(|u| !u.is_empty()))
                    .or_else(|| {
                        feed.id
                            .map(|id| format!("https://podcastindex.org/podcast/{}", id))
                    })?;
                let mut snippet = feed.description.unwrap_or_default();
                if let Some(author) = &feed.author {
                    if !author.is_empty() {
                        snippet = format!("By {} — {}", author, snippet);
                    }
                }
                let thumbnail_url = feed.image.filter(|u| !u.is_empty());
                let mut result = SearchResult::new(
                    title,
                    result_url,
                    snippet,
                    "podcastindex",
                );
                result.engine_rank = (i + 1) as u32;
                result.thumbnail = thumbnail_url;
                Some(result)
            })
            .collect();

        Ok(results)
    }
}
