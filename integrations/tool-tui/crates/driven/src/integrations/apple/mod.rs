//! # Apple Notes & Reminders Integration
//!
//! AppleScript-based integration for Apple Notes and Reminders.
//!
//! ## Usage
//!
//! ```rust,ignore
//! use driven::integrations::apple::{AppleNotes, AppleReminders, AppleConfig};
//!
//! let config = AppleConfig::from_file("~/.dx/config/apple.sr")?;
//!
//! // Notes
//! let notes = AppleNotes::new(&config)?;
//! notes.create_note("My Note", "Content here", Some("Personal")).await?;
//!
//! // Reminders
//! let reminders = AppleReminders::new(&config)?;
//! reminders.create_reminder("Buy groceries", Some("tomorrow at 5pm")).await?;
//! ```

use crate::error::{DrivenError, Result};
use serde::{Deserialize, Serialize};

/// Apple configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppleConfig {
    /// Whether Apple integration is enabled
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Default notes folder
    pub default_notes_folder: Option<String>,
    /// Default reminders list
    pub default_reminders_list: Option<String>,
}

fn default_true() -> bool {
    true
}

impl Default for AppleConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            default_notes_folder: None,
            default_reminders_list: None,
        }
    }
}

impl AppleConfig {
    /// Load from .sr config file
    pub fn from_file(path: impl AsRef<std::path::Path>) -> Result<Self> {
        let content = std::fs::read_to_string(path.as_ref())
            .map_err(|e| DrivenError::Io(e))?;
        Self::parse_sr(&content)
    }

    fn parse_sr(_content: &str) -> Result<Self> {
        Ok(Self::default())
    }
}

/// Apple Note
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppleNote {
    /// Note ID
    pub id: String,
    /// Note name/title
    pub name: String,
    /// Note body
    pub body: String,
    /// Folder name
    pub folder: String,
    /// Creation date
    pub creation_date: Option<String>,
    /// Modification date
    pub modification_date: Option<String>,
}

/// Apple Reminder
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppleReminder {
    /// Reminder ID
    pub id: String,
    /// Reminder name
    pub name: String,
    /// Reminder body/notes
    pub body: Option<String>,
    /// List name
    pub list: String,
    /// Due date
    pub due_date: Option<String>,
    /// Is completed
    pub completed: bool,
    /// Priority (0 = none, 1-4 = low, 5 = medium, 6-9 = high)
    pub priority: u8,
}

/// Apple Notes client
pub struct AppleNotes {
    config: AppleConfig,
}

impl AppleNotes {
    /// Create a new Apple Notes client
    pub fn new(config: &AppleConfig) -> Result<Self> {
        Ok(Self {
            config: config.clone(),
        })
    }

    /// Check if available (macOS only)
    pub fn is_available(&self) -> bool {
        cfg!(target_os = "macos") && self.config.enabled
    }

    /// Create a new note
    pub async fn create_note(
        &self,
        title: &str,
        body: &str,
        folder: Option<&str>,
    ) -> Result<AppleNote> {
        let folder_name = folder
            .map(String::from)
            .or_else(|| self.config.default_notes_folder.clone())
            .unwrap_or_else(|| "Notes".to_string());

        let script = format!(
            r#"
            tell application "Notes"
                set targetFolder to folder "{folder}"
                make new note at targetFolder with properties {{name:"{title}", body:"{body}"}}
                set newNote to result
                return id of newNote
            end tell
            "#,
            folder = escape_applescript(&folder_name),
            title = escape_applescript(title),
            body = escape_applescript(body)
        );

        let id = run_applescript(&script).await?;

        Ok(AppleNote {
            id: id.trim().to_string(),
            name: title.to_string(),
            body: body.to_string(),
            folder: folder_name,
            creation_date: None,
            modification_date: None,
        })
    }

