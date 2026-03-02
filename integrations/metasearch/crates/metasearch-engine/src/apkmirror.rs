//! APKMirror engine — search Android APK releases via HTML scraping.
//! Translated from SearXNG `searx/engines/apkmirror.py`.

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

pub struct ApkMirror {
    metadata: EngineMetadata,
    client: Client,
}

impl ApkMirror {
    pub fn new(client: Client) -> Self {
        Self {
            metadata: EngineMetadata {
                name: "apkmirror".to_string().into(),
                display_name: "APKMirror".to_string().into(),
                homepage: "https://www.apkmirror.com".to_string().into(),
                categories: smallvec![SearchCategory::Files],
                enabled: true,
                timeout_ms: 5000,
                weight: 0.7,
            },
            client,
        }
    }
}

#[async_trait]
impl SearchEngine for ApkMirror {
    fn metadata(&self) -> EngineMetadata {
        self.metadata.clone()
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>, MetasearchError> {
        let page = query.page;
        let url = format!(
            "https://www.apkmirror.com/?post_type=app_release&searchtype=apk&page={}&s={}",
            page,
            urlencoding::encode(&query.query),
        );

        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| MetasearchError::HttpError(e.to_string()))?;

        let html_text = resp
            .text()
            .await
            .map_err(|e| MetasearchError::ParseError(e.to_string()))?;

        let document = Html::parse_document(&html_text);
        let row_sel = Selector::parse("#content .listWidget > div > div.appRow").unwrap();
        let link_sel = Selector::parse("h5 a").unwrap();
        let img_sel = Selector::parse("img").unwrap();

        let mut results = Vec::new();

        for (i, row) in document.select(&row_sel).enumerate() {
            let link = match row.select(&link_sel).next() {
                Some(l) => l,
                None => continue,
            };

            let href = link.value().attr("href").unwrap_or_default();
            let title = link.text().collect::<String>();

            if title.is_empty() {
                continue;
            }

            let item_url = format!("https://www.apkmirror.com{}#downloads", href);

            let thumbnail = row
                .select(&img_sel)
                .next()
                .and_then(|e| e.value().attr("src"))
                .map(|s| format!("https://www.apkmirror.com{}", s));

            let mut result = SearchResult::new(
                title.trim().to_string(),
                item_url,
                "APK release on APKMirror".to_string(),
                "apkmirror".to_string(),
            );
            result.engine_rank = (i + 1) as u32;
            result.category = SearchCategory::Files.to_string();
            result.thumbnail = thumbnail;
            results.push(result);
        }

        Ok(results)
    }
}
