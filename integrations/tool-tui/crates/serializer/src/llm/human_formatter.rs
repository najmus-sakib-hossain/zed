//! Human Format Formatter
//!
//! Converts DxDocument to human-readable format for editing in text editors.
//!
//! ## Human Format Syntax (matching TypeScript implementation)
//!
//! ```dx
//! # Root scalars: key = value (padded to column 28)
//! name                        = dx
//! version                     = 0.0.1
//! title                       = Enhanced Developing Experience
//!
//! # [section] headers for grouped data
//! [workspace]
//! paths:
//! - @/www
//! - @/backend
//!
//! [editors]
//! default                     = neovim
//! items:
//! - neovim
//! - zed
//! - vscode
//!
//! # Tabular sections (multiple rows with name/version)
//! [dependencies]
//! dx-package-1                = 0.0.1
//! dx-package-2                = 0.0.1
//! ```

use crate::llm::types::{DxDocument, DxLlmValue, DxSection};
use indexmap::IndexMap;

/// Configuration for Human Format output
#[derive(Debug, Clone)]
pub struct HumanFormatConfig {
    /// Key padding to align = at column 28 (TypeScript default)
    pub key_padding: usize,
}

impl Default for HumanFormatConfig {
    fn default() -> Self {
        Self { key_padding: 28 }
    }
}

impl HumanFormatConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_key_padding(mut self, padding: usize) -> Self {
        self.key_padding = padding;
        self
    }

    pub fn for_tables() -> Self {
        Self { key_padding: 28 }
    }
}

/// Human Formatter - Clean TOML/INI-like format matching TypeScript implementation
pub struct HumanFormatter {
    config: HumanFormatConfig,
}

impl HumanFormatter {
    pub fn new() -> Self {
        Self {
            config: HumanFormatConfig::default(),
        }
    }

    pub fn with_config(config: HumanFormatConfig) -> Self {
        Self { config }
    }

    /// Format DxDocument to Human Format string
    pub fn format(&self, doc: &DxDocument) -> String {
        let mut sections: Vec<String> = Vec::new();

        // If entry_order is populated, use it to maintain original order
        if !doc.entry_order.is_empty() {
            // First, collect root scalars
            let mut root_scalar_keys = Vec::new();
            for entry_ref in &doc.entry_order {
                if let crate::llm::types::EntryRef::Context(key) = entry_ref {
                    if let Some(value) = doc.context.get(key) {
                        if !matches!(value, DxLlmValue::Obj(_) | DxLlmValue::Arr(_)) {
                            root_scalar_keys.push(key.clone());
                        }
                    }
                }
            }

            if !root_scalar_keys.is_empty() {
                let mut root_scalars_map = IndexMap::new();
                for key in root_scalar_keys {
                    if let Some(value) = doc.context.get(&key) {
                        root_scalars_map.insert(key, value.clone());
                    }
                }
                let root_scalars = self.format_root_scalars(&root_scalars_map);
                if !root_scalars.is_empty() {
                    sections.push(root_scalars);
                }
            }

            // Then process entries in order
            for entry_ref in &doc.entry_order {
                match entry_ref {
                    crate::llm::types::EntryRef::Context(key) => {
                        if let Some(DxLlmValue::Obj(fields)) = doc.context.get(key) {
                            let section_output = self.format_object_as_section(key, fields);
                            if !section_output.is_empty() {
                                sections.push(section_output);
                            }
                        }
                    }
                    crate::llm::types::EntryRef::Section(id) => {
                        if let Some(section) = doc.sections.get(id) {
                            let section_name = doc
                                .section_names
                                .get(id)
                                .cloned()
                                .unwrap_or_else(|| id.to_string());

                            let section_output = if self.is_tabular_section(section) {
                                self.format_tabular_section_multi_row(&section_name, section)
                            } else {
                                self.format_section(&section_name, section)
                            };

                            if !section_output.is_empty() {
                                sections.push(section_output);
                            }
                        }
                    }
                }
            }
        } else {
            // Fallback: use old behavior if entry_order is not populated
            let root_scalars = self.format_root_scalars(&doc.context);
            if !root_scalars.is_empty() {
                sections.push(root_scalars);
            }

            let mut context_keys: Vec<_> = doc.context.keys().collect();
            context_keys.sort();

            for key in context_keys {
                if let Some(DxLlmValue::Obj(fields)) = doc.context.get(key) {
                    let section_output = self.format_object_as_section(key, fields);
                    if !section_output.is_empty() {
                        sections.push(section_output);
                    }
                }
            }

            let mut section_ids: Vec<_> = doc.sections.keys().collect();
            section_ids.sort();

            for id in section_ids {
                if let Some(section) = doc.sections.get(id) {
                    let section_name =
                        doc.section_names.get(id).cloned().unwrap_or_else(|| id.to_string());

                    let section_output = if self.is_tabular_section(section) {
                        self.format_tabular_section_multi_row(&section_name, section)
                    } else {
                        self.format_section(&section_name, section)
                    };

                    if !section_output.is_empty() {
                        sections.push(section_output);
                    }
                }
            }
        }

        sections.join("\n\n")
    }

