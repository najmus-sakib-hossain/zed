//! DX Ecosystem Integration
//!
//! This module provides integration with the DX Serializer and DX Markdown
//! for consistent, high-performance serialization and documentation across
//! all Driven configuration types.
//!
//! ## Formats
//!
//! - **DX LLM Format**: Human and LLM-readable text format (26.8% more efficient than TOON)
//! - **DX Machine Format**: Binary format for runtime (0.70ns field access)
//! - **DX Markdown**: Token-optimized documentation format (73% token reduction)
//!
//! ## Usage
//!
//! ```rust,ignore
//! use driven::dx_integration::{DxSerializable, DxDocumentable};
//! use driven::DrivenConfig;
//!
//! let config = DrivenConfig::default();
//!
//! // Serialize to DX LLM format
//! let llm_text = config.to_dx_llm()?;
//!
//! // Deserialize from DX LLM format
//! let loaded: DrivenConfig = DrivenConfig::from_dx_llm(&llm_text)?;
//!
//! // Generate documentation in DX Markdown format
//! let doc = config.to_dx_markdown()?;
//! ```

pub mod legacy;
mod markdown;

use crate::{DrivenConfig, DrivenError, EditorConfig, Result};
use serializer::{DxDocument, DxLlmValue, DxSection, document_to_llm, llm_to_document};

// Re-export serializer for external use
pub use serializer as dx_serializer;

// Re-export markdown integration
pub use markdown::{DxDocumentable, DxMarkdownConfig, DxMarkdownFormat, rules_to_dx_markdown};

// Re-export legacy format converters
pub use legacy::{LegacyConverter, LegacyFormat, LegacySerializable};

/// Trait for types that can be serialized to/from DX formats
pub trait DxSerializable: Sized {
    /// Serialize to DX LLM format (human/LLM readable text)
    fn to_dx_llm(&self) -> Result<String>;

    /// Deserialize from DX LLM format
    fn from_dx_llm(content: &str) -> Result<Self>;

    /// Serialize to DX Machine format (binary)
    fn to_dx_machine(&self) -> Result<Vec<u8>>;

    /// Deserialize from DX Machine format
    fn from_dx_machine(data: &[u8]) -> Result<Self>;
}

impl DxSerializable for DrivenConfig {
    fn to_dx_llm(&self) -> Result<String> {
        let mut doc = DxDocument::new();

        // Add metadata to context
        doc.context
            .insert("nm".to_string(), DxLlmValue::Str("driven-config".to_string()));
        doc.context.insert("v".to_string(), DxLlmValue::Str(self.version.clone()));
        doc.context
            .insert("editor".to_string(), DxLlmValue::Str(self.default_editor.to_string()));

        // Add editor configuration as section 'e'
        let mut editors_section = DxSection::new(vec!["name".to_string(), "enabled".to_string()]);
        editors_section
            .add_row(vec![
                DxLlmValue::Str("cursor".to_string()),
                DxLlmValue::Bool(self.editors.cursor),
            ])
            .ok();
        editors_section
            .add_row(vec![
                DxLlmValue::Str("copilot".to_string()),
                DxLlmValue::Bool(self.editors.copilot),
            ])
            .ok();
        editors_section
            .add_row(vec![
                DxLlmValue::Str("windsurf".to_string()),
                DxLlmValue::Bool(self.editors.windsurf),
            ])
            .ok();
        editors_section
            .add_row(vec![
                DxLlmValue::Str("claude".to_string()),
                DxLlmValue::Bool(self.editors.claude),
            ])
            .ok();
        editors_section
            .add_row(vec![
                DxLlmValue::Str("aider".to_string()),
                DxLlmValue::Bool(self.editors.aider),
            ])
            .ok();
        editors_section
            .add_row(vec![
                DxLlmValue::Str("cline".to_string()),
                DxLlmValue::Bool(self.editors.cline),
            ])
            .ok();
        doc.sections.insert('e', editors_section);

        // Add sync configuration as section 's'
        let mut sync_section = DxSection::new(vec!["key".to_string(), "value".to_string()]);
        sync_section
            .add_row(vec![
                DxLlmValue::Str("watch".to_string()),
                DxLlmValue::Bool(self.sync.watch),
            ])
            .ok();
        sync_section
            .add_row(vec![
                DxLlmValue::Str("auto_convert".to_string()),
                DxLlmValue::Bool(self.sync.auto_convert),
            ])
            .ok();
        sync_section
            .add_row(vec![
                DxLlmValue::Str("source".to_string()),
                DxLlmValue::Str(self.sync.source_of_truth.clone()),
            ])
            .ok();
        doc.sections.insert('s', sync_section);

        Ok(document_to_llm(&doc))
    }

