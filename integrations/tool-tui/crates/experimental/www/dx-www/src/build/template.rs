//! # Template Compiler
//!
//! Compiles parsed templates into binary DOM format (DXT1).
//!
//! ## Binary Format (DXT1)
//!
//! The DXT1 format uses a bytecode representation:
//!
//! ```text
//! Header (8 bytes):
//!   [4] Magic: "DXT1"
//!   [1] Version
//!   [1] Flags
//!   [2] Reserved
//!
//! String Pool:
//!   [4] Count
//!   For each string:
//!     [2] Length
//!     [N] UTF-8 bytes
//!
//! Node Bytecode:
//!   [1] Node type
//!   ... type-specific data
//! ```

use crate::error::DxResult;
use crate::parser::template::{
    AttributeValue, ComponentRef, DirectiveNode, ElementNode, ParsedTemplate, TemplateDirective,
    TemplateNode,
};
use std::collections::HashMap;

/// Magic bytes for DXT1 format.
pub const TEMPLATE_MAGIC: &[u8; 4] = b"DXT1";

/// Current version of the template format.
pub const TEMPLATE_VERSION: u8 = 1;

/// Node type opcodes.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeType {
    Element = 0x01,
    Text = 0x02,
    Interpolation = 0x03,
    Comment = 0x04,
    Directive = 0x05,
    Component = 0x06,
}

/// Directive type opcodes.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DirectiveType {
    If = 0x01,
    Each = 0x02,
    Await = 0x03,
    Key = 0x04,
}

/// Attribute type opcodes.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttributeType {
    Static = 0x01,
    Dynamic = 0x02,
}

/// Compiles templates to binary format.
#[derive(Debug, Default)]
pub struct TemplateCompiler;

impl TemplateCompiler {
    /// Create a new template compiler.
    pub fn new() -> Self {
        Self
    }

    /// Compile a parsed template to binary format.
    pub fn compile(&self, template: &ParsedTemplate) -> DxResult<Vec<u8>> {
        let mut output = Vec::with_capacity(4096);

        // Write header
        output.extend_from_slice(TEMPLATE_MAGIC);
        output.push(TEMPLATE_VERSION);

        // Flags
        let mut flags: u8 = 0;
        if has_dynamic_content(&template.nodes) {
            flags |= 0x01;
        }
        if !template.bindings.is_empty() {
            flags |= 0x02;
        }
        output.push(flags);

        // Reserved bytes
        output.extend_from_slice(&[0u8; 2]);

        // Compile string pool first
        let mut compiler = TemplateCompilerInner::new();
        for node in &template.nodes {
            compiler.collect_strings(node);
        }

        // Write string pool
        compiler.write_string_pool(&mut output)?;

        // Write node count
        output.extend_from_slice(&(template.nodes.len() as u32).to_le_bytes());

        // Compile nodes
        for node in &template.nodes {
            compiler.compile_node(&mut output, node)?;
        }

        Ok(output)
    }
}

/// Check if nodes have dynamic content.
fn has_dynamic_content(nodes: &[TemplateNode]) -> bool {
    nodes.iter().any(|node| match node {
        TemplateNode::Interpolation(_) => true,
        TemplateNode::Directive(_) => true,
        TemplateNode::Component(_) => true,
        TemplateNode::Element(el) => {
            el.attributes
                .iter()
                .any(|attr| matches!(attr.value, Some(AttributeValue::Dynamic(_))))
                || has_dynamic_content(&el.children)
        }
        _ => false,
    })
}

struct TemplateCompilerInner {
    string_pool: Vec<String>,
    string_map: HashMap<String, u32>,
}

impl TemplateCompilerInner {
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

    fn collect_strings(&mut self, node: &TemplateNode) {
        match node {
            TemplateNode::Element(el) => {
                self.intern_string(&el.tag);
                for attr in &el.attributes {
                    self.intern_string(&attr.name);
                    if let Some(ref value) = attr.value {
                        match value {
                            AttributeValue::Static(s) | AttributeValue::Dynamic(s) => {
                                self.intern_string(s);
                            }
                        };
                    }
                }
                for child in &el.children {
                    self.collect_strings(child);
                }
            }
            TemplateNode::Text(text) => {
                self.intern_string(text);
            }
            TemplateNode::Interpolation(expr) => {
                self.intern_string(expr);
            }
            TemplateNode::Comment(text) => {
                self.intern_string(text);
            }
            TemplateNode::Directive(dir) => {
                self.collect_directive_strings(&dir.directive);
                for child in &dir.children {
                    self.collect_strings(child);
                }
                if let Some(ref else_branch) = dir.else_branch {
                    for child in else_branch {
                        self.collect_strings(child);
                    }
                }
            }
            TemplateNode::Component(comp) => {
                self.intern_string(&comp.name);
                for attr in &comp.props {
                    self.intern_string(&attr.name);
                    if let Some(ref value) = attr.value {
                        match value {
                            AttributeValue::Static(s) | AttributeValue::Dynamic(s) => {
                                self.intern_string(s);
                            }
                        };
                    }
                }
                for child in &comp.children {
                    self.collect_strings(child);
                }
            }
        }
    }

