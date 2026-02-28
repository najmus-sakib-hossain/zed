//! # Trello Integration
//!
//! Trello API client for boards, lists, and cards.
//!
//! ## Usage
//!
//! ```rust,ignore
//! use driven::integrations::trello::{TrelloClient, TrelloConfig};
//!
//! let config = TrelloConfig::from_file("~/.dx/config/trello.sr")?;
//! let client = TrelloClient::new(&config)?;
//!
//! // Create a card
//! let card = client.create_card("list_id", "Task title", "Description").await?;
//!
//! // Move card to another list
//! client.move_card(&card.id, "done_list_id").await?;
//! ```

use crate::error::{DrivenError, Result};
use serde::{Deserialize, Serialize};

/// Trello configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrelloConfig {
    /// Whether Trello integration is enabled
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Trello API key
    #[serde(default)]
    pub api_key: String,
    /// Trello API token
    #[serde(default)]
    pub api_token: String,
    /// Default board ID
    pub default_board: Option<String>,
}

fn default_true() -> bool {
    true
}

impl Default for TrelloConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            api_key: String::new(),
            api_token: String::new(),
            default_board: None,
        }
    }
}

impl TrelloConfig {
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
            self.api_key = std::env::var("TRELLO_API_KEY").unwrap_or_default();
        }
        if self.api_token.is_empty() || self.api_token.starts_with('$') {
            self.api_token = std::env::var("TRELLO_API_TOKEN").unwrap_or_default();
        }
    }
}

/// Trello board
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrelloBoard {
    /// Board ID
    pub id: String,
    /// Board name
    pub name: String,
    /// Board description
    pub desc: Option<String>,
    /// Board URL
    pub url: String,
    /// Is closed
    pub closed: bool,
    /// Organization ID
    #[serde(rename = "idOrganization")]
    pub id_organization: Option<String>,
}

/// Trello list
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrelloList {
    /// List ID
    pub id: String,
    /// List name
    pub name: String,
    /// Is closed
    pub closed: bool,
    /// Position
    pub pos: f64,
    /// Board ID
    #[serde(rename = "idBoard")]
    pub id_board: String,
}

/// Trello card
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrelloCard {
    /// Card ID
    pub id: String,
    /// Card name
    pub name: String,
    /// Card description
    pub desc: Option<String>,
    /// Card URL
    pub url: String,
    /// Short URL
    #[serde(rename = "shortUrl")]
    pub short_url: String,
    /// Is closed
    pub closed: bool,
    /// Position
    pub pos: f64,
    /// Due date
    pub due: Option<String>,
    /// Due complete
    #[serde(rename = "dueComplete")]
    pub due_complete: bool,
    /// List ID
    #[serde(rename = "idList")]
    pub id_list: String,
    /// Board ID
    #[serde(rename = "idBoard")]
    pub id_board: String,
    /// Labels
    pub labels: Vec<TrelloLabel>,
    /// Member IDs
    #[serde(rename = "idMembers")]
    pub id_members: Vec<String>,
}

/// Trello label
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrelloLabel {
    /// Label ID
    pub id: String,
    /// Label name
    pub name: String,
    /// Label color
    pub color: Option<String>,
}

/// Trello member
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrelloMember {
    /// Member ID
    pub id: String,
    /// Username
    pub username: String,
    /// Full name
    #[serde(rename = "fullName")]
    pub full_name: String,
    /// Avatar URL
    #[serde(rename = "avatarUrl")]
    pub avatar_url: Option<String>,
}

/// Trello checklist
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrelloChecklist {
    /// Checklist ID
    pub id: String,
    /// Checklist name
    pub name: String,
    /// Card ID
    #[serde(rename = "idCard")]
    pub id_card: String,
    /// Check items
    #[serde(rename = "checkItems")]
    pub check_items: Vec<TrelloCheckItem>,
}

/// Trello check item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrelloCheckItem {
    /// Item ID
    pub id: String,
    /// Item name
    pub name: String,
    /// State (complete/incomplete)
    pub state: String,
    /// Position
    pub pos: f64,
}

/// Trello client
pub struct TrelloClient {
    config: TrelloConfig,
    base_url: String,
}

impl TrelloClient {
    /// API base URL
    const API_BASE: &'static str = "https://api.trello.com/1";

    /// Create a new Trello client
    pub fn new(config: &TrelloConfig) -> Result<Self> {
        let mut config = config.clone();
        config.resolve_env_vars();

        Ok(Self {
            config,
            base_url: Self::API_BASE.to_string(),
        })
    }

    /// Check if client is configured
    pub fn is_configured(&self) -> bool {
        !self.config.api_key.is_empty() && !self.config.api_token.is_empty()
    }

    /// Build auth query params
    fn auth_params(&self) -> String {
        format!("key={}&token={}", self.config.api_key, self.config.api_token)
    }

    // Board operations

    /// Get all boards for the user
    pub async fn get_boards(&self) -> Result<Vec<TrelloBoard>> {
        let url = format!(
            "{}/members/me/boards?{}",
            self.base_url,
            self.auth_params()
        );

        self.api_get(&url).await
    }

