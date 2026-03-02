//! SVG Repo — SVG icon search engine.
//!
//! Scrapes HTML results from svgrepo.com.

use async_trait::async_trait;
use metasearch_core::{
    category::SearchCategory,
    engine::{EngineMetadata, SearchEngine},
    error::Result,
    query::SearchQuery,
    result::SearchResult,
};
use reqwest::Client;
use scraper::{Html, Selector};
use smallvec::smallvec;

pub struct SvgRepo {
    metadata: EngineMetadata,
    client: Client,
}

impl SvgRepo {
    pub fn new(client: Client) -> Self {
        Self {
            metadata: EngineMetadata {
                name: "svgrepo".to_string().into(),
                display_name: "SVG Repo".to_string().into(),
                homepage: "https://www.svgrepo.com".to_string().into(),
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
impl SearchEngine for SvgRepo {
    fn metadata(&self) -> EngineMetadata {
        self.metadata.clone()
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>> {
        let page = query.page;
        let url = format!(
            "https://www.svgrepo.com/vectors/{}/{}/",
            urlencoding::encode(&query.query),
            page
        );

        let resp = match self
            .client
            .get(&url)
            .timeout(std::time::Duration::from_secs(6))
            .send()
            .await
        {
            Ok(r) => r,
            Err(_) => return Ok(Vec::new()),
        };

        if !resp.status().is_success() {
            return Ok(Vec::new());
        }

        let body = match resp.text().await {
            Ok(t) => t,
            Err(_) => return Ok(Vec::new()),
        };

        let document = Html::parse_document(&body);
        // CSS module class names change with each deploy; try multiple selectors
        let item_sel = Selector::parse("[class*='nodeListing'] > div, [class*='node-listing'] > div, [class*='NodeListing'] > div")
            .unwrap_or_else(|_| Selector::parse("div").expect("div"));
        let link_sel = Selector::parse("a").expect("a");
        let img_sel = Selector::parse("img").expect("img");

        let mut results = Vec::new();

        for (i, item) in document.select(&item_sel).enumerate() {
            let link_el = match item.select(&link_sel).next() {
                Some(el) => el,
                None => continue,
            };

            let href = link_el.value().attr("href").unwrap_or("");
            let title = link_el
                .value()
                .attr("title")
                .unwrap_or("")
                .replace(" SVG File", "")
                .replace("Show ", "");
            let img_src = item
                .select(&img_sel)
                .next()
                .and_then(|e| e.value().attr("src"))
                .unwrap_or("")
                .to_string();

            let page_url = format!("https://www.svgrepo.com{}", href);
            let mut r =
                SearchResult::new(title, page_url, String::new(), self.metadata.name.clone());
            r.engine_rank = (i + 1) as u32;
            r.thumbnail = Some(img_src);
            results.push(r);
        }

        Ok(results)
    }
}
