//! Google Images search engine.
//! Translated from SearXNG's `google_images.py`.
//! Uses Google's internal async JSON API (same approach as SearXNG).
//! No API key required.
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
use tracing::{info, warn};
use smallvec::smallvec;

pub struct GoogleImages {
    metadata: EngineMetadata,
    client: Client,
}

impl GoogleImages {
    pub fn new(client: Client) -> Self {
        Self {
            metadata: EngineMetadata {
                name: "google_images".to_string().into(),
                display_name: "Google Images".to_string().into(),
                homepage: "https://images.google.com".to_string().into(),
                categories: smallvec![SearchCategory::General],
                enabled: true,
                timeout_ms: 5000,
                weight: 1.0,
            },
            client,
        }
    }
}

#[derive(Deserialize)]
struct GImagesResponse {
    ischj: Option<GImagesIschj>,
}

#[derive(Deserialize)]
struct GImagesIschj {
    metadata: Option<Vec<GImagesMeta>>,
}

#[derive(Deserialize)]
struct GImagesMeta {
    result: Option<GImagesResult>,
    original_image: Option<GImagesOriginal>,
    text_in_grid: Option<GImagesTextInGrid>,
}

#[derive(Deserialize)]
struct GImagesResult {
    referrer_url: Option<String>,
    page_title: Option<String>,
    #[allow(dead_code)]
    site_title: Option<String>,
}

#[derive(Deserialize)]
#[allow(dead_code)]
struct GImagesOriginal {
    url: Option<String>,
    width: Option<u32>,
    height: Option<u32>,
}

#[derive(Deserialize)]
struct GImagesTextInGrid {
    snippet: Option<String>,
}

#[async_trait]
impl SearchEngine for GoogleImages {
    fn metadata(&self) -> EngineMetadata {
        self.metadata.clone()
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>> {
        // Zero-based page index for Google's ijn parameter
        let page = query.page.max(1) - 1;
        let url = format!(
            "https://www.google.com/search?q={}&tbm=isch&asearch=isch&async=_fmt:json,p:1,ijn:{}",
            urlencoding::encode(&query.query),
            page
        );

        let resp = self
            .client
            .get(&url)
            .header(
                "User-Agent",
                "NSTN/3.60.474802233.release Dalvik/2.1.0 (Linux; U; Android 12; US) gzip",
            )
            .send()
            .await
            .map_err(|e| MetasearchError::HttpError(e.to_string()))?;

        let text = resp
            .text()
            .await
            .map_err(|e| MetasearchError::ParseError(e.to_string()))?;

        // Check for Google sorry/captcha
        if text.contains("/sorry/index") || text.contains("detected unusual traffic") {
            warn!(engine = "google_images", "Captcha detected");
            return Err(MetasearchError::EngineError {
                engine: "google_images".to_string(),
                message: "Google captcha detected".to_string(),
            });
        }

        // Find the JSON payload starting with {"ischj":
        let json_start = match text.find("{\"ischj\":") {
            Some(pos) => pos,
            None => return Ok(Vec::new()),
        };
        let json_text = &text[json_start..];

        let data: GImagesResponse = serde_json::from_str(json_text)
            .map_err(|e| MetasearchError::ParseError(format!("JSON parse error: {}", e)))?;

        let mut results = Vec::new();
        if let Some(ischj) = data.ischj {
            if let Some(metadata) = ischj.metadata {
                for (i, item) in metadata.iter().enumerate() {
                    let title = item
                        .result
                        .as_ref()
                        .and_then(|r| r.page_title.as_deref())
                        .unwrap_or("");
                    let result_url = item
                        .result
                        .as_ref()
                        .and_then(|r| r.referrer_url.as_deref())
                        .unwrap_or("");
                    let content = item
                        .text_in_grid
                        .as_ref()
                        .and_then(|t| t.snippet.as_deref())
                        .unwrap_or("");

                    if title.is_empty() || result_url.is_empty() {
                        continue;
                    }

                    // Build snippet with resolution info
                    let resolution = item
                        .original_image
                        .as_ref()
                        .map(|o| format!("{} x {}", o.width.unwrap_or(0), o.height.unwrap_or(0)))
                        .unwrap_or_default();

                    let full_content = if resolution.is_empty() {
                        content.to_string()
                    } else {
                        format!("{} ({})", content, resolution)
                    };

                    // Get the actual image URL
                    let img_url = item
                        .original_image
                        .as_ref()
                        .and_then(|o| o.url.as_deref())
                        .unwrap_or("");

                    let mut r =
                        SearchResult::new(title, result_url, &full_content, "google_images");
                    r.engine_rank = (i + 1) as u32;
                    r.category = SearchCategory::General.to_string();
                    
                    // Set thumbnail to the actual image URL for display
                    if !img_url.is_empty() {
                        r.thumbnail = Some(img_url.to_string());
                        
                        // Store image metadata
                        if let Some(orig) = &item.original_image {
                            r.metadata = serde_json::json!({
                                "img_src": img_url,
                                "width": orig.width,
                                "height": orig.height,
                                "resolution": resolution,
                            });
                        }
                    }
                    
                    results.push(r);
                }
            }
        }

        info!(
            engine = "google_images",
            count = results.len(),
            "Search complete"
        );
        Ok(results)
    }
}
