//! Yahoo News — news search via HTML scraping.
//!
//! Reference: <https://news.yahoo.com>

use async_trait::async_trait;
use reqwest::Client;
use scraper::{Html, Selector};
use smallvec::smallvec;

use metasearch_core::{
    category::SearchCategory,
    engine::{EngineMetadata, SearchEngine},
    error::{MetasearchError, Result},
    query::SearchQuery,
    result::SearchResult,
};

pub struct YahooNews {
    metadata: EngineMetadata,
    client: Client,
}

impl YahooNews {
    pub fn new(client: Client) -> Self {
        Self {
            metadata: EngineMetadata {
                name: "yahoo_news".to_string().into(),
                display_name: "Yahoo News".to_string().into(),
                homepage: "https://news.yahoo.com".to_string().into(),
                categories: smallvec![SearchCategory::News],
                enabled: true,
                timeout_ms: 8000,
                weight: 1.0,
            },
            client,
        }
    }
}

#[async_trait]
impl SearchEngine for YahooNews {
    fn metadata(&self) -> EngineMetadata {
        self.metadata.clone()
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>> {
        let offset = (query.page.saturating_sub(1)) * 10 + 1;

        let url = format!(
            "https://news.search.yahoo.com/search?p={}&b={}",
            urlencoding::encode(&query.query),
            offset,
        );

        let resp = self
            .client
            .get(&url)
            .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
            .header("Accept-Language", "en-US,en;q=0.9")
            .send()
            .await
            .map_err(|e| MetasearchError::HttpError(e.to_string()))?;

        let body = resp
            .text()
            .await
            .map_err(|e| MetasearchError::HttpError(e.to_string()))?;

        let document = Html::parse_document(&body);

        let item_sel = Selector::parse("ol.searchCenterMiddle li")
            .map_err(|e| MetasearchError::ParseError(format!("{e:?}")))?;
        let title_sel = Selector::parse("h4 a")
            .map_err(|e| MetasearchError::ParseError(format!("{e:?}")))?;
        let content_sel = Selector::parse("p")
            .map_err(|e| MetasearchError::ParseError(format!("{e:?}")))?;

        let mut results = Vec::new();

        for (i, item) in document.select(&item_sel).enumerate() {
            let anchor = match item.select(&title_sel).next() {
                Some(a) => a,
                None => continue,
            };

            let title = anchor.text().collect::<String>().trim().to_string();
            if title.is_empty() {
                continue;
            }

            let href = anchor.value().attr("href").unwrap_or_default();
            if href.is_empty() {
                continue;
            }

            let content = item
                .select(&content_sel)
                .next()
                .map(|el| el.text().collect::<String>().trim().to_string())
                .unwrap_or_default();

            let mut result = SearchResult::new(
                title,
                href.to_string(),
                content,
                self.metadata.name.clone(),
            );
            result.engine_rank = (i + 1) as u32;
            result.category = SearchCategory::News.to_string();
            results.push(result);
        }

        Ok(results)
    }
}
