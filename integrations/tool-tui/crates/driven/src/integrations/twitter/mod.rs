//! # Twitter/X Integration
//!
//! Twitter API v2 client for tweets and user data.
//!
//! ## Usage
//!
//! ```rust,ignore
//! use driven::integrations::twitter::{TwitterClient, TwitterConfig};
//!
//! let config = TwitterConfig::from_file("~/.dx/config/twitter.sr")?;
//! let client = TwitterClient::new(&config)?;
//!
//! // Post a tweet
//! let tweet = client.post_tweet("Hello, world!").await?;
//!
//! // Get user timeline
//! let tweets = client.get_user_tweets("username").await?;
//! ```

use crate::error::{DrivenError, Result};
use serde::{Deserialize, Serialize};

/// Twitter configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TwitterConfig {
    /// Whether Twitter integration is enabled
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// API Key (Consumer Key)
    #[serde(default)]
    pub api_key: String,
    /// API Secret (Consumer Secret)
    #[serde(default)]
    pub api_secret: String,
    /// Bearer Token (for app-only auth)
    #[serde(default)]
    pub bearer_token: String,
    /// Access Token (for user auth)
    #[serde(default)]
    pub access_token: String,
    /// Access Token Secret (for user auth)
    #[serde(default)]
    pub access_token_secret: String,
}

fn default_true() -> bool {
    true
}

impl Default for TwitterConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            api_key: String::new(),
            api_secret: String::new(),
            bearer_token: String::new(),
            access_token: String::new(),
            access_token_secret: String::new(),
        }
    }
}

impl TwitterConfig {
    /// Load from .sr config file
    pub fn from_file(path: impl AsRef<std::path::Path>) -> Result<Self> {
        let content = std::fs::read_to_string(path.as_ref())
            .map_err(|e| DrivenError::Io(e))?;
        Self::parse_sr(&content)
    }

    fn parse_sr(_content: &str) -> Result<Self> {
        Ok(Self::default())
    }

    /// Resolve environment variables
    pub fn resolve_env_vars(&mut self) {
        if self.api_key.is_empty() || self.api_key.starts_with('$') {
            self.api_key = std::env::var("TWITTER_API_KEY")
                .or_else(|_| std::env::var("TWITTER_CONSUMER_KEY"))
                .unwrap_or_default();
        }
        if self.api_secret.is_empty() || self.api_secret.starts_with('$') {
            self.api_secret = std::env::var("TWITTER_API_SECRET")
                .or_else(|_| std::env::var("TWITTER_CONSUMER_SECRET"))
                .unwrap_or_default();
        }
        if self.bearer_token.is_empty() || self.bearer_token.starts_with('$') {
            self.bearer_token = std::env::var("TWITTER_BEARER_TOKEN").unwrap_or_default();
        }
        if self.access_token.is_empty() || self.access_token.starts_with('$') {
            self.access_token = std::env::var("TWITTER_ACCESS_TOKEN").unwrap_or_default();
        }
        if self.access_token_secret.is_empty() || self.access_token_secret.starts_with('$') {
            self.access_token_secret = std::env::var("TWITTER_ACCESS_TOKEN_SECRET").unwrap_or_default();
        }
    }
}

/// Tweet
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tweet {
    /// Tweet ID
    pub id: String,
    /// Tweet text
    pub text: String,
    /// Author ID
    pub author_id: String,
    /// Created at
    pub created_at: Option<String>,
    /// Conversation ID
    pub conversation_id: Option<String>,
    /// In reply to user ID
    pub in_reply_to_user_id: Option<String>,
    /// Public metrics
    pub public_metrics: Option<TweetMetrics>,
    /// Entities
    pub entities: Option<TweetEntities>,
    /// Attachments
    pub attachments: Option<TweetAttachments>,
}

/// Tweet metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TweetMetrics {
    pub retweet_count: u64,
    pub reply_count: u64,
    pub like_count: u64,
    pub quote_count: u64,
    pub impression_count: Option<u64>,
}

/// Tweet entities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TweetEntities {
    pub hashtags: Option<Vec<Hashtag>>,
    pub mentions: Option<Vec<Mention>>,
    pub urls: Option<Vec<UrlEntity>>,
}

/// Hashtag
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hashtag {
    pub tag: String,
    pub start: u32,
    pub end: u32,
}

