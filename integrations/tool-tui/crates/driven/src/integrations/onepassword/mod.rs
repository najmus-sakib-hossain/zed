//! # 1Password Integration
//!
//! 1Password CLI wrapper for secrets management.
//!
//! ## Usage
//!
//! ```rust,ignore
//! use driven::integrations::onepassword::{OnePasswordClient, OnePasswordConfig};
//!
//! let config = OnePasswordConfig::from_file("~/.dx/config/onepassword.sr")?;
//! let client = OnePasswordClient::new(&config)?;
//!
//! // Get a secret
//! let secret = client.get_item("API Key", "Development").await?;
//!
//! // Get a specific field
//! let password = client.get_field("API Key", "password", "Development").await?;
//! ```

use crate::error::{DrivenError, Result};
use serde::{Deserialize, Serialize};

/// 1Password configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OnePasswordConfig {
    /// Whether 1Password integration is enabled
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Default vault
    pub default_vault: Option<String>,
    /// Service account token (for automation)
    pub service_account_token: Option<String>,
    /// Account shorthand
    pub account: Option<String>,
}

fn default_true() -> bool {
    true
}

impl Default for OnePasswordConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            default_vault: None,
            service_account_token: None,
            account: None,
        }
    }
}

impl OnePasswordConfig {
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
        if self.service_account_token.is_none() {
            self.service_account_token = std::env::var("OP_SERVICE_ACCOUNT_TOKEN").ok();
        }
    }
}

/// 1Password vault
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vault {
    /// Vault ID
    pub id: String,
    /// Vault name
    pub name: String,
    /// Content version
    pub content_version: Option<u64>,
}

/// 1Password item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Item {
    /// Item ID
    pub id: String,
    /// Item title
    pub title: String,
    /// Vault ID
    pub vault_id: String,
    /// Category
    pub category: ItemCategory,
    /// Tags
    #[serde(default)]
    pub tags: Vec<String>,
    /// Fields
    #[serde(default)]
    pub fields: Vec<Field>,
    /// URLs
    #[serde(default)]
    pub urls: Vec<ItemUrl>,
    /// Created at
    pub created_at: Option<String>,
    /// Updated at
    pub updated_at: Option<String>,
}

/// Item category
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ItemCategory {
    Login,
    Password,
    SecureNote,
    CreditCard,
    Identity,
    Document,
    ApiCredential,
    Database,
    Server,
    SshKey,
    SoftwareLicense,
    Email,
    Custom,
    #[serde(other)]
    Unknown,
}

/// Item field
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Field {
    /// Field ID
    pub id: String,
    /// Field label
    pub label: String,
    /// Field value (may be concealed)
    pub value: Option<String>,
    /// Field type
    #[serde(rename = "type")]
    pub field_type: FieldType,
    /// Purpose (username, password, etc.)
    pub purpose: Option<String>,
    /// Section
    pub section: Option<FieldSection>,
}

/// Field type
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum FieldType {
    String,
    Concealed,
    Email,
    Url,
    Date,
    MonthYear,
    Phone,
    Otp,
    File,
    #[serde(other)]
    Unknown,
}

/// Field section
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldSection {
    pub id: String,
    pub label: Option<String>,
}

/// Item URL
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemUrl {
    pub label: Option<String>,
    pub primary: bool,
    pub href: String,
}

/// 1Password client
pub struct OnePasswordClient {
    config: OnePasswordConfig,
}

impl OnePasswordClient {
    /// Create a new 1Password client
    pub fn new(config: &OnePasswordConfig) -> Result<Self> {
        let mut config = config.clone();
        config.resolve_env_vars();

        Ok(Self { config })
    }

