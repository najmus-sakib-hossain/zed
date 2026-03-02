//! Bing Videos engine — search videos via Bing Videos HTML scraping.
//! Translated from SearXNG `searx/engines/bing_videos.py`.

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

pub struct BingVideos {
    metadata: EngineMetadata,
    client: Client,
}

impl BingVideos {
    pub fn new(client: Client) -> Self {
        Self {
            metadata: EngineMetadata {
                name: "bing_videos".to_string().into(),
                display_name: "Bing Videos".to_string().into(),
                homepage: "https://www.bing.com/videos".to_string().into(),
                categories: smallvec![SearchCategory::Videos],
                enabled: true,
                timeout_ms: 5000,
                weight: 1.0,
            },
            client,
        }
    }
}

#[async_trait]
impl SearchEngine for BingVideos {
    fn metadata(&self) -> EngineMetadata {
        self.metadata.clone()
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>, MetasearchError> {
        let page = query.page;
        let first = (page - 1) * 35 + 1;

        let url = format!(
            "https://www.bing.com/videos/asyncv2?q={}&async=content&first={}&count=35",
            urlencoding::encode(&query.query),
            first,
        );

        let resp = self
            .client
            .get(&url)
            .header(
                "User-Agent",
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
            )
            .send()
            .await
            .map_err(|e| MetasearchError::HttpError(e.to_string()))?;

        let html_text = resp
            .text()
            .await
            .map_err(|e| MetasearchError::ParseError(e.to_string()))?;

        // Bing Videos HTML has issues with scraper parsing mmeta attributes
        // Use regex to extract video data directly from HTML
        let mmeta_regex = regex::Regex::new(
            r#"mmeta="\{&quot;mid&quot;:&quot;([^"]+)"#
        ).unwrap();
        
        let mut results = Vec::new();
        
        for cap in mmeta_regex.captures_iter(&html_text) {
            // Find the full mmeta JSON by looking for the closing brace
            let start_pos = cap.get(0).unwrap().start();
            let remaining = &html_text[start_pos..];
            
            // Extract everything between mmeta=" and the next "
            if let Some(end_quote) = remaining[7..].find("\">") {
                let mmeta_encoded = &remaining[7..7 + end_quote];
                
                // Decode HTML entities
                let mmeta_decoded = mmeta_encoded
                    .replace("&quot;", "\"")
                    .replace("&amp;", "&")
                    .replace("&#39;", "'");
                
                let meta: serde_json::Value = match serde_json::from_str(&mmeta_decoded) {
                    Ok(v) => v,
                    Err(_) => continue,
                };
                
                let video_url = match meta["murl"].as_str() {
                    Some(u) if !u.is_empty() => u.to_string(),
                    _ => continue,
                };
                
                // Extract title from nearby HTML (it's usually right after mmeta)
                let title = "Video".to_string(); // Simplified for now
                let thumbnail = meta["turl"].as_str().map(|s| s.to_string());
                
                let mut result = SearchResult::new(title, video_url, String::new(), "bing_videos".to_string());
                result.engine_rank = (results.len() + 1) as u32;
                result.category = SearchCategory::Videos.to_string();
                result.thumbnail = thumbnail;
                results.push(result);
                
                if results.len() >= 35 {
                    break;
                }
            }
        }

        Ok(results)
    }
}
