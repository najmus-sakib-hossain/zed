//! # Bear Notes Integration
//!
//! Bear notes x-callback-url integration.
//!
//! ## Usage
//!
//! ```rust,ignore
//! use driven::integrations::bear::{BearClient, BearConfig};
//!
//! let config = BearConfig::from_file("~/.dx/config/bear.sr")?;
//! let client = BearClient::new(&config)?;
//!
//! // Create a note
//! client.create_note("My Note", "Content with #tags").await?;
//!
//! // Search notes
//! client.search("keyword").await?;
//! ```

use crate::error::{DrivenError, Result};
use serde::{Deserialize, Serialize};

/// Bear configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BearConfig {
    /// Whether Bear integration is enabled
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Bear API token (for certain operations)
    pub api_token: Option<String>,
    /// Default tags to add
    #[serde(default)]
    pub default_tags: Vec<String>,
}

fn default_true() -> bool {
    true
}

impl Default for BearConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            api_token: None,
            default_tags: Vec::new(),
        }
    }
}

impl BearConfig {
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
        if let Ok(token) = std::env::var("BEAR_API_TOKEN") {
            self.api_token = Some(token);
        }
    }
}

/// Bear note
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BearNote {
    /// Note identifier
    pub identifier: String,
    /// Note title
    pub title: String,
    /// Tags
    #[serde(default)]
    pub tags: Vec<String>,
    /// Is trashed
    pub is_trashed: bool,
    /// Is encrypted
    pub is_encrypted: bool,
    /// Creation date
    pub creation_date: Option<String>,
    /// Modification date
    pub modification_date: Option<String>,
}

/// X-Callback result
#[derive(Debug, Clone)]
pub struct CallbackResult {
    /// Generated URL
    pub url: String,
    /// Whether URL was opened
    pub opened: bool,
    /// Response identifier (if any)
    pub identifier: Option<String>,
}

/// Bear client
pub struct BearClient {
    config: BearConfig,
}

impl BearClient {
    /// URL scheme prefix
    const URL_SCHEME: &'static str = "bear://x-callback-url/";

    /// Create a new Bear client
    pub fn new(config: &BearConfig) -> Result<Self> {
        let mut config = config.clone();
        config.resolve_env_vars();

        Ok(Self { config })
    }

    /// Check if client is configured
    pub fn is_configured(&self) -> bool {
        self.config.enabled
    }

    /// Create a new note
    pub async fn create_note(&self, title: &str, text: &str) -> Result<CallbackResult> {
        let mut params = vec![
            format!("title={}", urlencoding::encode(title)),
            format!("text={}", urlencoding::encode(text)),
        ];

        // Add default tags
        if !self.config.default_tags.is_empty() {
            let tags = self.config.default_tags.join(",");
            params.push(format!("tags={}", urlencoding::encode(&tags)));
        }

        // Request identifier back
        params.push("open_note=no".to_string());
        params.push("new_window=no".to_string());

        let url = format!("{}create?{}", Self::URL_SCHEME, params.join("&"));
        self.open_url(&url).await
    }

    /// Create a note with options
    pub async fn create_note_with_options(
        &self,
        title: &str,
        text: &str,
        tags: Option<Vec<&str>>,
        pin: bool,
        open_note: bool,
    ) -> Result<CallbackResult> {
        let mut params = vec![
            format!("title={}", urlencoding::encode(title)),
            format!("text={}", urlencoding::encode(text)),
        ];

        if let Some(t) = tags {
            let tag_list = t.join(",");
            params.push(format!("tags={}", urlencoding::encode(&tag_list)));
        } else if !self.config.default_tags.is_empty() {
            let tags = self.config.default_tags.join(",");
            params.push(format!("tags={}", urlencoding::encode(&tags)));
        }

        if pin {
            params.push("pin=yes".to_string());
        }

        if open_note {
            params.push("open_note=yes".to_string());
        } else {
            params.push("open_note=no".to_string());
        }

        let url = format!("{}create?{}", Self::URL_SCHEME, params.join("&"));
        self.open_url(&url).await
    }