    /// Check if CLI is available
    pub async fn is_available(&self) -> bool {
        use tokio::process::Command;

        Command::new("op")
            .arg("--version")
            .output()
            .await
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    /// Check if signed in
    pub async fn is_signed_in(&self) -> bool {
        self.run_op(&["account", "get"]).await.is_ok()
    }

    // Vault operations

    /// List vaults
    pub async fn list_vaults(&self) -> Result<Vec<Vault>> {
        let output = self.run_op(&["vault", "list", "--format=json"]).await?;
        serde_json::from_str(&output)
            .map_err(|e| DrivenError::Parse(e.to_string()))
    }

    /// Get vault by name or ID
    pub async fn get_vault(&self, name_or_id: &str) -> Result<Vault> {
        let output = self.run_op(&["vault", "get", name_or_id, "--format=json"]).await?;
        serde_json::from_str(&output)
            .map_err(|e| DrivenError::Parse(e.to_string()))
    }

    // Item operations

    /// List items in a vault
    pub async fn list_items(&self, vault: Option<&str>) -> Result<Vec<Item>> {
        let vault_name = vault
            .map(String::from)
            .or_else(|| self.config.default_vault.clone());

        let mut args = vec!["item", "list", "--format=json"];
        if let Some(ref v) = vault_name {
            args.push("--vault");
            args.push(v);
        }

        let output = self.run_op(&args).await?;
        serde_json::from_str(&output)
            .map_err(|e| DrivenError::Parse(e.to_string()))
    }

    /// Get an item by title or ID
    pub async fn get_item(&self, name_or_id: &str, vault: Option<&str>) -> Result<Item> {
        let vault_name = vault
            .map(String::from)
            .or_else(|| self.config.default_vault.clone());

        let mut args = vec!["item", "get", name_or_id, "--format=json"];
        if let Some(ref v) = vault_name {
            args.push("--vault");
            args.push(v);
        }

        let output = self.run_op(&args).await?;
        serde_json::from_str(&output)
            .map_err(|e| DrivenError::Parse(e.to_string()))
    }

    /// Get a specific field value
    pub async fn get_field(&self, item: &str, field: &str, vault: Option<&str>) -> Result<String> {
        let vault_name = vault
            .map(String::from)
            .or_else(|| self.config.default_vault.clone());

        let field_ref = format!("op://{}/{}/{}", 
            vault_name.as_deref().unwrap_or("Private"),
            item,
            field
        );

        self.read_reference(&field_ref).await
    }

    /// Get password for an item
    pub async fn get_password(&self, item: &str, vault: Option<&str>) -> Result<String> {
        self.get_field(item, "password", vault).await
    }

    /// Get username for an item
    pub async fn get_username(&self, item: &str, vault: Option<&str>) -> Result<String> {
        self.get_field(item, "username", vault).await
    }

    /// Get OTP code for an item
    pub async fn get_otp(&self, item: &str, vault: Option<&str>) -> Result<String> {
        let vault_name = vault
            .map(String::from)
            .or_else(|| self.config.default_vault.clone());

        let mut args = vec!["item", "get", item, "--otp"];
        if let Some(ref v) = vault_name {
            args.push("--vault");
            args.push(v);
        }

        self.run_op(&args).await
    }

    /// Read a secret reference (op://vault/item/field)
    pub async fn read_reference(&self, reference: &str) -> Result<String> {
        self.run_op(&["read", reference]).await
    }

    /// Inject secrets into a template
    pub async fn inject(&self, template: &str) -> Result<String> {
        use tokio::process::Command;

        let mut cmd = Command::new("op");
        cmd.args(["inject"]);

        if let Some(ref token) = self.config.service_account_token {
            cmd.env("OP_SERVICE_ACCOUNT_TOKEN", token);
        }

        let child = cmd
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| DrivenError::Process(format!("Failed to spawn op: {}", e)))?;

        use tokio::io::AsyncWriteExt;
        
        let mut stdin = child.stdin.ok_or_else(|| {
            DrivenError::Process("Failed to open stdin".into())
        })?;
        
        stdin.write_all(template.as_bytes()).await
            .map_err(|e| DrivenError::Io(e))?;
        drop(stdin);

        let output = child.wait_with_output().await
            .map_err(|e| DrivenError::Io(e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(DrivenError::Process(format!("op inject failed: {}", stderr)));
        }

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    /// Create a new item
    pub async fn create_item(
        &self,
        category: ItemCategory,
        title: &str,
        vault: Option<&str>,
        fields: Vec<(&str, &str)>,
    ) -> Result<Item> {
        let vault_name = vault
            .map(String::from)
            .or_else(|| self.config.default_vault.clone());

        let category_str = match category {
            ItemCategory::Login => "login",
            ItemCategory::Password => "password",
            ItemCategory::SecureNote => "secure note",
            ItemCategory::ApiCredential => "api credential",
            ItemCategory::Database => "database",
            ItemCategory::Server => "server",
            _ => "login",
        };

        let mut args = vec![
            "item".to_string(),
            "create".to_string(),
            "--category".to_string(),
            category_str.to_string(),
            "--title".to_string(),
            title.to_string(),
            "--format=json".to_string(),
        ];

        if let Some(ref v) = vault_name {
            args.push("--vault".to_string());
            args.push(v.clone());
        }

        for (label, value) in fields {
            args.push(format!("{}={}", label, value));
        }

        let args_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
        let output = self.run_op(&args_refs).await?;
        
        serde_json::from_str(&output)
            .map_err(|e| DrivenError::Parse(e.to_string()))
    }

    /// Delete an item
    pub async fn delete_item(&self, item: &str, vault: Option<&str>) -> Result<()> {
        let vault_name = vault
            .map(String::from)
            .or_else(|| self.config.default_vault.clone());

        let mut args = vec!["item", "delete", item];
        if let Some(ref v) = vault_name {
            args.push("--vault");
            args.push(v);
        }

        self.run_op(&args).await?;
        Ok(())
    }

    /// Generate a password
    pub async fn generate_password(&self, length: u8, symbols: bool) -> Result<String> {
        let mut recipe = format!("letters,digits,{}", length);
        if symbols {
            recipe.push_str(",symbols");
        }

        self.run_op(&["item", "get", "--generate-password", &recipe]).await
    }

    /// Search items
    pub async fn search(&self, query: &str) -> Result<Vec<Item>> {
        let output = self.run_op(&[
            "item", "list", 
            "--format=json",
            "--tags", query,
        ]).await;

        if let Ok(o) = output {
            let items: Vec<Item> = serde_json::from_str(&o)
                .unwrap_or_default();
            return Ok(items);
        }

        // Fallback: list all and filter
        let all_items = self.list_items(None).await?;
        let query_lower = query.to_lowercase();
        
        Ok(all_items
            .into_iter()
            .filter(|i| {
                i.title.to_lowercase().contains(&query_lower)
                    || i.tags.iter().any(|t| t.to_lowercase().contains(&query_lower))
            })
            .collect())
    }

    async fn run_op(&self, args: &[&str]) -> Result<String> {
        use tokio::process::Command;

        let mut cmd = Command::new("op");
        cmd.args(args);

        // Add service account token if available
        if let Some(ref token) = self.config.service_account_token {
            cmd.env("OP_SERVICE_ACCOUNT_TOKEN", token);
        }

        // Add account if specified
        if let Some(ref account) = self.config.account {
            cmd.args(["--account", account]);
        }

        let output = cmd
            .output()
            .await
            .map_err(|e| DrivenError::Process(format!("Failed to run op: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(DrivenError::Process(format!("op command failed: {}", stderr)));
        }

        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }
}

/// Build a secret reference
pub fn secret_ref(vault: &str, item: &str, field: &str) -> String {
    format!("op://{}/{}/{}", vault, item, field)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = OnePasswordConfig::default();
        assert!(config.enabled);
    }

    #[test]
    fn test_secret_ref() {
        let ref_str = secret_ref("Private", "GitHub", "token");
        assert_eq!(ref_str, "op://Private/GitHub/token");
    }
}