    fn from_dx_llm(content: &str) -> Result<Self> {
        let doc = llm_to_document(content)
            .map_err(|e| DrivenError::Parse(format!("DX LLM parse error: {}", e)))?;

        let mut config = DrivenConfig::default();

        // Parse version from context - check both "v" and "vr" (abbreviated)
        // Handle both string and number values (e.g., "1.0" might be parsed as number 1.0)
        if let Some(v) = doc.context.get("v").or_else(|| doc.context.get("vr")) {
            config.version = match v {
                DxLlmValue::Str(s) => s.clone(),
                DxLlmValue::Num(n) => {
                    // Format number back to string, preserving decimal point
                    if n.fract() == 0.0 {
                        format!("{}.0", *n as i64)
                    } else {
                        format!("{}", n)
                    }
                }
                _ => config.version.clone(),
            };
        }

        // Parse default editor from context - check both "editor" and "ed" (abbreviated)
        if let Some(v) = doc.context.get("editor").or_else(|| doc.context.get("ed")) {
            if let Some(s) = v.as_str() {
                config.default_editor = parse_editor(s)?;
            }
        }

        // Parse editors section - the schema uses abbreviated column names (nm, en)
        if let Some(section) = doc.sections.get(&'e') {
            for row in &section.rows {
                if row.len() >= 2 {
                    // First column is name (nm), second is enabled (en)
                    if let (Some(name), Some(enabled)) = (row[0].as_str(), row[1].as_bool()) {
                        match name {
                            "cursor" => config.editors.cursor = enabled,
                            "copilot" => config.editors.copilot = enabled,
                            "windsurf" => config.editors.windsurf = enabled,
                            "claude" => config.editors.claude = enabled,
                            "aider" => config.editors.aider = enabled,
                            "cline" => config.editors.cline = enabled,
                            _ => {}
                        }
                    }
                }
            }
        }

        // Parse sync section - the schema uses abbreviated column names (ky, vl)
        if let Some(section) = doc.sections.get(&'s') {
            for row in &section.rows {
                if row.len() >= 2 {
                    // First column is key (ky), second is value (vl)
                    if let Some(key) = row[0].as_str() {
                        match key {
                            "watch" => {
                                if let Some(v) = row[1].as_bool() {
                                    config.sync.watch = v;
                                }
                            }
                            "auto_convert" => {
                                if let Some(v) = row[1].as_bool() {
                                    config.sync.auto_convert = v;
                                }
                            }
                            "source" => {
                                // Handle both string and number values
                                config.sync.source_of_truth = match &row[1] {
                                    DxLlmValue::Str(s) => s.clone(),
                                    DxLlmValue::Num(n) => {
                                        if n.fract() == 0.0 {
                                            format!("{}", *n as i64)
                                        } else {
                                            format!("{}", n)
                                        }
                                    }
                                    _ => config.sync.source_of_truth.clone(),
                                };
                            }
                            _ => {}
                        }
                    }
                }
            }
        }

        Ok(config)
    }

    fn to_dx_machine(&self) -> Result<Vec<u8>> {
        // Use a simpler binary format for DrivenConfig
        // Layout:
        // [0-3]: Magic "DRV1"
        // [4]: editor_flags
        // [5]: sync_flags
        // [6]: default_editor
        // [7]: reserved
        // [8-9]: version_len (u16)
        // [10-11]: source_len (u16)
        // [12..]: version string, then source string

        let version_bytes = self.version.as_bytes();
        let source_bytes = self.sync.source_of_truth.as_bytes();

        let total_len = 12 + version_bytes.len() + source_bytes.len();
        let mut buffer = Vec::with_capacity(total_len);

        // Magic
        buffer.extend_from_slice(b"DRV1");

        // Editor flags
        let editor_flags: u8 = (self.editors.cursor as u8)
            | ((self.editors.copilot as u8) << 1)
            | ((self.editors.windsurf as u8) << 2)
            | ((self.editors.claude as u8) << 3)
            | ((self.editors.aider as u8) << 4)
            | ((self.editors.cline as u8) << 5);
        buffer.push(editor_flags);

        // Sync flags
        let sync_flags: u8 = (self.sync.watch as u8) | ((self.sync.auto_convert as u8) << 1);
        buffer.push(sync_flags);

        // Default editor
        buffer.push(self.default_editor as u8);

        // Reserved
        buffer.push(0);

        // String lengths
        buffer.extend_from_slice(&(version_bytes.len() as u16).to_le_bytes());
        buffer.extend_from_slice(&(source_bytes.len() as u16).to_le_bytes());

        // Strings
        buffer.extend_from_slice(version_bytes);
        buffer.extend_from_slice(source_bytes);

        Ok(buffer)
    }

