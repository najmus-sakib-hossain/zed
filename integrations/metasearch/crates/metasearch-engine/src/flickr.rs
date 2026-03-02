//! Flickr engine — search images via Flickr web scraping.
//! Translated from SearXNG `searx/engines/flickr_noapi.py`.

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

pub struct Flickr {
    metadata: EngineMetadata,
    client: Client,
}

impl Flickr {
    pub fn new(client: Client) -> Self {
        Self {
            metadata: EngineMetadata {
                name: "flickr".to_string().into(),
                display_name: "Flickr".to_string().into(),
                homepage: "https://www.flickr.com".to_string().into(),
                categories: smallvec![SearchCategory::Images],
                enabled: true,
                timeout_ms: 5000,
                weight: 1.0,
            },
            client,
        }
    }
}

#[async_trait]
impl SearchEngine for Flickr {
    fn metadata(&self) -> EngineMetadata {
        self.metadata.clone()
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>, MetasearchError> {
        let page = query.page;
        let url = format!(
            "https://www.flickr.com/search?text={}&page={}",
            urlencoding::encode(&query.query),
            page,
        );

        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| MetasearchError::HttpError(e.to_string()))?;

        let html_text = resp
            .text()
            .await
            .map_err(|e| MetasearchError::ParseError(e.to_string()))?;

        // Extract modelExport JSON from page
        let re = regex::Regex::new(r"(?m)^\s*modelExport:\s*(\{.*\}),?$")
            .map_err(|e| MetasearchError::ParseError(e.to_string()))?;

        let model_json = match re.captures(&html_text) {
            Some(caps) => caps.get(1).map(|m| m.as_str().to_string()),
            None => None,
        };

        let model_json = match model_json {
            Some(j) => j,
            None => return Ok(Vec::new()),
        };

        let model: serde_json::Value = serde_json::from_str(&model_json)
            .map_err(|e| MetasearchError::ParseError(e.to_string()))?;

        let mut results = Vec::new();

        if let Some(legend) = model["legend"].as_array() {
            for (i, index) in legend.iter().enumerate() {
                if let Some(idx_arr) = index.as_array() {
                    if idx_arr.len() != 8 {
                        continue;
                    }

                    // Navigate through the nested model structure
                    let k0 = idx_arr[0].as_str().unwrap_or("");
                    let k1 = idx_arr[1].as_u64().unwrap_or(0) as usize;
                    let k2 = idx_arr[2].as_str().unwrap_or("");
                    let k3 = idx_arr[3].as_str().unwrap_or("");
                    let k4 = idx_arr[4].as_str().unwrap_or("");
                    let k5 = idx_arr[5].as_str().unwrap_or("");
                    let k6 = idx_arr[6].as_u64().unwrap_or(0) as usize;
                    let k7 = idx_arr[7].as_str().unwrap_or("");

                    let photo = &model["main"][k0][k1][k2][k3][k4][k5][k6][k7];

                    if photo.is_null() {
                        continue;
                    }

                    let title = photo["title"].as_str().unwrap_or("Untitled");
                    let owner_nsid = photo["ownerNsid"].as_str().unwrap_or("");
                    let photo_id = photo["id"].as_str().unwrap_or("");
                    let username = photo["username"].as_str().unwrap_or("");

                    let photo_url = if owner_nsid.is_empty() {
                        String::new()
                    } else {
                        format!("https://www.flickr.com/photos/{}/{}", owner_nsid, photo_id)
                    };

                    if photo_url.is_empty() {
                        continue;
                    }

                    // Find best image size
                    let sizes = &photo["sizes"]["data"];
                    let size_keys = ["o", "k", "h", "b", "c", "z", "m", "n", "t", "q", "s"];
                    let mut img_src = String::new();
                    for key in &size_keys {
                        if let Some(url) = sizes[key]["data"]["url"].as_str() {
                            img_src = url.to_string();
                            break;
                        }
                    }

                    if img_src.is_empty() {
                        continue;
                    }

                    // Fix protocol-relative URLs
                    let img_src = if img_src.starts_with("//") {
                        format!("https:{}", img_src)
                    } else {
                        img_src
                    };

                    let snippet = format!("{} @ Flickr", username);

                    let mut result = SearchResult::new(
                        title.to_string(),
                        photo_url,
                        snippet,
                        "flickr".to_string(),
                    );
                    result.engine_rank = (i + 1) as u32;
                    result.category = SearchCategory::Images.to_string();
                    result.thumbnail = Some(img_src);
                    results.push(result);
                }
            }
        }

        Ok(results)
    }
}
