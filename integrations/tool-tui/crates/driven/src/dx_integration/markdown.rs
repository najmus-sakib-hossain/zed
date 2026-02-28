//! DX Markdown Integration
//!
//! Provides integration with DX Markdown for generating token-optimized
//! documentation from Driven types.
//!
//! ## Features
//!
//! - 73% token reduction compared to standard Markdown
//! - Three format outputs: LLM, Human, and Machine
//! - Automatic table rendering for structured data

use crate::parser::UnifiedRule;
use crate::{DrivenConfig, Result};
use dx_markdown::{
    CodeBlockNode, DxmDocument, DxmNode, HeaderNode, InlineNode, ListItem, ListNode, doc_to_human,
    doc_to_llm, doc_to_machine,
};

/// Configuration for DX Markdown output
#[derive(Debug, Clone)]
pub struct DxMarkdownConfig {
    /// Include table of contents
    pub include_toc: bool,
    /// Include metadata section
    pub include_metadata: bool,
    /// Output format preference
    pub format: DxMarkdownFormat,
}

impl Default for DxMarkdownConfig {
    fn default() -> Self {
        Self {
            include_toc: true,
            include_metadata: true,
            format: DxMarkdownFormat::Llm,
        }
    }
}

/// Output format for DX Markdown
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DxMarkdownFormat {
    /// LLM-optimized format (token efficient)
    Llm,
    /// Human-readable format (pretty printed)
    Human,
    /// Machine format (binary)
    Machine,
}

/// Trait for types that can generate DX Markdown documentation
pub trait DxDocumentable {
    /// Generate DX Markdown documentation with default config
    fn to_dx_markdown(&self) -> Result<String>;

    /// Generate DX Markdown documentation with custom config
    fn to_dx_markdown_with_config(&self, config: &DxMarkdownConfig) -> Result<String>;

    /// Generate DX Markdown as a DxmDocument
    fn to_dxm_document(&self) -> Result<DxmDocument>;
}

/// Helper to create a text inline node
fn text(s: impl Into<String>) -> InlineNode {
    InlineNode::Text(s.into())
}

/// Helper to create a paragraph node
fn paragraph(s: impl Into<String>) -> DxmNode {
    DxmNode::Paragraph(vec![text(s)])
}

/// Helper to create a header node
fn header(level: u8, s: impl Into<String>) -> DxmNode {
    DxmNode::Header(HeaderNode {
        level,
        content: vec![text(s)],
        priority: None,
    })
}

/// Helper to create a list node
fn list(ordered: bool, items: Vec<String>) -> DxmNode {
    DxmNode::List(ListNode {
        ordered,
        items: items
            .into_iter()
            .map(|s| ListItem {
                content: vec![text(s)],
                nested: None,
            })
            .collect(),
    })
}

/// Helper to create a code block node
fn code_block(language: Option<&str>, code: impl Into<String>) -> DxmNode {
    DxmNode::CodeBlock(CodeBlockNode {
        language: language.map(String::from),
        content: code.into(),
        priority: None,
    })
}

impl DxDocumentable for DrivenConfig {
    fn to_dx_markdown(&self) -> Result<String> {
        self.to_dx_markdown_with_config(&DxMarkdownConfig::default())
    }

    fn to_dx_markdown_with_config(&self, config: &DxMarkdownConfig) -> Result<String> {
        let doc = self.to_dxm_document()?;

        match config.format {
            DxMarkdownFormat::Llm => Ok(doc_to_llm(&doc)),
            DxMarkdownFormat::Human => Ok(doc_to_human(&doc)),
            DxMarkdownFormat::Machine => {
                let bytes = doc_to_machine(&doc);
                // Return base64 encoded for string representation
                Ok(base64_encode(&bytes))
            }
        }
    }

