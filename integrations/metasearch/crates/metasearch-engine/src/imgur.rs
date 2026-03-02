//! Imgur — image search on Imgur.
//!
//! Scrapes HTML search results from imgur.com.
//!
//! Reference: <https://imgur.com>

use async_trait::async_trait;
use metasearch_core::{
    category::SearchCategory,
    engine::{EngineMetadata, SearchEngine},
    error::{MetasearchError, Result},
    query::SearchQuery,
    result::SearchResult,
};
use reqwest::Client;
use scraper::{Html, Selector};
use smallvec::smallvec;

pub struct Imgur {
    metadata: EngineMetadata,
    client: Client,
}

impl Imgur {
    pub fn new(client: Client) -> Self {
        Self {
            metadata: EngineMetadata {
                name: "imgur".to_string().into(),
                display_name: "Imgur".to_string().into(),
                homepage: "https://imgur.com".to_string().into(),
                categories: smallvec![SearchCategory::Images],
                enabled: true,
                timeout_ms: 8000,
                weight: 0.7,
            },
            client,
        }
    }
}

#[async_trait]
impl SearchEngine for Imgur {
    fn metadata(&self) -> EngineMetadata {
        self.metadata.clone()
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>> {
        let page_index = query.page.saturating_sub(1);

        let url = format!(
            "https://imgur.com/search/score/all?q={}&qs=thumbs&p={}",
            urlencoding::encode(&query.query),
            page_index,
        );

        let resp = self
            .client
            .get(&url)
            .header("User-Agent", "Mozilla/5.0")
            .send()
            .await
            .map_err(|e| MetasearchError::HttpError(e.to_string()))?;

        let body = resp
            .text()
            .await
            .map_err(|e| MetasearchError::HttpError(e.to_string()))?;

        let document = Html::parse_document(&body);
        let card_sel = Selector::parse("div.cards div.post, div.post")
            .map_err(|e| MetasearchError::ParseError(format!("{e:?}")))?;
        let link_sel =
            Selector::parse("a").map_err(|e| MetasearchError::ParseError(format!("{e:?}")))?;
        let img_sel =
            Selector::parse("a img").map_err(|e| MetasearchError::ParseError(format!("{e:?}")))?;

        let mut results = Vec::new();

        for (rank, card) in document.select(&card_sel).enumerate() {
            let link = card
                .select(&link_sel)
                .next()
                .and_then(|a| a.value().attr("href"));
            let img = card.select(&img_sel).next();

            let (thumbnail_src, title) = match img {
                Some(el) => (
                    el.value().attr("src").unwrap_or_default().to_string(),
                    el.value().attr("alt").unwrap_or_default().to_string(),
                ),
                None => continue,
            };

            if thumbnail_src.len() < 25 {
                continue;
            }

            let img_src = thumbnail_src.replace("b.", ".");
            let page_url = match link {
                Some(href) => format!("https://imgur.com{}", href),
                None => continue,
            };

            let mut result =
                SearchResult::new(title, page_url, String::new(), self.metadata.name.clone());
            result.engine_rank = (rank + 1) as u32;
            result.thumbnail = Some(img_src);
            results.push(result);
        }

        Ok(results)
    }
}