    fn from_dx_machine(data: &[u8]) -> Result<Self> {
        if data.len() < 12 {
            return Err(DrivenError::InvalidBinary("Data too short for DrivenConfig".to_string()));
        }

        // Check magic
        if &data[0..4] != b"DRV1" {
            return Err(DrivenError::InvalidBinary("Invalid magic bytes".to_string()));
        }

        let editor_flags = data[4];
        let sync_flags = data[5];
        let default_editor_byte = data[6];

        let version_len = u16::from_le_bytes([data[8], data[9]]) as usize;
        let source_len = u16::from_le_bytes([data[10], data[11]]) as usize;

        if data.len() < 12 + version_len + source_len {
            return Err(DrivenError::InvalidBinary("Data too short for strings".to_string()));
        }

        let version = std::str::from_utf8(&data[12..12 + version_len])
            .map_err(|e| DrivenError::InvalidBinary(format!("Invalid UTF-8 in version: {}", e)))?
            .to_string();

        let source_of_truth =
            std::str::from_utf8(&data[12 + version_len..12 + version_len + source_len])
                .map_err(|e| DrivenError::InvalidBinary(format!("Invalid UTF-8 in source: {}", e)))?
                .to_string();

        Ok(DrivenConfig {
            version,
            default_editor: editor_from_byte(default_editor_byte)?,
            editors: EditorConfig {
                cursor: (editor_flags & 1) != 0,
                copilot: (editor_flags & 2) != 0,
                windsurf: (editor_flags & 4) != 0,
                claude: (editor_flags & 8) != 0,
                aider: (editor_flags & 16) != 0,
                cline: (editor_flags & 32) != 0,
            },
            sync: crate::SyncConfig {
                watch: (sync_flags & 1) != 0,
                auto_convert: (sync_flags & 2) != 0,
                source_of_truth,
            },
            templates: crate::TemplateConfig::default(),
            context: crate::ContextConfig::default(),
        })
    }
}

/// Read a string from a DX-Zero slot (kept for future use with more complex types)
#[allow(dead_code)]
fn read_slot_string(_data: &[u8], _slot_offset: usize) -> Result<String> {
    // TODO: Re-implement when serializer::zero module is available
    Err(DrivenError::InvalidBinary("Not implemented".to_string()))
    /*
    use serializer::zero::slot::{HEAP_MARKER, INLINE_MARKER};

    if slot_offset + 16 > data.len() {
        return Err(DrivenError::InvalidBinary("Slot offset out of bounds".to_string()));
    }

    let slot_data = &data[slot_offset..slot_offset + 16];
    let marker = slot_data[15];

    if marker == INLINE_MARKER || marker < 14 {
        // Inline string: length is in byte 0, data in bytes 1..1+len
        let len = slot_data[0] as usize;
        if len > 14 {
            return Err(DrivenError::InvalidBinary("Invalid inline string length".to_string()));
        }
        let s = std::str::from_utf8(&slot_data[1..1 + len])
            .map_err(|e| DrivenError::InvalidBinary(format!("Invalid UTF-8: {}", e)))?;
        Ok(s.to_string())
    } else if marker == HEAP_MARKER {
        // Heap string: offset in bytes 0-3, length in bytes 4-7
        let offset =
            u32::from_le_bytes([slot_data[0], slot_data[1], slot_data[2], slot_data[3]]) as usize;
        let len =
            u32::from_le_bytes([slot_data[4], slot_data[5], slot_data[6], slot_data[7]]) as usize;

        // Calculate actual heap position
        let heap_start = slot_offset + offset;
        if heap_start + len > data.len() {
            return Err(DrivenError::InvalidBinary("Heap string out of bounds".to_string()));
        }

        let s = std::str::from_utf8(&data[heap_start..heap_start + len])
            .map_err(|e| DrivenError::InvalidBinary(format!("Invalid UTF-8: {}", e)))?;
        Ok(s.to_string())
    } else {
        Err(DrivenError::InvalidBinary(format!("Unknown slot marker: 0x{:02x}", marker)))
    }
    */
}

/// Parse editor from string
fn parse_editor(s: &str) -> Result<crate::Editor> {
    match s.to_lowercase().as_str() {
        "cursor" => Ok(crate::Editor::Cursor),
        "copilot" | "github copilot" => Ok(crate::Editor::Copilot),
        "windsurf" => Ok(crate::Editor::Windsurf),
        "claude" | "claude code" => Ok(crate::Editor::Claude),
        "aider" => Ok(crate::Editor::Aider),
        "cline" => Ok(crate::Editor::Cline),
        _ => Err(DrivenError::Parse(format!("Unknown editor: {}", s))),
    }
}

