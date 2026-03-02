//! YouTube search engine implementation (no API key required).
//! Extracts ytInitialData JSON from HTML using regex.
//! URL: https://www.youtube.com/results?search_query={query}
//! Features: No pagination (first page only)

use async_trait::async_trait;
use metasearch_core::{
    category::SearchCategory,
    engine::{EngineMetadata, SearchEngine},
    error::{MetasearchError, Result},
    query::SearchQuery,
    result::SearchResult,
};
use regex::Regex;
use reqwest::Client;
use tracing::info;
use smallvec::smallvec;

pub struct YoutubeNoapi {
    metadata: EngineMetadata,
    client: Client,
}

impl YoutubeNoapi {
    pub fn new(client: Client) -> Self {
        Self {
            metadata: EngineMetadata {
                name: "youtube_noapi".to_string().into(),
                display_name: "YouTube".to_string().into(),
                homepage: "https://www.youtube.com".to_string().into(),
                categories: smallvec![SearchCategory::Videos, SearchCategory::Music],
                enabled: true,
                timeout_ms: 5000,
                weight: 1.5,
            },
            client,
        }
    }
}

#[async_trait]
impl SearchEngine for YoutubeNoapi {
    fn metadata(&self) -> EngineMetadata {
        self.metadata.clone()
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>> {
        let url = format!(
            "https://www.youtube.com/results?search_query={}",
            urlencoding::encode(&query.query),
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

        // Extract ytInitialData JSON using regex
        let re = Regex::new(r"var ytInitialData = (\{.*?\});")
            .map_err(|e| MetasearchError::ParseError(format!("Regex error: {}", e)))?;

        let json_str = match re.captures(&html_text) {
            Some(caps) => match caps.get(1) {
                Some(m) => m.as_str().to_string(),
                None => return Ok(Vec::new()),
            },
            None => return Ok(Vec::new()),
        };

        let data: serde_json::Value = serde_json::from_str(&json_str)
            .map_err(|e| MetasearchError::ParseError(e.to_string()))?;

        let mut results = Vec::new();

        // Navigate to video results
        let contents = &data["contents"]["twoColumnSearchResultsRenderer"]["primaryContents"]
            ["sectionListRenderer"]["contents"];

        let sections = match contents.as_array() {
            Some(arr) => arr,
            None => return Ok(results),
        };

        // Parse the first section's item list
        if let Some(section) = sections.first() {
            let items = &section["itemSectionRenderer"]["contents"];
            if let Some(items_arr) = items.as_array() {
                for item in items_arr {
                    let video = &item["videoRenderer"];
                    if video.is_null() {
                        continue;
                    }

                    let video_id = match video["videoId"].as_str() {
                        Some(id) if !id.is_empty() => id,
                        _ => continue,
                    };

                    let title = video["title"]["runs"]
                        .as_array()
                        .and_then(|runs| runs.first())
                        .and_then(|r| r["text"].as_str())
                        .unwrap_or_default();

                    if title.is_empty() {
                        continue;
                    }

                    let description = video["descriptionSnippet"]["runs"]
                        .as_array()
                        .map(|runs| {
                            runs.iter()
                                .filter_map(|r| r["text"].as_str())
                                .collect::<Vec<_>>()
                                .join("")
                        })
                        .unwrap_or_default();

                    let author = video["ownerText"]["runs"]
                        .as_array()
                        .and_then(|runs| runs.first())
                        .and_then(|r| r["text"].as_str())
                        .unwrap_or_default();

                    let content = if !author.is_empty() {
                        format!("{} — by {}", description, author)
                    } else {
                        description
                    };

                    let video_url = format!("https://www.youtube.com/watch?v={}", video_id);

                    let thumbnail = video["thumbnail"]["thumbnails"]
                        .as_array()
                        .and_then(|arr| arr.last())
                        .and_then(|t| t["url"].as_str())
                        .map(|s| s.to_string());

                    let mut r =
                        SearchResult::new(title, &video_url, &content, "youtube_noapi");
                    r.engine_rank = (results.len() + 1) as u32;
                    r.category = SearchCategory::Videos.to_string();
                    r.thumbnail = thumbnail;
                    results.push(r);
                }
            }
        }

        info!(
            engine = "youtube_noapi",
            count = results.len(),
            "Search complete"
        );
        Ok(results)
    }
}