    /// Format an object from context as a [section]
    fn format_object_as_section(
        &self,
        section_id: &str,
        fields: &IndexMap<String, DxLlmValue>,
    ) -> String {
        let mut lines: Vec<String> = Vec::new();

        lines.push(format!("[{}]", section_id));

        // Separate scalars and arrays
        let mut scalars: Vec<(String, String)> = Vec::new();
        let mut arrays: Vec<(String, Vec<String>)> = Vec::new();

        for (key, value) in fields {
            if let DxLlmValue::Arr(items) = value {
                let array_items: Vec<String> = items.iter().map(|v| self.format_value(v)).collect();
                arrays.push((key.clone(), array_items));
            } else {
                scalars.push((key.clone(), self.format_value(value)));
            }
        }

        // Output scalars first
        for (key, value) in scalars {
            lines.push(self.format_key_value(&key, &value));
        }

        // Output arrays (key: followed by - item lines)
        for (key, items) in arrays {
            lines.push(format!("{}:", key));
            for item in items {
                lines.push(format!("- {}", item));
            }
        }

        lines.join("\n")
    }

    /// Format root scalars (context values that are not arrays or objects)
    fn format_root_scalars(&self, context: &IndexMap<String, DxLlmValue>) -> String {
        let mut lines: Vec<String> = Vec::new();

        for (key, value) in context.iter() {
            // Skip arrays and objects - they'll be formatted as sections
            if !matches!(value, DxLlmValue::Arr(_) | DxLlmValue::Obj(_)) {
                lines.push(self.format_key_value(key, &self.format_value(value)));
            }
        }

        lines.join("\n")
    }

    /// Format a section with [section] header
    fn format_section(&self, section_id: &str, section: &DxSection) -> String {
        let mut lines: Vec<String> = Vec::new();

        lines.push(format!("[{}]", section_id));

        if section.rows.is_empty() {
            return lines.join("\n");
        }

        let row = &section.rows[0];

        // Separate scalars and arrays
        let mut scalars: Vec<(String, String)> = Vec::new();
        let mut arrays: Vec<(String, Vec<String>)> = Vec::new();

        for (i, field) in section.schema.iter().enumerate() {
            if i >= row.len() {
                break;
            }

            let key = field.clone();
            let value = &row[i];

            if let DxLlmValue::Arr(items) = value {
                let array_items: Vec<String> = items.iter().map(|v| self.format_value(v)).collect();
                arrays.push((key, array_items));
            } else {
                scalars.push((key, self.format_value(value)));
            }
        }

        // Output scalars first
        for (key, value) in scalars {
            lines.push(self.format_key_value(&key, &value));
        }

        // Output arrays (key: followed by - item lines)
        for (key, items) in arrays {
            lines.push(format!("{}:", key));
            for item in items {
                lines.push(format!("- {}", item));
            }
        }

        lines.join("\n")
    }