    /// Get all notes
    pub async fn get_notes(&self, folder: Option<&str>) -> Result<Vec<AppleNote>> {
        let folder_clause = if let Some(f) = folder {
            format!("of folder \"{}\"", escape_applescript(f))
        } else {
            String::new()
        };

        let script = format!(
            r#"
            tell application "Notes"
                set noteList to {{}}
                repeat with n in notes {folder_clause}
                    set noteInfo to (id of n) & "|||" & (name of n) & "|||" & (container of n as string)
                    set end of noteList to noteInfo
                end repeat
                return noteList
            end tell
            "#,
            folder_clause = folder_clause
        );

        let output = run_applescript(&script).await?;
        let mut notes = Vec::new();

        for line in output.lines() {
            let parts: Vec<&str> = line.split("|||").collect();
            if parts.len() >= 3 {
                notes.push(AppleNote {
                    id: parts[0].to_string(),
                    name: parts[1].to_string(),
                    body: String::new(), // Would need separate call to get body
                    folder: parts[2].to_string(),
                    creation_date: None,
                    modification_date: None,
                });
            }
        }

        Ok(notes)
    }

    /// Get a note by ID
    pub async fn get_note(&self, note_id: &str) -> Result<AppleNote> {
        let script = format!(
            r#"
            tell application "Notes"
                set n to note id "{note_id}"
                return (name of n) & "|||" & (body of n) & "|||" & (container of n as string)
            end tell
            "#,
            note_id = escape_applescript(note_id)
        );

        let output = run_applescript(&script).await?;
        let parts: Vec<&str> = output.split("|||").collect();

        if parts.len() < 3 {
            return Err(DrivenError::Parse("Invalid note response".into()));
        }

        Ok(AppleNote {
            id: note_id.to_string(),
            name: parts[0].to_string(),
            body: parts[1].to_string(),
            folder: parts[2].to_string(),
            creation_date: None,
            modification_date: None,
        })
    }

    /// Update a note
    pub async fn update_note(&self, note_id: &str, body: &str) -> Result<()> {
        let script = format!(
            r#"
            tell application "Notes"
                set n to note id "{note_id}"
                set body of n to "{body}"
            end tell
            "#,
            note_id = escape_applescript(note_id),
            body = escape_applescript(body)
        );

        run_applescript(&script).await?;
        Ok(())
    }

    /// Delete a note
    pub async fn delete_note(&self, note_id: &str) -> Result<()> {
        let script = format!(
            r#"
            tell application "Notes"
                delete note id "{note_id}"
            end tell
            "#,
            note_id = escape_applescript(note_id)
        );

        run_applescript(&script).await?;
        Ok(())
    }

    /// Get all folders
    pub async fn get_folders(&self) -> Result<Vec<String>> {
        let script = r#"
            tell application "Notes"
                set folderNames to {}
                repeat with f in folders
                    set end of folderNames to name of f
                end repeat
                return folderNames
            end tell
        "#;

        let output = run_applescript(script).await?;
        Ok(output
            .split(", ")
            .map(|s| s.trim().to_string())
            .collect())
    }

    /// Search notes
    pub async fn search(&self, query: &str) -> Result<Vec<AppleNote>> {
        let script = format!(
            r#"
            tell application "Notes"
                set noteList to {{}}
                repeat with n in notes
                    if (name of n contains "{query}") or (body of n contains "{query}") then
                        set noteInfo to (id of n) & "|||" & (name of n) & "|||" & (container of n as string)
                        set end of noteList to noteInfo
                    end if
                end repeat
                return noteList
            end tell
            "#,
            query = escape_applescript(query)
        );

        let output = run_applescript(&script).await?;
        let mut notes = Vec::new();

        for line in output.lines() {
            let parts: Vec<&str> = line.split("|||").collect();
            if parts.len() >= 3 {
                notes.push(AppleNote {
                    id: parts[0].to_string(),
                    name: parts[1].to_string(),
                    body: String::new(),
                    folder: parts[2].to_string(),
                    creation_date: None,
                    modification_date: None,
                });
            }
        }

        Ok(notes)
    }
}

