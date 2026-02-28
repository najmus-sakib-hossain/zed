//! # Things 3 Integration
//!
//! Things 3 macOS URL scheme integration for task management.
//!
//! ## Usage
//!
//! ```rust,ignore
//! use driven::integrations::things::{ThingsClient, ThingsConfig};
//!
//! let config = ThingsConfig::from_file("~/.dx/config/things.sr")?;
//! let client = ThingsClient::new(&config)?;
//!
//! // Create a task
//! client.create_task("Buy groceries", Some("Milk, eggs, bread")).await?;
//!
//! // Create a project
//! client.create_project("Home Renovation", vec!["Paint walls", "Fix roof"]).await?;
//! ```

use crate::error::{DrivenError, Result};
use serde::{Deserialize, Serialize};

/// Things 3 configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThingsConfig {
    /// Whether Things integration is enabled
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Things auth token (for JSON API)
    pub auth_token: Option<String>,
    /// Default list/area
    pub default_list: Option<String>,
    /// Default tags
    #[serde(default)]
    pub default_tags: Vec<String>,
}

fn default_true() -> bool {
    true
}

impl Default for ThingsConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            auth_token: None,
            default_list: None,
            default_tags: Vec::new(),
        }
    }
}

impl ThingsConfig {
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
        if let Ok(token) = std::env::var("THINGS_AUTH_TOKEN") {
            self.auth_token = Some(token);
        }
    }
}

/// Things task
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThingsTask {
    /// Task title
    pub title: String,
    /// Task notes
    pub notes: Option<String>,
    /// When (today, tonight, tomorrow, anytime, someday, or date string)
    pub when: Option<String>,
    /// Deadline date
    pub deadline: Option<String>,
    /// Tags
    #[serde(default)]
    pub tags: Vec<String>,
    /// Checklist items
    #[serde(default)]
    pub checklist_items: Vec<String>,
    /// List/Area ID
    pub list_id: Option<String>,
    /// Heading (within a project)
    pub heading: Option<String>,
    /// Completed
    pub completed: bool,
    /// Canceled
    pub canceled: bool,
}

impl Default for ThingsTask {
    fn default() -> Self {
        Self {
            title: String::new(),
            notes: None,
            when: None,
            deadline: None,
            tags: Vec::new(),
            checklist_items: Vec::new(),
            list_id: None,
            heading: None,
            completed: false,
            canceled: false,
        }
    }
}

/// Things project
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThingsProject {
    /// Project title
    pub title: String,
    /// Project notes
    pub notes: Option<String>,
    /// When (today, tonight, tomorrow, anytime, someday, or date string)
    pub when: Option<String>,
    /// Deadline date
    pub deadline: Option<String>,
    /// Tags
    #[serde(default)]
    pub tags: Vec<String>,
    /// Area ID
    pub area_id: Option<String>,
    /// Headings
    #[serde(default)]
    pub headings: Vec<String>,
    /// Tasks within the project
    #[serde(default)]
    pub tasks: Vec<ThingsTask>,
}

impl Default for ThingsProject {
    fn default() -> Self {
        Self {
            title: String::new(),
            notes: None,
            when: None,
            deadline: None,
            tags: Vec::new(),
            area_id: None,
            headings: Vec::new(),
            tasks: Vec::new(),
        }
    }
}

/// URL scheme result
#[derive(Debug, Clone)]
pub struct UrlSchemeResult {
    /// Generated URL
    pub url: String,
    /// Whether URL was opened
    pub opened: bool,
}

/// Things 3 client
pub struct ThingsClient {
    config: ThingsConfig,
}

impl ThingsClient {
    /// URL scheme prefix
    const URL_SCHEME: &'static str = "things:///";

    /// Create a new Things client
    pub fn new(config: &ThingsConfig) -> Result<Self> {
        let mut config = config.clone();
        config.resolve_env_vars();

        Ok(Self { config })
    }

    /// Check if client is configured
    pub fn is_configured(&self) -> bool {
        self.config.enabled
    }

    /// Create a task
    pub async fn create_task(&self, title: &str, notes: Option<&str>) -> Result<UrlSchemeResult> {
        let mut task = ThingsTask::default();
        task.title = title.to_string();
        task.notes = notes.map(String::from);
        task.tags = self.config.default_tags.clone();
        task.list_id = self.config.default_list.clone();

        self.add_task(&task).await
    }