    fn collect_directive_strings(&mut self, directive: &TemplateDirective) {
        match directive {
            TemplateDirective::If { condition } => {
                self.intern_string(condition);
            }
            TemplateDirective::Each { items, item, index } => {
                self.intern_string(items);
                self.intern_string(item);
                if let Some(idx) = index {
                    self.intern_string(idx);
                }
            }
            TemplateDirective::Await {
                promise,
                then_var,
                catch_var,
            } => {
                self.intern_string(promise);
                if let Some(t) = then_var {
                    self.intern_string(t);
                }
                if let Some(c) = catch_var {
                    self.intern_string(c);
                }
            }
            TemplateDirective::Key { expression } => {
                self.intern_string(expression);
            }
        }
    }

    fn write_string_pool(&self, output: &mut Vec<u8>) -> DxResult<()> {
        // Write string count
        output.extend_from_slice(&(self.string_pool.len() as u32).to_le_bytes());

        // Write each string
        for s in &self.string_pool {
            let bytes = s.as_bytes();
            output.extend_from_slice(&(bytes.len() as u16).to_le_bytes());
            output.extend_from_slice(bytes);
        }

        Ok(())
    }

    fn compile_node(&mut self, output: &mut Vec<u8>, node: &TemplateNode) -> DxResult<()> {
        match node {
            TemplateNode::Element(el) => self.compile_element(output, el),
            TemplateNode::Text(text) => self.compile_text(output, text),
            TemplateNode::Interpolation(expr) => self.compile_interpolation(output, expr),
            TemplateNode::Comment(text) => self.compile_comment(output, text),
            TemplateNode::Directive(dir) => self.compile_directive(output, dir),
            TemplateNode::Component(comp) => self.compile_component(output, comp),
        }
    }

    fn compile_element(&mut self, output: &mut Vec<u8>, el: &ElementNode) -> DxResult<()> {
        output.push(NodeType::Element as u8);

        // Tag name index
        let tag_idx = self.intern_string(&el.tag);
        output.extend_from_slice(&tag_idx.to_le_bytes());

        // Attribute count
        output.push(el.attributes.len() as u8);

        // Attributes
        for attr in &el.attributes {
            let name_idx = self.intern_string(&attr.name);
            output.extend_from_slice(&name_idx.to_le_bytes());

            match &attr.value {
                None => {
                    // Boolean attribute
                    output.push(0x00);
                }
                Some(AttributeValue::Static(s)) => {
                    output.push(AttributeType::Static as u8);
                    let val_idx = self.intern_string(s);
                    output.extend_from_slice(&val_idx.to_le_bytes());
                }
                Some(AttributeValue::Dynamic(s)) => {
                    output.push(AttributeType::Dynamic as u8);
                    let val_idx = self.intern_string(s);
                    output.extend_from_slice(&val_idx.to_le_bytes());
                }
            }
        }

        // Child count
        output.extend_from_slice(&(el.children.len() as u32).to_le_bytes());

        // Children
        for child in &el.children {
            self.compile_node(output, child)?;
        }

        Ok(())
    }

    fn compile_text(&mut self, output: &mut Vec<u8>, text: &str) -> DxResult<()> {
        output.push(NodeType::Text as u8);
        let text_idx = self.intern_string(text);
        output.extend_from_slice(&text_idx.to_le_bytes());
        Ok(())
    }

    fn compile_interpolation(&mut self, output: &mut Vec<u8>, expr: &str) -> DxResult<()> {
        output.push(NodeType::Interpolation as u8);
        let expr_idx = self.intern_string(expr);
        output.extend_from_slice(&expr_idx.to_le_bytes());
        Ok(())
    }

    fn compile_comment(&mut self, output: &mut Vec<u8>, text: &str) -> DxResult<()> {
        output.push(NodeType::Comment as u8);
        let text_idx = self.intern_string(text);
        output.extend_from_slice(&text_idx.to_le_bytes());
        Ok(())
    }