/// Mention
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mention {
    pub username: String,
    pub id: String,
    pub start: u32,
    pub end: u32,
}

/// URL entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UrlEntity {
    pub url: String,
    pub expanded_url: String,
    pub display_url: String,
    pub start: u32,
    pub end: u32,
}

/// Tweet attachments
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TweetAttachments {
    pub media_keys: Option<Vec<String>>,
    pub poll_ids: Option<Vec<String>>,
}

/// Twitter user
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TwitterUser {
    /// User ID
    pub id: String,
    /// Username (handle)
    pub username: String,
    /// Display name
    pub name: String,
    /// Bio
    pub description: Option<String>,
    /// Profile image URL
    pub profile_image_url: Option<String>,
    /// Location
    pub location: Option<String>,
    /// URL
    pub url: Option<String>,
    /// Is verified
    pub verified: bool,
    /// Public metrics
    pub public_metrics: Option<UserMetrics>,
    /// Created at
    pub created_at: Option<String>,
}

/// User metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserMetrics {
    pub followers_count: u64,
    pub following_count: u64,
    pub tweet_count: u64,
    pub listed_count: u64,
}

/// Twitter list
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TwitterList {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub owner_id: String,
    pub member_count: Option<u64>,
    pub follower_count: Option<u64>,
    pub private: bool,
}

/// Search result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub tweets: Vec<Tweet>,
    pub users: Vec<TwitterUser>,
    pub next_token: Option<String>,
    pub result_count: u32,
}

/// Twitter client
pub struct TwitterClient {
    config: TwitterConfig,
    base_url: String,
}

impl TwitterClient {
    const API_BASE: &'static str = "https://api.twitter.com/2";

    /// Create a new Twitter client
    pub fn new(config: &TwitterConfig) -> Result<Self> {
        let mut config = config.clone();
        config.resolve_env_vars();

        Ok(Self {
            config,
            base_url: Self::API_BASE.to_string(),
        })
    }

    /// Check if client is configured
    pub fn is_configured(&self) -> bool {
        self.config.enabled && !self.config.bearer_token.is_empty()
    }

    /// Check if can post (requires user auth)
    pub fn can_post(&self) -> bool {
        self.is_configured()
            && !self.config.access_token.is_empty()
            && !self.config.access_token_secret.is_empty()
    }

    // Tweet operations

    /// Post a tweet
    pub async fn post_tweet(&self, text: &str) -> Result<Tweet> {
        if !self.can_post() {
            return Err(DrivenError::Auth("User authentication required".into()));
        }

        let url = format!("{}/tweets", self.base_url);
        let body = serde_json::json!({ "text": text });

        let response = self.api_post_oauth(&url, body).await?;
        self.parse_tweet(&response["data"])
    }

    /// Post a reply
    pub async fn post_reply(&self, text: &str, reply_to: &str) -> Result<Tweet> {
        if !self.can_post() {
            return Err(DrivenError::Auth("User authentication required".into()));
        }

        let url = format!("{}/tweets", self.base_url);
        let body = serde_json::json!({
            "text": text,
            "reply": { "in_reply_to_tweet_id": reply_to }
        });

        let response = self.api_post_oauth(&url, body).await?;
        self.parse_tweet(&response["data"])
    }

    /// Delete a tweet
    pub async fn delete_tweet(&self, tweet_id: &str) -> Result<()> {
        if !self.can_post() {
            return Err(DrivenError::Auth("User authentication required".into()));
        }

        let url = format!("{}/tweets/{}", self.base_url, tweet_id);
        self.api_delete_oauth(&url).await
    }

    /// Get a tweet by ID
    pub async fn get_tweet(&self, tweet_id: &str) -> Result<Tweet> {
        let url = format!(
            "{}/tweets/{}?tweet.fields=created_at,public_metrics,entities,conversation_id,in_reply_to_user_id&expansions=author_id",
            self.base_url,
            tweet_id
        );

        let response = self.api_get(&url).await?;
        self.parse_tweet(&response["data"])
    }

    /// Get user's tweets
    pub async fn get_user_tweets(&self, username: &str) -> Result<Vec<Tweet>> {
        let user = self.get_user(username).await?;
        self.get_user_tweets_by_id(&user.id).await
    }

