//! Arxiv search engine implementation.
//!
//! Translated from SearXNG's `arxiv.py` (129 lines, XML-RSS API).
//! arXiv is a free open-access archive for scholarly articles in physics,
//! mathematics, computer science, and more.
//! Website: https://arxiv.org
//! Features: Paging
//! API: https://info.arxiv.org/help/api/user-manual.html

use async_trait::async_trait;
use metasearch_core::{
    category::SearchCategory,
    engine::{EngineMetadata, SearchEngine},
    error::{MetasearchError, Result},
    query::SearchQuery,
    result::SearchResult,
};
use reqwest::Client;
use tracing::info;
use smallvec::smallvec;

pub struct Arxiv {
    metadata: EngineMetadata,
    client: Client,
}

impl Arxiv {
    pub fn new(client: Client) -> Self {
        Self {
            metadata: EngineMetadata {
                name: "arxiv".to_string().into(),
                display_name: "arXiv".to_string().into(),
                homepage: "https://arxiv.org".to_string().into(),
                categories: smallvec![SearchCategory::Science],
                enabled: true,
                timeout_ms: 5000,
                weight: 1.1,
            },
            client,
        }
    }

    /// Parse a simple XML text value between tags.
    fn extract_tag_text<'a>(xml: &'a str, tag: &str) -> Option<&'a str> {
        let open = format!("<{}", tag);
        let close = format!("</{}>", tag);
        if let Some(start_pos) = xml.find(&open) {
            // Find the end of the opening tag (after possible attributes)
            if let Some(gt_pos) = xml[start_pos..].find('>') {
                let content_start = start_pos + gt_pos + 1;
                if let Some(end_pos) = xml[content_start..].find(&close) {
                    return Some(&xml[content_start..content_start + end_pos]);
                }
            }
        }
        None
    }
}

#[async_trait]
impl SearchEngine for Arxiv {
    fn metadata(&self) -> EngineMetadata {
        self.metadata.clone()
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>> {
        let max_results = 10;
        let start = ((query.page.max(1) - 1) * max_results) as u64;
        let encoded = urlencoding::encode(&query.query);

        let url = format!(
            "https://export.arxiv.org/api/query?search_query=all:{}&start={}&max_results={}",
            encoded, start, max_results
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
            .map_err(|e| MetasearchError::ParseError(e.to_string()))?;

        let mut results = Vec::new();

        // Split by <entry> tags — arXiv returns Atom XML
        for (i, entry_chunk) in body.split("<entry>").skip(1).enumerate() {
            let entry = match entry_chunk.split("</entry>").next() {
                Some(e) => e,
                None => continue,
            };

            let title = Self::extract_tag_text(entry, "title")
                .unwrap_or("")
                .replace('\n', " ")
                .trim()
                .to_string();

            // Extract the id (URL) — <id>http://arxiv.org/abs/...</id>
            let entry_url = Self::extract_tag_text(entry, "id")
                .unwrap_or("")
                .trim()
                .to_string();

            let summary = Self::extract_tag_text(entry, "summary")
                .unwrap_or("")
                .replace('\n', " ")
                .trim()
                .to_string();

            // Extract authors: <author><name>...</name></author>
            let authors: Vec<String> = entry
                .split("<author>")
                .skip(1)
                .filter_map(|a| Self::extract_tag_text(a, "name").map(|n| n.trim().to_string()))
                .collect();

            let author_str = if authors.is_empty() {
                String::new()
            } else {
                authors.join(", ")
            };

            let snippet = if author_str.is_empty() {
                summary.chars().take(300).collect()
            } else {
                format!(
                    "[{}] {}",
                    author_str,
                    summary.chars().take(250).collect::<String>()
                )
            };

            if !title.is_empty() && !entry_url.is_empty() {
                let mut r = SearchResult::new(&title, &entry_url, &snippet, "arxiv");
                r.engine_rank = i as u32;
                r.category = SearchCategory::Science.to_string();

                // Try to extract published date
                if let Some(date_str) = Self::extract_tag_text(entry, "published") {
                    if let Ok(dt) =
                        chrono::NaiveDateTime::parse_from_str(date_str.trim(), "%Y-%m-%dT%H:%M:%SZ")
                    {
                        r.published_date =
                            Some(chrono::DateTime::from_naive_utc_and_offset(dt, chrono::Utc));
                    }
                }

                results.push(r);
            }
        }

        info!(engine = "arxiv", count = results.len(), "Search complete");
        Ok(results)
    }
}
