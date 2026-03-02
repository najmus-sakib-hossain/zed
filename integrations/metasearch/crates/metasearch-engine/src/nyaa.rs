//! Nyaa — anime torrent tracker search.
//!
//! Scrapes HTML results from nyaa.si.

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

pub struct Nyaa {
    metadata: EngineMetadata,
    client: Client,
}

impl Nyaa {
    pub fn new(client: Client) -> Self {
        Self {
            metadata: EngineMetadata {
                name: "nyaa".to_string().into(),
                display_name: "Nyaa".to_string().into(),
                homepage: "https://nyaa.si".to_string().into(),
                categories: smallvec![SearchCategory::Files],
                enabled: true,
                timeout_ms: 5000,
                weight: 1.0,
            },
            client,
        }
    }
}

#[async_trait]
impl SearchEngine for Nyaa {
    fn metadata(&self) -> EngineMetadata {
        self.metadata.clone()
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>> {
        let page = query.page;
        let url = format!(
            "https://nyaa.si/?q={}&p={}",
            urlencoding::encode(&query.query),
            page
        );

        let body = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| MetasearchError::HttpError(e.to_string()))?
            .text()
            .await
            .map_err(|e| MetasearchError::HttpError(e.to_string()))?;

        let document = Html::parse_document(&body);
        let row_sel = Selector::parse("table.torrent-list tr")
            .map_err(|e| MetasearchError::ParseError(format!("{e:?}")))?;
        let th_sel =
            Selector::parse("th").map_err(|e| MetasearchError::ParseError(format!("{e:?}")))?;
        let title_sel = Selector::parse("td:nth-child(2) a:last-child")
            .map_err(|e| MetasearchError::ParseError(format!("{e:?}")))?;
        let size_sel = Selector::parse("td:nth-child(4)")
            .map_err(|e| MetasearchError::ParseError(format!("{e:?}")))?;
        let seeds_sel = Selector::parse("td:nth-child(6)")
            .map_err(|e| MetasearchError::ParseError(format!("{e:?}")))?;
        let leeches_sel = Selector::parse("td:nth-child(7)")
            .map_err(|e| MetasearchError::ParseError(format!("{e:?}")))?;

        let mut results = Vec::new();
        let mut rank = 0u32;

        for row in document.select(&row_sel) {
            // Skip header rows
            if row.select(&th_sel).next().is_some() {
                continue;
            }

            let title_el = match row.select(&title_sel).next() {
                Some(el) => el,
                None => continue,
            };

            rank += 1;
            let title = title_el.text().collect::<String>().trim().to_string();
            let href = title_el.value().attr("href").unwrap_or("");
            let page_url = format!("https://nyaa.si{}", href);

            let size = row
                .select(&size_sel)
                .next()
                .map(|e| e.text().collect::<String>().trim().to_string())
                .unwrap_or_default();
            let seeds = row
                .select(&seeds_sel)
                .next()
                .map(|e| e.text().collect::<String>().trim().to_string())
                .unwrap_or_default();
            let leeches = row
                .select(&leeches_sel)
                .next()
                .map(|e| e.text().collect::<String>().trim().to_string())
                .unwrap_or_default();

            let snippet = format!("Size: {} | Seeds: {} | Leeches: {}", size, seeds, leeches);

            let mut r = SearchResult::new(title, page_url, snippet, self.metadata.name.clone());
            r.engine_rank = rank;
            results.push(r);
        }

        Ok(results)
    }
}
