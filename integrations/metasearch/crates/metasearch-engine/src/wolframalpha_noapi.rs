//! Wolfram|Alpha search engine (no API key required).
//! Two-step process: get token from code endpoint, then query with token.
//! Features: No pagination

use async_trait::async_trait;
use metasearch_core::{
    category::SearchCategory,
    engine::{EngineMetadata, SearchEngine},
    error::{Result},
    query::SearchQuery,
    result::SearchResult,
};
use reqwest::Client;
use tracing::info;
use smallvec::smallvec;

pub struct WolframAlphaNoapi {
    metadata: EngineMetadata,
    client: Client,
}

impl WolframAlphaNoapi {
    pub fn new(client: Client) -> Self {
        Self {
            metadata: EngineMetadata {
                name: "wolframalpha_noapi".to_string().into(),
                display_name: "Wolfram|Alpha".to_string().into(),
                homepage: "https://www.wolframalpha.com".to_string().into(),
                categories: smallvec![SearchCategory::Science],
                enabled: true,
                timeout_ms: 10000,
                weight: 1.0,
            },
            client,
        }
    }
}

#[async_trait]
impl SearchEngine for WolframAlphaNoapi {
    fn metadata(&self) -> EngineMetadata {
        self.metadata.clone()
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>> {
        // Step 1: Get proxy token (ts must be current timestamp in milliseconds)
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();
        let token_url = format!(
            "https://www.wolframalpha.com/input/api/v1/code?ts={}",
            ts
        );

        let token_resp = self
            .client
            .get(&token_url)
            .timeout(std::time::Duration::from_secs(3))
            .header(
                "User-Agent",
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
            )
            .send()
            .await;

        // Wolfram|Alpha may be slow or unavailable
        let token_resp = match token_resp {
            Ok(r) => r,
            Err(_) => return Ok(Vec::new()),
        };

        let token_data: serde_json::Value = match token_resp.json().await {
            Ok(v) => v,
            Err(_) => return Ok(Vec::new()),
        };

        let token = match token_data["code"].as_str() {
            Some(t) => t.to_string(),
            None => return Ok(Vec::new()),
        };

        let encoded_query = urlencoding::encode(&query.query);
        let referer = format!(
            "https://www.wolframalpha.com/input/?i={}",
            encoded_query
        );

        // Step 2: Query with token
        let query_url = format!(
            "https://www.wolframalpha.com/input/json.jsp\
             ?async=false&banners=raw&format=image,plaintext\
             &input={}&output=JSON&proxycode={}",
            encoded_query,
            urlencoding::encode(&token),
        );

        let resp = match self
            .client
            .get(&query_url)
            .timeout(std::time::Duration::from_secs(3))
            .header("Referer", &referer)
            .header(
                "User-Agent",
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
            )
            .send()
            .await
        {
            Ok(r) => r,
            Err(_) => return Ok(Vec::new()),
        };

        let data: serde_json::Value = match resp.json().await {
            Ok(v) => v,
            Err(_) => return Ok(Vec::new()),
        };

        let result_url = format!(
            "https://www.wolframalpha.com/input/?i={}",
            encoded_query
        );

        let mut results = Vec::new();

        let pods = match data["queryresult"]["pods"].as_array() {
            Some(pods) => pods,
            None => return Ok(results),
        };

        for (i, pod) in pods.iter().enumerate() {
            let title = pod["title"].as_str().unwrap_or_default();
            if title.is_empty() {
                continue;
            }

            let content = pod["subpods"]
                .as_array()
                .and_then(|s| s.first())
                .and_then(|sp| sp["plaintext"].as_str())
                .unwrap_or_default();

            if content.is_empty() {
                continue;
            }

            let mut r =
                SearchResult::new(title, &result_url, content, "wolframalpha_noapi");
            r.engine_rank = (i + 1) as u32;
            r.category = SearchCategory::Science.to_string();
            results.push(r);
        }

        info!(
            engine = "wolframalpha_noapi",
            count = results.len(),
            "Search complete"
        );
        Ok(results)
    }
}
