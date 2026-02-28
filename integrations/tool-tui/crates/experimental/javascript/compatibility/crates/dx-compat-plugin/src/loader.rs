//! Loader types.

/// File loader type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Loader {
    /// JavaScript
    Js,
    /// JSX
    Jsx,
    /// TypeScript
    Ts,
    /// TSX
    Tsx,
    /// JSON
    Json,
    /// Plain text
    Text,
    /// Binary
    Binary,
    /// CSS
    Css,
}

impl Loader {
    /// Get the loader from a file extension.
    pub fn from_extension(ext: &str) -> Option<Self> {
        match ext.to_lowercase().as_str() {
            "js" | "mjs" | "cjs" => Some(Loader::Js),
            "jsx" => Some(Loader::Jsx),
            "ts" | "mts" | "cts" => Some(Loader::Ts),
            "tsx" => Some(Loader::Tsx),
            "json" => Some(Loader::Json),
            "txt" => Some(Loader::Text),
            "css" => Some(Loader::Css),
            _ => None,
        }
    }
}
