//! Reddit engine — search posts via Reddit JSON API.
//! Translated from SearXNG `searx/engines/reddit.py`.

use async_trait::async_trait;
use metasearch_core::{
    category::SearchCategory,
    engine::{EngineMetadata, SearchEngine},
    error::Result,
    query::SearchQuery,
    result::SearchResult,
};
use reqwest::Client;
use smallvec::smallvec;

pub struct Reddit {
    metadata: EngineMetadata,
    client: Client,
}

impl Reddit {
    pub fn new(client: Client) -> Self {
        Self {
            metadata: EngineMetadata {
                name: "reddit".to_string().into(),
                display_name: "Reddit".to_string().into(),
                homepage: "https://www.reddit.com".to_string().into(),
                categories: smallvec![SearchCategory::SocialMedia],
                enabled: true,
                timeout_ms: 5000,
                weight: 1.0,
            },
            client,
        }
    }
}

#[async_trait]
impl SearchEngine for Reddit {
    fn metadata(&self) -> EngineMetadata {
        self.metadata.clone()
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>> {
        // Use old.reddit.com which is more lenient with bots, and use a browser-like UA
        let url = format!(
            "https://old.reddit.com/search.json?q={}&limit=25&sort=relevance",
            urlencoding::encode(&query.query),
        );

        let resp = match self
            .client
            .get(&url)
            .timeout(std::time::Duration::from_secs(7))
            .header(
                "User-Agent",
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:120.0) Gecko/20100101 Firefox/120.0",
            )
            .header("Accept", "application/json")
            .send()
            .await
        {
            Ok(r) => r,
            Err(_) => return Ok(Vec::new()),
        };

        // Reddit may return 429 (rate-limit) or a redirect/HTML page instead of JSON
        if !resp.status().is_success() {
            return Ok(Vec::new());
        }

        let text = match resp.text().await {
            Ok(t) => t,
            Err(_) => return Ok(Vec::new()),
        };

        // Guard against HTML responses (bot detection, CAPTCHA, login wall)
        if text.trim_start().starts_with('<') {
            return Ok(Vec::new());
        }

        let data: serde_json::Value = match serde_json::from_str(&text) {
            Ok(v) => v,
            Err(_) => return Ok(Vec::new()),
        };

        let mut results = Vec::new();

        if let Some(children) = data["data"]["children"].as_array() {
            for (i, child) in children.iter().enumerate() {
                let post = &child["data"];
                let title = post["title"].as_str().unwrap_or_default();
                let permalink = post["permalink"].as_str().unwrap_or("");
                let subreddit = post["subreddit_name_prefixed"].as_str().unwrap_or("");
                let score = post["score"].as_i64().unwrap_or(0);
                let num_comments = post["num_comments"].as_u64().unwrap_or(0);
                let selftext = post["selftext"].as_str().unwrap_or("");

                let post_url = format!("https://www.reddit.com{}", permalink);

                let content = if selftext.len() > 300 {
                    format!("{}...", &selftext[..300])
                } else {
                    selftext.to_string()
                };

                let snippet = format!(
                    "{} — ⬆ {} | 💬 {} | {}",
                    content, score, num_comments, subreddit,
                );

                let mut result =
                    SearchResult::new(title.to_string(), post_url, snippet, "reddit".to_string());
                result.engine_rank = (i + 1) as u32;
                result.category = SearchCategory::SocialMedia.to_string();
                result.thumbnail = post["thumbnail"]
                    .as_str()
                    .filter(|t| t.starts_with("http"))
                    .map(|t| t.to_string());
                results.push(result);
            }
        }

        Ok(results)
    }
}
