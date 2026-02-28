//! Legacy Format Converters
//!
//! This module provides converters for JSON, YAML, and TOML formats via DX Serializer
//! to maintain backward compatibility with existing configurations.
//!
//! ## Supported Formats
//!
//! - **JSON**: Standard JSON configuration files
//! - **YAML**: YAML configuration files (common in many tools)
//! - **TOML**: TOML configuration files (Rust ecosystem standard)
//!
//! ## Usage
//!
//! ```rust,ignore
//! use driven::dx_integration::legacy::{LegacyFormat, LegacyConverter};
//! use driven::DrivenConfig;
//!
//! // Convert from JSON
//! let json_content = r#"{"version": "1.0", "default_editor": "cursor"}"#;
//! let config = LegacyConverter::from_json::<DrivenConfig>(json_content)?;
//!
//! // Convert to JSON
//! let json_output = LegacyConverter::to_json(&config)?;
//!
//! // Auto-detect format and convert
//! let config = LegacyConverter::from_auto::<DrivenConfig>(content, "config.yaml")?;
//! ```

use crate::{DrivenConfig, DrivenError, Result};
use std::path::Path;

/// Supported legacy formats
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LegacyFormat {
    /// JSON format
    Json,
    /// YAML format
    Yaml,
    /// TOML format
    Toml,
}

impl LegacyFormat {
    /// Detect format from file extension
    pub fn from_extension(path: &Path) -> Option<Self> {
        path.extension().and_then(|ext| ext.to_str()).and_then(|ext| {
            match ext.to_lowercase().as_str() {
                "json" => Some(LegacyFormat::Json),
                "yaml" | "yml" => Some(LegacyFormat::Yaml),
                "toml" => Some(LegacyFormat::Toml),
                _ => None,
            }
        })
    }

    /// Get the file extension for this format
    pub fn extension(&self) -> &'static str {
        match self {
            LegacyFormat::Json => "json",
            LegacyFormat::Yaml => "yaml",
            LegacyFormat::Toml => "toml",
        }
    }

    /// Get the MIME type for this format
    pub fn mime_type(&self) -> &'static str {
        match self {
            LegacyFormat::Json => "application/json",
            LegacyFormat::Yaml => "application/x-yaml",
            LegacyFormat::Toml => "application/toml",
        }
    }
}

/// Legacy format converter
///
/// Provides conversion between legacy formats (JSON, YAML, TOML) and DrivenConfig.
pub struct LegacyConverter;

impl LegacyConverter {
    // ========== JSON Conversion ==========

    /// Parse DrivenConfig from JSON string
    pub fn from_json(content: &str) -> Result<DrivenConfig> {
        serde_json::from_str(content)
            .map_err(|e| DrivenError::Parse(format!("JSON parse error: {}", e)))
    }

    /// Serialize DrivenConfig to JSON string
    pub fn to_json(config: &DrivenConfig) -> Result<String> {
        serde_json::to_string_pretty(config)
            .map_err(|e| DrivenError::Format(format!("JSON serialization error: {}", e)))
    }

    /// Serialize DrivenConfig to compact JSON string
    pub fn to_json_compact(config: &DrivenConfig) -> Result<String> {
        serde_json::to_string(config)
            .map_err(|e| DrivenError::Format(format!("JSON serialization error: {}", e)))
    }

    // ========== YAML Conversion ==========

    /// Parse DrivenConfig from YAML string
    pub fn from_yaml(content: &str) -> Result<DrivenConfig> {
        serde_yaml::from_str(content)
            .map_err(|e| DrivenError::Parse(format!("YAML parse error: {}", e)))
    }

    /// Serialize DrivenConfig to YAML string
    pub fn to_yaml(config: &DrivenConfig) -> Result<String> {
        serde_yaml::to_string(config)
            .map_err(|e| DrivenError::Format(format!("YAML serialization error: {}", e)))
    }

    // ========== TOML Conversion ==========

