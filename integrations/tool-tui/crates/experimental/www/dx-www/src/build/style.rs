//! # Style Compiler
//!
//! Compiles parsed CSS into binary format (DXC1).
//!
//! ## Binary Format (DXC1)
//!
//! ```text
//! Header (8 bytes):
//!   [4] Magic: "DXC1"
//!   [1] Version
//!   [1] Flags (scoped, has variables, has animations)
//!   [2] Reserved
//!
//! String Pool:
//!   [4] Count
//!   For each string:
//!     [2] Length
//!     [N] UTF-8 bytes
//!
//! Rules:
//!   [4] Rule count
//!   For each rule:
//!     [1] Rule type
//!     ... type-specific data
//! ```

use crate::error::DxResult;
use crate::parser::style::{AtRule, CssDeclaration, CssRule, ParsedStyle};
use std::collections::HashMap;

/// Magic bytes for DXC1 format.
pub const STYLE_MAGIC: &[u8; 4] = b"DXC1";

/// Current version of the style format.
pub const STYLE_VERSION: u8 = 1;

/// Rule type opcodes.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuleType {
    Normal = 0x01,
    Media = 0x02,
    Keyframes = 0x03,
    Supports = 0x04,
    Layer = 0x05,
}

/// Selector type for optimization.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectorType {
    Class = 0x01,
    Id = 0x02,
    Tag = 0x03,
    Universal = 0x04,
    Compound = 0x05,
}

/// Compiles styles to binary format.
#[derive(Debug, Default)]
pub struct StyleCompiler;

impl StyleCompiler {
    /// Create a new style compiler.
    pub fn new() -> Self {
        Self
    }

    /// Compile a parsed style to binary format.
    pub fn compile(&self, style: &ParsedStyle) -> DxResult<Vec<u8>> {
        let mut output = Vec::with_capacity(4096);

        // Write header
        output.extend_from_slice(STYLE_MAGIC);
        output.push(STYLE_VERSION);

        // Flags
        let mut flags: u8 = 0;
        if style.scoped {
            flags |= 0x01;
        }
        if !style.custom_properties.is_empty() {
            flags |= 0x02;
        }
        if style.rules.iter().any(|r| matches!(r.at_rule, Some(AtRule::Keyframes { .. }))) {
            flags |= 0x04;
        }
        output.push(flags);

        // Reserved bytes
        output.extend_from_slice(&[0u8; 2]);

        // Build string pool
        let mut compiler = StyleCompilerInner::new();
        for rule in &style.rules {
            compiler.collect_strings(rule);
        }

        // Write string pool
        compiler.write_string_pool(&mut output)?;

        // Write rule count
        output.extend_from_slice(&(style.rules.len() as u32).to_le_bytes());

        // Compile rules
        for rule in &style.rules {
            compiler.compile_rule(&mut output, rule)?;
        }

        Ok(output)
    }
}

struct StyleCompilerInner {
    string_pool: Vec<String>,
    string_map: HashMap<String, u32>,
}

impl StyleCompilerInner {
    fn new() -> Self {
        Self {
            string_pool: Vec::new(),
            string_map: HashMap::new(),
        }
    }

    fn intern_string(&mut self, s: &str) -> u32 {
        if let Some(&idx) = self.string_map.get(s) {
            return idx;
        }
        let idx = self.string_pool.len() as u32;
        self.string_pool.push(s.to_string());
        self.string_map.insert(s.to_string(), idx);
        idx
    }

    fn collect_strings(&mut self, rule: &CssRule) {
        self.intern_string(&rule.selector);

        for decl in &rule.declarations {
            self.intern_string(&decl.property);
            self.intern_string(&decl.value);
        }

        if let Some(ref at_rule) = rule.at_rule {
            match at_rule {
                AtRule::Media { query } => {
                    self.intern_string(query);
                }
                AtRule::Keyframes { name } => {
                    self.intern_string(name);
                }
                AtRule::Import { url } => {
                    self.intern_string(url);
                }
                AtRule::Supports { condition } => {
                    self.intern_string(condition);
                }
                AtRule::Layer { name } => {
                    if let Some(n) = name {
                        self.intern_string(n);
                    }
                }
            }
        }
    }

    fn write_string_pool(&self, output: &mut Vec<u8>) -> DxResult<()> {
        output.extend_from_slice(&(self.string_pool.len() as u32).to_le_bytes());

        for s in &self.string_pool {
            let bytes = s.as_bytes();
            output.extend_from_slice(&(bytes.len() as u16).to_le_bytes());
            output.extend_from_slice(bytes);
        }

        Ok(())
    }