    fn compile_directive(&mut self, output: &mut Vec<u8>, dir: &DirectiveNode) -> DxResult<()> {
        output.push(NodeType::Directive as u8);

        // Directive type
        match &dir.directive {
            TemplateDirective::If { condition } => {
                output.push(DirectiveType::If as u8);
                let cond_idx = self.intern_string(condition);
                output.extend_from_slice(&cond_idx.to_le_bytes());
            }
            TemplateDirective::Each { items, item, index } => {
                output.push(DirectiveType::Each as u8);
                let items_idx = self.intern_string(items);
                let item_idx = self.intern_string(item);
                output.extend_from_slice(&items_idx.to_le_bytes());
                output.extend_from_slice(&item_idx.to_le_bytes());

                // Optional index variable
                if let Some(idx) = index {
                    output.push(0x01); // Has index
                    let idx_idx = self.intern_string(idx);
                    output.extend_from_slice(&idx_idx.to_le_bytes());
                } else {
                    output.push(0x00); // No index
                }
            }
            TemplateDirective::Await {
                promise,
                then_var,
                catch_var,
            } => {
                output.push(DirectiveType::Await as u8);
                let promise_idx = self.intern_string(promise);
                output.extend_from_slice(&promise_idx.to_le_bytes());

                // Then variable
                if let Some(t) = then_var {
                    output.push(0x01);
                    let t_idx = self.intern_string(t);
                    output.extend_from_slice(&t_idx.to_le_bytes());
                } else {
                    output.push(0x00);
                }

                // Catch variable
                if let Some(c) = catch_var {
                    output.push(0x01);
                    let c_idx = self.intern_string(c);
                    output.extend_from_slice(&c_idx.to_le_bytes());
                } else {
                    output.push(0x00);
                }
            }
            TemplateDirective::Key { expression } => {
                output.push(DirectiveType::Key as u8);
                let expr_idx = self.intern_string(expression);
                output.extend_from_slice(&expr_idx.to_le_bytes());
            }
        }

        // Child count
        output.extend_from_slice(&(dir.children.len() as u32).to_le_bytes());

        // Children
        for child in &dir.children {
            self.compile_node(output, child)?;
        }

        // Else branch
        if let Some(ref else_branch) = dir.else_branch {
            output.push(0x01); // Has else branch
            output.extend_from_slice(&(else_branch.len() as u32).to_le_bytes());
            for child in else_branch {
                self.compile_node(output, child)?;
            }
        } else {
            output.push(0x00); // No else branch
        }

        Ok(())
    }

    fn compile_component(&mut self, output: &mut Vec<u8>, comp: &ComponentRef) -> DxResult<()> {
        output.push(NodeType::Component as u8);

        // Component name
        let name_idx = self.intern_string(&comp.name);
        output.extend_from_slice(&name_idx.to_le_bytes());

        // Prop count
        output.push(comp.props.len() as u8);

        // Props
        for prop in &comp.props {
            let name_idx = self.intern_string(&prop.name);
            output.extend_from_slice(&name_idx.to_le_bytes());

            match &prop.value {
                None => {
                    output.push(0x00);
                }
                Some(AttributeValue::Static(s)) => {
                    output.push(AttributeType::Static as u8);
                    let val_idx = self.intern_string(s);
                    output.extend_from_slice(&val_idx.to_le_bytes());
                }
                Some(AttributeValue::Dynamic(s)) => {
                    output.push(AttributeType::Dynamic as u8);
                    let val_idx = self.intern_string(s);
                    output.extend_from_slice(&val_idx.to_le_bytes());
                }
            }
        }

        // Child count
        output.extend_from_slice(&(comp.children.len() as u32).to_le_bytes());

        // Children (slot content)
        for child in &comp.children {
            self.compile_node(output, child)?;
        }

        Ok(())
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compile_empty_template() {
        let compiler = TemplateCompiler::new();
        let template = ParsedTemplate {
            source: String::new(),
            nodes: vec![],
            bindings: vec![],
            event_handlers: vec![],
            directives: vec![],
        };

        let output = compiler.compile(&template).unwrap();

        // Check header
        assert_eq!(&output[0..4], b"DXT1");
        assert_eq!(output[4], 1); // Version
    }

    #[test]
    fn test_compile_text_node() {
        let compiler = TemplateCompiler::new();
        let template = ParsedTemplate {
            source: "Hello".to_string(),
            nodes: vec![TemplateNode::Text("Hello".to_string())],
            bindings: vec![],
            event_handlers: vec![],
            directives: vec![],
        };

        let output = compiler.compile(&template).unwrap();

        // Check header exists
        assert_eq!(&output[0..4], b"DXT1");
        assert!(output.len() > 8);
    }

    #[test]
    fn test_compile_element() {
        let compiler = TemplateCompiler::new();
        let template = ParsedTemplate {
            source: "<div></div>".to_string(),
            nodes: vec![TemplateNode::Element(ElementNode {
                tag: "div".to_string(),
                attributes: vec![],
                children: vec![],
                self_closing: false,
            })],
            bindings: vec![],
            event_handlers: vec![],
            directives: vec![],
        };

        let output = compiler.compile(&template).unwrap();

        // Check header
        assert_eq!(&output[0..4], b"DXT1");
        // Should contain element node type
        assert!(output.iter().any(|&b| b == NodeType::Element as u8));
    }
}
