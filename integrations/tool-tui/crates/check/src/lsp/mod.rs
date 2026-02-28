//! dx-check Language Server Protocol Implementation
//!
//! Provides real-time linting diagnostics, auto-fix code actions,
//! and rule documentation in IDEs.
//!
//! # Features
//! - Real-time diagnostics as you type
//! - Auto-fix suggestions for fixable rules
//! - Hover documentation for rules
//! - Format on save integration
//!
//! # Protocol Support
//! - LSP 3.17 specification
//! - textDocument/publishDiagnostics
//! - textDocument/codeAction (for fixes)
//! - textDocument/hover (for rule docs)
//! - textDocument/formatting

#[cfg(feature = "lsp")]
mod server;

#[cfg(test)]
mod tests;

#[cfg(feature = "lsp")]
pub use server::DxCheckLanguageServer;
#[cfg(feature = "lsp")]
pub use server::start_lsp_server;

/// LSP server configuration
#[derive(Debug, Clone)]
pub struct LspConfig {
    /// Enable real-time diagnostics
    pub enable_diagnostics: bool,
    /// Enable auto-fix code actions
    pub enable_code_actions: bool,
    /// Enable hover documentation
    pub enable_hover: bool,
    /// Enable formatting
    pub enable_formatting: bool,
    /// Debounce delay for diagnostics (ms)
    pub diagnostics_delay_ms: u64,
    /// Path to .sr rules directory
    pub rules_dir: Option<String>,
}

impl Default for LspConfig {
    fn default() -> Self {
        Self {
            enable_diagnostics: true,
            enable_code_actions: true,
            enable_hover: true,
            enable_formatting: true,
            diagnostics_delay_ms: 150,
            rules_dir: None,
        }
    }
}
