//! Tokyo Toshokan — BitTorrent library for Japanese media
//!
//! Tokyo Toshokan is a torrent listing site focused on Japanese media.
//! Results are scraped from HTML since there is no official API.
//!
//! Reference: <https://www.tokyotosho.info/>

use async_trait::async_trait;
use reqwest::Client;
use scraper::{Html, Selector};
use smallvec::smallvec;

use metasearch_core::{
    category::SearchCategory,
    engine::{EngineMetadata, SearchEngine},
    error::Result,
    query::SearchQuery,
    result::SearchResult,
};

pub struct TokyoToshokan {
    client: Client,
}

impl TokyoToshokan {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl SearchEngine for TokyoToshokan {
    fn metadata(&self) -> EngineMetadata {
        EngineMetadata {
            name: "Tokyo Toshokan".to_string().into(),
            display_name: "Tokyo Toshokan".to_string().into(),
            homepage: "https://Tokyo Toshokan.com".to_string().into(),
            categories: smallvec![SearchCategory::Files],
            enabled: true,
            timeout_ms: 5000,
            weight: 1.0,
        }
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>> {
        let url = format!(
            "https://www.tokyotosho.info/search.php?terms={}&page={}",
            urlencoding::encode(&query.query),
            query.page
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

        let text = match resp.text().await {
            Ok(t) => t,
            Err(_) => return Ok(Vec::new()),
        };
        let document = Html::parse_document(&text);

        let row_sel = Selector::parse("table.listing tr.category_0").unwrap();
        let link_sel = Selector::parse("td.desc-top a").unwrap();
        let info_sel = Selector::parse("td.desc-bot").unwrap();

        let rows: Vec<_> = document.select(&row_sel).collect();
        let mut results = Vec::new();
        let mut rank: u32 = 1;

        // Results come in pairs of rows: name row + info row
        let mut i = 0;
        while i + 1 < rows.len() {
            let name_row = rows[i];
            let info_row = rows[i + 1];
            i += 2;

            // Get the last link in desc-top (the title/URL link)
            let links: Vec<_> = name_row.select(&link_sel).collect();
            let title_link = match links.last() {
                Some(l) => l,
                None => continue,
            };

            let title = title_link.text().collect::<String>().trim().to_string();
            let href = match title_link.value().attr("href") {
                Some(h) => {
                    if h.starts_with("http") {
                        h.to_string()
                    } else {
                        format!("https://www.tokyotosho.info{}", h)
                    }
                }
                None => continue,
            };

            // Extract info from desc-bot
            let content = info_row
                .select(&info_sel)
                .next()
                .map(|el| el.text().collect::<String>().trim().to_string())
                .unwrap_or_default();

            // Truncate overly long content
            let content = if content.len() > 300 {
                format!("{}…", &content[..300])
            } else {
                content
            };

            let mut result = SearchResult::new(
                title,
                href,
                content,
                "tokyotoshokan",
            );
            result.engine_rank = rank;
            results.push(result);
            rank += 1;
        }

        Ok(results)
    }
}

