//! DeStatis — German Federal Statistical Office search engine.
//!
//! Scrapes destatis.de for statistical publications and data.

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

pub struct Destatis {
    metadata: EngineMetadata,
    client: Client,
}

impl Destatis {
    pub fn new(client: Client) -> Self {
        Self {
            metadata: EngineMetadata {
                name: "destatis".to_string().into(),
                display_name: "DeStatis".to_string().into(),
                homepage: "https://www.destatis.de".to_string().into(),
                categories: smallvec![SearchCategory::General],
                enabled: true,
                timeout_ms: 5000,
                weight: 0.5,
            },
            client,
        }
    }
}

#[async_trait]
impl SearchEngine for Destatis {
    fn metadata(&self) -> EngineMetadata {
        self.metadata.clone()
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>, MetasearchError> {
        let page = query.page;
        let url = format!(
            "https://www.destatis.de/SiteGlobals/Forms/Suche/Expertensuche_Formular.html?templateQueryString={}&gtp=474_list%3D{}",
            urlencoding::encode(&query.query),
            page
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
            .map_err(|e| MetasearchError::HttpError(e.to_string()))?;

        let document = Html::parse_document(&body);
        let result_sel = Selector::parse("div.c-result").unwrap();
        let link_sel = Selector::parse("a").unwrap();
        let content_sel = Selector::parse("div.column p").unwrap();
        let doctype_sel = Selector::parse("div.c-result__doctype p").unwrap();
        let date_sel = Selector::parse("a span.c-result__date").unwrap();

        let mut results = Vec::new();

        for element in document.select(&result_sel) {
            // Skip recommended results on pages > 1 (they repeat)
            if page > 1 {
                let classes = element.value().attr("class").unwrap_or("");
                if classes.contains("c-result--recommended") {
                    continue;
                }
            }

            let link_el = match element.select(&link_sel).next() {
                Some(el) => el,
                None => continue,
            };

            let title = link_el.text().collect::<String>().trim().to_string();
            let href = link_el
                .value()
                .attr("href")
                .map(|h| {
                    if h.starts_with("http") {
                        h.to_string()
                    } else {
                        format!("https://www.destatis.de/{}", h.trim_start_matches('/'))
                    }
                })
                .unwrap_or_default();

            let content = element
                .select(&content_sel)
                .next()
                .map(|el| el.text().collect::<String>().trim().to_string())
                .unwrap_or_default();

            let doctype = element
                .select(&doctype_sel)
                .next()
                .map(|el| el.text().collect::<String>().trim().to_string())
                .unwrap_or_default();

            let date = element
                .select(&date_sel)
                .next()
                .map(|el| el.text().collect::<String>().trim().to_string())
                .unwrap_or_default();

            let mut meta_parts = Vec::new();
            if !doctype.is_empty() {
                meta_parts.push(doctype);
            }
            if !date.is_empty() {
                meta_parts.push(date);
            }

            let snippet = if meta_parts.is_empty() {
                content
            } else {
                format!("{} — {}", meta_parts.join(", "), content)
            };

            if !title.is_empty() && !href.is_empty() {
                let mut result = SearchResult::new(&title, &href, &snippet, "destatis");
                result.category = SearchCategory::General.to_string();
                results.push(result);
            }
        }

        Ok(results)
    }
}
