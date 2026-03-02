//! Quark (Shenma) search engine implementation.
//!
//! Translated from SearXNG's `quark.py` (HTML with embedded JSON).
//! Quark is a Chinese search engine by Alibaba (UCWeb/Shenma).
//! Website: https://quark.sm.cn
//! Features: Paging, Time Range

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
use serde_json::Value;
use tracing::{info, warn};
use smallvec::smallvec;

pub struct Quark {
    metadata: EngineMetadata,
    client: Client,
}

impl Quark {
    pub fn new(client: Client) -> Self {
        Self {
            metadata: EngineMetadata {
                name: "quark".to_string().into(),
                display_name: "Quark".to_string().into(),
                homepage: "https://quark.sm.cn".to_string().into(),
                categories: smallvec![SearchCategory::General],
                enabled: true,
                timeout_ms: 5000,
                weight: 1.0,
            },
            client,
        }
    }

    /// Map time_range to Quark's tl_request parameter.
    fn time_range_param(time_range: Option<&str>) -> Option<&'static str> {
        match time_range {
            Some("day") => Some("4"),
            Some("week") => Some("3"),
            Some("month") => Some("2"),
            Some("year") => Some("1"),
            _ => None,
        }
    }

    /// Check if the response contains an Alibaba X5SEC CAPTCHA.
    fn is_captcha(text: &str) -> bool {
        let re =
            Regex::new(r#"\{[^{]*?"action"\s*:\s*"captcha"\s*,\s*"url"\s*:\s*"[^"]+"[^{]*?\}"#)
                .unwrap();
        re.is_match(text)
    }

    /// Parse a single result from various Quark source category formats.
    fn parse_initial_data(data: &Value) -> Option<SearchResult> {
        // Try titleProps/summaryProps/sourceProps (ss_doc, ss_kv, ss_pic, ss_text, ss_video, etc.)
        let title = data
            .pointer("/titleProps/content")
            .and_then(|v| v.as_str())
            .or_else(|| data.get("title").and_then(|v| v.as_str()))
            .or_else(|| data.pointer("/title/content").and_then(|v| v.as_str()));

        let url = data
            .pointer("/sourceProps/dest_url")
            .and_then(|v| v.as_str())
            .or_else(|| data.get("normal_url").and_then(|v| v.as_str()))
            .or_else(|| data.get("url").and_then(|v| v.as_str()))
            .or_else(|| data.pointer("/source/url").and_then(|v| v.as_str()));

        let content = data
            .pointer("/summaryProps/content")
            .and_then(|v| v.as_str())
            .or_else(|| {
                data.pointer("/message/replyContent")
                    .and_then(|v| v.as_str())
            })
            .or_else(|| data.get("show_body").and_then(|v| v.as_str()))
            .or_else(|| data.get("desc").and_then(|v| v.as_str()))
            .or_else(|| data.pointer("/summary/content").and_then(|v| v.as_str()));

        let title_str = html_escape::decode_html_entities(title?).to_string();
        let url_str = url?.to_string();

        if title_str.is_empty() || url_str.is_empty() {
            return None;
        }

        let content_str = content
            .map(|c| html_escape::decode_html_entities(c).to_string())
            .unwrap_or_default();

        let mut r = SearchResult::new(&title_str, &url_str, &content_str, "quark");
        r.category = SearchCategory::General.to_string();

        // Try to parse published date from sourceProps.time or source.time
        let timestamp = data
            .pointer("/sourceProps/time")
            .or_else(|| data.pointer("/source/time"))
            .and_then(|v| {
                v.as_i64()
                    .or_else(|| v.as_str().and_then(|s| s.parse().ok()))
            });

        if let Some(ts) = timestamp {
            if ts > 0 {
                r.published_date = chrono::DateTime::from_timestamp(ts, 0);
            }
        }

        Some(r)
    }

    /// Parse list-style results (ai_page, news_uchq, etc.).
    fn parse_list_data(data: &Value) -> Vec<SearchResult> {
        let mut results = Vec::new();

        // ai_page: list[] items
        if let Some(list) = data.get("list").and_then(|v| v.as_array()) {
            for item in list {
                let title = item
                    .get("title")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();
                let url = item.get("url").and_then(|v| v.as_str()).unwrap_or_default();

                if title.is_empty() || url.is_empty() {
                    continue;
                }

                let content = item
                    .get("content")
                    .and_then(|v| {
                        if let Some(arr) = v.as_array() {
                            Some(
                                arr.iter()
                                    .filter_map(|x| x.as_str())
                                    .collect::<Vec<_>>()
                                    .join(" | "),
                            )
                        } else {
                            v.as_str().map(|s| s.to_string())
                        }
                    })
                    .unwrap_or_default();

                let mut r = SearchResult::new(
                    html_escape::decode_html_entities(title).into_owned(),
                    url,
                    html_escape::decode_html_entities(&content).into_owned(),
                    "quark",
                );
                r.category = SearchCategory::General.to_string();

                let timestamp = item.pointer("/source/time").and_then(|v| {
                    v.as_i64()
                        .or_else(|| v.as_str().and_then(|s| s.parse().ok()))
                });
                if let Some(ts) = timestamp {
                    if ts > 0 {
                        r.published_date = chrono::DateTime::from_timestamp(ts, 0);
                    }
                }

                results.push(r);
            }
        }

        // news_uchq: feed[] items
        if let Some(feed) = data.get("feed").and_then(|v| v.as_array()) {
            for item in feed {
                let title = item
                    .get("title")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();
                let url = item.get("url").and_then(|v| v.as_str()).unwrap_or_default();

                if title.is_empty() || url.is_empty() {
                    continue;
                }

                let content = item
                    .get("summary")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();

                let mut r = SearchResult::new(
                    html_escape::decode_html_entities(title).into_owned(),
                    url,
                    html_escape::decode_html_entities(content).into_owned(),
                    "quark",
                );
                r.category = SearchCategory::General.to_string();
                results.push(r);
            }
        }

        results
    }
}