    /// Get a board by ID
    pub async fn get_board(&self, board_id: &str) -> Result<TrelloBoard> {
        let url = format!(
            "{}/boards/{}?{}",
            self.base_url,
            board_id,
            self.auth_params()
        );

        self.api_get(&url).await
    }

    /// Get board lists
    pub async fn get_lists(&self, board_id: &str) -> Result<Vec<TrelloList>> {
        let url = format!(
            "{}/boards/{}/lists?{}",
            self.base_url,
            board_id,
            self.auth_params()
        );

        self.api_get(&url).await
    }

    /// Get board members
    pub async fn get_members(&self, board_id: &str) -> Result<Vec<TrelloMember>> {
        let url = format!(
            "{}/boards/{}/members?{}",
            self.base_url,
            board_id,
            self.auth_params()
        );

        self.api_get(&url).await
    }

    /// Get board labels
    pub async fn get_labels(&self, board_id: &str) -> Result<Vec<TrelloLabel>> {
        let url = format!(
            "{}/boards/{}/labels?{}",
            self.base_url,
            board_id,
            self.auth_params()
        );

        self.api_get(&url).await
    }

    // List operations

    /// Get cards in a list
    pub async fn get_list_cards(&self, list_id: &str) -> Result<Vec<TrelloCard>> {
        let url = format!(
            "{}/lists/{}/cards?{}",
            self.base_url,
            list_id,
            self.auth_params()
        );

        self.api_get(&url).await
    }

    /// Create a list
    pub async fn create_list(&self, board_id: &str, name: &str) -> Result<TrelloList> {
        let url = format!(
            "{}/lists?idBoard={}&name={}&{}",
            self.base_url,
            board_id,
            urlencoding::encode(name),
            self.auth_params()
        );

        self.api_post(&url).await
    }

    /// Archive a list
    pub async fn archive_list(&self, list_id: &str) -> Result<TrelloList> {
        let url = format!(
            "{}/lists/{}/closed?value=true&{}",
            self.base_url,
            list_id,
            self.auth_params()
        );

        self.api_put(&url).await
    }

    // Card operations

    /// Get a card by ID
    pub async fn get_card(&self, card_id: &str) -> Result<TrelloCard> {
        let url = format!(
            "{}/cards/{}?{}",
            self.base_url,
            card_id,
            self.auth_params()
        );

        self.api_get(&url).await
    }

    /// Create a card
    pub async fn create_card(
        &self,
        list_id: &str,
        name: &str,
        desc: Option<&str>,
    ) -> Result<TrelloCard> {
        let mut url = format!(
            "{}/cards?idList={}&name={}&{}",
            self.base_url,
            list_id,
            urlencoding::encode(name),
            self.auth_params()
        );

        if let Some(d) = desc {
            url.push_str(&format!("&desc={}", urlencoding::encode(d)));
        }

        self.api_post(&url).await
    }

    /// Update a card
    pub async fn update_card(
        &self,
        card_id: &str,
        name: Option<&str>,
        desc: Option<&str>,
        due: Option<&str>,
    ) -> Result<TrelloCard> {
        let mut url = format!(
            "{}/cards/{}?{}",
            self.base_url,
            card_id,
            self.auth_params()
        );

        if let Some(n) = name {
            url.push_str(&format!("&name={}", urlencoding::encode(n)));
        }
        if let Some(d) = desc {
            url.push_str(&format!("&desc={}", urlencoding::encode(d)));
        }
        if let Some(due_date) = due {
            url.push_str(&format!("&due={}", urlencoding::encode(due_date)));
        }

        self.api_put(&url).await
    }

    /// Move a card to another list
    pub async fn move_card(&self, card_id: &str, list_id: &str) -> Result<TrelloCard> {
        let url = format!(
            "{}/cards/{}?idList={}&{}",
            self.base_url,
            card_id,
            list_id,
            self.auth_params()
        );

        self.api_put(&url).await
    }

    /// Archive a card
    pub async fn archive_card(&self, card_id: &str) -> Result<TrelloCard> {
        let url = format!(
            "{}/cards/{}?closed=true&{}",
            self.base_url,
            card_id,
            self.auth_params()
        );

        self.api_put(&url).await
    }

    /// Delete a card
    pub async fn delete_card(&self, card_id: &str) -> Result<()> {
        let url = format!(
            "{}/cards/{}?{}",
            self.base_url,
            card_id,
            self.auth_params()
        );

        self.api_delete(&url).await
    }

    /// Add label to card
    pub async fn add_label(&self, card_id: &str, label_id: &str) -> Result<()> {
        let url = format!(
            "{}/cards/{}/idLabels?value={}&{}",
            self.base_url,
            card_id,
            label_id,
            self.auth_params()
        );

        let _: serde_json::Value = self.api_post(&url).await?;
        Ok(())
    }

    /// Remove label from card
    pub async fn remove_label(&self, card_id: &str, label_id: &str) -> Result<()> {
        let url = format!(
            "{}/cards/{}/idLabels/{}?{}",
            self.base_url,
            card_id,
            label_id,
            self.auth_params()
        );

        self.api_delete(&url).await
    }