    /// Parse DrivenConfig from TOML string
    pub fn from_toml(content: &str) -> Result<DrivenConfig> {
        toml::from_str(content).map_err(|e| DrivenError::Parse(format!("TOML parse error: {}", e)))
    }

    /// Serialize DrivenConfig to TOML string
    pub fn to_toml(config: &DrivenConfig) -> Result<String> {
        toml::to_string_pretty(config)
            .map_err(|e| DrivenError::Format(format!("TOML serialization error: {}", e)))
    }

    // ========== Auto-Detection ==========

    /// Parse DrivenConfig from content with auto-detected format
    ///
    /// The format is detected from the file path extension.
    pub fn from_auto(content: &str, path: &Path) -> Result<DrivenConfig> {
        let format = LegacyFormat::from_extension(path).ok_or_else(|| {
            DrivenError::UnsupportedFormat(format!(
                "Cannot detect format from path: {}",
                path.display()
            ))
        })?;

        Self::from_format(content, format)
    }

    /// Parse DrivenConfig from content with specified format
    pub fn from_format(content: &str, format: LegacyFormat) -> Result<DrivenConfig> {
        match format {
            LegacyFormat::Json => Self::from_json(content),
            LegacyFormat::Yaml => Self::from_yaml(content),
            LegacyFormat::Toml => Self::from_toml(content),
        }
    }

    /// Serialize DrivenConfig to string with specified format
    pub fn to_format(config: &DrivenConfig, format: LegacyFormat) -> Result<String> {
        match format {
            LegacyFormat::Json => Self::to_json(config),
            LegacyFormat::Yaml => Self::to_yaml(config),
            LegacyFormat::Toml => Self::to_toml(config),
        }
    }

    // ========== File Operations ==========

    /// Load DrivenConfig from a legacy format file
    pub fn load_file(path: &Path) -> Result<DrivenConfig> {
        let content = std::fs::read_to_string(path)?;
        Self::from_auto(&content, path)
    }

    /// Save DrivenConfig to a legacy format file
    ///
    /// The format is detected from the file path extension.
    pub fn save_file(config: &DrivenConfig, path: &Path) -> Result<()> {
        let format = LegacyFormat::from_extension(path).ok_or_else(|| {
            DrivenError::UnsupportedFormat(format!(
                "Cannot detect format from path: {}",
                path.display()
            ))
        })?;

        let content = Self::to_format(config, format)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    // ========== Format Conversion ==========

    /// Convert content from one legacy format to another
    pub fn convert(
        content: &str,
        from_format: LegacyFormat,
        to_format: LegacyFormat,
    ) -> Result<String> {
        let config = Self::from_format(content, from_format)?;
        Self::to_format(&config, to_format)
    }

    /// Convert a file from one legacy format to another
    pub fn convert_file(input_path: &Path, output_path: &Path) -> Result<()> {
        let config = Self::load_file(input_path)?;
        Self::save_file(&config, output_path)
    }
}

/// Trait for types that can be converted from legacy formats
pub trait LegacySerializable: Sized {
    /// Parse from JSON string
    fn from_legacy_json(content: &str) -> Result<Self>;

    /// Serialize to JSON string
    fn to_legacy_json(&self) -> Result<String>;

    /// Parse from YAML string
    fn from_legacy_yaml(content: &str) -> Result<Self>;

    /// Serialize to YAML string
    fn to_legacy_yaml(&self) -> Result<String>;

    /// Parse from TOML string
    fn from_legacy_toml(content: &str) -> Result<Self>;

    /// Serialize to TOML string
    fn to_legacy_toml(&self) -> Result<String>;
}

impl LegacySerializable for DrivenConfig {
    fn from_legacy_json(content: &str) -> Result<Self> {
        LegacyConverter::from_json(content)
    }

    fn to_legacy_json(&self) -> Result<String> {
        LegacyConverter::to_json(self)
    }

    fn from_legacy_yaml(content: &str) -> Result<Self> {
        LegacyConverter::from_yaml(content)
    }

    fn to_legacy_yaml(&self) -> Result<String> {
        LegacyConverter::to_yaml(self)
    }

