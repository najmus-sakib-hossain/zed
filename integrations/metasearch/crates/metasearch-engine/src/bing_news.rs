//! Bing News engine — search news via Bing News HTML scraping.
//! Translated from SearXNG `searx/engines/bing_news.py`.

use async_trait::async_trait;
use metasearch_core::{
    category::SearchCategory,
    engine::{EngineMetadata, SearchEngine},
    error::MetasearchError,
    query::SearchQuery,
    result::SearchResult,
};
use reqwest::Client;
use scraper::{Html, Selector};
use smallvec::smallvec;

pub struct BingNews {
    metadata: EngineMetadata,
    client: Client,
}

impl BingNews {
    pub fn new(client: Client) -> Self {
        Self {
            metadata: EngineMetadata {
                name: "bing_news".to_string().into(),
                display_name: "Bing News".to_string().into(),
                homepage: "https://www.bing.com/news".to_string().into(),
                categories: smallvec![SearchCategory::News],
                enabled: true,
                timeout_ms: 5000,
                weight: 1.0,
            },
            client,
        }
    }
}

#[async_trait]
impl SearchEngine for BingNews {
    fn metadata(&self) -> EngineMetadata {
        self.metadata.clone()
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>, MetasearchError> {
        let page = query.page;
        let first = (page - 1) * 10 + 1;

        let url = format!(
            "https://www.bing.com/news/infinitescrollajax?q={}&InfiniteScroll=1&first={}&SFX={}&form=PTFTNR",
            urlencoding::encode(&query.query),
            first,
            page - 1,
        );

        let resp = self
            .client
            .get(&url)
            .header(
                "User-Agent",
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
            )
            .send()
            .await
            .map_err(|e| MetasearchError::HttpError(e.to_string()))?;

        let html_text = resp
            .text()
            .await
            .map_err(|e| MetasearchError::ParseError(e.to_string()))?;

        let document = Html::parse_document(&html_text);
        let news_sel = Selector::parse("div.newsitem").unwrap();
        let title_link_sel = Selector::parse("a.title").unwrap();
        let snippet_sel = Selector::parse("div.snippet").unwrap();

        let mut results = Vec::new();

        for (i, newsitem) in document.select(&news_sel).enumerate() {
            let link = match newsitem.select(&title_link_sel).next() {
                Some(el) => el,
                None => continue,
            };

            let href = link.value().attr("href").unwrap_or_default();
            let title = link.text().collect::<String>().trim().to_string();

            let content = newsitem
                .select(&snippet_sel)
                .next()
                .map(|el| el.text().collect::<String>())
                .unwrap_or_default();

            let mut result = SearchResult::new(
                title,
                href.to_string(),
                content.trim().to_string(),
                "bing_news".to_string(),
            );
            result.engine_rank = (i + 1) as u32;
            result.category = SearchCategory::News.to_string();
            results.push(result);
        }

        Ok(results)
    }
}
