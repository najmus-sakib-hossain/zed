//! DX Forge Language Server
//!
//! LSP server for DX component detection, auto-completion, and semantic analysis

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

use crate::api::reactivity::trigger_realtime_event;
use crate::patterns::PatternDetector;

/// LSP Server state
pub struct LspServer {
    /// Pattern detector for DX components
    pattern_detector: PatternDetector,

    /// Document store (uri -> content)
    documents: Arc<RwLock<std::collections::HashMap<String, String>>>,
}

impl LspServer {
    pub fn new() -> Result<Self> {
        Ok(Self {
            pattern_detector: PatternDetector::new()?,
            documents: Arc::new(RwLock::new(std::collections::HashMap::new())),
        })
    }

    /// Handle document open
    pub async fn did_open(&self, uri: String, text: String) -> Result<()> {
        info!("📄 Document opened: {}", uri);
        self.documents.write().await.insert(uri.clone(), text.clone());

        // Detect DX patterns
        let matches = self.pattern_detector.detect_in_file(std::path::Path::new(&uri), &text)?;

        if !matches.is_empty() {
            info!("🔍 Found {} DX patterns in {}", matches.len(), uri);
        }

        Ok(())
    }

    /// Handle document change
    pub async fn did_change(&self, uri: String, text: String) -> Result<()> {
        info!("✏️  Document changed: {}", uri);
        self.documents.write().await.insert(uri.clone(), text.clone());

        // Trigger realtime event in Forge
        let path = std::path::PathBuf::from(uri.trim_start_matches("file://"));
        if let Err(e) = trigger_realtime_event(path, text) {
            tracing::error!("Failed to trigger realtime event: {}", e);
        }

        Ok(())
    }

    /// Handle document close
    pub async fn did_close(&self, uri: String) -> Result<()> {
        info!("📪 Document closed: {}", uri);
        self.documents.write().await.remove(&uri);
        Ok(())
    }

    /// Provide completions for DX components
    pub async fn completion(
        &self,
        uri: String,
        line: u32,
        character: u32,
    ) -> Result<Vec<CompletionItem>> {
        info!("💡 Completion requested at {}:{}:{}", uri, line, character);

        // Get document text
        let documents = self.documents.read().await;
        let text = documents.get(&uri).map(|s| s.as_str()).unwrap_or("");

        // Get line text
        let lines: Vec<&str> = text.lines().collect();
        if line as usize >= lines.len() {
            return Ok(Vec::new());
        }

        let line_text = lines[line as usize];
        let prefix = &line_text[..character.min(line_text.len() as u32) as usize];

        // Provide DX completions if typing "dx"
        if prefix.ends_with("dx") || prefix.ends_with("<dx") {
            Ok(self.get_dx_completions())
        } else {
            Ok(Vec::new())
        }
    }

    /// Get DX component completions
    fn get_dx_completions(&self) -> Vec<CompletionItem> {
        vec![
            // dx-ui components
            CompletionItem {
                label: "dxButton".to_string(),
                kind: CompletionItemKind::Component,
                detail: Some("DX UI Button component".to_string()),
                documentation: Some("Auto-injected button component from dx-ui".to_string()),
            },
            CompletionItem {
                label: "dxInput".to_string(),
                kind: CompletionItemKind::Component,
                detail: Some("DX UI Input component".to_string()),
                documentation: Some("Auto-injected input component from dx-ui".to_string()),
            },
            CompletionItem {
                label: "dxCard".to_string(),
                kind: CompletionItemKind::Component,
                detail: Some("DX UI Card component".to_string()),
                documentation: Some("Auto-injected card component from dx-ui".to_string()),
            },
            // dx-icons
            CompletionItem {
                label: "dxiHome".to_string(),
                kind: CompletionItemKind::Component,
                detail: Some("DX Icon: Home".to_string()),
                documentation: Some("Auto-injected home icon from dx-icons".to_string()),
            },
            CompletionItem {
                label: "dxiUser".to_string(),
                kind: CompletionItemKind::Component,
                detail: Some("DX Icon: User".to_string()),
                documentation: Some("Auto-injected user icon from dx-icons".to_string()),
            },
        ]
    }

    /// Provide hover information
    pub async fn hover(
        &self,
        uri: String,
        line: u32,
        _character: u32,
    ) -> Result<Option<HoverInfo>> {
        let documents = self.documents.read().await;
        let text = documents.get(&uri).map(|s| s.as_str()).unwrap_or("");

        // Detect DX patterns at position
        let matches = self.pattern_detector.detect_in_file(std::path::Path::new(&uri), text)?;

        // Find pattern at cursor position
        for m in matches {
            if m.line == (line + 1) as usize {
                let info = HoverInfo {
                    contents: format!(
                        "**{}** from {}\n\nAuto-injected DX component",
                        m.component_name,
                        m.tool.tool_name()
                    ),
                };
                return Ok(Some(info));
            }
        }

        Ok(None)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionItem {
    pub label: String,
    pub kind: CompletionItemKind,
    pub detail: Option<String>,
    pub documentation: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CompletionItemKind {
    Component,
    Function,
    Variable,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HoverInfo {
    pub contents: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_lsp_server_creation() {
        let server = LspServer::new().unwrap();
        assert!(server.documents.read().await.is_empty());
    }

    #[tokio::test]
    async fn test_did_open() {
        let server = LspServer::new().unwrap();
        let content = "<dxButton>Click me</dxButton>";

        server.did_open("test.tsx".to_string(), content.to_string()).await.unwrap();

        let docs = server.documents.read().await;
        assert_eq!(docs.get("test.tsx"), Some(&content.to_string()));
    }

    #[tokio::test]
    async fn test_completion() {
        let server = LspServer::new().unwrap();
        // Simulate a client opening a document and requesting completions
        let content = "dx";
        server.did_open("test.tsx".to_string(), content.to_string()).await.unwrap();

        let completions = server.completion("test.tsx".to_string(), 0, 2).await.unwrap();

        // Should provide DX completions
        assert!(!completions.is_empty());
    }
}
