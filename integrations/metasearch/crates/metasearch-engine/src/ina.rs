//! INA engine — search the French National Audiovisual Institute.
//! Translated from SearXNG `searx/engines/ina.py`.

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

const BASE_URL: &str = "https://www.ina.fr";
const PAGE_SIZE: u32 = 12;

pub struct Ina {
    metadata: EngineMetadata,
    client: Client,
}

impl Ina {
    pub fn new(client: Client) -> Self {
        Self {
            metadata: EngineMetadata {
                name: "ina".to_string().into(),
                display_name: "INA".to_string().into(),
                homepage: "https://www.ina.fr".to_string().into(),
                categories: smallvec![SearchCategory::Videos],
                enabled: true,
                timeout_ms: 6000,
                weight: 1.0,
            },
            client,
        }
    }
}

#[async_trait]
impl SearchEngine for Ina {
    fn metadata(&self) -> EngineMetadata {
        self.metadata.clone()
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>, MetasearchError> {
        let offset = query.page * PAGE_SIZE;
        let url = format!(
            "{}/ajax/recherche?q={}&espace=1&sort=pertinence&order=desc&offset={}&modified=size",
            BASE_URL,
            urlencoding::encode(&query.query),
            offset,
        );

        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| MetasearchError::HttpError(e.to_string()))?;

        let body = resp
            .text()
            .await
            .map_err(|e| MetasearchError::ParseError(e.to_string()))?;

        let document = Html::parse_document(&body);

        let results_sel = Selector::parse("div.contentResult")
            .map_err(|e| MetasearchError::ParseError(format!("{e:?}")))?;
        let link_sel = Selector::parse("a[href]")
            .map_err(|e| MetasearchError::ParseError(format!("{e:?}")))?;
        let title_sel = Selector::parse("div.title-bloc-small")
            .map_err(|e| MetasearchError::ParseError(format!("{e:?}")))?;
        let content_sel = Selector::parse("div.sous-titre-fonction")
            .map_err(|e| MetasearchError::ParseError(format!("{e:?}")))?;
        let thumb_sel = Selector::parse("img[data-src]")
            .map_err(|e| MetasearchError::ParseError(format!("{e:?}")))?;

        let mut results = Vec::new();

        for (i, el) in document.select(&results_sel).enumerate() {
            let href = el
                .select(&link_sel)
                .next()
                .and_then(|a| a.value().attr("href"))
                .unwrap_or_default();

            let result_url = if href.starts_with('/') {
                format!("{}{}", BASE_URL, href)
            } else {
                href.to_string()
            };

            let title = el
                .select(&title_sel)
                .next()
                .map(|e| e.text().collect::<String>())
                .unwrap_or_default()
                .trim()
                .to_string();

            let content = el
                .select(&content_sel)
                .next()
                .map(|e| e.text().collect::<String>())
                .unwrap_or_default()
                .trim()
                .to_string();

            let thumbnail = el
                .select(&thumb_sel)
                .next()
                .and_then(|img| img.value().attr("data-src"))
                .map(|s| s.to_string());

            if title.is_empty() && result_url.is_empty() {
                continue;
            }

            let mut result = SearchResult::new(title, result_url, content, "ina".to_string());
            result.engine_rank = (i + 1) as u32;
            result.category = SearchCategory::Videos.to_string();
            result.thumbnail = thumbnail;
            results.push(result);
        }

        Ok(results)
    }
}