    /// Get user's tweets by ID
    pub async fn get_user_tweets_by_id(&self, user_id: &str) -> Result<Vec<Tweet>> {
        let url = format!(
            "{}/users/{}/tweets?tweet.fields=created_at,public_metrics,entities,conversation_id&max_results=100",
            self.base_url,
            user_id
        );

        let response = self.api_get(&url).await?;
        let tweets = response["data"]
            .as_array()
            .ok_or_else(|| DrivenError::Parse("Invalid response".into()))?;

        tweets.iter().map(|t| self.parse_tweet(t)).collect()
    }

    /// Like a tweet
    pub async fn like_tweet(&self, tweet_id: &str) -> Result<()> {
        if !self.can_post() {
            return Err(DrivenError::Auth("User authentication required".into()));
        }

        let me = self.get_me().await?;
        let url = format!("{}/users/{}/likes", self.base_url, me.id);
        let body = serde_json::json!({ "tweet_id": tweet_id });

        self.api_post_oauth(&url, body).await?;
        Ok(())
    }

    /// Unlike a tweet
    pub async fn unlike_tweet(&self, tweet_id: &str) -> Result<()> {
        if !self.can_post() {
            return Err(DrivenError::Auth("User authentication required".into()));
        }

        let me = self.get_me().await?;
        let url = format!("{}/users/{}/likes/{}", self.base_url, me.id, tweet_id);

        self.api_delete_oauth(&url).await
    }

    /// Retweet
    pub async fn retweet(&self, tweet_id: &str) -> Result<()> {
        if !self.can_post() {
            return Err(DrivenError::Auth("User authentication required".into()));
        }

        let me = self.get_me().await?;
        let url = format!("{}/users/{}/retweets", self.base_url, me.id);
        let body = serde_json::json!({ "tweet_id": tweet_id });

        self.api_post_oauth(&url, body).await?;
        Ok(())
    }

    // User operations

    /// Get current user
    pub async fn get_me(&self) -> Result<TwitterUser> {
        let url = format!(
            "{}/users/me?user.fields=created_at,description,location,profile_image_url,public_metrics,url,verified",
            self.base_url
        );

        let response = self.api_get(&url).await?;
        self.parse_user(&response["data"])
    }

    /// Get user by username
    pub async fn get_user(&self, username: &str) -> Result<TwitterUser> {
        let url = format!(
            "{}/users/by/username/{}?user.fields=created_at,description,location,profile_image_url,public_metrics,url,verified",
            self.base_url,
            username
        );

        let response = self.api_get(&url).await?;
        self.parse_user(&response["data"])
    }

    /// Get user by ID
    pub async fn get_user_by_id(&self, user_id: &str) -> Result<TwitterUser> {
        let url = format!(
            "{}/users/{}?user.fields=created_at,description,location,profile_image_url,public_metrics,url,verified",
            self.base_url,
            user_id
        );

        let response = self.api_get(&url).await?;
        self.parse_user(&response["data"])
    }

    /// Follow a user
    pub async fn follow(&self, username: &str) -> Result<()> {
        if !self.can_post() {
            return Err(DrivenError::Auth("User authentication required".into()));
        }

        let me = self.get_me().await?;
        let target = self.get_user(username).await?;
        let url = format!("{}/users/{}/following", self.base_url, me.id);
        let body = serde_json::json!({ "target_user_id": target.id });

        self.api_post_oauth(&url, body).await?;
        Ok(())
    }

    /// Unfollow a user
    pub async fn unfollow(&self, username: &str) -> Result<()> {
        if !self.can_post() {
            return Err(DrivenError::Auth("User authentication required".into()));
        }

        let me = self.get_me().await?;
        let target = self.get_user(username).await?;
        let url = format!("{}/users/{}/following/{}", self.base_url, me.id, target.id);

        self.api_delete_oauth(&url).await
    }

    // Search operations

    /// Search tweets
    pub async fn search_tweets(&self, query: &str, max_results: u32) -> Result<SearchResult> {
        let url = format!(
            "{}/tweets/search/recent?query={}&max_results={}&tweet.fields=created_at,public_metrics,entities,author_id&expansions=author_id",
            self.base_url,
            urlencoding::encode(query),
            max_results.min(100)
        );

        let response = self.api_get(&url).await?;
        
        let tweets = response["data"]
            .as_array()
            .map(|arr| arr.iter().filter_map(|t| self.parse_tweet(t).ok()).collect())
            .unwrap_or_default();

        let users = response["includes"]["users"]
            .as_array()
            .map(|arr| arr.iter().filter_map(|u| self.parse_user(u).ok()).collect())
            .unwrap_or_default();

        Ok(SearchResult {
            tweets,
            users,
            next_token: response["meta"]["next_token"].as_str().map(String::from),
            result_count: response["meta"]["result_count"].as_u64().unwrap_or(0) as u32,
        })
    }