/// Apple Reminders client
pub struct AppleReminders {
    config: AppleConfig,
}

impl AppleReminders {
    /// Create a new Apple Reminders client
    pub fn new(config: &AppleConfig) -> Result<Self> {
        Ok(Self {
            config: config.clone(),
        })
    }

    /// Check if available (macOS only)
    pub fn is_available(&self) -> bool {
        cfg!(target_os = "macos") && self.config.enabled
    }

    /// Create a reminder
    pub async fn create_reminder(
        &self,
        name: &str,
        due_date: Option<&str>,
    ) -> Result<AppleReminder> {
        let list_name = self
            .config
            .default_reminders_list
            .clone()
            .unwrap_or_else(|| "Reminders".to_string());

        let due_clause = if let Some(due) = due_date {
            format!(", due date:(date \"{}\")", escape_applescript(due))
        } else {
            String::new()
        };

        let script = format!(
            r#"
            tell application "Reminders"
                set targetList to list "{list}"
                make new reminder at targetList with properties {{name:"{name}"{due_clause}}}
                set newReminder to result
                return id of newReminder
            end tell
            "#,
            list = escape_applescript(&list_name),
            name = escape_applescript(name),
            due_clause = due_clause
        );

        let id = run_applescript(&script).await?;

        Ok(AppleReminder {
            id: id.trim().to_string(),
            name: name.to_string(),
            body: None,
            list: list_name,
            due_date: due_date.map(String::from),
            completed: false,
            priority: 0,
        })
    }

    /// Create a reminder with full options
    pub async fn create_reminder_full(
        &self,
        name: &str,
        body: Option<&str>,
        list: Option<&str>,
        due_date: Option<&str>,
        priority: Option<u8>,
    ) -> Result<AppleReminder> {
        let list_name = list
            .map(String::from)
            .or_else(|| self.config.default_reminders_list.clone())
            .unwrap_or_else(|| "Reminders".to_string());

        let mut props = vec![format!("name:\"{}\"", escape_applescript(name))];

        if let Some(b) = body {
            props.push(format!("body:\"{}\"", escape_applescript(b)));
        }
        if let Some(due) = due_date {
            props.push(format!("due date:(date \"{}\")", escape_applescript(due)));
        }
        if let Some(p) = priority {
            props.push(format!("priority:{}", p));
        }

        let script = format!(
            r#"
            tell application "Reminders"
                set targetList to list "{list}"
                make new reminder at targetList with properties {{{props}}}
                set newReminder to result
                return id of newReminder
            end tell
            "#,
            list = escape_applescript(&list_name),
            props = props.join(", ")
        );

        let id = run_applescript(&script).await?;

        Ok(AppleReminder {
            id: id.trim().to_string(),
            name: name.to_string(),
            body: body.map(String::from),
            list: list_name,
            due_date: due_date.map(String::from),
            completed: false,
            priority: priority.unwrap_or(0),
        })
    }

    /// Get reminders from a list
    pub async fn get_reminders(&self, list: Option<&str>) -> Result<Vec<AppleReminder>> {
        let list_clause = if let Some(l) = list {
            format!("of list \"{}\"", escape_applescript(l))
        } else {
            String::new()
        };

        let script = format!(
            r#"
            tell application "Reminders"
                set reminderList to {{}}
                repeat with r in reminders {list_clause}
                    set reminderInfo to (id of r) & "|||" & (name of r) & "|||" & (completed of r) & "|||" & (container of r as string)
                    set end of reminderList to reminderInfo
                end repeat
                return reminderList
            end tell
            "#,
            list_clause = list_clause
        );

        let output = run_applescript(&script).await?;
        let mut reminders = Vec::new();

        for line in output.lines() {
            let parts: Vec<&str> = line.split("|||").collect();
            if parts.len() >= 4 {
                reminders.push(AppleReminder {
                    id: parts[0].to_string(),
                    name: parts[1].to_string(),
                    body: None,
                    list: parts[3].to_string(),
                    due_date: None,
                    completed: parts[2] == "true",
                    priority: 0,
                });
            }
        }

        Ok(reminders)
    }

