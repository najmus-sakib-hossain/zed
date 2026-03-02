//! YouTube engine — search videos via YouTube web scraping (no API key needed).
//! Translated from SearXNG `searx/engines/youtube_noapi.py`.

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

pub struct YouTube {
    metadata: EngineMetadata,
    client: Client,
}

impl YouTube {
    pub fn new(client: Client) -> Self {
        Self {
            metadata: EngineMetadata {
                name: "youtube".to_string().into(),
                display_name: "YouTube".to_string().into(),
                homepage: "https://www.youtube.com".to_string().into(),
                categories: smallvec![SearchCategory::Videos],
                enabled: true,
                timeout_ms: 5000,
                weight: 1.5,
            },
            client,
        }
    }
}

#[async_trait]
impl SearchEngine for YouTube {
    fn metadata(&self) -> EngineMetadata {
        self.metadata.clone()
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>, MetasearchError> {
        let page = query.page;
        let url = format!(
            "https://www.youtube.com/results?search_query={}&page={}",
            urlencoding::encode(&query.query),
            page,
        );

        let resp = self
            .client
            .get(&url)
            .header("Cookie", "CONSENT=YES+")
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

        // Extract ytInitialData JSON from the HTML page
        let json_str = extract_between(&html_text, "ytInitialData = ", ";</script>");
        let json_str = match json_str {
            Some(s) => s,
            None => return Ok(Vec::new()),
        };

        let data: serde_json::Value = serde_json::from_str(&json_str)
            .map_err(|e| MetasearchError::ParseError(e.to_string()))?;

        let mut results = Vec::new();

        // Navigate: contents -> twoColumnSearchResultsRenderer -> primaryContents
        //           -> sectionListRenderer -> contents
        let sections = &data["contents"]["twoColumnSearchResultsRenderer"]["primaryContents"]["sectionListRenderer"]
            ["contents"];

        if let Some(sections_arr) = sections.as_array() {
            for section in sections_arr {
                let contents = &section["itemSectionRenderer"]["contents"];
                if let Some(items) = contents.as_array() {
                    for item in items {
                        let video = &item["videoRenderer"];
                        if video.is_null() {
                            continue;
                        }

                        let video_id = video["videoId"].as_str().unwrap_or_default();
                        if video_id.is_empty() {
                            continue;
                        }

                        let title = get_text_from_runs(&video["title"]);
                        let content = get_text_from_runs(&video["descriptionSnippet"]);
                        let author = get_text_from_runs(&video["ownerText"]);
                        let length = get_text_simple(&video["lengthText"]);

                        let video_url = format!("https://www.youtube.com/watch?v={}", video_id);
                        let thumbnail =
                            format!("https://i.ytimg.com/vi/{}/hqdefault.jpg", video_id);

                        let snippet = format!(
                            "{} — by {} [{}]",
                            if content.is_empty() { "-" } else { &content },
                            author,
                            length,
                        );

                        let mut result =
                            SearchResult::new(title, video_url, snippet, "youtube".to_string());
                        result.engine_rank = (results.len() + 1) as u32;
                        result.category = SearchCategory::Videos.to_string();
                        result.thumbnail = Some(thumbnail);
                        results.push(result);
                    }
                }
            }
        }

        Ok(results)
    }
}

fn extract_between(text: &str, start: &str, end: &str) -> Option<String> {
    let start_idx = text.find(start)? + start.len();
    let remaining = &text[start_idx..];
    let end_idx = remaining.find(end)?;
    Some(remaining[..end_idx].to_string())
}

fn get_text_from_runs(element: &serde_json::Value) -> String {
    if let Some(runs) = element["runs"].as_array() {
        runs.iter()
            .filter_map(|r| r["text"].as_str())
            .collect::<Vec<_>>()
            .join("")
    } else {
        get_text_simple(element)
    }
}

fn get_text_simple(element: &serde_json::Value) -> String {
    element["simpleText"].as_str().unwrap_or("").to_string()
}
