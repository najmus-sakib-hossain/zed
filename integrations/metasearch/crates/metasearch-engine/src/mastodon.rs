//! Mastodon engine — search accounts/hashtags via Mastodon API.
//! Translated from SearXNG `searx/engines/mastodon.py`.

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

static HTML_TAG_RE: std::sync::LazyLock<regex::Regex> =
    std::sync::LazyLock::new(|| regex::Regex::new(r"<[^>]+>").unwrap());

pub struct Mastodon {
    metadata: EngineMetadata,
    client: Client,
    base_url: String,
}

impl Mastodon {
    pub fn new(client: Client) -> Self {
        Self {
            metadata: EngineMetadata {
                name: "mastodon".to_string().into(),
                display_name: "Mastodon".to_string().into(),
                homepage: "https://mastodon.social".to_string().into(),
                categories: smallvec![SearchCategory::SocialMedia],
                enabled: true,
                timeout_ms: 5000,
                weight: 0.8,
            },
            client,
            base_url: "https://mastodon.social".to_string(),
        }
    }

    pub fn with_base_url(client: Client, base_url: &str) -> Self {
        Self {
            metadata: EngineMetadata {
                name: "mastodon".to_string().into(),
                display_name: "Mastodon".to_string().into(),
                homepage: base_url.to_string().into(),
                categories: smallvec![SearchCategory::SocialMedia],
                enabled: true,
                timeout_ms: 5000,
                weight: 0.8,
            },
            client,
            base_url: base_url.to_string(),
        }
    }
}

#[async_trait]
impl SearchEngine for Mastodon {
    fn metadata(&self) -> EngineMetadata {
        self.metadata.clone()
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>, MetasearchError> {
        let url = format!(
            "{}/api/v2/search?q={}&resolve=false&type=accounts&limit=40",
            self.base_url,
            urlencoding::encode(&query.query),
        );

        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| MetasearchError::HttpError(e.to_string()))?;

        let data: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| MetasearchError::ParseError(e.to_string()))?;

        let mut results = Vec::new();

        // Search accounts
        if let Some(accounts) = data["accounts"].as_array() {
            for (i, account) in accounts.iter().enumerate() {
                let username = account["username"].as_str().unwrap_or_default();
                let uri = account["uri"].as_str().unwrap_or_default();
                let followers = account["followers_count"].as_u64().unwrap_or(0);
                let note = account["note"].as_str().unwrap_or("");

                // Strip HTML from note
                let clean_note =
                    html_escape::decode_html_entities(&HTML_TAG_RE.replace_all(note, " "))
                        .to_string();

                let title = format!("{} ({} followers)", username, followers);

                let mut result = SearchResult::new(
                    title,
                    uri.to_string(),
                    clean_note.trim().to_string(),
                    "mastodon".to_string(),
                );
                result.engine_rank = (i + 1) as u32;
                result.category = SearchCategory::SocialMedia.to_string();
                result.thumbnail = account["avatar"].as_str().map(|s| s.to_string());
                results.push(result);
            }
        }

        // Also search hashtags
        if let Some(hashtags) = data["hashtags"].as_array() {
            for (i, hashtag) in hashtags.iter().enumerate() {
                let name = hashtag["name"].as_str().unwrap_or_default();
                let tag_url = hashtag["url"].as_str().unwrap_or_default();

                let uses: u64 = hashtag["history"]
                    .as_array()
                    .unwrap_or(&Vec::new())
                    .iter()
                    .filter_map(|h| h["uses"].as_str().and_then(|s| s.parse::<u64>().ok()))
                    .sum();

                let snippet = format!("#{} — used {} times", name, uses);

                let mut result = SearchResult::new(
                    format!("#{}", name),
                    tag_url.to_string(),
                    snippet,
                    "mastodon".to_string(),
                );
                result.engine_rank = (results.len() + i + 1) as u32;
                result.category = SearchCategory::SocialMedia.to_string();
                results.push(result);
            }
        }

        Ok(results)
    }
}
