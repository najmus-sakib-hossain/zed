//! Google News search — RSS feed parsing.
//! Uses the public Google News RSS feed which returns up to 100 items.

use async_trait::async_trait;
use base64::Engine as _;
use base64::engine::general_purpose;
use reqwest::Client;
use smallvec::smallvec;

use metasearch_core::{
    category::SearchCategory,
    engine::{EngineMetadata, SearchEngine},
    error::Result,
    query::SearchQuery,
    result::SearchResult,
};

pub struct GoogleNews {
    client: Client,
}

impl GoogleNews {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

/// Decode a Google News internal link to the actual article URL.
/// Google News encodes article URLs in base64 within the href path.
fn decode_google_news_url(href: &str) -> Option<String> {
    let path = href.split('?').next()?;
    let encoded = path.rsplit('/').next()?;
    if encoded.is_empty() {
        return None;
    }
    // Add proper base64 padding
    let remainder = encoded.len() % 4;
    let padded = if remainder > 0 {
        format!("{}{}", encoded, "=".repeat(4 - remainder))
    } else {
        encoded.to_string()
    };
    let bytes = general_purpose::URL_SAFE.decode(padded.as_bytes()).ok()?;
    // Find "http" in decoded bytes
    let http_pos = bytes.windows(4).position(|w| w == b"http")?;
    let url_bytes = &bytes[http_pos..];
    // URL ends at 0xd2 byte or end of data
    let end = url_bytes
        .iter()
        .position(|&b| b == 0xd2)
        .unwrap_or(url_bytes.len());
    String::from_utf8(url_bytes[..end].to_vec()).ok()
}

/// Extract text content of an XML element from raw XML by tag name.
fn extract_xml_text<'a>(item_xml: &'a str, tag: &str) -> Option<&'a str> {
    let open = format!("<{}>", tag);
    let close = format!("</{}>", tag);
    let start = item_xml.find(&open)? + open.len();
    let end = item_xml[start..].find(&close)?;
    Some(&item_xml[start..start + end])
}

/// Extract attribute value from an XML element.
fn extract_xml_attr<'a>(item_xml: &'a str, tag: &str, attr: &str) -> Option<&'a str> {
    let tag_start = item_xml.find(&format!("<{}", tag))? ;
    let tag_end = item_xml[tag_start..].find('>')? + tag_start;
    let tag_content = &item_xml[tag_start..tag_end];
    let attr_marker = format!("{}=\"", attr);
    let val_start = tag_content.find(&attr_marker)? + attr_marker.len();
    let remaining = &tag_content[val_start..];
    let val_end = remaining.find('"')?;
    Some(&remaining[..val_end])
}

#[async_trait]
impl SearchEngine for GoogleNews {
    fn metadata(&self) -> EngineMetadata {
        EngineMetadata {
            name: "Google News".to_string().into(),
            display_name: "Google News".to_string().into(),
            homepage: "https://news.google.com".to_string().into(),
            categories: smallvec![SearchCategory::News],
            enabled: true,
            timeout_ms: 8000,
            weight: 1.0,
        }
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>> {
        let url = format!(
            "https://news.google.com/rss/search?q={}&hl=en-US&gl=US&ceid=US:en",
            urlencoding::encode(&query.query),
        );

        let resp = match self
            .client
            .get(&url)
            .timeout(std::time::Duration::from_secs(7))
            .header("User-Agent", "Mozilla/5.0 Firefox/120.0")
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

        // Split into <item> blocks and parse each
        let mut results = Vec::new();
        let page = query.page.max(1) as usize;
        let per_page = 20usize;
        let skip = (page - 1) * per_page;

        let mut item_idx = 0usize;
        let mut search_pos = 0usize;

        while let Some(start) = text[search_pos..].find("<item>") {
            let abs_start = search_pos + start;
            let rest_after_open = abs_start + "<item>".len();
            if let Some(end_rel) = text[rest_after_open..].find("</item>") {
                let abs_end = rest_after_open + end_rel;
                let item_xml = &text[abs_start..abs_end + "</item>".len()];
                search_pos = abs_end + "</item>".len();

                item_idx += 1;
                if item_idx <= skip {
                    continue;
                }
                if results.len() >= per_page {
                    break;
                }

                let raw_title = extract_xml_text(item_xml, "title").unwrap_or_default();
                // Strip CDATA if present
                let title = if raw_title.starts_with("<![CDATA[") {
                    raw_title.trim_start_matches("<![CDATA[").trim_end_matches("]]>")
                } else {
                    raw_title
                };
                // Remove source suffix " - Source Name" if present
                let title = if let Some(sep) = title.rfind(" - ") {
                    &title[..sep]
                } else {
                    title
                };
                let title = title.trim();
                if title.is_empty() {
                    continue;
                }

                let link = extract_xml_text(item_xml, "link").unwrap_or_default().trim();
                if link.is_empty() {
                    continue;
                }

                // Try to decode the base64 article URL; fall back to the Google News link
                let article_url = decode_google_news_url(link)
                    .unwrap_or_else(|| link.to_string());

                let pub_date = extract_xml_text(item_xml, "pubDate")
                    .unwrap_or_default()
                    .trim()
                    .to_string();

                let source_name = extract_xml_text(item_xml, "source")
                    .unwrap_or_default()
                    .trim()
                    .to_string();

                let source_url = extract_xml_attr(item_xml, "source", "url")
                    .unwrap_or_default()
                    .to_string();

                let content = if !source_name.is_empty() && !pub_date.is_empty() {
                    format!("{} — {}", source_name, pub_date)
                } else if !source_name.is_empty() {
                    source_name.clone()
                } else {
                    pub_date.clone()
                };

                let mut r = SearchResult::new(title, &article_url, &content, "Google News");
                r.engine_rank = item_idx as u32;
                r.category = SearchCategory::News.to_string();
                if !source_url.is_empty() {
                    r.metadata = serde_json::json!({ "source": source_name, "source_url": source_url });
                }
                results.push(r);
            } else {
                break;
            }
        }

        Ok(results)
    }
}