    fn compile_rule(&mut self, output: &mut Vec<u8>, rule: &CssRule) -> DxResult<()> {
        // Rule type
        let rule_type = match &rule.at_rule {
            None => RuleType::Normal,
            Some(AtRule::Media { .. }) => RuleType::Media,
            Some(AtRule::Keyframes { .. }) => RuleType::Keyframes,
            Some(AtRule::Supports { .. }) => RuleType::Supports,
            Some(AtRule::Layer { .. }) => RuleType::Layer,
            Some(AtRule::Import { .. }) => RuleType::Normal, // Imports handled separately
        };
        output.push(rule_type as u8);

        // At-rule specific data
        if let Some(ref at_rule) = rule.at_rule {
            match at_rule {
                AtRule::Media { query } => {
                    let query_idx = self.intern_string(query);
                    output.extend_from_slice(&query_idx.to_le_bytes());
                }
                AtRule::Keyframes { name } => {
                    let name_idx = self.intern_string(name);
                    output.extend_from_slice(&name_idx.to_le_bytes());
                }
                AtRule::Supports { condition } => {
                    let cond_idx = self.intern_string(condition);
                    output.extend_from_slice(&cond_idx.to_le_bytes());
                }
                AtRule::Layer { name } => {
                    if let Some(n) = name {
                        output.push(0x01);
                        let name_idx = self.intern_string(n);
                        output.extend_from_slice(&name_idx.to_le_bytes());
                    } else {
                        output.push(0x00);
                    }
                }
                AtRule::Import { url } => {
                    let url_idx = self.intern_string(url);
                    output.extend_from_slice(&url_idx.to_le_bytes());
                }
            }
        }

        // Selector
        let selector_idx = self.intern_string(&rule.selector);
        output.extend_from_slice(&selector_idx.to_le_bytes());

        // Selector type hint
        let selector_type = classify_selector(&rule.selector);
        output.push(selector_type as u8);

        // Declaration count
        output.push(rule.declarations.len() as u8);

        // Declarations
        for decl in &rule.declarations {
            self.compile_declaration(output, decl)?;
        }

        Ok(())
    }

    fn compile_declaration(&mut self, output: &mut Vec<u8>, decl: &CssDeclaration) -> DxResult<()> {
        // Property
        let prop_idx = self.intern_string(&decl.property);
        output.extend_from_slice(&prop_idx.to_le_bytes());

        // Value
        let val_idx = self.intern_string(&decl.value);
        output.extend_from_slice(&val_idx.to_le_bytes());

        // Important flag
        output.push(if decl.important { 0x01 } else { 0x00 });

        Ok(())
    }
}

/// Classify a selector for optimization hints.
fn classify_selector(selector: &str) -> SelectorType {
    let trimmed = selector.trim();

    if trimmed == "*" {
        SelectorType::Universal
    } else if trimmed.starts_with('#') && !trimmed.contains(' ') {
        SelectorType::Id
    } else if trimmed.starts_with('.') && !trimmed.contains(' ') {
        SelectorType::Class
    } else if !trimmed.contains(' ')
        && !trimmed.contains('.')
        && !trimmed.contains('#')
        && !trimmed.contains('[')
    {
        SelectorType::Tag
    } else {
        SelectorType::Compound
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compile_empty_style() {
        let compiler = StyleCompiler::new();
        let style = ParsedStyle {
            source: String::new(),
            scoped: false,
            rules: vec![],
            atomic_classes: vec![],
            custom_properties: vec![],
            imports: vec![],
        };

        let output = compiler.compile(&style).unwrap();

        // Check header
        assert_eq!(&output[0..4], b"DXC1");
        assert_eq!(output[4], 1); // Version
    }

    #[test]
    fn test_compile_simple_rule() {
        let compiler = StyleCompiler::new();
        let style = ParsedStyle {
            source: ".test { color: red; }".to_string(),
            scoped: false,
            rules: vec![CssRule {
                selector: ".test".to_string(),
                declarations: vec![CssDeclaration {
                    property: "color".to_string(),
                    value: "red".to_string(),
                    important: false,
                }],
                at_rule: None,
            }],
            atomic_classes: vec![],
            custom_properties: vec![],
            imports: vec![],
        };

        let output = compiler.compile(&style).unwrap();

        // Check header
        assert_eq!(&output[0..4], b"DXC1");
        // Should have rule type normal
        assert!(output.iter().any(|&b| b == RuleType::Normal as u8));
    }

    #[test]
    fn test_classify_selector() {
        assert_eq!(classify_selector("*"), SelectorType::Universal);
        assert_eq!(classify_selector("#id"), SelectorType::Id);
        assert_eq!(classify_selector(".class"), SelectorType::Class);
        assert_eq!(classify_selector("div"), SelectorType::Tag);
        assert_eq!(classify_selector("div .class"), SelectorType::Compound);
    }
}