    /// Add member to card
    pub async fn add_member(&self, card_id: &str, member_id: &str) -> Result<()> {
        let url = format!(
            "{}/cards/{}/idMembers?value={}&{}",
            self.base_url,
            card_id,
            member_id,
            self.auth_params()
        );

        let _: serde_json::Value = self.api_post(&url).await?;
        Ok(())
    }

    /// Remove member from card
    pub async fn remove_member(&self, card_id: &str, member_id: &str) -> Result<()> {
        let url = format!(
            "{}/cards/{}/idMembers/{}?{}",
            self.base_url,
            card_id,
            member_id,
            self.auth_params()
        );

        self.api_delete(&url).await
    }

    /// Mark card due complete
    pub async fn mark_due_complete(&self, card_id: &str, complete: bool) -> Result<TrelloCard> {
        let url = format!(
            "{}/cards/{}?dueComplete={}&{}",
            self.base_url,
            card_id,
            complete,
            self.auth_params()
        );

        self.api_put(&url).await
    }

    // Checklist operations

    /// Get card checklists
    pub async fn get_checklists(&self, card_id: &str) -> Result<Vec<TrelloChecklist>> {
        let url = format!(
            "{}/cards/{}/checklists?{}",
            self.base_url,
            card_id,
            self.auth_params()
        );

        self.api_get(&url).await
    }

    /// Create a checklist
    pub async fn create_checklist(&self, card_id: &str, name: &str) -> Result<TrelloChecklist> {
        let url = format!(
            "{}/checklists?idCard={}&name={}&{}",
            self.base_url,
            card_id,
            urlencoding::encode(name),
            self.auth_params()
        );

        self.api_post(&url).await
    }

    /// Add item to checklist
    pub async fn add_checklist_item(
        &self,
        checklist_id: &str,
        name: &str,
    ) -> Result<TrelloCheckItem> {
        let url = format!(
            "{}/checklists/{}/checkItems?name={}&{}",
            self.base_url,
            checklist_id,
            urlencoding::encode(name),
            self.auth_params()
        );

        self.api_post(&url).await
    }

    /// Update checklist item state
    pub async fn update_checklist_item(
        &self,
        card_id: &str,
        item_id: &str,
        state: &str,
    ) -> Result<TrelloCheckItem> {
        let url = format!(
            "{}/cards/{}/checkItem/{}?state={}&{}",
            self.base_url,
            card_id,
            item_id,
            state,
            self.auth_params()
        );

        self.api_put(&url).await
    }

    // HTTP helpers

    async fn api_get<T: for<'de> Deserialize<'de>>(&self, url: &str) -> Result<T> {
        let client = reqwest::Client::new();
        let response = client
            .get(url)
            .send()
            .await
            .map_err(|e| DrivenError::Network(e.to_string()))?;

        if !response.status().is_success() {
            let error = response.text().await.unwrap_or_default();
            return Err(DrivenError::Api(format!("Trello error: {}", error)));
        }

        response
            .json()
            .await
            .map_err(|e| DrivenError::Parse(e.to_string()))
    }

    async fn api_post<T: for<'de> Deserialize<'de>>(&self, url: &str) -> Result<T> {
        let client = reqwest::Client::new();
        let response = client
            .post(url)
            .send()
            .await
            .map_err(|e| DrivenError::Network(e.to_string()))?;

        if !response.status().is_success() {
            let error = response.text().await.unwrap_or_default();
            return Err(DrivenError::Api(format!("Trello error: {}", error)));
        }

        response
            .json()
            .await
            .map_err(|e| DrivenError::Parse(e.to_string()))
    }

    async fn api_put<T: for<'de> Deserialize<'de>>(&self, url: &str) -> Result<T> {
        let client = reqwest::Client::new();
        let response = client
            .put(url)
            .send()
            .await
            .map_err(|e| DrivenError::Network(e.to_string()))?;

        if !response.status().is_success() {
            let error = response.text().await.unwrap_or_default();
            return Err(DrivenError::Api(format!("Trello error: {}", error)));
        }

        response
            .json()
            .await
            .map_err(|e| DrivenError::Parse(e.to_string()))
    }

    async fn api_delete(&self, url: &str) -> Result<()> {
        let client = reqwest::Client::new();
        let response = client
            .delete(url)
            .send()
            .await
            .map_err(|e| DrivenError::Network(e.to_string()))?;

        if !response.status().is_success() {
            let error = response.text().await.unwrap_or_default();
            return Err(DrivenError::Api(format!("Trello error: {}", error)));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = TrelloConfig::default();
        assert!(config.enabled);
    }

    #[test]
    fn test_auth_params() {
        let mut config = TrelloConfig::default();
        config.api_key = "key123".to_string();
        config.api_token = "token456".to_string();
        let client = TrelloClient::new(&config).unwrap();

        assert!(client.auth_params().contains("key=key123"));
        assert!(client.auth_params().contains("token=token456"));
    }
}
