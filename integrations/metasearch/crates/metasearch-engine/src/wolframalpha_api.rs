//! Wolfram|Alpha API search engine implementation.
//! Requires API key (AppID) from https://developer.wolframalpha.com/
//! XML API: https://api.wolframalpha.com/v2/query
//! Features: No pagination

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

pub struct WolframAlphaApi {
    metadata: EngineMetadata,
    client: Client,
    api_key: Option<String>,
}

impl WolframAlphaApi {
    pub fn new(client: Client, api_key: Option<String>) -> Self {
        Self {
            metadata: EngineMetadata {
                name: "wolframalpha_api".to_string().into(),
                display_name: "Wolfram|Alpha API".to_string().into(),
                homepage: "https://www.wolframalpha.com".to_string().into(),
                categories: smallvec![SearchCategory::Science],
                enabled: api_key.is_some(),
                timeout_ms: 10000,
                weight: 1.0,
            },
            client,
            api_key,
        }
    }
}

#[async_trait]
impl SearchEngine for WolframAlphaApi {
    fn metadata(&self) -> EngineMetadata {
        self.metadata.clone()
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>> {
        let key = self.api_key.as_deref().unwrap_or("");
        if key.is_empty() {
            return Err(MetasearchError::EngineError {
                engine: "wolframalpha_api".to_string(),
                message:
                    "No API key configured. Get one at https://developer.wolframalpha.com/"
                        .to_string(),
            });
        }

        let url = format!(
            "https://api.wolframalpha.com/v2/query?appid={}&input={}",
            urlencoding::encode(key),
            urlencoding::encode(&query.query),
        );

        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| MetasearchError::HttpError(e.to_string()))?;

        let xml_text = resp
            .text()
            .await
            .map_err(|e| MetasearchError::ParseError(e.to_string()))?;

        let result_url = format!(
            "https://www.wolframalpha.com/input/?i={}",
            urlencoding::encode(&query.query),
        );

        // Parse pods from XML using regex (dot-all mode for multiline content)
        let pod_re = Regex::new(r#"(?s)<pod\s+title="([^"]*)"[^>]*>(.*?)</pod>"#)
            .map_err(|e| MetasearchError::ParseError(format!("Regex error: {}", e)))?;
        let plaintext_re = Regex::new(r"(?s)<plaintext>(.*?)</plaintext>")
            .map_err(|e| MetasearchError::ParseError(format!("Regex error: {}", e)))?;

        let mut results = Vec::new();

        for (i, pod_cap) in pod_re.captures_iter(&xml_text).enumerate() {
            let pod_title = pod_cap
                .get(1)
                .map(|m| m.as_str())
                .unwrap_or_default();
            let pod_body = pod_cap
                .get(2)
                .map(|m| m.as_str())
                .unwrap_or_default();

            // Extract plaintext content from within this pod
            let content = plaintext_re
                .captures(pod_body)
                .and_then(|c| c.get(1))
                .map(|m| m.as_str().trim().to_string())
                .unwrap_or_default();

            if content.is_empty() {
                continue;
            }

            // Unescape basic XML entities
            let content = content
                .replace("&amp;", "&")
                .replace("&lt;", "<")
                .replace("&gt;", ">")
                .replace("&quot;", "\"")
                .replace("&apos;", "'");

            let mut r =
                SearchResult::new(pod_title, &result_url, &content, "wolframalpha_api");
            r.engine_rank = (i + 1) as u32;
            r.category = SearchCategory::Science.to_string();
            results.push(r);
        }

        info!(
            engine = "wolframalpha_api",
            count = results.len(),
            "Search complete"
        );
        Ok(results)
    }
}
