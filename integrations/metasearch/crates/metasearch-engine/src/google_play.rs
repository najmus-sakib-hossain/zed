//! Google Play — Apps store search (HTML scraping)
//!
//! Reference: <https://github.com/searxng/searxng/blob/master/searx/engines/google_play.py>

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

pub struct GooglePlay {
    metadata: EngineMetadata,
    client: Client,
}

impl GooglePlay {
    pub fn new(client: Client) -> Self {
        Self {
            metadata: EngineMetadata {
                name: "google_play".to_string().into(),
                display_name: "Google Play".to_string().into(),
                homepage: "https://play.google.com".to_string().into(),
                categories: smallvec![SearchCategory::General],
                enabled: true,
                timeout_ms: 5000,
                weight: 0.6,
            },
            client,
        }
    }
}

#[async_trait]
impl SearchEngine for GooglePlay {
    fn metadata(&self) -> EngineMetadata {
        self.metadata.clone()
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>, MetasearchError> {
        let url = format!(
            "https://play.google.com/store/search?q={}&c=apps",
            urlencoding::encode(&query.query)
        );

        let resp = self
            .client
            .get(&url)
            .header("Cookie", "CONSENT=YES+")
            .send()
            .await
            .map_err(|e| MetasearchError::HttpError(e.to_string()))?;

        let html_text = resp
            .text()
            .await
            .map_err(|e| MetasearchError::HttpError(e.to_string()))?;

        let document = Html::parse_document(&html_text);
        let mut results = Vec::new();
        let mut rank = 0u32;

        // Spot / featured result
        let spot_sel = Selector::parse("div.ipRz4").unwrap();
        if let Some(spot) = document.select(&spot_sel).next() {
            let link_sel = Selector::parse("a.Qfxief").unwrap();
            let title_sel = Selector::parse("div.vWM94c").unwrap();
            let content_sel = Selector::parse("div.LbQbAe").unwrap();

            let link = spot
                .select(&link_sel)
                .next()
                .and_then(|el| el.value().attr("href"))
                .map(|href| {
                    if href.starts_with('/') {
                        format!("https://play.google.com{}", href)
                    } else {
                        href.to_string()
                    }
                })
                .unwrap_or_default();

            let title = spot
                .select(&title_sel)
                .next()
                .map(|el| el.text().collect::<String>())
                .unwrap_or_default();

            let content = spot
                .select(&content_sel)
                .next()
                .map(|el| el.text().collect::<String>())
                .unwrap_or_default();

            if !title.is_empty() && !link.is_empty() {
                rank += 1;
                let mut result = SearchResult::new(
                    title.trim().to_string(),
                    link,
                    content.trim().to_string(),
                    self.metadata.name.clone(),
                );
                result.engine_rank = rank;
                results.push(result);
            }
        }

        // List results
        let item_sel = Selector::parse("div[role='listitem']").unwrap();
        let a_sel = Selector::parse("a").unwrap();
        let title_sel = Selector::parse("span.DdYX5").unwrap();
        let desc_sel = Selector::parse("span.wMUdtb").unwrap();

        for item in document.select(&item_sel) {
            let link = item
                .select(&a_sel)
                .next()
                .and_then(|el| el.value().attr("href"))
                .map(|href| {
                    if href.starts_with('/') {
                        format!("https://play.google.com{}", href)
                    } else {
                        href.to_string()
                    }
                })
                .unwrap_or_default();

            let title = item
                .select(&title_sel)
                .next()
                .map(|el| el.text().collect::<String>())
                .unwrap_or_default();

            let content = item
                .select(&desc_sel)
                .next()
                .map(|el| el.text().collect::<String>())
                .unwrap_or_default();

            if !title.is_empty() && !link.is_empty() {
                rank += 1;
                let mut result = SearchResult::new(
                    title.trim().to_string(),
                    link,
                    content.trim().to_string(),
                    self.metadata.name.clone(),
                );
                result.engine_rank = rank;
                results.push(result);
            }
        }

        Ok(results)
    }
}