    fn from_legacy_toml(content: &str) -> Result<Self> {
        LegacyConverter::from_toml(content)
    }

    fn to_legacy_toml(&self) -> Result<String> {
        LegacyConverter::to_toml(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Editor;

    #[test]
    fn test_json_roundtrip() {
        let config = DrivenConfig::default();
        let json = LegacyConverter::to_json(&config).unwrap();
        let loaded = LegacyConverter::from_json(&json).unwrap();

        assert_eq!(config.version, loaded.version);
        assert_eq!(config.default_editor, loaded.default_editor);
        assert_eq!(config.editors.cursor, loaded.editors.cursor);
    }

    #[test]
    fn test_yaml_roundtrip() {
        let config = DrivenConfig::default();
        let yaml = LegacyConverter::to_yaml(&config).unwrap();
        let loaded = LegacyConverter::from_yaml(&yaml).unwrap();

        assert_eq!(config.version, loaded.version);
        assert_eq!(config.default_editor, loaded.default_editor);
        assert_eq!(config.editors.cursor, loaded.editors.cursor);
    }

    #[test]
    fn test_toml_roundtrip() {
        let config = DrivenConfig::default();
        let toml_str = LegacyConverter::to_toml(&config).unwrap();
        let loaded = LegacyConverter::from_toml(&toml_str).unwrap();

        assert_eq!(config.version, loaded.version);
        assert_eq!(config.default_editor, loaded.default_editor);
        assert_eq!(config.editors.cursor, loaded.editors.cursor);
    }

    #[test]
    fn test_format_detection() {
        assert_eq!(
            LegacyFormat::from_extension(Path::new("config.json")),
            Some(LegacyFormat::Json)
        );
        assert_eq!(
            LegacyFormat::from_extension(Path::new("config.yaml")),
            Some(LegacyFormat::Yaml)
        );
        assert_eq!(LegacyFormat::from_extension(Path::new("config.yml")), Some(LegacyFormat::Yaml));
        assert_eq!(
            LegacyFormat::from_extension(Path::new("config.toml")),
            Some(LegacyFormat::Toml)
        );
        assert_eq!(LegacyFormat::from_extension(Path::new("config.txt")), None);
    }

    #[test]
    fn test_format_conversion() {
        let config = DrivenConfig::default();

        // JSON -> YAML
        let json = LegacyConverter::to_json(&config).unwrap();
        let yaml = LegacyConverter::convert(&json, LegacyFormat::Json, LegacyFormat::Yaml).unwrap();
        let loaded = LegacyConverter::from_yaml(&yaml).unwrap();
        assert_eq!(config.version, loaded.version);

        // YAML -> TOML
        let toml_str =
            LegacyConverter::convert(&yaml, LegacyFormat::Yaml, LegacyFormat::Toml).unwrap();
        let loaded = LegacyConverter::from_toml(&toml_str).unwrap();
        assert_eq!(config.version, loaded.version);

        // TOML -> JSON
        let json2 =
            LegacyConverter::convert(&toml_str, LegacyFormat::Toml, LegacyFormat::Json).unwrap();
        let loaded = LegacyConverter::from_json(&json2).unwrap();
        assert_eq!(config.version, loaded.version);
    }

    #[test]
    fn test_legacy_serializable_trait() {
        let config = DrivenConfig::default();

        // Test JSON via trait
        let json = config.to_legacy_json().unwrap();
        let loaded = DrivenConfig::from_legacy_json(&json).unwrap();
        assert_eq!(config.version, loaded.version);

        // Test YAML via trait
        let yaml = config.to_legacy_yaml().unwrap();
        let loaded = DrivenConfig::from_legacy_yaml(&yaml).unwrap();
        assert_eq!(config.version, loaded.version);

        // Test TOML via trait
        let toml_str = config.to_legacy_toml().unwrap();
        let loaded = DrivenConfig::from_legacy_toml(&toml_str).unwrap();
        assert_eq!(config.version, loaded.version);
    }

    #[test]
    fn test_json_with_all_fields() {
        let json = r#"{
            "version": "2.0",
            "default_editor": "copilot",
            "editors": {
                "cursor": false,
                "copilot": true,
                "windsurf": true,
                "claude": false,
                "aider": true,
                "cline": false
            },
            "templates": {
                "personas": ["developer", "reviewer"],
                "project": "rust",
                "standards": ["clean-code"],
                "workflow": "agile"
            },
            "sync": {
                "watch": false,
                "auto_convert": true,
                "source_of_truth": "custom/path.drv"
            },
            "context": {
                "include": ["src/**", "lib/**"],
                "exclude": ["target/**"],
                "index_path": "custom/index.drv"
            }
        }"#;

        let config = LegacyConverter::from_json(json).unwrap();
        assert_eq!(config.version, "2.0");
        assert_eq!(config.default_editor, Editor::Copilot);
        assert!(!config.editors.cursor);
        assert!(config.editors.copilot);
        assert!(config.editors.windsurf);
        assert!(!config.sync.watch);
        assert_eq!(config.sync.source_of_truth, "custom/path.drv");
    }
}

#[cfg(test)]
mod prop_tests {
    use super::*;
    use crate::{ContextConfig, Editor, EditorConfig, SyncConfig, TemplateConfig};
    use proptest::prelude::*;

