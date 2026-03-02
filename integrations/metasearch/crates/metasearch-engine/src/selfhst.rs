//! Selfh.st engine — search self-hosted application icons via selfh.st index.

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

pub struct Selfhst {
    metadata: EngineMetadata,
    client: Client,
}

impl Selfhst {
    pub fn new(client: Client) -> Self {
        Self {
            metadata: EngineMetadata {
                name: "selfhst".to_string().into(),
                display_name: "Selfh.st Icons".to_string().into(),
                homepage: "https://selfh.st/icons/".to_string().into(),
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
impl SearchEngine for Selfhst {
    fn metadata(&self) -> EngineMetadata {
        self.metadata.clone()
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>, MetasearchError> {
        let url = "https://cdn.jsdelivr.net/gh/selfhst/icons/index.json";

        let resp = self
            .client
            .get(url)
            .send()
            .await
            .map_err(|e| MetasearchError::HttpError(e.to_string()))?;

        let data: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| MetasearchError::ParseError(e.to_string()))?;

        let items = match data.as_array() {
            Some(arr) => arr,
            None => return Ok(Vec::new()),
        };

        let query_lower = query.query.to_lowercase();
        let query_words: Vec<&str> = query_lower.split_whitespace().collect();

        let mut results = Vec::new();

        for item in items {
            let reference = item["Reference"].as_str().unwrap_or_default();
            let name = item["Name"].as_str().unwrap_or(reference);
            let tags = item["Tags"].as_str().unwrap_or_default();
            let category = item["Category"].as_str().unwrap_or_default();

            let search_text = format!("{} {} {} {}", name, reference, tags, category).to_lowercase();

            let matches = query_words.iter().any(|word| search_text.contains(word));
            if !matches {
                continue;
            }

            // Values are strings "Yes" / "" — not JSON booleans
            let is_yes = |v: &serde_json::Value| {
                v.as_str().map(|s| s.eq_ignore_ascii_case("yes")).unwrap_or(false)
                    || v.as_bool().unwrap_or(false)
            };

            let format = if is_yes(&item["SVG"]) {
                "svg"
            } else if is_yes(&item["PNG"]) {
                "png"
            } else if is_yes(&item["WebP"]) {
                "webp"
            } else {
                "png" // fallback
            };

            let image_url = format!(
                "https://cdn.jsdelivr.net/gh/selfhst/icons/{}/{}.{}",
                format, reference, format,
            );

            let mut result = SearchResult::new(
                name.to_string(),
                "https://selfh.st/icons/".to_string(),
                format!("Self-hosted icon: {}", reference),
                "selfhst".to_string(),
            );
            result.engine_rank = (results.len() + 1) as u32;
            result.category = SearchCategory::Images.to_string();
            result.thumbnail = Some(image_url);
            results.push(result);

            if results.len() >= 20 {
                break;
            }
        }

        Ok(results)
    }
}
