//! Bing Images engine — search images via Bing Images HTML scraping.
//! Translated from SearXNG `searx/engines/bing_images.py`.

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

pub struct BingImages {
    metadata: EngineMetadata,
    client: Client,
}

impl BingImages {
    pub fn new(client: Client) -> Self {
        Self {
            metadata: EngineMetadata {
                name: "bing_images".to_string().into(),
                display_name: "Bing Images".to_string().into(),
                homepage: "https://www.bing.com/images".to_string().into(),
                categories: smallvec![SearchCategory::Images],
                enabled: true,
                timeout_ms: 5000,
                weight: 1.0,
            },
            client,
        }
    }
}

#[async_trait]
impl SearchEngine for BingImages {
    fn metadata(&self) -> EngineMetadata {
        self.metadata.clone()
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>, MetasearchError> {
        let page = query.page;
        let first = (page - 1) * 35 + 1;

        let url = format!(
            "https://www.bing.com/images/async?q={}&async=1&first={}&count=35",
            urlencoding::encode(&query.query),
            first,
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
        let item_sel = Selector::parse("ul.dgControl_list li").unwrap();
        let meta_sel = Selector::parse("a.iusc").unwrap();
        let title_sel = Selector::parse("div.infnmpt a").unwrap();

        let mut results = Vec::new();

        for (i, item) in document.select(&item_sel).enumerate() {
            // Extract metadata JSON from the 'm' attribute of the <a class="iusc"> tag
            let meta_el = match item.select(&meta_sel).next() {
                Some(el) => el,
                None => continue,
            };

            let meta_json = match meta_el.value().attr("m") {
                Some(m) => m,
                None => continue,
            };

            let meta: serde_json::Value = match serde_json::from_str(meta_json) {
                Ok(v) => v,
                Err(_) => continue,
            };

            let page_url = meta["purl"].as_str().unwrap_or_default();
            let img_src = meta["murl"].as_str().unwrap_or_default();
            let thumbnail_src = meta["turl"].as_str().unwrap_or_default();
            let description = meta["desc"].as_str().unwrap_or("");

            let title = item
                .select(&title_sel)
                .next()
                .map(|el| el.text().collect::<String>())
                .unwrap_or_default();

            let snippet = format!("{} — Image: {}", description, img_src);

            let mut result = SearchResult::new(
                title.trim().to_string(),
                page_url.to_string(),
                snippet,
                "bing_images".to_string(),
            );
            result.engine_rank = (i + 1) as u32;
            result.category = SearchCategory::Images.to_string();
            result.thumbnail = if thumbnail_src.is_empty() {
                None
            } else {
                Some(thumbnail_src.to_string())
            };
            results.push(result);
        }

        Ok(results)
    }
}