#[async_trait]
impl SearchEngine for Quark {
    fn metadata(&self) -> EngineMetadata {
        self.metadata.clone()
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>> {
        let page = query.page.max(1);
        let encoded = urlencoding::encode(&query.query);

        let mut url = format!(
            "https://quark.sm.cn/s?q={}&layout=html&page={}",
            encoded, page
        );

        if let Some(tr) = Self::time_range_param(query.time_range.as_deref()) {
            url.push_str(&format!("&tl_request={}", tr));
        }

        let resp = self
            .client
            .get(&url)
            .header(
                "User-Agent",
                "Mozilla/5.0 (Linux; Android 12) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/99.0 Mobile Safari/537.36",
            )
            .header("Accept-Language", "zh-CN,zh;q=0.9,en;q=0.8")
            .send()
            .await
            .map_err(|e| MetasearchError::HttpError(e.to_string()))?;

        let body = resp
            .text()
            .await
            .map_err(|e| MetasearchError::ParseError(e.to_string()))?;

        // Check for Alibaba CAPTCHA
        if Self::is_captcha(&body) {
            warn!(engine = "quark", "Alibaba CAPTCHA detected");
            return Err(MetasearchError::EngineError {
                engine: "quark".to_string(),
                message: "Alibaba CAPTCHA detected. Please try again later.".to_string(),
            });
        }

        let mut results = Vec::new();

        // Quark embeds JSON data in script tags with id="s-data-*"
        let json_re = Regex::new(
            r#"<script\s+type="application/json"\s+id="s-data-[^"]+"\s+data-used-by="hydrate">(.*?)</script>"#,
        )
        .unwrap();

        // Also try HTML-entity-encoded variant
        let json_re_encoded = Regex::new(
            r#"&lt;script\s+type="application/json"\s+id="s-data-[^"]+"\s+data-used-by="hydrate"&gt;(.*?)&lt;/script&gt;"#,
        )
        .unwrap();

        let mut rank: u32 = 0;

        // Supported source categories
        let single_result_scs = [
            "addition",
            "baike_sc",
            "finance_shuidi",
            "kk_yidian_all",
            "med_struct",
            "nature_result",
            "ss_doc",
            "ss_kv",
            "ss_pic",
            "ss_text",
            "ss_video",
            "ss_note",
            "baike",
            "structure_web_novel",
            "travel_dest_overview",
            "travel_ranking_list",
        ];
        let list_scs = ["ai_page", "news_uchq"];

        for captures in json_re
            .captures_iter(&body)
            .chain(json_re_encoded.captures_iter(&body))
        {
            let json_str = &captures[1];
            let data: Value = match serde_json::from_str(json_str) {
                Ok(d) => d,
                Err(_) => continue,
            };

            let sc = data
                .pointer("/extraData/sc")
                .and_then(|v| v.as_str())
                .unwrap_or_default();

            let initial_data = data
                .pointer("/data/initialData")
                .cloned()
                .unwrap_or(Value::Null);

            if single_result_scs.contains(&sc) {
                if let Some(mut r) = Self::parse_initial_data(&initial_data) {
                    r.engine_rank = rank;
                    results.push(r);
                    rank += 1;
                }
            } else if list_scs.contains(&sc) {
                for mut r in Self::parse_list_data(&initial_data) {
                    r.engine_rank = rank;
                    results.push(r);
                    rank += 1;
                }
            }
        }

        info!(engine = "quark", count = results.len(), "Search complete");
        Ok(results)
    }
}
