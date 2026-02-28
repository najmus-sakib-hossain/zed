//! # Session Transcript Export
//!
//! Exports sessions to various formats:
//! - JSON: Full structured export
//! - Markdown: Human-readable conversation format
//! - HTML: Styled conversation for sharing

use super::{MessageRole, Session, SessionError};

/// Export format options
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportFormat {
    /// Full JSON export
    Json,
    /// Markdown format
    Markdown,
    /// HTML format
    Html,
}

impl ExportFormat {
    /// Parse from string
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "json" => Some(Self::Json),
            "markdown" | "md" => Some(Self::Markdown),
            "html" => Some(Self::Html),
            _ => None,
        }
    }

    /// Get file extension for this format
    pub fn extension(&self) -> &str {
        match self {
            Self::Json => "json",
            Self::Markdown => "md",
            Self::Html => "html",
        }
    }
}

/// Export a session to the specified format
pub fn export_session(session: &Session, format: ExportFormat) -> Result<String, SessionError> {
    match format {
        ExportFormat::Json => export_json(session),
        ExportFormat::Markdown => export_markdown(session),
        ExportFormat::Html => export_html(session),
    }
}

/// Export session as JSON
fn export_json(session: &Session) -> Result<String, SessionError> {
    serde_json::to_string_pretty(session)
        .map_err(|e| SessionError::ExportError(format!("JSON serialization failed: {}", e)))
}

/// Export session as Markdown
fn export_markdown(session: &Session) -> Result<String, SessionError> {
    let mut md = String::new();

    // Header
    let title = session.title.as_deref().unwrap_or("Untitled Session");
    md.push_str(&format!("# {}\n\n", title));

    // Metadata
    md.push_str(&format!("- **Session**: `{}`\n", session.key));
    md.push_str(&format!("- **Agent**: {}\n", session.agent_id));
    if let Some(ref model) = session.model {
        md.push_str(&format!("- **Model**: {}\n", model));
    }
    md.push_str(&format!(
        "- **Created**: {}\n",
        session.created_at.format("%Y-%m-%d %H:%M:%S UTC")
    ));
    md.push_str(&format!(
        "- **Updated**: {}\n",
        session.updated_at.format("%Y-%m-%d %H:%M:%S UTC")
    ));
    md.push_str(&format!("- **Messages**: {}\n", session.messages.len()));
    if !session.tags.is_empty() {
        md.push_str(&format!("- **Tags**: {}\n", session.tags.join(", ")));
    }
    md.push_str("\n---\n\n");

    // Messages
    for msg in &session.messages {
        let role_label = match msg.role {
            MessageRole::User => "**User**",
            MessageRole::Assistant => "**Assistant**",
            MessageRole::System => "**System**",
            MessageRole::Tool => "**Tool**",
        };

        let timestamp = msg.timestamp.format("%H:%M:%S");

        md.push_str(&format!("### {} ({})\n\n", role_label, timestamp));

        if let Some(ref tool_name) = msg.tool_name {
            md.push_str(&format!("> Tool: `{}`\n\n", tool_name));
        }

        // Format content - indent code blocks
        md.push_str(&msg.content);
        md.push_str("\n\n");
    }

    // Footer
    md.push_str("---\n\n");
    md.push_str(&format!(
        "*Exported from DX CLI on {}*\n",
        chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
    ));

    Ok(md)
}

/// Export session as HTML
fn export_html(session: &Session) -> Result<String, SessionError> {
    let mut html = String::new();

    let title = session.title.as_deref().unwrap_or("Untitled Session");

    // HTML header
    html.push_str("<!DOCTYPE html>\n<html lang=\"en\">\n<head>\n");
    html.push_str("  <meta charset=\"UTF-8\">\n");
    html.push_str("  <meta name=\"viewport\" content=\"width=device-width, initial-scale=1.0\">\n");
    html.push_str(&format!("  <title>{}</title>\n", escape_html(title)));
    html.push_str("  <style>\n");
    html.push_str(HTML_STYLES);
    html.push_str("  </style>\n");
    html.push_str("</head>\n<body>\n");

    // Header
    html.push_str(&format!("  <h1>{}</h1>\n", escape_html(title)));
    html.push_str("  <div class=\"metadata\">\n");
    html.push_str(&format!("    <span>Agent: {}</span>\n", escape_html(&session.agent_id)));
    if let Some(ref model) = session.model {
        html.push_str(&format!("    <span>Model: {}</span>\n", escape_html(model)));
    }
    html.push_str(&format!(
        "    <span>Created: {}</span>\n",
        session.created_at.format("%Y-%m-%d %H:%M:%S UTC")
    ));
    html.push_str(&format!("    <span>Messages: {}</span>\n", session.messages.len()));
    html.push_str("  </div>\n\n");

    // Messages
    html.push_str("  <div class=\"conversation\">\n");
    for msg in &session.messages {
        let role_class = match msg.role {
            MessageRole::User => "user",
            MessageRole::Assistant => "assistant",
            MessageRole::System => "system",
            MessageRole::Tool => "tool",
        };
        let role_label = match msg.role {
            MessageRole::User => "User",
            MessageRole::Assistant => "Assistant",
            MessageRole::System => "System",
            MessageRole::Tool => "Tool",
        };

        html.push_str(&format!("    <div class=\"message {}\">\n", role_class));
        html.push_str(&format!("      <div class=\"role\">{}</div>\n", role_label));
        html.push_str(&format!(
            "      <div class=\"timestamp\">{}</div>\n",
            msg.timestamp.format("%H:%M:%S")
        ));

        if let Some(ref tool_name) = msg.tool_name {
            html.push_str(&format!(
                "      <div class=\"tool-name\">Tool: {}</div>\n",
                escape_html(tool_name)
            ));
        }

        html.push_str(&format!(
            "      <div class=\"content\">{}</div>\n",
            escape_html(&msg.content)
        ));
        html.push_str("    </div>\n");
    }
    html.push_str("  </div>\n\n");

    // Footer
    html.push_str(&format!(
        "  <footer>Exported from DX CLI on {}</footer>\n",
        chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
    ));
    html.push_str("</body>\n</html>\n");

    Ok(html)
}

