//! Duden — German dictionary search.
//!
//! Scrapes HTML results from duden.de.

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

pub struct Duden {
    metadata: EngineMetadata,
    client: Client,
}

impl Duden {
    pub fn new(client: Client) -> Self {
        Self {
            metadata: EngineMetadata {
                name: "duden".to_string().into(),
                display_name: "Duden".to_string().into(),
                homepage: "https://www.duden.de".to_string().into(),
                categories: smallvec![SearchCategory::General],
                enabled: true,
                timeout_ms: 5000,
                weight: 1.0,
            },
            client,
        }
    }
}

#[async_trait]
impl SearchEngine for Duden {
    fn metadata(&self) -> EngineMetadata {
        self.metadata.clone()
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>> {
        let page = query.page;
        let url = if page <= 1 {
            format!(
                "https://www.duden.de/suchen/dudenonline/{}",
                urlencoding::encode(&query.query)
            )
        } else {
            format!(
                "https://www.duden.de/suchen/dudenonline/{}?search_api_fulltext=&page={}",
                urlencoding::encode(&query.query),
                page - 1
            )
        };

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
        let section_sel = Selector::parse("section:not(.essay)")
            .map_err(|e| MetasearchError::ParseError(format!("{e:?}")))?;
        let link_sel =
            Selector::parse("h2 a").map_err(|e| MetasearchError::ParseError(format!("{e:?}")))?;
        let content_sel =
            Selector::parse("p").map_err(|e| MetasearchError::ParseError(format!("{e:?}")))?;

        let mut results = Vec::new();

        for (rank, section) in document.select(&section_sel).enumerate() {
            let link_el = match section.select(&link_sel).next() {
                Some(el) => el,
                None => continue,
            };

            let title = link_el.text().collect::<String>().trim().to_string();
            let href = match link_el.value().attr("href") {
                Some(h) => {
                    if h.starts_with("http") {
                        h.to_string()
                    } else {
                        format!("https://www.duden.de{h}")
                    }
                }
                None => continue,
            };

            if title.is_empty() {
                continue;
            }

            let snippet = section
                .select(&content_sel)
                .next()
                .map(|el| el.text().collect::<String>().trim().to_string())
                .unwrap_or_default();

            let mut result = SearchResult::new(title, href, snippet, self.metadata.name.clone());
            result.engine_rank = (rank + 1) as u32;
            results.push(result);
        }

        Ok(results)
    }
}
