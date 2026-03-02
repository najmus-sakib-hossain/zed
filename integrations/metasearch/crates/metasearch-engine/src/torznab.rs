//! Torznab/Newznab torrent indexer engine.
//! Queries a self-hosted Torznab-compatible API (e.g., Jackett, Prowlarr).
//!
//! Returns XML results parsed with regex.

use async_trait::async_trait;
use metasearch_core::{
    category::SearchCategory,
    engine::{EngineMetadata, SearchEngine},
    error::MetasearchError,
    query::SearchQuery,
    result::SearchResult,
};
use regex::Regex;
use reqwest::Client;
use smallvec::smallvec;

pub struct Torznab {
    metadata: EngineMetadata,
    client: Client,
    base_url: String,
    api_key: Option<String>,
}

impl Torznab {
    pub fn new(client: Client, base_url: &str, api_key: Option<String>) -> Self {
        let base = base_url.trim_end_matches('/').to_string();
        let enabled = !base.is_empty();
        let homepage = if base.is_empty() {
            "https://github.com/Prowlarr/Prowlarr".to_string()
        } else {
            base.clone()
        };
        Self {
            metadata: EngineMetadata {
                name: "torznab".to_string().into(),
                display_name: "Torznab".to_string().into(),
                homepage: homepage.into(),
                categories: smallvec![SearchCategory::Files],
                enabled,
                timeout_ms: 8000,
                weight: 0.6,
            },
            client,
            base_url: base,
            api_key,
        }
    }
}

#[async_trait]
impl SearchEngine for Torznab {
    fn metadata(&self) -> EngineMetadata {
        self.metadata.clone()
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>, MetasearchError> {
        if self.base_url.is_empty() {
            return Ok(Vec::new());
        }

        let mut url = format!(
            "{}?t=search&q={}",
            self.base_url,
            urlencoding::encode(&query.query),
        );

        if let Some(ref key) = self.api_key {
            if !key.is_empty() {
                url.push_str(&format!("&apikey={}", urlencoding::encode(key)));
            }
        }

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

        let item_re = Regex::new(r"(?s)<item>(.*?)</item>")
            .map_err(|e| MetasearchError::ParseError(e.to_string()))?;

        let mut results = Vec::new();
        let mut rank = 1u32;

        for cap in item_re.captures_iter(&body) {
            let item_xml = &cap[1];

            let title = extract_tag(item_xml, "title").unwrap_or_default();
            if title.is_empty() {
                continue;
            }

            let link = extract_tag(item_xml, "guid")
                .or_else(|| extract_tag(item_xml, "link"))
                .unwrap_or_default();
            if link.is_empty() {
                continue;
            }

            let size = extract_tag(item_xml, "size").unwrap_or_default();
            let seeders = extract_torznab_attr(item_xml, "seeders").unwrap_or_default();
            let leechers = extract_torznab_attr(item_xml, "peers")
                .or_else(|| extract_torznab_attr(item_xml, "leechers"))
                .unwrap_or_default();

            let size_display = if !size.is_empty() {
                format_size(&size)
            } else {
                "Unknown".to_string()
            };

            let content = format!(
                "Size: {} | Seeders: {} | Leechers: {}",
                size_display,
                if seeders.is_empty() { "?" } else { &seeders },
                if leechers.is_empty() {
                    "?"
                } else {
                    &leechers
                },
            );

            let mut result = SearchResult::new(&title, &link, &content, "torznab");
            result.engine_rank = rank;
            result.category = SearchCategory::Files.to_string();
            results.push(result);

            rank += 1;
        }

        Ok(results)
    }
}

/// Extract text content from an XML tag.
fn extract_tag(xml: &str, tag: &str) -> Option<String> {
    let open = format!("<{}", tag);
    let close = format!("</{}>", tag);
    let start_idx = xml.find(&open)?;
    // Find the end of the opening tag (handle attributes)
    let after_open = start_idx + open.len();
    let tag_end = xml[after_open..].find('>')? + after_open + 1;
    let end_idx = xml[tag_end..].find(&close)? + tag_end;
    let content = &xml[tag_end..end_idx];
    // Handle CDATA
    let content = content.trim();
    if content.starts_with("<![CDATA[") && content.ends_with("]]>") {
        Some(content[9..content.len() - 3].to_string())
    } else {
        Some(html_escape::decode_html_entities(content).to_string())
    }
}

/// Extract a torznab:attr value by name.
fn extract_torznab_attr(xml: &str, name: &str) -> Option<String> {
    let pattern = format!(
        r#"torznab:attr\s+name=['"]{name}['"]\s+value=['"]([^'"]*?)['"]"#
    );
    let re = Regex::new(&pattern).ok()?;
    re.captures(xml)
        .and_then(|caps| caps.get(1).map(|m| m.as_str().to_string()))
}

/// Format a byte size string into a human-readable form.
fn format_size(bytes_str: &str) -> String {
    match bytes_str.parse::<u64>() {
        Ok(bytes) => {
            if bytes >= 1_073_741_824 {
                format!("{:.2} GB", bytes as f64 / 1_073_741_824.0)
            } else if bytes >= 1_048_576 {
                format!("{:.2} MB", bytes as f64 / 1_048_576.0)
            } else if bytes >= 1024 {
                format!("{:.2} KB", bytes as f64 / 1024.0)
            } else {
                format!("{} B", bytes)
            }
        }
        Err(_) => bytes_str.to_string(),
    }
}