/// Escape HTML special characters
fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#x27;")
}

/// CSS styles for HTML export
const HTML_STYLES: &str = r#"
    body {
      font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, 'Helvetica Neue', Arial, sans-serif;
      max-width: 800px;
      margin: 0 auto;
      padding: 2rem;
      background: #0d1117;
      color: #c9d1d9;
    }
    h1 {
      color: #58a6ff;
      border-bottom: 1px solid #21262d;
      padding-bottom: 0.5rem;
    }
    .metadata {
      display: flex;
      gap: 1rem;
      flex-wrap: wrap;
      color: #8b949e;
      font-size: 0.875rem;
      margin-bottom: 2rem;
    }
    .conversation {
      display: flex;
      flex-direction: column;
      gap: 1rem;
    }
    .message {
      padding: 1rem;
      border-radius: 8px;
      border: 1px solid #21262d;
    }
    .message.user {
      background: #161b22;
      border-left: 3px solid #58a6ff;
    }
    .message.assistant {
      background: #0d1117;
      border-left: 3px solid #3fb950;
    }
    .message.system {
      background: #161b22;
      border-left: 3px solid #d29922;
      font-style: italic;
    }
    .message.tool {
      background: #161b22;
      border-left: 3px solid #bc8cff;
      font-family: monospace;
    }
    .role {
      font-weight: 600;
      font-size: 0.875rem;
      margin-bottom: 0.25rem;
    }
    .user .role { color: #58a6ff; }
    .assistant .role { color: #3fb950; }
    .system .role { color: #d29922; }
    .tool .role { color: #bc8cff; }
    .timestamp {
      color: #484f58;
      font-size: 0.75rem;
      margin-bottom: 0.5rem;
    }
    .tool-name {
      color: #bc8cff;
      font-size: 0.8rem;
      margin-bottom: 0.5rem;
    }
    .content {
      white-space: pre-wrap;
      word-wrap: break-word;
      line-height: 1.5;
    }
    footer {
      margin-top: 2rem;
      padding-top: 1rem;
      border-top: 1px solid #21262d;
      color: #484f58;
      font-size: 0.75rem;
    }
"#;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::Session;

    #[test]
    fn test_export_json() {
        let mut session = Session::new("agent-1");
        session.add_message(MessageRole::User, "Hello");
        session.add_message(MessageRole::Assistant, "Hi there!");

        let json = export_session(&session, ExportFormat::Json).unwrap();
        assert!(json.contains("agent-1"));
        assert!(json.contains("Hello"));
        assert!(json.contains("Hi there!"));

        // Verify it parses back
        let _: serde_json::Value = serde_json::from_str(&json).unwrap();
    }

    #[test]
    fn test_export_markdown() {
        let mut session = Session::new("agent-1");
        session.title = Some("Test Session".to_string());
        session.add_message(MessageRole::User, "What is Rust?");
        session.add_message(MessageRole::Assistant, "Rust is a systems programming language.");

        let md = export_session(&session, ExportFormat::Markdown).unwrap();
        assert!(md.contains("# Test Session"));
        assert!(md.contains("**User**"));
        assert!(md.contains("**Assistant**"));
        assert!(md.contains("What is Rust?"));
        assert!(md.contains("Rust is a systems programming language."));
    }

    #[test]
    fn test_export_html() {
        let mut session = Session::new("agent-1");
        session.title = Some("Test Session".to_string());
        session.add_message(MessageRole::User, "Hello <world>");
        session.add_message(MessageRole::Assistant, "Hi & welcome!");

        let html = export_session(&session, ExportFormat::Html).unwrap();
        assert!(html.contains("<!DOCTYPE html>"));
        assert!(html.contains("Test Session"));
        assert!(html.contains("Hello &lt;world&gt;")); // Escaped
        assert!(html.contains("Hi &amp; welcome!")); // Escaped
    }

    #[test]
    fn test_export_format_parse() {
        assert_eq!(ExportFormat::from_str("json"), Some(ExportFormat::Json));
        assert_eq!(ExportFormat::from_str("markdown"), Some(ExportFormat::Markdown));
        assert_eq!(ExportFormat::from_str("md"), Some(ExportFormat::Markdown));
        assert_eq!(ExportFormat::from_str("html"), Some(ExportFormat::Html));
        assert_eq!(ExportFormat::from_str("xml"), None);
    }

    #[test]
    fn test_export_format_extension() {
        assert_eq!(ExportFormat::Json.extension(), "json");
        assert_eq!(ExportFormat::Markdown.extension(), "md");
        assert_eq!(ExportFormat::Html.extension(), "html");
    }

    #[test]
    fn test_escape_html() {
        assert_eq!(escape_html("<script>"), "&lt;script&gt;");
        assert_eq!(escape_html("a & b"), "a &amp; b");
        assert_eq!(escape_html("\"hello\""), "&quot;hello&quot;");
    }
}
