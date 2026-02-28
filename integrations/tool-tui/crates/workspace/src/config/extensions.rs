//! Extension recommendations configuration.

use crate::platforms::Platform;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Extension and plugin recommendations.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ExtensionRecommendations {
    /// Core dx extensions (always recommended).
    #[serde(default)]
    pub core: Vec<ExtensionInfo>,

    /// Optional but useful extensions.
    #[serde(default)]
    pub recommended: Vec<ExtensionInfo>,

    /// Extensions to avoid/uninstall.
    #[serde(default)]
    pub unwanted: Vec<String>,

    /// Platform-specific extensions.
    #[serde(default)]
    pub platform_specific: HashMap<Platform, Vec<ExtensionInfo>>,
}

impl ExtensionRecommendations {
    /// Get default dx extension recommendations.
    pub fn dx_defaults() -> Self {
        Self {
            core: vec![
                ExtensionInfo {
                    id: "rust-lang.rust-analyzer".to_string(),
                    name: "rust-analyzer".to_string(),
                    description: Some(
                        "Rust language support with code completion and diagnostics".to_string(),
                    ),
                    required: true,
                    platforms: vec![],
                },
                ExtensionInfo {
                    id: "tamasfe.even-better-toml".to_string(),
                    name: "Even Better TOML".to_string(),
                    description: Some("TOML language support for Cargo.toml".to_string()),
                    required: false,
                    platforms: vec![],
                },
                ExtensionInfo {
                    id: "vadimcn.vscode-lldb".to_string(),
                    name: "CodeLLDB".to_string(),
                    description: Some("Native debugger for Rust via LLDB".to_string()),
                    required: false,
                    platforms: vec![],
                },
            ],
            recommended: vec![
                ExtensionInfo {
                    id: "serayuzgur.crates".to_string(),
                    name: "crates".to_string(),
                    description: Some("Helps manage crate dependencies in Cargo.toml".to_string()),
                    required: false,
                    platforms: vec![],
                },
                ExtensionInfo {
                    id: "fill-labs.dependi".to_string(),
                    name: "Dependi".to_string(),
                    description: Some("Dependency version management".to_string()),
                    required: false,
                    platforms: vec![],
                },
                ExtensionInfo {
                    id: "usernamehw.errorlens".to_string(),
                    name: "Error Lens".to_string(),
                    description: Some("Inline error and warning display".to_string()),
                    required: false,
                    platforms: vec![],
                },
                ExtensionInfo {
                    id: "eamodio.gitlens".to_string(),
                    name: "GitLens".to_string(),
                    description: Some("Git supercharged with blame, history, and more".to_string()),
                    required: false,
                    platforms: vec![],
                },
                ExtensionInfo {
                    id: "GitHub.copilot".to_string(),
                    name: "GitHub Copilot".to_string(),
                    description: Some("AI pair programmer".to_string()),
                    required: false,
                    platforms: vec![],
                },
            ],
            unwanted: vec![
                "rust-lang.rust".to_string(), // Old Rust extension, conflicts with rust-analyzer
                "kalitaalexey.vscode-rust".to_string(), // Deprecated
            ],
            platform_specific: HashMap::new(),
        }
    }

    /// Get all extension IDs for a platform.
    pub fn get_ids_for_platform(&self, platform: Platform) -> Vec<String> {
        let mut ids: Vec<String> = self.core.iter().map(|e| e.id.clone()).collect();

        ids.extend(self.recommended.iter().map(|e| e.id.clone()));

        if let Some(platform_exts) = self.platform_specific.get(&platform) {
            ids.extend(platform_exts.iter().map(|e| e.id.clone()));
        }

        ids
    }

    /// Get required extension IDs.
    pub fn get_required_ids(&self) -> Vec<String> {
        self.core.iter().filter(|e| e.required).map(|e| e.id.clone()).collect()
    }
}

/// Information about a single extension.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionInfo {
    /// Extension identifier (publisher.name format).
    pub id: String,

    /// Human-readable name.
    pub name: String,

    /// Description of what the extension does.
    #[serde(default)]
    pub description: Option<String>,

    /// Is this extension required for dx development.
    #[serde(default)]
    pub required: bool,

    /// Platforms this extension is specific to.
    #[serde(default)]
    pub platforms: Vec<Platform>,
}

impl ExtensionInfo {
    /// Create a new extension info.
    pub fn new(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            description: None,
            required: false,
            platforms: vec![],
        }
    }

    /// Set description.
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Mark as required.
    pub fn required(mut self) -> Self {
        self.required = true;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_defaults() {
        let recs = ExtensionRecommendations::dx_defaults();
        assert!(!recs.core.is_empty());
        assert!(recs.core.iter().any(|e| e.id.contains("rust-analyzer")));
    }

    #[test]
    fn test_get_ids() {
        let recs = ExtensionRecommendations::dx_defaults();
        let ids = recs.get_ids_for_platform(Platform::VsCode);
        assert!(ids.contains(&"rust-lang.rust-analyzer".to_string()));
    }

    #[test]
    fn test_extension_builder() {
        let ext = ExtensionInfo::new("test.ext", "Test Extension")
            .with_description("A test extension")
            .required();

        assert_eq!(ext.id, "test.ext");
        assert!(ext.required);
        assert!(ext.description.is_some());
    }
}