    /// Generate arbitrary Editor values
    fn arb_editor() -> impl Strategy<Value = Editor> {
        prop_oneof![
            Just(Editor::Cursor),
            Just(Editor::Copilot),
            Just(Editor::Windsurf),
            Just(Editor::Claude),
            Just(Editor::Aider),
            Just(Editor::Cline),
        ]
    }

    /// Generate arbitrary EditorConfig values
    fn arb_editor_config() -> impl Strategy<Value = EditorConfig> {
        (
            any::<bool>(),
            any::<bool>(),
            any::<bool>(),
            any::<bool>(),
            any::<bool>(),
            any::<bool>(),
        )
            .prop_map(|(cursor, copilot, windsurf, claude, aider, cline)| EditorConfig {
                cursor,
                copilot,
                windsurf,
                claude,
                aider,
                cline,
            })
    }

    /// Generate arbitrary SyncConfig values
    fn arb_sync_config() -> impl Strategy<Value = SyncConfig> {
        (any::<bool>(), any::<bool>(), "[a-zA-Z0-9_./]{1,50}").prop_map(
            |(watch, auto_convert, source)| SyncConfig {
                watch,
                auto_convert,
                source_of_truth: source,
            },
        )
    }

    /// Generate arbitrary TemplateConfig values
    fn arb_template_config() -> impl Strategy<Value = TemplateConfig> {
        (
            prop::collection::vec("[a-zA-Z0-9_-]{1,20}", 0..5),
            prop::option::of("[a-zA-Z0-9_-]{1,20}"),
            prop::collection::vec("[a-zA-Z0-9_-]{1,20}", 0..5),
            prop::option::of("[a-zA-Z0-9_-]{1,20}"),
        )
            .prop_map(|(personas, project, standards, workflow)| TemplateConfig {
                personas,
                project,
                standards,
                workflow,
            })
    }

    /// Generate arbitrary ContextConfig values
    fn arb_context_config() -> impl Strategy<Value = ContextConfig> {
        (
            prop::collection::vec("[a-zA-Z0-9_*/.]{1,30}", 0..5),
            prop::collection::vec("[a-zA-Z0-9_*/.]{1,30}", 0..5),
            "[a-zA-Z0-9_./]{1,50}",
        )
            .prop_map(|(include, exclude, index_path)| ContextConfig {
                include,
                exclude,
                index_path,
            })
    }

    /// Generate arbitrary DrivenConfig values
    fn arb_driven_config() -> impl Strategy<Value = DrivenConfig> {
        (
            "[0-9]+\\.[0-9]+",
            arb_editor(),
            arb_editor_config(),
            arb_sync_config(),
            arb_template_config(),
            arb_context_config(),
        )
            .prop_map(|(version, default_editor, editors, sync, templates, context)| {
                DrivenConfig {
                    version,
                    default_editor,
                    editors,
                    sync,
                    templates,
                    context,
                }
            })
    }