    /// Create a task with full options
    pub async fn add_task(&self, task: &ThingsTask) -> Result<UrlSchemeResult> {
        let mut params = vec![format!("title={}", urlencoding::encode(&task.title))];

        if let Some(ref notes) = task.notes {
            params.push(format!("notes={}", urlencoding::encode(notes)));
        }
        if let Some(ref when) = task.when {
            params.push(format!("when={}", urlencoding::encode(when)));
        }
        if let Some(ref deadline) = task.deadline {
            params.push(format!("deadline={}", urlencoding::encode(deadline)));
        }
        if !task.tags.is_empty() {
            params.push(format!("tags={}", urlencoding::encode(&task.tags.join(","))));
        }
        if !task.checklist_items.is_empty() {
            params.push(format!(
                "checklist-items={}",
                urlencoding::encode(&task.checklist_items.join("\n"))
            ));
        }
        if let Some(ref list) = task.list_id {
            params.push(format!("list-id={}", urlencoding::encode(list)));
        }
        if let Some(ref heading) = task.heading {
            params.push(format!("heading={}", urlencoding::encode(heading)));
        }
        if task.completed {
            params.push("completed=true".to_string());
        }
        if task.canceled {
            params.push("canceled=true".to_string());
        }

        let url = format!("{}add?{}", Self::URL_SCHEME, params.join("&"));
        self.open_url(&url).await
    }

    /// Create a project
    pub async fn create_project(&self, title: &str, tasks: Vec<&str>) -> Result<UrlSchemeResult> {
        let mut project = ThingsProject::default();
        project.title = title.to_string();
        project.tasks = tasks
            .into_iter()
            .map(|t| {
                let mut task = ThingsTask::default();
                task.title = t.to_string();
                task
            })
            .collect();

        self.add_project(&project).await
    }

    /// Create a project with full options
    pub async fn add_project(&self, project: &ThingsProject) -> Result<UrlSchemeResult> {
        let mut params = vec![format!("title={}", urlencoding::encode(&project.title))];

        if let Some(ref notes) = project.notes {
            params.push(format!("notes={}", urlencoding::encode(notes)));
        }
        if let Some(ref when) = project.when {
            params.push(format!("when={}", urlencoding::encode(when)));
        }
        if let Some(ref deadline) = project.deadline {
            params.push(format!("deadline={}", urlencoding::encode(deadline)));
        }
        if !project.tags.is_empty() {
            params.push(format!("tags={}", urlencoding::encode(&project.tags.join(","))));
        }
        if let Some(ref area) = project.area_id {
            params.push(format!("area-id={}", urlencoding::encode(area)));
        }
        if !project.headings.is_empty() {
            params.push(format!(
                "headings={}",
                urlencoding::encode(&project.headings.join("\n"))
            ));
        }

        // Add tasks as to-dos parameter
        if !project.tasks.is_empty() {
            let todos: Vec<String> = project
                .tasks
                .iter()
                .map(|t| t.title.clone())
                .collect();
            params.push(format!("to-dos={}", urlencoding::encode(&todos.join("\n"))));
        }

        let url = format!("{}add-project?{}", Self::URL_SCHEME, params.join("&"));
        self.open_url(&url).await
    }

    /// Add to today
    pub async fn add_to_today(&self, title: &str, notes: Option<&str>) -> Result<UrlSchemeResult> {
        let mut task = ThingsTask::default();
        task.title = title.to_string();
        task.notes = notes.map(String::from);
        task.when = Some("today".to_string());

        self.add_task(&task).await
    }

    /// Add to tonight
    pub async fn add_to_tonight(&self, title: &str, notes: Option<&str>) -> Result<UrlSchemeResult> {
        let mut task = ThingsTask::default();
        task.title = title.to_string();
        task.notes = notes.map(String::from);
        task.when = Some("tonight".to_string());

        self.add_task(&task).await
    }

    /// Add to someday
    pub async fn add_to_someday(&self, title: &str, notes: Option<&str>) -> Result<UrlSchemeResult> {
        let mut task = ThingsTask::default();
        task.title = title.to_string();
        task.notes = notes.map(String::from);
        task.when = Some("someday".to_string());

        self.add_task(&task).await
    }

