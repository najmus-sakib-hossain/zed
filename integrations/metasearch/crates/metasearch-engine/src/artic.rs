//! Art Institute of Chicago search engine implementation.
//!
//! Translated from SearXNG's `artic.py` (67 lines, JSON API).
//! Explore thousands of artworks from The Art Institute of Chicago.
//! Website: https://www.artic.edu
//! Features: Paging
//! API: http://api.artic.edu/docs/

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

pub struct Artic {
    metadata: EngineMetadata,
    client: Client,
}

impl Artic {
    pub fn new(client: Client) -> Self {
        Self {
            metadata: EngineMetadata {
                name: "artic".to_string().into(),
                display_name: "Art Institute of Chicago".to_string().into(),
                homepage: "https://www.artic.edu".to_string().into(),
                categories: smallvec![SearchCategory::Images],
                enabled: true,
                timeout_ms: 3000,
                weight: 0.8,
            },
            client,
        }
    }
}

#[derive(Deserialize, Debug)]
struct ArticResponse {
    data: Vec<ArticItem>,
}

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
struct ArticItem {
    id: u64,
    title: String,
    #[serde(default)]
    artist_display: String,
    #[serde(default)]
    medium_display: String,
    image_id: Option<String>,
    #[serde(default)]
    date_display: String,
    #[serde(default)]
    dimensions: String,
    #[serde(default)]
    artist_titles: Vec<String>,
}

#[async_trait]
impl SearchEngine for Artic {
    fn metadata(&self) -> EngineMetadata {
        self.metadata.clone()
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>> {
        let page = query.page.max(1);
        let per_page = 20;
        let encoded = urlencoding::encode(&query.query);

        let url = format!(
            "https://api.artic.edu/api/v1/artworks/search?q={}&page={}&fields=id,title,artist_display,medium_display,image_id,date_display,dimensions,artist_titles&limit={}",
            encoded, page, per_page
        );

        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| MetasearchError::HttpError(e.to_string()))?;

        let data: ArticResponse = resp
            .json()
            .await
            .map_err(|e| MetasearchError::ParseError(format!("JSON error: {}", e)))?;

        let mut results = Vec::new();

        for (i, item) in data.data.iter().enumerate() {
            let image_id = match &item.image_id {
                Some(id) if !id.is_empty() => id,
                _ => continue,
            };

            let title = format!(
                "{} ({}) // {}",
                item.title, item.date_display, item.artist_display
            );

            let snippet = format!("{} // {}", item.medium_display, item.dimensions);
            let result_url = format!("https://artic.edu/artworks/{}", item.id);
            let img_src = format!(
                "https://www.artic.edu/iiif/2/{}/full/843,/0/default.jpg",
                image_id
            );

            let mut r = SearchResult::new(&title, &result_url, &snippet, "artic");
            r.engine_rank = i as u32;
            r.category = SearchCategory::Images.to_string();
            r.thumbnail = Some(img_src);

            results.push(r);
        }

        info!(engine = "artic", count = results.len(), "Search complete");
        Ok(results)
    }
}
