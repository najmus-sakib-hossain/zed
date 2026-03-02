//! Discourse forum search — configurable instance URL + optional API key.
//! SearXNG equivalent: `discourse.py`
//!
//! Discourse is an open-source forum system. This engine queries any
//! Discourse instance via its JSON search API. If the forum is private,
//! configure `api_key` and `api_username`.

use async_trait::async_trait;
use reqwest::Client;
use serde::Deserialize;
use std::collections::HashMap;
use smallvec::smallvec;

use metasearch_core::{
    category::SearchCategory,
    engine::{EngineMetadata, SearchEngine},
    error::MetasearchError,
    query::SearchQuery,
    result::SearchResult,
};

pub struct Discourse {
    client: Client,
    base_url: String,
    api_key: Option<String>,
    api_username: Option<String>,
}

impl Discourse {
    pub fn new(
        client: Client,
        base_url: &str,
        api_key: Option<String>,
        api_username: Option<String>,
    ) -> Self {
        Self {
            client,
            base_url: base_url.trim_end_matches('/').to_string(),
            api_key,
            api_username,
        }
    }
}

#[derive(Deserialize)]
struct DiscourseResponse {
    topics: Option<Vec<DiscourseTopic>>,
    posts: Option<Vec<DiscoursePost>>,
}

#[derive(Deserialize)]
struct DiscourseTopic {
    id: u64,
    title: String,
    #[serde(default)]
    #[allow(dead_code)]
    posts_count: u32,
    #[serde(default)]
    #[allow(dead_code)]
    created_at: String,
}

#[derive(Deserialize)]
struct DiscoursePost {
    id: u64,
    topic_id: u64,
    #[serde(default)]
    #[allow(dead_code)]
    username: String,
    #[serde(default)]
    blurb: String,
}

#[async_trait]
impl SearchEngine for Discourse {
    fn metadata(&self) -> EngineMetadata {
        EngineMetadata {
            name: "discourse".to_string().into(),
            display_name: "Discourse".to_string().into(),
            homepage: "https://www.discourse.org".to_string().into(),
            categories: smallvec![SearchCategory::General],
            enabled: !self.base_url.is_empty(),
            timeout_ms: 5000,
            weight: 1.0,
        }
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>, MetasearchError> {
        if self.base_url.is_empty() {
            return Ok(Vec::new());
        }

        let url = format!(
            "{}/search.json?q={} order:likes&page={}",
            self.base_url,
            urlencoding::encode(&query.query),
            query.page,
        );

        let mut req = self
            .client
            .get(&url)
            .header("Accept", "application/json")
            .header("X-Requested-With", "XMLHttpRequest");

        if let Some(ref key) = self.api_key {
            req = req.header("Api-Key", key.as_str());
        }
        if let Some(ref username) = self.api_username {
            req = req.header("Api-Username", username.as_str());
        }

        let resp = req
            .send()
            .await
            .map_err(|e| MetasearchError::Engine(format!("Discourse: {e}")))?;

        let data: DiscourseResponse = resp
            .json()
            .await
            .map_err(|e| MetasearchError::Engine(format!("Discourse JSON: {e}")))?;

        let topics = data.topics.unwrap_or_default();
        let posts = data.posts.unwrap_or_default();

        let topic_map: HashMap<u64, &DiscourseTopic> = topics.iter().map(|t| (t.id, t)).collect();

        let mut results = Vec::new();
        for (i, post) in posts.iter().enumerate() {
            let topic = topic_map.get(&post.topic_id);
            let title = topic.map(|t| t.title.clone()).unwrap_or_default();
            let post_url = format!("{}/p/{}", self.base_url, post.id);
            let content = html_escape::decode_html_entities(&post.blurb).to_string();

            results.push(SearchResult {
                title,
                url: post_url,
                content,
                engine: "Discourse".to_string(),
                engine_rank: (i + 1) as u32,
                    score: 0.0,
                    thumbnail: None,
                    published_date: None,
                    category: String::new(),
                    metadata: serde_json::Value::Null,
                });
        }
        Ok(results)
    }
}