    fn to_dxm_document(&self) -> Result<DxmDocument> {
        let mut doc = DxmDocument::default();

        // Title
        doc.nodes.push(header(1, "Driven Configuration"));

        // Version info
        doc.nodes.push(paragraph(format!("Version: {}", self.version)));

        // Default editor
        doc.nodes.push(header(2, "Default Editor"));
        doc.nodes.push(paragraph(format!("{}", self.default_editor)));

        // Editor configuration section
        doc.nodes.push(header(2, "Editor Configuration"));

        let editor_items = vec![
            format!(
                "Cursor: {}",
                if self.editors.cursor {
                    "enabled"
                } else {
                    "disabled"
                }
            ),
            format!(
                "Copilot: {}",
                if self.editors.copilot {
                    "enabled"
                } else {
                    "disabled"
                }
            ),
            format!(
                "Windsurf: {}",
                if self.editors.windsurf {
                    "enabled"
                } else {
                    "disabled"
                }
            ),
            format!(
                "Claude: {}",
                if self.editors.claude {
                    "enabled"
                } else {
                    "disabled"
                }
            ),
            format!(
                "Aider: {}",
                if self.editors.aider {
                    "enabled"
                } else {
                    "disabled"
                }
            ),
            format!(
                "Cline: {}",
                if self.editors.cline {
                    "enabled"
                } else {
                    "disabled"
                }
            ),
        ];
        doc.nodes.push(list(false, editor_items));

        // Sync configuration section
        doc.nodes.push(header(2, "Sync Configuration"));

        let sync_items = vec![
            format!(
                "Watch: {}",
                if self.sync.watch {
                    "enabled"
                } else {
                    "disabled"
                }
            ),
            format!(
                "Auto Convert: {}",
                if self.sync.auto_convert {
                    "enabled"
                } else {
                    "disabled"
                }
            ),
            format!("Source of Truth: {}", self.sync.source_of_truth),
        ];
        doc.nodes.push(list(false, sync_items));

        Ok(doc)
    }
}

impl DxDocumentable for UnifiedRule {
    fn to_dx_markdown(&self) -> Result<String> {
        self.to_dx_markdown_with_config(&DxMarkdownConfig::default())
    }

    fn to_dx_markdown_with_config(&self, config: &DxMarkdownConfig) -> Result<String> {
        let doc = self.to_dxm_document()?;

        match config.format {
            DxMarkdownFormat::Llm => Ok(doc_to_llm(&doc)),
            DxMarkdownFormat::Human => Ok(doc_to_human(&doc)),
            DxMarkdownFormat::Machine => {
                let bytes = doc_to_machine(&doc);
                Ok(base64_encode(&bytes))
            }
        }
    }

