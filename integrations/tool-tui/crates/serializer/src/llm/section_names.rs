//! Section name dictionary for Human V3 format conversion
//!
//! This module provides bidirectional mappings between full section names
//! (used in Human V3 format) and abbreviated section IDs (used in LLM format).

use std::collections::HashMap;

/// Bidirectional section name dictionary
///
/// Provides mappings between full section names (e.g., "forge", "style")
/// and abbreviated section IDs (e.g., "f", "y").
#[derive(Debug, Clone)]
pub struct SectionNameDict {
    /// Full name → Section ID (for compression)
    name_to_id: HashMap<&'static str, &'static str>,
    /// Section ID → Full name (for expansion)
    id_to_name: HashMap<&'static str, &'static str>,
}

impl SectionNameDict {
    /// Create dictionary with all standard section mappings
    pub fn new() -> Self {
        let mut name_to_id = HashMap::new();
        let mut id_to_name = HashMap::new();

        // Helper to add bidirectional mapping
        let mut add = |name: &'static str, id: &'static str| {
            name_to_id.insert(name, id);
            id_to_name.insert(id, name);
        };

        // Section mappings from TypeScript SECTION_NAMES
        add("config", "c");
        add("forge", "f");
        add("stack", "k");
        add("style", "y");
        add("ui", "u");
        add("media", "m");
        add("i18n", "i");
        add("icon", "o");
        add("font", "t");
        add("driven", "d");
        add("generator", "g");
        add("scripts", "s");
        add("dependencies", "x");
        add("js", "j");
        add("python", "p");
        add("rust", "r");

        Self {
            name_to_id,
            id_to_name,
        }
    }

    /// Convert full section name to abbreviated ID
    ///
    /// Returns the original name if no mapping exists.
    pub fn name_to_id(&self, name: &str) -> String {
        self.name_to_id
            .get(name)
            .map(|s| s.to_string())
            .unwrap_or_else(|| name.to_string())
    }

    /// Convert abbreviated ID to full section name
    ///
    /// Returns the original ID if no mapping exists.
    pub fn id_to_name(&self, id: &str) -> String {
        self.id_to_name.get(id).map(|s| s.to_string()).unwrap_or_else(|| id.to_string())
    }

    /// Check if a section name exists in the dictionary
    pub fn has_name(&self, name: &str) -> bool {
        self.name_to_id.contains_key(name)
    }

    /// Check if a section ID exists in the dictionary
    pub fn has_id(&self, id: &str) -> bool {
        self.id_to_name.contains_key(id)
    }

    /// Get the number of mappings
    pub fn len(&self) -> usize {
        self.name_to_id.len()
    }

    /// Check if the dictionary is empty
    pub fn is_empty(&self) -> bool {
        self.name_to_id.is_empty()
    }
}

impl Default for SectionNameDict {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_section_name_dict_has_all_mappings() {
        let dict = SectionNameDict::new();
        assert_eq!(dict.len(), 16, "Should have 16 section mappings");
    }

    #[test]
    fn test_name_to_id() {
        let dict = SectionNameDict::new();

        assert_eq!(dict.name_to_id("config"), "c");
        assert_eq!(dict.name_to_id("forge"), "f");
        assert_eq!(dict.name_to_id("stack"), "k");
        assert_eq!(dict.name_to_id("style"), "y");
        assert_eq!(dict.name_to_id("ui"), "u");
        assert_eq!(dict.name_to_id("media"), "m");
        assert_eq!(dict.name_to_id("i18n"), "i");
        assert_eq!(dict.name_to_id("icon"), "o");
        assert_eq!(dict.name_to_id("font"), "t");
        assert_eq!(dict.name_to_id("driven"), "d");
        assert_eq!(dict.name_to_id("generator"), "g");
        assert_eq!(dict.name_to_id("scripts"), "s");
        assert_eq!(dict.name_to_id("dependencies"), "x");
        assert_eq!(dict.name_to_id("js"), "j");
        assert_eq!(dict.name_to_id("python"), "p");
        assert_eq!(dict.name_to_id("rust"), "r");
    }

    #[test]
    fn test_id_to_name() {
        let dict = SectionNameDict::new();

        assert_eq!(dict.id_to_name("c"), "config");
        assert_eq!(dict.id_to_name("f"), "forge");
        assert_eq!(dict.id_to_name("k"), "stack");
        assert_eq!(dict.id_to_name("y"), "style");
        assert_eq!(dict.id_to_name("u"), "ui");
        assert_eq!(dict.id_to_name("m"), "media");
        assert_eq!(dict.id_to_name("i"), "i18n");
        assert_eq!(dict.id_to_name("o"), "icon");
        assert_eq!(dict.id_to_name("t"), "font");
        assert_eq!(dict.id_to_name("d"), "driven");
        assert_eq!(dict.id_to_name("g"), "generator");
        assert_eq!(dict.id_to_name("s"), "scripts");
        assert_eq!(dict.id_to_name("x"), "dependencies");
        assert_eq!(dict.id_to_name("j"), "js");
        assert_eq!(dict.id_to_name("p"), "python");
        assert_eq!(dict.id_to_name("r"), "rust");
    }

    #[test]
    fn test_unknown_passthrough() {
        let dict = SectionNameDict::new();

        // Unknown names pass through unchanged
        assert_eq!(dict.name_to_id("unknown"), "unknown");
        assert_eq!(dict.name_to_id("custom_section"), "custom_section");

        // Unknown IDs pass through unchanged
        assert_eq!(dict.id_to_name("z"), "z");
        assert_eq!(dict.id_to_name("custom"), "custom");
    }

    #[test]
    fn test_round_trip() {
        let dict = SectionNameDict::new();

        // All known names should round-trip correctly
        let names = [
            "config",
            "forge",
            "stack",
            "style",
            "ui",
            "media",
            "i18n",
            "icon",
            "font",
            "driven",
            "generator",
            "scripts",
            "dependencies",
            "js",
            "python",
            "rust",
        ];

        for name in names {
            let id = dict.name_to_id(name);
            let back = dict.id_to_name(&id);
            assert_eq!(back, name, "Round-trip failed for {}", name);
        }
    }

    #[test]
    fn test_has_name_and_id() {
        let dict = SectionNameDict::new();

        assert!(dict.has_name("forge"));
        assert!(dict.has_name("style"));
        assert!(!dict.has_name("unknown"));

        assert!(dict.has_id("f"));
        assert!(dict.has_id("y"));
        assert!(!dict.has_id("z"));
    }
}
