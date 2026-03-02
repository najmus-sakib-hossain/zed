//! Alpine Linux packages search engine implementation.
//!
//! Translated from SearXNG's `alpinelinux.py` (83 lines, HTML scraping).
//! Search Alpine Linux binary packages.
//! Website: https://www.alpinelinux.org
//! Features: Paging

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
use tracing::info;
use smallvec::smallvec;

pub struct AlpineLinux {
    metadata: EngineMetadata,
    client: Client,
}

impl AlpineLinux {
    pub fn new(client: Client) -> Self {
        Self {
            metadata: EngineMetadata {
                name: "alpinelinux".to_string().into(),
                display_name: "Alpine Linux Packages".to_string().into(),
                homepage: "https://pkgs.alpinelinux.org".to_string().into(),
                categories: smallvec![SearchCategory::IT],
                enabled: true,
                timeout_ms: 3000,
                weight: 0.7,
            },
            client,
        }
    }
}

#[async_trait]
impl SearchEngine for AlpineLinux {
    fn metadata(&self) -> EngineMetadata {
        self.metadata.clone()
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>> {
        let page = query.page.max(1);
        let encoded = urlencoding::encode(&query.query);

        // Use wildcards to match more than exact package names
        let url = format!(
            "https://pkgs.alpinelinux.org/packages?name=*{}*&page={}&arch=x86_64",
            encoded, page
        );

        let resp = match self
            .client
            .get(&url)
            .timeout(std::time::Duration::from_secs(7))
            .header("User-Agent", "Mozilla/5.0")
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
            Ok(b) => b,
            Err(_) => return Ok(Vec::new()),
        };

        let document = Html::parse_document(&body);

        let row_selector = Selector::parse("table tbody tr").unwrap();
        let cell_selector = Selector::parse("td").unwrap();
        let link_selector = Selector::parse("td.package a").unwrap();

        let mut results = Vec::new();

        for (i, row) in document.select(&row_selector).enumerate() {
            let cells: Vec<_> = row.select(&cell_selector).collect();
            if cells.len() < 9 {
                continue;
            }

            let link_el = match row.select(&link_selector).next() {
                Some(el) => el,
                None => continue,
            };

            let package_name: String = link_el.text().collect::<String>().trim().to_string();
            let href = link_el.value().attr("href").unwrap_or("");
            let result_url = format!("https://pkgs.alpinelinux.org{}", href);

            let version: String = cells
                .get(2)
                .map(|c| c.text().collect::<String>().trim().to_string())
                .unwrap_or_default();

            let repo: String = cells
                .get(4)
                .map(|c| c.text().collect::<String>().trim().to_string())
                .unwrap_or_default();

            let snippet = format!(
                "Alpine Linux package {} v{} [{}]",
                package_name, version, repo
            );

            let mut r = SearchResult::new(&package_name, &result_url, &snippet, "alpinelinux");
            r.engine_rank = i as u32;
            r.category = SearchCategory::IT.to_string();

            results.push(r);
        }

        info!(
            engine = "alpinelinux",
            count = results.len(),
            "Search complete"
        );
        Ok(results)
    }
}