    /// Open a note by identifier
    pub async fn open_note(&self, id: &str) -> Result<CallbackResult> {
        let url = format!(
            "{}open-note?id={}&new_window=no",
            Self::URL_SCHEME,
            urlencoding::encode(id)
        );
        self.open_url(&url).await
    }

    /// Open a note by title
    pub async fn open_note_by_title(&self, title: &str) -> Result<CallbackResult> {
        let url = format!(
            "{}open-note?title={}&new_window=no",
            Self::URL_SCHEME,
            urlencoding::encode(title)
        );
        self.open_url(&url).await
    }

    /// Add text to a note
    pub async fn add_text(&self, id: &str, text: &str, mode: AddTextMode) -> Result<CallbackResult> {
        let mode_str = match mode {
            AddTextMode::Append => "append",
            AddTextMode::Prepend => "prepend",
            AddTextMode::PrependAll => "prepend_all",
            AddTextMode::Replace => "replace",
            AddTextMode::ReplaceAll => "replace_all",
        };

        let url = format!(
            "{}add-text?id={}&text={}&mode={}",
            Self::URL_SCHEME,
            urlencoding::encode(id),
            urlencoding::encode(text),
            mode_str
        );
        self.open_url(&url).await
    }

    /// Add text to today's note
    pub async fn add_to_today(&self, text: &str) -> Result<CallbackResult> {
        let today = chrono::Local::now().format("%Y-%m-%d").to_string();
        let title = format!("Daily Note {}", today);

        // Try to open existing, or create new
        let url = format!(
            "{}add-text?title={}&text={}&mode=append",
            Self::URL_SCHEME,
            urlencoding::encode(&title),
            urlencoding::encode(text)
        );
        self.open_url(&url).await
    }

    /// Search notes
    pub async fn search(&self, term: &str) -> Result<CallbackResult> {
        let url = format!(
            "{}search?term={}&show_window=yes",
            Self::URL_SCHEME,
            urlencoding::encode(term)
        );
        self.open_url(&url).await
    }

    /// Search by tag
    pub async fn search_by_tag(&self, tag: &str) -> Result<CallbackResult> {
        let url = format!(
            "{}open-tag?name={}&new_window=no",
            Self::URL_SCHEME,
            urlencoding::encode(tag)
        );
        self.open_url(&url).await
    }

    /// Grab URL (clip webpage to Bear)
    pub async fn grab_url(&self, url: &str, tags: Option<Vec<&str>>) -> Result<CallbackResult> {
        let mut params = vec![
            format!("url={}", urlencoding::encode(url)),
        ];

        if let Some(t) = tags {
            let tag_list = t.join(",");
            params.push(format!("tags={}", urlencoding::encode(&tag_list)));
        }

        let callback_url = format!("{}grab-url?{}", Self::URL_SCHEME, params.join("&"));
        self.open_url(&callback_url).await
    }

    /// Trash a note
    pub async fn trash_note(&self, id: &str) -> Result<CallbackResult> {
        let url = format!(
            "{}trash?id={}",
            Self::URL_SCHEME,
            urlencoding::encode(id)
        );
        self.open_url(&url).await
    }

    /// Archive a note
    pub async fn archive_note(&self, id: &str) -> Result<CallbackResult> {
        let url = format!(
            "{}archive?id={}",
            Self::URL_SCHEME,
            urlencoding::encode(id)
        );
        self.open_url(&url).await
    }

    /// Add tags to a note
    pub async fn add_tags(&self, id: &str, tags: Vec<&str>) -> Result<CallbackResult> {
        let url = format!(
            "{}add-text?id={}&tags={}&mode=append&text=",
            Self::URL_SCHEME,
            urlencoding::encode(id),
            urlencoding::encode(&tags.join(","))
        );
        self.open_url(&url).await
    }

    /// Change font
    pub async fn change_font(&self, font: BearFont) -> Result<CallbackResult> {
        let font_str = match font {
            BearFont::Avenir => "Avenir Next",
            BearFont::System => "System",
            BearFont::Courier => "Courier Prime",
            BearFont::Menlo => "Menlo",
            BearFont::Georgia => "Georgia",
            BearFont::SourceSansPro => "Source Sans Pro",
            BearFont::Charter => "Charter",
        };

        let url = format!(
            "{}change-font?font={}",
            Self::URL_SCHEME,
            urlencoding::encode(font_str)
        );
        self.open_url(&url).await
    }

