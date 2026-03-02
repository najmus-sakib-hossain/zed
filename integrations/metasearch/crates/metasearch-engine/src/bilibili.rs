//! Bilibili search engine implementation.
//!
//! Translated from SearXNG's `bilibili.py` (97 lines, JSON API).
//! Bilibili is a Chinese video sharing website (often called "B站").
//! Website: https://www.bilibili.com
//! Features: Paging

use async_trait::async_trait;
use metasearch_core::{
    category::SearchCategory,
    engine::{EngineMetadata, SearchEngine},
    error::{MetasearchError, Result},
    query::SearchQuery,
    result::SearchResult,
};
use reqwest::Client;
use serde::Deserialize;
use tracing::info;
use smallvec::smallvec;

pub struct Bilibili {
    metadata: EngineMetadata,
    client: Client,
}

impl Bilibili {
    pub fn new(client: Client) -> Self {
        Self {
            metadata: EngineMetadata {
                name: "bilibili".to_string().into(),
                display_name: "Bilibili".to_string().into(),
                homepage: "https://www.bilibili.com".to_string().into(),
                categories: smallvec![SearchCategory::Videos],
                enabled: true,
                timeout_ms: 4000,
                weight: 0.9,
            },
            client,
        }
    }
}

/// Strip HTML tags from a string (simple implementation).
fn strip_html(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut in_tag = false;
    for ch in s.chars() {
        if ch == '<' {
            in_tag = true;
        } else if ch == '>' {
            in_tag = false;
        } else if !in_tag {
            result.push(ch);
        }
    }
    result
}

#[derive(Deserialize, Debug)]
struct BiliResponse {
    data: Option<BiliData>,
}

#[derive(Deserialize, Debug)]
struct BiliData {
    result: Option<Vec<BiliItem>>,
}

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
struct BiliItem {
    title: String,
    arcurl: String,
    description: String,
    author: String,
    pic: String,
    aid: u64,
    pubdate: i64,
    #[serde(default)]
    duration: String,
}

#[async_trait]
impl SearchEngine for Bilibili {
    fn metadata(&self) -> EngineMetadata {
        self.metadata.clone()
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>> {
        let page = query.page.max(1);
        let results_per_page = 20;
        let encoded = urlencoding::encode(&query.query);

        let url = format!(
            "https://api.bilibili.com/x/web-interface/search/type?__refresh__=true&page={}&page_size={}&single_column=0&keyword={}&search_type=video",
            page, results_per_page, encoded
        );

        // Bilibili requires specific cookies to work properly
        let resp = self
            .client
            .get(&url)
            .header("Referer", "https://www.bilibili.com")
            .header(
                "User-Agent",
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
            )
            .header("Cookie", "innersign=0; buvid3=0123456789ABCDEFinfoc; i-wanna-go-back=-1; b_ut=7; FEED_LIVE_VERSION=V8; header_theme_version=undefined; home_feed_column=4")
            .send()
            .await
            .map_err(|e| MetasearchError::HttpError(e.to_string()))?;

        let data: BiliResponse = resp
            .json()
            .await
            .map_err(|e| MetasearchError::ParseError(format!("JSON error: {}", e)))?;

        let mut results = Vec::new();

        if let Some(bili_data) = data.data {
            if let Some(items) = bili_data.result {
                for (i, item) in items.iter().enumerate() {
                    let title = strip_html(&item.title);

                    let snippet = format!(
                        "by {} | {} | {}",
                        item.author,
                        item.duration,
                        item.description.chars().take(200).collect::<String>()
                    );

                    let thumbnail = if item.pic.starts_with("//") {
                        format!("https:{}", item.pic)
                    } else {
                        item.pic.clone()
                    };

                    let mut r = SearchResult::new(&title, &item.arcurl, &snippet, "bilibili");
                    r.engine_rank = i as u32;
                    r.category = SearchCategory::Videos.to_string();
                    r.thumbnail = Some(thumbnail);

                    if item.pubdate > 0 {
                        if let Some(dt) = chrono::DateTime::from_timestamp(item.pubdate, 0) {
                            r.published_date = Some(dt);
                        }
                    }

                    results.push(r);
                }
            }
        }

        info!(
            engine = "bilibili",
            count = results.len(),
            "Search complete"
        );
        Ok(results)
    }
}
