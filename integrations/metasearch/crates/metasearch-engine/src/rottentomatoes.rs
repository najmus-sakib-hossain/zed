//! Rotten Tomatoes — movie review aggregator (HTML scraping)
//!
//! Searches rottentomatoes.com for movies, TV shows, and celebrities.

use async_trait::async_trait;
use reqwest::Client;
use scraper::{Html, Selector};
use smallvec::smallvec;

use metasearch_core::{
    category::SearchCategory,
    engine::{EngineMetadata, SearchEngine},
    error::MetasearchError,
    query::SearchQuery,
    result::SearchResult,
};

const BASE_URL: &str = "https://www.rottentomatoes.com";

pub struct RottenTomatoes {
    client: Client,
}

impl RottenTomatoes {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl SearchEngine for RottenTomatoes {
    fn metadata(&self) -> EngineMetadata {
        EngineMetadata {
            name: "rottentomatoes".to_string().into(),
            display_name: "rottentomatoes".to_string().into(),
            homepage: "https://rottentomatoes.com".to_string().into(),
            categories: smallvec![SearchCategory::General],
            enabled: true,
            timeout_ms: 5000,
            weight: 1.0,
        }
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>, MetasearchError> {
        let url = format!(
            "{}/search?search={}",
            BASE_URL,
            urlencoding::encode(&query.query)
        );

        let resp = self.client.get(&url).send().await.map_err(|e| {
            MetasearchError::Engine(format!("RottenTomatoes request failed: {}", e))
        })?;

        let html_text = resp
            .text()
            .await
            .map_err(|e| MetasearchError::Engine(format!("RottenTomatoes read failed: {}", e)))?;

        let document = Html::parse_document(&html_text);
        let result_sel = Selector::parse("search-page-media-row").unwrap();
        let link_sel = Selector::parse("a").unwrap();
        let img_sel = Selector::parse("a img").unwrap();

        let mut results = Vec::new();

        for (i, element) in document.select(&result_sel).enumerate() {
            let link_el = match element.select(&link_sel).next() {
                Some(el) => el,
                None => continue,
            };

            let href = link_el.value().attr("href").unwrap_or_default();
            let result_url = if href.starts_with("http") {
                href.to_string()
            } else {
                format!("{}{}", BASE_URL, href)
            };

            let title = element
                .select(&img_sel)
                .next()
                .and_then(|el| el.value().attr("alt"))
                .unwrap_or_default()
                .to_string();

            let thumbnail = element
                .select(&img_sel)
                .next()
                .and_then(|el| el.value().attr("src"))
                .map(|s| s.to_string());

            if title.is_empty() {
                continue;
            }

            let mut content_parts = Vec::new();
            if let Some(year) = element.value().attr("releaseyear") {
                if !year.is_empty() {
                    content_parts.push(format!("From {}", year));
                }
            }
            if let Some(score) = element.value().attr("tomatometerscore") {
                if !score.is_empty() {
                    content_parts.push(format!("Score: {}", score));
                }
            }
            if let Some(cast) = element.value().attr("cast") {
                if !cast.is_empty() {
                    content_parts.push(format!("Starring {}", cast));
                }
            }
            let snippet = content_parts.join(", ");

            let mut result = SearchResult::new(&title, &result_url, &snippet, "rottentomatoes");
            result.engine_rank = (i + 1) as u32;
            result.category = SearchCategory::General.to_string();
            result.thumbnail = thumbnail;
            results.push(result);
        }

        Ok(results)
    }
}