    // Parsing helpers

    fn parse_tweet(&self, data: &serde_json::Value) -> Result<Tweet> {
        Ok(Tweet {
            id: data["id"].as_str().unwrap_or_default().to_string(),
            text: data["text"].as_str().unwrap_or_default().to_string(),
            author_id: data["author_id"].as_str().unwrap_or_default().to_string(),
            created_at: data["created_at"].as_str().map(String::from),
            conversation_id: data["conversation_id"].as_str().map(String::from),
            in_reply_to_user_id: data["in_reply_to_user_id"].as_str().map(String::from),
            public_metrics: data["public_metrics"].as_object().map(|m| TweetMetrics {
                retweet_count: m["retweet_count"].as_u64().unwrap_or(0),
                reply_count: m["reply_count"].as_u64().unwrap_or(0),
                like_count: m["like_count"].as_u64().unwrap_or(0),
                quote_count: m["quote_count"].as_u64().unwrap_or(0),
                impression_count: m["impression_count"].as_u64(),
            }),
            entities: None, // Would need more parsing
            attachments: None,
        })
    }

    fn parse_user(&self, data: &serde_json::Value) -> Result<TwitterUser> {
        Ok(TwitterUser {
            id: data["id"].as_str().unwrap_or_default().to_string(),
            username: data["username"].as_str().unwrap_or_default().to_string(),
            name: data["name"].as_str().unwrap_or_default().to_string(),
            description: data["description"].as_str().map(String::from),
            profile_image_url: data["profile_image_url"].as_str().map(String::from),
            location: data["location"].as_str().map(String::from),
            url: data["url"].as_str().map(String::from),
            verified: data["verified"].as_bool().unwrap_or(false),
            public_metrics: data["public_metrics"].as_object().map(|m| UserMetrics {
                followers_count: m["followers_count"].as_u64().unwrap_or(0),
                following_count: m["following_count"].as_u64().unwrap_or(0),
                tweet_count: m["tweet_count"].as_u64().unwrap_or(0),
                listed_count: m["listed_count"].as_u64().unwrap_or(0),
            }),
            created_at: data["created_at"].as_str().map(String::from),
        })
    }

    // API helpers

    async fn api_get(&self, url: &str) -> Result<serde_json::Value> {
        let client = reqwest::Client::new();
        let response = client
            .get(url)
            .header("Authorization", format!("Bearer {}", self.config.bearer_token))
            .send()
            .await
            .map_err(|e| DrivenError::Network(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let error = response.text().await.unwrap_or_default();
            return Err(DrivenError::Api(format!("Twitter API error ({}): {}", status, error)));
        }

        response
            .json()
            .await
            .map_err(|e| DrivenError::Parse(e.to_string()))
    }

    async fn api_post_oauth(&self, url: &str, body: serde_json::Value) -> Result<serde_json::Value> {
        // Note: Full OAuth 1.0a would require proper signature generation
        // This is a simplified version using bearer token
        let client = reqwest::Client::new();
        let response = client
            .post(url)
            .header("Authorization", format!("Bearer {}", self.config.bearer_token))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| DrivenError::Network(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let error = response.text().await.unwrap_or_default();
            return Err(DrivenError::Api(format!("Twitter API error ({}): {}", status, error)));
        }

        response
            .json()
            .await
            .map_err(|e| DrivenError::Parse(e.to_string()))
    }

    async fn api_delete_oauth(&self, url: &str) -> Result<()> {
        let client = reqwest::Client::new();
        let response = client
            .delete(url)
            .header("Authorization", format!("Bearer {}", self.config.bearer_token))
            .send()
            .await
            .map_err(|e| DrivenError::Network(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let error = response.text().await.unwrap_or_default();
            return Err(DrivenError::Api(format!("Twitter API error ({}): {}", status, error)));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = TwitterConfig::default();
        assert!(config.enabled);
    }
}
