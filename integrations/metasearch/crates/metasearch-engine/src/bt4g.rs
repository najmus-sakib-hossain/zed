//! BT4G torrent search engine.
//!
//! Searches bt4gprx.com via its RSS/XML API for torrent metadata.

use async_trait::async_trait;
use metasearch_core::{
    category::SearchCategory,
    engine::{EngineMetadata, SearchEngine},
    error::MetasearchError,
    query::SearchQuery,
    result::SearchResult,
};
use reqwest::Client;
use smallvec::smallvec;

pub struct Bt4g {
    metadata: EngineMetadata,
    client: Client,
}

impl Bt4g {
    pub fn new(client: Client) -> Self {
        Self {
            metadata: EngineMetadata {
                name: "bt4g".to_string().into(),
                display_name: "BT4G".to_string().into(),
                homepage: "https://bt4gprx.com".to_string().into(),
                categories: smallvec![SearchCategory::Files],
                enabled: true,
                timeout_ms: 5000,
                weight: 0.6,
            },
            client,
        }
    }
}

#[async_trait]
impl SearchEngine for Bt4g {
    fn metadata(&self) -> EngineMetadata {
        self.metadata.clone()
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>, MetasearchError> {
        let search_term = urlencoding::encode(&query.query);
        let page = query.page;
        let url = format!(
            "https://bt4gprx.com/search?q={}&orderby=relevance&category=all&p={}&page=rss",
            search_term, page
        );

        let resp = match self
            .client
            .get(&url)
            .timeout(std::time::Duration::from_secs(6))
            .header("User-Agent", "Mozilla/5.0 (compatible; metasearch-bot/1.0)")
            .header("Accept", "application/rss+xml, application/xml, text/xml")
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

        let mut results = Vec::new();

        // Simple XML parsing for RSS items
        let mut pos = 0;
        while let Some(item_start) = body[pos..].find("<item>") {
            let item_start = pos + item_start;
            let item_end = match body[item_start..].find("</item>") {
                Some(e) => item_start + e + 7,
                None => break,
            };
            let item = &body[item_start..item_end];

            let title = extract_tag(item, "title").unwrap_or_default();
            let link = extract_tag(item, "guid").unwrap_or_default();
            let description = extract_tag(item, "description").unwrap_or_default();

            // Description contains file info separated by &lt;br&gt;
            let snippet = html_escape::decode_html_entities(&description)
                .to_string()
                .replace("<br>", " | ");

            if !title.is_empty() && !link.is_empty() {
                let mut result = SearchResult::new(&title, &link, &snippet, "bt4g");
                result.category = SearchCategory::Files.to_string();
                results.push(result);
            }

            pos = item_end;
        }

        Ok(results)
    }
}

fn extract_tag(xml: &str, tag: &str) -> Option<String> {
    // Match opening tag with optional attributes: <tag> or <tag ...>
    let simple = format!("<{}>", tag);
    let with_attr = format!("<{} ", tag);
    let close = format!("</{}>", tag);

    // Find the start of the opening tag
    let tag_start = xml.find(&simple).or_else(|| xml.find(&with_attr))?;
    // Find the end of the opening tag (skip past '>')
    let content_start = xml[tag_start..].find('>')? + tag_start + 1;
    let end = xml[content_start..].find(&close)? + content_start;
    let content = &xml[content_start..end];
    // Handle CDATA
    if content.starts_with("<![CDATA[") && content.ends_with("]]>") {
        Some(content[9..content.len() - 3].to_string())
    } else {
        Some(content.to_string())
    }
}