    /// Complete a reminder
    pub async fn complete_reminder(&self, reminder_id: &str) -> Result<()> {
        let script = format!(
            r#"
            tell application "Reminders"
                set r to reminder id "{reminder_id}"
                set completed of r to true
            end tell
            "#,
            reminder_id = escape_applescript(reminder_id)
        );

        run_applescript(&script).await?;
        Ok(())
    }

    /// Uncomplete a reminder
    pub async fn uncomplete_reminder(&self, reminder_id: &str) -> Result<()> {
        let script = format!(
            r#"
            tell application "Reminders"
                set r to reminder id "{reminder_id}"
                set completed of r to false
            end tell
            "#,
            reminder_id = escape_applescript(reminder_id)
        );

        run_applescript(&script).await?;
        Ok(())
    }

    /// Delete a reminder
    pub async fn delete_reminder(&self, reminder_id: &str) -> Result<()> {
        let script = format!(
            r#"
            tell application "Reminders"
                delete reminder id "{reminder_id}"
            end tell
            "#,
            reminder_id = escape_applescript(reminder_id)
        );

        run_applescript(&script).await?;
        Ok(())
    }

    /// Get all lists
    pub async fn get_lists(&self) -> Result<Vec<String>> {
        let script = r#"
            tell application "Reminders"
                set listNames to {}
                repeat with l in lists
                    set end of listNames to name of l
                end repeat
                return listNames
            end tell
        "#;

        let output = run_applescript(script).await?;
        Ok(output
            .split(", ")
            .map(|s| s.trim().to_string())
            .collect())
    }

    /// Get today's reminders
    pub async fn get_today(&self) -> Result<Vec<AppleReminder>> {
        let script = r#"
            tell application "Reminders"
                set today to current date
                set todayReminders to {}
                repeat with r in reminders
                    if due date of r is not missing value then
                        if (due date of r) <= (today + 1 * days) then
                            set reminderInfo to (id of r) & "|||" & (name of r) & "|||" & (completed of r) & "|||" & (container of r as string)
                            set end of todayReminders to reminderInfo
                        end if
                    end if
                end repeat
                return todayReminders
            end tell
        "#;

        let output = run_applescript(script).await?;
        let mut reminders = Vec::new();

        for line in output.lines() {
            let parts: Vec<&str> = line.split("|||").collect();
            if parts.len() >= 4 {
                reminders.push(AppleReminder {
                    id: parts[0].to_string(),
                    name: parts[1].to_string(),
                    body: None,
                    list: parts[3].to_string(),
                    due_date: None,
                    completed: parts[2] == "true",
                    priority: 0,
                });
            }
        }

        Ok(reminders)
    }
}

/// Escape string for AppleScript
fn escape_applescript(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
}

/// Run an AppleScript
async fn run_applescript(script: &str) -> Result<String> {
    #[cfg(target_os = "macos")]
    {
        use tokio::process::Command;

        let output = Command::new("osascript")
            .arg("-e")
            .arg(script)
            .output()
            .await
            .map_err(|e| DrivenError::Process(format!("Failed to run AppleScript: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(DrivenError::Process(format!("AppleScript error: {}", stderr)));
        }

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    #[cfg(not(target_os = "macos"))]
    {
        Err(DrivenError::Unsupported(
            "AppleScript is only available on macOS".into(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = AppleConfig::default();
        assert!(config.enabled);
    }

    #[test]
    fn test_escape_applescript() {
        assert_eq!(escape_applescript("Hello \"World\""), "Hello \\\"World\\\"");
        assert_eq!(escape_applescript("Line1\nLine2"), "Line1\\nLine2");
    }
}