    proptest! {
        /// Property 3: Legacy Format Backward Compatibility
        /// *For any* valid DrivenConfig, serializing to JSON and deserializing back
        /// SHALL produce an equivalent configuration.
        /// **Validates: Requirements 1.6**
        #[test]
        fn prop_json_roundtrip(config in arb_driven_config()) {
            let json = LegacyConverter::to_json(&config).expect("JSON serialization should succeed");
            let loaded = LegacyConverter::from_json(&json).expect("JSON deserialization should succeed");

            // Verify all fields are preserved
            prop_assert_eq!(config.version, loaded.version);
            prop_assert_eq!(config.default_editor, loaded.default_editor);
            prop_assert_eq!(config.editors.cursor, loaded.editors.cursor);
            prop_assert_eq!(config.editors.copilot, loaded.editors.copilot);
            prop_assert_eq!(config.editors.windsurf, loaded.editors.windsurf);
            prop_assert_eq!(config.editors.claude, loaded.editors.claude);
            prop_assert_eq!(config.editors.aider, loaded.editors.aider);
            prop_assert_eq!(config.editors.cline, loaded.editors.cline);
            prop_assert_eq!(config.sync.watch, loaded.sync.watch);
            prop_assert_eq!(config.sync.auto_convert, loaded.sync.auto_convert);
            prop_assert_eq!(config.sync.source_of_truth, loaded.sync.source_of_truth);
            prop_assert_eq!(config.templates.personas, loaded.templates.personas);
            prop_assert_eq!(config.templates.project, loaded.templates.project);
            prop_assert_eq!(config.templates.standards, loaded.templates.standards);
            prop_assert_eq!(config.templates.workflow, loaded.templates.workflow);
            prop_assert_eq!(config.context.include, loaded.context.include);
            prop_assert_eq!(config.context.exclude, loaded.context.exclude);
            prop_assert_eq!(config.context.index_path, loaded.context.index_path);
        }

        /// Property 3: Legacy Format Backward Compatibility (YAML)
        /// *For any* valid DrivenConfig, serializing to YAML and deserializing back
        /// SHALL produce an equivalent configuration.
        /// **Validates: Requirements 1.6**
        #[test]
        fn prop_yaml_roundtrip(config in arb_driven_config()) {
            let yaml = LegacyConverter::to_yaml(&config).expect("YAML serialization should succeed");
            let loaded = LegacyConverter::from_yaml(&yaml).expect("YAML deserialization should succeed");

            // Verify all fields are preserved
            prop_assert_eq!(config.version, loaded.version);
            prop_assert_eq!(config.default_editor, loaded.default_editor);
            prop_assert_eq!(config.editors.cursor, loaded.editors.cursor);
            prop_assert_eq!(config.editors.copilot, loaded.editors.copilot);
            prop_assert_eq!(config.editors.windsurf, loaded.editors.windsurf);
            prop_assert_eq!(config.editors.claude, loaded.editors.claude);
            prop_assert_eq!(config.editors.aider, loaded.editors.aider);
            prop_assert_eq!(config.editors.cline, loaded.editors.cline);
            prop_assert_eq!(config.sync.watch, loaded.sync.watch);
            prop_assert_eq!(config.sync.auto_convert, loaded.sync.auto_convert);
            prop_assert_eq!(config.sync.source_of_truth, loaded.sync.source_of_truth);
            prop_assert_eq!(config.templates.personas, loaded.templates.personas);
            prop_assert_eq!(config.templates.project, loaded.templates.project);
            prop_assert_eq!(config.templates.standards, loaded.templates.standards);
            prop_assert_eq!(config.templates.workflow, loaded.templates.workflow);
            prop_assert_eq!(config.context.include, loaded.context.include);
            prop_assert_eq!(config.context.exclude, loaded.context.exclude);
            prop_assert_eq!(config.context.index_path, loaded.context.index_path);
        }

        /// Property 3: Legacy Format Backward Compatibility (TOML)
        /// *For any* valid DrivenConfig, serializing to TOML and deserializing back
        /// SHALL produce an equivalent configuration.
        /// **Validates: Requirements 1.6**
        #[test]
        fn prop_toml_roundtrip(config in arb_driven_config()) {
            let toml_str = LegacyConverter::to_toml(&config).expect("TOML serialization should succeed");
            let loaded = LegacyConverter::from_toml(&toml_str).expect("TOML deserialization should succeed");

            // Verify all fields are preserved
            prop_assert_eq!(config.version, loaded.version);
            prop_assert_eq!(config.default_editor, loaded.default_editor);
            prop_assert_eq!(config.editors.cursor, loaded.editors.cursor);
            prop_assert_eq!(config.editors.copilot, loaded.editors.copilot);
            prop_assert_eq!(config.editors.windsurf, loaded.editors.windsurf);
            prop_assert_eq!(config.editors.claude, loaded.editors.claude);
            prop_assert_eq!(config.editors.aider, loaded.editors.aider);
            prop_assert_eq!(config.editors.cline, loaded.editors.cline);
            prop_assert_eq!(config.sync.watch, loaded.sync.watch);
            prop_assert_eq!(config.sync.auto_convert, loaded.sync.auto_convert);
            prop_assert_eq!(config.sync.source_of_truth, loaded.sync.source_of_truth);
            prop_assert_eq!(config.templates.personas, loaded.templates.personas);
            prop_assert_eq!(config.templates.project, loaded.templates.project);
            prop_assert_eq!(config.templates.standards, loaded.templates.standards);
            prop_assert_eq!(config.templates.workflow, loaded.templates.workflow);
            prop_assert_eq!(config.context.include, loaded.context.include);
            prop_assert_eq!(config.context.exclude, loaded.context.exclude);
            prop_assert_eq!(config.context.index_path, loaded.context.index_path);
        }

        /// Property 3: Legacy Format Cross-Conversion
        /// *For any* valid DrivenConfig, converting between any two legacy formats
        /// SHALL preserve all configuration data.
        /// **Validates: Requirements 1.6**
        #[test]
        fn prop_cross_format_conversion(config in arb_driven_config()) {
            // JSON -> YAML -> TOML -> JSON should preserve data
            let json1 = LegacyConverter::to_json(&config).expect("JSON serialization should succeed");
            let yaml = LegacyConverter::convert(&json1, LegacyFormat::Json, LegacyFormat::Yaml)
                .expect("JSON to YAML conversion should succeed");
            let toml_str = LegacyConverter::convert(&yaml, LegacyFormat::Yaml, LegacyFormat::Toml)
                .expect("YAML to TOML conversion should succeed");
            let json2 = LegacyConverter::convert(&toml_str, LegacyFormat::Toml, LegacyFormat::Json)
                .expect("TOML to JSON conversion should succeed");

            let loaded = LegacyConverter::from_json(&json2).expect("Final JSON deserialization should succeed");

            // Verify all fields are preserved after round-trip through all formats
            prop_assert_eq!(config.version, loaded.version);
            prop_assert_eq!(config.default_editor, loaded.default_editor);
            prop_assert_eq!(config.editors.cursor, loaded.editors.cursor);
            prop_assert_eq!(config.editors.copilot, loaded.editors.copilot);
            prop_assert_eq!(config.editors.windsurf, loaded.editors.windsurf);
            prop_assert_eq!(config.editors.claude, loaded.editors.claude);
            prop_assert_eq!(config.editors.aider, loaded.editors.aider);
            prop_assert_eq!(config.editors.cline, loaded.editors.cline);
            prop_assert_eq!(config.sync.watch, loaded.sync.watch);
            prop_assert_eq!(config.sync.auto_convert, loaded.sync.auto_convert);
            prop_assert_eq!(config.sync.source_of_truth, loaded.sync.source_of_truth);
        }
    }
}