    fn to_dxm_document(&self) -> Result<DxmDocument> {
        let mut doc = DxmDocument::default();

        match self {
            UnifiedRule::Persona {
                name,
                role,
                identity,
                style,
                traits,
                principles,
            } => {
                doc.nodes.push(header(1, format!("Persona: {}", name)));
                doc.nodes.push(paragraph(format!("Role: {}", role)));

                if let Some(id) = identity {
                    doc.nodes.push(paragraph(format!("Identity: {}", id)));
                }
                if let Some(s) = style {
                    doc.nodes.push(paragraph(format!("Style: {}", s)));
                }

                if !traits.is_empty() {
                    doc.nodes.push(header(2, "Traits"));
                    doc.nodes.push(list(false, traits.clone()));
                }

                if !principles.is_empty() {
                    doc.nodes.push(header(2, "Principles"));
                    doc.nodes.push(list(false, principles.clone()));
                }
            }
            UnifiedRule::Standard {
                category,
                priority,
                description,
                pattern,
            } => {
                doc.nodes.push(header(1, format!("Standard: {:?}", category)));
                doc.nodes.push(paragraph(format!("Priority: {}", priority)));
                doc.nodes.push(paragraph(description.clone()));

                if let Some(p) = pattern {
                    doc.nodes.push(header(2, "Pattern"));
                    doc.nodes.push(code_block(None, p.clone()));
                }
            }
            UnifiedRule::Context {
                includes,
                excludes,
                focus,
            } => {
                doc.nodes.push(header(1, "Context"));

                if !includes.is_empty() {
                    doc.nodes.push(header(2, "Include Patterns"));
                    doc.nodes.push(list(false, includes.clone()));
                }

                if !excludes.is_empty() {
                    doc.nodes.push(header(2, "Exclude Patterns"));
                    doc.nodes.push(list(false, excludes.clone()));
                }

                if !focus.is_empty() {
                    doc.nodes.push(header(2, "Focus Areas"));
                    doc.nodes.push(list(false, focus.clone()));
                }
            }
            UnifiedRule::Workflow { name, steps } => {
                doc.nodes.push(header(1, format!("Workflow: {}", name)));

                for (i, step) in steps.iter().enumerate() {
                    doc.nodes.push(header(2, format!("Step {}: {}", i + 1, step.name)));
                    doc.nodes.push(paragraph(step.description.clone()));

                    if let Some(cond) = &step.condition {
                        doc.nodes.push(paragraph(format!("Condition: {}", cond)));
                    }

                    if !step.actions.is_empty() {
                        doc.nodes.push(header(3, "Actions"));
                        doc.nodes.push(list(true, step.actions.clone()));
                    }
                }
            }
            UnifiedRule::Raw { content } => {
                doc.nodes.push(header(1, "Raw Content"));
                doc.nodes.push(code_block(None, content.clone()));
            }
        }

        Ok(doc)
    }
}

/// Generate documentation for a collection of rules
pub fn rules_to_dx_markdown(rules: &[UnifiedRule], config: &DxMarkdownConfig) -> Result<String> {
    let mut doc = DxmDocument::default();

    doc.nodes.push(header(1, "Driven Rules Documentation"));
    doc.nodes.push(paragraph(format!("Total rules: {}", rules.len())));

    // Group rules by type
    let mut personas = Vec::new();
    let mut standards = Vec::new();
    let mut contexts = Vec::new();
    let mut workflows = Vec::new();

    for rule in rules {
        match rule {
            UnifiedRule::Persona { .. } => personas.push(rule),
            UnifiedRule::Standard { .. } => standards.push(rule),
            UnifiedRule::Context { .. } => contexts.push(rule),
            UnifiedRule::Workflow { .. } => workflows.push(rule),
            UnifiedRule::Raw { .. } => {} // Skip raw rules in documentation
        }
    }

    // Add sections for each type
    if !personas.is_empty() {
        doc.nodes.push(header(2, format!("Personas ({})", personas.len())));
        for rule in personas {
            let rule_doc = rule.to_dxm_document()?;
            doc.nodes.extend(rule_doc.nodes);
        }
    }

    if !standards.is_empty() {
        doc.nodes.push(header(2, format!("Standards ({})", standards.len())));
        for rule in standards {
            let rule_doc = rule.to_dxm_document()?;
            doc.nodes.extend(rule_doc.nodes);
        }
    }

    if !contexts.is_empty() {
        doc.nodes.push(header(2, format!("Contexts ({})", contexts.len())));
        for rule in contexts {
            let rule_doc = rule.to_dxm_document()?;
            doc.nodes.extend(rule_doc.nodes);
        }
    }

    if !workflows.is_empty() {
        doc.nodes.push(header(2, format!("Workflows ({})", workflows.len())));
        for rule in workflows {
            let rule_doc = rule.to_dxm_document()?;
            doc.nodes.extend(rule_doc.nodes);
        }
    }

    match config.format {
        DxMarkdownFormat::Llm => Ok(doc_to_llm(&doc)),
        DxMarkdownFormat::Human => Ok(doc_to_human(&doc)),
        DxMarkdownFormat::Machine => {
            let bytes = doc_to_machine(&doc);
            Ok(base64_encode(&bytes))
        }
    }
}