    /// Show Today view
    pub async fn show_today(&self) -> Result<UrlSchemeResult> {
        let url = format!("{}show?id=today", Self::URL_SCHEME);
        self.open_url(&url).await
    }

    /// Show Inbox view
    pub async fn show_inbox(&self) -> Result<UrlSchemeResult> {
        let url = format!("{}show?id=inbox", Self::URL_SCHEME);
        self.open_url(&url).await
    }

    /// Show Upcoming view
    pub async fn show_upcoming(&self) -> Result<UrlSchemeResult> {
        let url = format!("{}show?id=upcoming", Self::URL_SCHEME);
        self.open_url(&url).await
    }

    /// Show Anytime view
    pub async fn show_anytime(&self) -> Result<UrlSchemeResult> {
        let url = format!("{}show?id=anytime", Self::URL_SCHEME);
        self.open_url(&url).await
    }

    /// Show Someday view
    pub async fn show_someday(&self) -> Result<UrlSchemeResult> {
        let url = format!("{}show?id=someday", Self::URL_SCHEME);
        self.open_url(&url).await
    }

    /// Show Logbook view
    pub async fn show_logbook(&self) -> Result<UrlSchemeResult> {
        let url = format!("{}show?id=logbook", Self::URL_SCHEME);
        self.open_url(&url).await
    }

    /// Show a specific item by ID
    pub async fn show_item(&self, item_id: &str) -> Result<UrlSchemeResult> {
        let url = format!("{}show?id={}", Self::URL_SCHEME, urlencoding::encode(item_id));
        self.open_url(&url).await
    }

    /// Search for items
    pub async fn search(&self, query: &str) -> Result<UrlSchemeResult> {
        let url = format!("{}search?query={}", Self::URL_SCHEME, urlencoding::encode(query));
        self.open_url(&url).await
    }

    /// Update an existing item (via JSON)
    pub async fn update_item(&self, item_id: &str, updates: serde_json::Value) -> Result<UrlSchemeResult> {
        if self.config.auth_token.is_none() {
            return Err(DrivenError::Config("Auth token required for updates".into()));
        }

        let json_data = serde_json::json!([{
            "type": "to-do",
            "id": item_id,
            "attributes": updates
        }]);

        let encoded = urlencoding::encode(&json_data.to_string());
        let url = format!(
            "{}json?auth-token={}&data={}",
            Self::URL_SCHEME,
            self.config.auth_token.as_ref().unwrap(),
            encoded
        );

        self.open_url(&url).await
    }

    /// Add multiple items via JSON API
    pub async fn add_json(&self, items: serde_json::Value) -> Result<UrlSchemeResult> {
        let encoded = urlencoding::encode(&items.to_string());
        let mut url = format!("{}json?data={}", Self::URL_SCHEME, encoded);

        if let Some(ref token) = self.config.auth_token {
            url.push_str(&format!("&auth-token={}", token));
        }

        self.open_url(&url).await
    }

    /// Open a URL scheme URL
    async fn open_url(&self, url: &str) -> Result<UrlSchemeResult> {
        // On macOS, use `open` command
        #[cfg(target_os = "macos")]
        {
            use tokio::process::Command;

            let status = Command::new("open")
                .arg(url)
                .status()
                .await
                .map_err(|e| DrivenError::Process(format!("Failed to open URL: {}", e)))?;

            Ok(UrlSchemeResult {
                url: url.to_string(),
                opened: status.success(),
            })
        }

        // On other platforms, just return the URL
        #[cfg(not(target_os = "macos"))]
        {
            Ok(UrlSchemeResult {
                url: url.to_string(),
                opened: false,
            })
        }
    }
}

/// Build a Things URL for quick add
pub fn quick_add_url(title: &str) -> String {
    format!("things:///add?title={}", urlencoding::encode(title))
}

/// Build a Things URL for showing a list
pub fn show_list_url(list: &str) -> String {
    format!("things:///show?id={}", urlencoding::encode(list))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = ThingsConfig::default();
        assert!(config.enabled);
        assert!(config.default_tags.is_empty());
    }

    #[test]
    fn test_quick_add_url() {
        let url = quick_add_url("Buy milk");
        assert!(url.contains("things:///add"));
        assert!(url.contains("title=Buy%20milk"));
    }

    #[test]
    fn test_show_list_url() {
        let url = show_list_url("today");
        assert_eq!(url, "things:///show?id=today");
    }
}