    /// Format tabular section (multiple rows with name/version columns)
    #[allow(dead_code)]
    fn format_tabular_section(&self, section_id: &str, section: &DxSection) -> String {
        let mut lines: Vec<String> = Vec::new();

        lines.push(format!("[{}]", section_id));

        for row in &section.rows {
            if row.len() >= 2 {
                let key = self.format_value(&row[0]);
                let value = self.format_value(&row[1]);
                lines.push(self.format_key_value(&key, &value));
            }
        }

        lines.join("\n")
    }

    /// Format tabular section with multiple rows as separate [section:N] sections
    fn format_tabular_section_multi_row(&self, section_id: &str, section: &DxSection) -> String {
        let mut all_sections: Vec<String> = Vec::new();

        for (row_idx, row) in section.rows.iter().enumerate() {
            let mut lines: Vec<String> = Vec::new();

            // Section header with row number: [dependencies:1], [dependencies:2], etc.
            lines.push(format!("[{}:{}]", section_id, row_idx + 1));

            // Format each field as key = value
            for (i, field) in section.schema.iter().enumerate() {
                if i >= row.len() {
                    break;
                }

                let key = field.clone();
                let value = &row[i];

                if let DxLlmValue::Arr(items) = value {
                    // Arrays as key: followed by - items
                    lines.push(format!("{}:", key));
                    for item in items {
                        lines.push(format!("- {}", self.format_value(item)));
                    }
                } else {
                    lines.push(self.format_key_value(&key, &self.format_value(value)));
                }
            }

            all_sections.push(lines.join("\n"));
        }

        all_sections.join("\n\n")
    }

    /// Check if a section is tabular (has multiple rows with name/version schema)
    fn is_tabular_section(&self, section: &DxSection) -> bool {
        if section.rows.len() <= 1 {
            return false;
        }

        let schema_names: Vec<String> = section.schema.iter().map(|f| f.to_lowercase()).collect();

        schema_names.contains(&"name".to_string()) && schema_names.contains(&"version".to_string())
    }

    /// Format a DxValue to string for display
    fn format_value(&self, value: &DxLlmValue) -> String {
        match value {
            DxLlmValue::Str(s) => {
                // Replace underscores with spaces for human readability
                s.replace('_', " ")
            }
            DxLlmValue::Num(n) => {
                if n.fract() == 0.0 {
                    format!("{}", *n as i64)
                } else {
                    format!("{}", n)
                }
            }
            DxLlmValue::Bool(b) => {
                if *b {
                    "true".to_string()
                } else {
                    "false".to_string()
                }
            }
            DxLlmValue::Null => "none".to_string(),
            DxLlmValue::Arr(_) => String::new(),
            DxLlmValue::Obj(_) => String::new(),
            DxLlmValue::Ref(key) => format!("^{}", key),
        }
    }

    /// Format a key with padding to align = at column 28
    fn format_key_value(&self, key: &str, value: &str) -> String {
        let padding = self.config.key_padding.saturating_sub(key.len()).max(1);
        format!("{}{} = {}", key, " ".repeat(padding), value)
    }
}

impl Default for HumanFormatter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_empty_document() {
        let formatter = HumanFormatter::new();
        let doc = DxDocument::new();
        let output = formatter.format(&doc);
        assert!(output.is_empty());
    }

    #[test]
    fn test_format_scalar_values() {
        let formatter = HumanFormatter::new();
        let mut doc = DxDocument::new();
        doc.context.insert("name".to_string(), DxLlmValue::Str("dx".to_string()));
        doc.context.insert("version".to_string(), DxLlmValue::Str("0.0.1".to_string()));
        doc.context.insert("count".to_string(), DxLlmValue::Num(42.0));
        doc.context.insert("active".to_string(), DxLlmValue::Bool(true));

        let output = formatter.format(&doc);
        assert!(output.contains("name"));
        assert!(output.contains("dx"));
        assert!(output.contains("version"));
        assert!(output.contains("0.0.1"));
        assert!(output.contains("count"));
        assert!(output.contains("42"));
        assert!(output.contains("active"));
        assert!(output.contains("true"));
    }
}