/// Simple base64 encoding for binary output
fn base64_encode(data: &[u8]) -> String {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

    let mut result = String::with_capacity((data.len() + 2) / 3 * 4);

    for chunk in data.chunks(3) {
        let b0 = chunk[0] as usize;
        let b1 = chunk.get(1).copied().unwrap_or(0) as usize;
        let b2 = chunk.get(2).copied().unwrap_or(0) as usize;

        result.push(ALPHABET[b0 >> 2] as char);
        result.push(ALPHABET[((b0 & 0x03) << 4) | (b1 >> 4)] as char);

        if chunk.len() > 1 {
            result.push(ALPHABET[((b1 & 0x0f) << 2) | (b2 >> 6)] as char);
        } else {
            result.push('=');
        }

        if chunk.len() > 2 {
            result.push(ALPHABET[b2 & 0x3f] as char);
        } else {
            result.push('=');
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::format::RuleCategory;

    #[test]
    fn test_driven_config_to_dx_markdown() {
        let config = DrivenConfig::default();
        let md = config.to_dx_markdown().unwrap();

        // The LLM format uses abbreviated syntax
        assert!(!md.is_empty());
    }

    #[test]
    fn test_driven_config_to_dx_markdown_human() {
        let config = DrivenConfig::default();
        let md = config
            .to_dx_markdown_with_config(&DxMarkdownConfig {
                format: DxMarkdownFormat::Human,
                ..Default::default()
            })
            .unwrap();

        assert!(md.contains("Driven Configuration"));
        assert!(md.contains("Editor Configuration"));
    }

    #[test]
    fn test_unified_rule_persona_to_dx_markdown() {
        let rule = UnifiedRule::Persona {
            name: "Architect".to_string(),
            role: "Senior system architect".to_string(),
            identity: Some("Expert in distributed systems".to_string()),
            style: Some("Concise and technical".to_string()),
            traits: vec!["Analytical".to_string(), "Detail-oriented".to_string()],
            principles: vec!["SOLID principles".to_string()],
        };

        let md = rule
            .to_dx_markdown_with_config(&DxMarkdownConfig {
                format: DxMarkdownFormat::Human,
                ..Default::default()
            })
            .unwrap();
        assert!(md.contains("Architect"));
        assert!(md.contains("Traits"));
    }

    #[test]
    fn test_unified_rule_standard_to_dx_markdown() {
        let rule = UnifiedRule::Standard {
            category: RuleCategory::Naming,
            priority: 1,
            description: "Use snake_case for functions".to_string(),
            pattern: Some("fn my_function()".to_string()),
        };

        let md = rule
            .to_dx_markdown_with_config(&DxMarkdownConfig {
                format: DxMarkdownFormat::Human,
                ..Default::default()
            })
            .unwrap();
        assert!(md.contains("Naming"));
        assert!(md.contains("snake_case"));
    }

    #[test]
    fn test_rules_to_dx_markdown() {
        let rules = vec![
            UnifiedRule::Persona {
                name: "Dev".to_string(),
                role: "Developer".to_string(),
                identity: None,
                style: None,
                traits: vec![],
                principles: vec![],
            },
            UnifiedRule::Standard {
                category: RuleCategory::Style,
                priority: 1,
                description: "Use 4 spaces".to_string(),
                pattern: None,
            },
        ];

        let config = DxMarkdownConfig {
            format: DxMarkdownFormat::Human,
            ..Default::default()
        };
        let md = rules_to_dx_markdown(&rules, &config).unwrap();

        assert!(md.contains("Driven Rules Documentation"));
        assert!(md.contains("Personas"));
        assert!(md.contains("Standards"));
    }

    #[test]
    fn test_base64_encode() {
        assert_eq!(base64_encode(b""), "");
        assert_eq!(base64_encode(b"f"), "Zg==");
        assert_eq!(base64_encode(b"fo"), "Zm8=");
        assert_eq!(base64_encode(b"foo"), "Zm9v");
        assert_eq!(base64_encode(b"foob"), "Zm9vYg==");
    }
}
