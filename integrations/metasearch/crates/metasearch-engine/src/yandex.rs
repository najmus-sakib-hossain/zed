//! Yandex — Russian web search engine (HTML scraping)
//!
//! Reference: <https://github.com/searxng/searxng/blob/master/searx/engines/yandex.py>

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

pub struct Yandex {
    metadata: EngineMetadata,
    client: Client,
}

impl Yandex {
    pub fn new(client: Client) -> Self {
        Self {
            metadata: EngineMetadata {
                name: "yandex".to_string().into(),
                display_name: "Yandex".to_string().into(),
                homepage: "https://yandex.com".to_string().into(),
                categories: smallvec![SearchCategory::General],
                enabled: true,
                timeout_ms: 5000,
                weight: 0.7,
            },
            client,
        }
    }
}

#[async_trait]
impl SearchEngine for Yandex {
    fn metadata(&self) -> EngineMetadata {
        self.metadata.clone()
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>, MetasearchError> {
        let page_param = if query.page > 1 {
            format!("&p={}", query.page - 1)
        } else {
            String::new()
        };

        let url = format!(
            "https://yandex.com/search/site/?tmpl_version=releases&text={}&web=1&frame=1&searchid=3131712{}",
            urlencoding::encode(&query.query),
            page_param
        );

        let resp = self
            .client
            .get(&url)
            .header(
                "Cookie",
                "yp=1716337604.sp.family%3A0#1685406411.szm.1:1920x1080:1920x999",
            )
            .send()
            .await
            .map_err(|e| MetasearchError::HttpError(e.to_string()))?;

        let html_text = resp
            .text()
            .await
            .map_err(|e| MetasearchError::HttpError(e.to_string()))?;

        let document = Html::parse_document(&html_text);

        let result_sel = Selector::parse("li.serp-item").unwrap();
        let url_sel = Selector::parse("a.b-serp-item__title-link").unwrap();
        let title_sel =
            Selector::parse("h3.b-serp-item__title a.b-serp-item__title-link span").unwrap();
        let content_sel = Selector::parse("div.b-serp-item__text").unwrap();

        let mut results = Vec::new();

        for (i, item) in document.select(&result_sel).enumerate() {
            let link = item
                .select(&url_sel)
                .next()
                .and_then(|el| el.value().attr("href"))
                .unwrap_or_default()
                .to_string();

            let title = item
                .select(&title_sel)
                .next()
                .map(|el| el.text().collect::<String>())
                .unwrap_or_default();

            let content = item
                .select(&content_sel)
                .next()
                .map(|el| el.text().collect::<String>())
                .unwrap_or_default();

            if !title.is_empty() && !link.is_empty() {
                let mut result = SearchResult::new(
                    title.trim().to_string(),
                    link,
                    content.trim().to_string(),
                    self.metadata.name.clone(),
                );
                result.engine_rank = (i + 1) as u32;
                results.push(result);
            }
        }

        Ok(results)
    }
}