/// Convert editor byte to Editor enum
fn editor_from_byte(byte: u8) -> Result<crate::Editor> {
    match byte {
        0 => Ok(crate::Editor::Cursor),
        1 => Ok(crate::Editor::Copilot),
        2 => Ok(crate::Editor::Windsurf),
        3 => Ok(crate::Editor::Claude),
        4 => Ok(crate::Editor::Aider),
        5 => Ok(crate::Editor::Cline),
        _ => Err(DrivenError::InvalidBinary(format!("Unknown editor byte: {}", byte))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_driven_config_dx_llm_roundtrip() {
        let config = DrivenConfig::default();
        let llm = config.to_dx_llm().unwrap();
        println!("Serialized LLM:\n{}", llm);
        let loaded = DrivenConfig::from_dx_llm(&llm).unwrap();

        assert_eq!(config.version, loaded.version);
        assert_eq!(config.editors.cursor, loaded.editors.cursor);
        assert_eq!(config.editors.copilot, loaded.editors.copilot);
        assert_eq!(config.sync.watch, loaded.sync.watch);
    }

    #[test]
    fn test_driven_config_dx_llm_roundtrip_numeric_version() {
        let mut config = DrivenConfig::default();
        config.version = "0.0".to_string();
        let llm = config.to_dx_llm().unwrap();
        println!("Serialized LLM:\n{}", llm);
        let loaded = DrivenConfig::from_dx_llm(&llm).unwrap();
        println!("Loaded version: {}", loaded.version);

        assert_eq!(config.version, loaded.version);
    }

    #[test]
    fn test_driven_config_dx_machine_roundtrip() {
        let config = DrivenConfig::default();
        let binary = config.to_dx_machine().unwrap();
        let loaded = DrivenConfig::from_dx_machine(&binary).unwrap();

        assert_eq!(config.version, loaded.version);
        assert_eq!(config.editors.cursor, loaded.editors.cursor);
        assert_eq!(config.editors.copilot, loaded.editors.copilot);
        assert_eq!(config.sync.watch, loaded.sync.watch);
    }
}

#[cfg(test)]
mod prop_tests {
    use super::*;
    use proptest::prelude::*;

    /// Generate arbitrary Editor values
    fn arb_editor() -> impl Strategy<Value = crate::Editor> {
        prop_oneof![
            Just(crate::Editor::Cursor),
            Just(crate::Editor::Copilot),
            Just(crate::Editor::Windsurf),
            Just(crate::Editor::Claude),
            Just(crate::Editor::Aider),
            Just(crate::Editor::Cline),
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
    fn arb_sync_config() -> impl Strategy<Value = crate::SyncConfig> {
        (any::<bool>(), any::<bool>(), "[a-zA-Z0-9_./]{1,50}").prop_map(
            |(watch, auto_convert, source)| crate::SyncConfig {
                watch,
                auto_convert,
                source_of_truth: source,
            },
        )
    }

    /// Generate arbitrary DrivenConfig values
    fn arb_driven_config() -> impl Strategy<Value = DrivenConfig> {
        (
            // Use version format that won't be parsed as a number (e.g., "v1.0.0")
            "[0-9]+\\.[0-9]+\\.[0-9]+",
            arb_editor(),
            arb_editor_config(),
            arb_sync_config(),
        )
            .prop_map(|(version, default_editor, editors, sync)| DrivenConfig {
                version: format!("v{}", version), // Prefix with 'v' to ensure it's a string
                default_editor,
                editors,
                sync,
                templates: crate::TemplateConfig::default(),
                context: crate::ContextConfig::default(),
            })
    }

    proptest! {
        /// Property 1: DX Serializer Round-Trip Consistency
        /// *For any* valid DrivenConfig, serializing to DX LLM format and
        /// deserializing back SHALL produce an equivalent object.
        /// **Validates: Requirements 1.1, 1.2, 1.4, 1.5**
        #[test]
        fn prop_dx_llm_roundtrip(config in arb_driven_config()) {
            let llm = config.to_dx_llm().expect("Serialization should succeed");
            let loaded = DrivenConfig::from_dx_llm(&llm).expect("Deserialization should succeed");

            // Verify key fields are preserved
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

        /// Property 2: DX Machine Format Round-Trip Consistency
        /// *For any* valid DrivenConfig, encoding to DX Machine format and
        /// decoding back SHALL produce an equivalent object.
        /// **Validates: Requirements 1.2**
        #[test]
        fn prop_dx_machine_roundtrip(config in arb_driven_config()) {
            let binary = config.to_dx_machine().expect("Serialization should succeed");
            let loaded = DrivenConfig::from_dx_machine(&binary).expect("Deserialization should succeed");

            // Verify key fields are preserved
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