    /// Change theme
    pub async fn change_theme(&self, theme: BearTheme) -> Result<CallbackResult> {
        let theme_str = match theme {
            BearTheme::RedGraphite => "Red Graphite",
            BearTheme::Charcoal => "Charcoal",
            BearTheme::Solarized => "Solarized Light",
            BearTheme::Toothpaste => "Toothpaste",
            BearTheme::Dracula => "Dracula",
            BearTheme::Panic => "Panic Mode",
            BearTheme::HighContrast => "High Contrast",
        };

        let url = format!(
            "{}change-theme?theme={}",
            Self::URL_SCHEME,
            urlencoding::encode(theme_str)
        );
        self.open_url(&url).await
    }

    /// Open Bear
    pub async fn open(&self) -> Result<CallbackResult> {
        self.open_url("bear://").await
    }

    /// Open today view
    pub async fn open_today(&self) -> Result<CallbackResult> {
        let url = format!("{}today", Self::URL_SCHEME);
        self.open_url(&url).await
    }

    /// Open untagged view
    pub async fn open_untagged(&self) -> Result<CallbackResult> {
        let url = format!("{}untagged", Self::URL_SCHEME);
        self.open_url(&url).await
    }

    /// Open locked notes
    pub async fn open_locked(&self) -> Result<CallbackResult> {
        let url = format!("{}locked", Self::URL_SCHEME);
        self.open_url(&url).await
    }

    /// Open trash
    pub async fn open_trash(&self) -> Result<CallbackResult> {
        let url = format!("{}trash", Self::URL_SCHEME);
        self.open_url(&url).await
    }

    /// Open a URL scheme URL
    async fn open_url(&self, url: &str) -> Result<CallbackResult> {
        // On macOS, use `open` command
        #[cfg(target_os = "macos")]
        {
            use tokio::process::Command;

            let status = Command::new("open")
                .arg(url)
                .status()
                .await
                .map_err(|e| DrivenError::Process(format!("Failed to open URL: {}", e)))?;

            Ok(CallbackResult {
                url: url.to_string(),
                opened: status.success(),
                identifier: None,
            })
        }

        // On other platforms, just return the URL
        #[cfg(not(target_os = "macos"))]
        {
            Ok(CallbackResult {
                url: url.to_string(),
                opened: false,
                identifier: None,
            })
        }
    }
}

/// Mode for adding text to a note
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AddTextMode {
    /// Append at the end of the note
    Append,
    /// Prepend at the beginning (after title)
    Prepend,
    /// Prepend at the very beginning
    PrependAll,
    /// Replace note body
    Replace,
    /// Replace entire note including title
    ReplaceAll,
}

/// Bear fonts
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BearFont {
    Avenir,
    System,
    Courier,
    Menlo,
    Georgia,
    SourceSansPro,
    Charter,
}

/// Bear themes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BearTheme {
    RedGraphite,
    Charcoal,
    Solarized,
    Toothpaste,
    Dracula,
    Panic,
    HighContrast,
}

/// Generate a Bear create URL
pub fn create_url(title: &str, text: &str) -> String {
    format!(
        "bear://x-callback-url/create?title={}&text={}",
        urlencoding::encode(title),
        urlencoding::encode(text)
    )
}

/// Generate a Bear search URL
pub fn search_url(term: &str) -> String {
    format!(
        "bear://x-callback-url/search?term={}",
        urlencoding::encode(term)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = BearConfig::default();
        assert!(config.enabled);
        assert!(config.default_tags.is_empty());
    }

    #[test]
    fn test_create_url() {
        let url = create_url("Test Note", "Some content");
        assert!(url.contains("bear://x-callback-url/create"));
        assert!(url.contains("title=Test%20Note"));
    }

    #[test]
    fn test_search_url() {
        let url = search_url("keyword");
        assert!(url.contains("bear://x-callback-url/search"));
        assert!(url.contains("term=keyword"));
    }
}
