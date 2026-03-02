//! Presearch search engine implementation.
//! Two-step process: get searchId from HTML, then fetch JSON results.
//! URL: https://presearch.com/search
//! Features: Paging

use async_trait::async_trait;
use metasearch_core::{
    category::SearchCategory,
    engine::{EngineMetadata, SearchEngine},
    error::Result,
    query::SearchQuery,
    result::SearchResult,
};
use regex::Regex;
use reqwest::Client;
use tracing::info;
use smallvec::smallvec;

pub struct Presearch {
    metadata: EngineMetadata,
    client: Client,
}

impl Presearch {
    pub fn new(client: Client) -> Self {
        Self {
            metadata: EngineMetadata {
                name: "presearch".to_string().into(),
                display_name: "Presearch".to_string().into(),
                homepage: "https://presearch.io".to_string().into(),
                categories: smallvec![SearchCategory::General],
                enabled: true,
                timeout_ms: 8000,
                weight: 1.0,
            },
            client,
        }
    }
}

#[async_trait]
impl SearchEngine for Presearch {
    fn metadata(&self) -> EngineMetadata {
        self.metadata.clone()
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>> {
        let page = query.page.max(1);

        // Step 1: GET search page to extract searchId
        let search_url = format!(
            "https://presearch.com/search?q={}&page={}",
            urlencoding::encode(&query.query),
            page,
        );

        let resp = match self
            .client
            .get(&search_url)
            .timeout(std::time::Duration::from_secs(5))
            .header(
                "User-Agent",
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:120.0) Gecko/20100101 Firefox/120.0",
            )
            .header("Accept", "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8")
            .header("Cookie", "b=1; presearch_session=; use_local_search_results=false; use_safe_search=true")
            .send()
            .await
        {
            Ok(r) => r,
            Err(_) => return Ok(Vec::new()),
        };

        // Capture response cookies for step 2
        let cookies: Vec<String> = resp
            .headers()
            .get_all(reqwest::header::SET_COOKIE)
            .iter()
            .filter_map(|c| c.to_str().ok().map(|s| s.to_owned()))
            .collect();
        let cookie_str = cookies.join("; ");

        let html_text = match resp.text().await {
            Ok(t) => t,
            Err(_) => return Ok(Vec::new()),
        };

        // Extract searchId from HTML
        let id_re = match Regex::new(r#"window\.searchId\s*=\s*"([^"]+)""#) {
            Ok(r) => r,
            Err(_) => return Ok(Vec::new()),
        };

        let search_id = match id_re.captures(&html_text) {
            Some(caps) => match caps.get(1) {
                Some(m) => m.as_str().to_string(),
                None => return Ok(Vec::new()),
            },
            None => return Ok(Vec::new()),
        };

        // Step 2: Fetch JSON results using searchId
        let results_url = format!(
            "https://presearch.com/results?id={}",
            urlencoding::encode(&search_id),
        );

        let resp = match self
            .client
            .get(&results_url)
            .timeout(std::time::Duration::from_secs(4))
            .header(
                "User-Agent",
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:120.0) Gecko/20100101 Firefox/120.0",
            )
            .header("Accept", "application/json")
            .header("Cookie", &cookie_str)
            .send()
            .await
        {
            Ok(r) => r,
            Err(_) => return Ok(Vec::new()),
        };

        let text = match resp.text().await {
            Ok(t) => t,
            Err(_) => return Ok(Vec::new()),
        };

        // Handle non-JSON responses
        if text.trim().is_empty() || text.starts_with("<!") || text.starts_with("<html") {
            return Ok(Vec::new());
        }

        let data: serde_json::Value = match serde_json::from_str(&text) {
            Ok(v) => v,
            Err(_) => return Ok(Vec::new()),
        };

        let mut results = Vec::new();

        // Try results.standardResults first, then top-level standardResults
        let items = data
            .get("results")
            .and_then(|r| r.get("standardResults"))
            .and_then(|s| s.as_array())
            .or_else(|| data.get("standardResults").and_then(|s| s.as_array()));

        let items = match items {
            Some(items) => items,
            None => return Ok(results),
        };

        for (i, item) in items.iter().enumerate() {
            let link = item["link"].as_str().unwrap_or_default();
            let title = item["title"].as_str().unwrap_or_default();
            let description = item["description"].as_str().unwrap_or_default();

            if title.is_empty() || link.is_empty() {
                continue;
            }

            let clean_title = html_escape::decode_html_entities(title).to_string();
            let clean_desc = html_escape::decode_html_entities(description).to_string();

            let mut r = SearchResult::new(&clean_title, link, &clean_desc, "presearch");
            r.engine_rank = (i + 1) as u32;
            r.category = SearchCategory::General.to_string();
            results.push(r);
        }

        info!(
            engine = "presearch",
            count = results.len(),
            "Search complete"
        );
        Ok(results)
    }
}
