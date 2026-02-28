//! # Binary Compiler
//!
//! Compiles DX components (.cp, .pg, .lyt) to HTIP binary (.dxob) files.
//!
//! ## Pipeline
//!
//! ```text
//! Source (.cp/.pg)  →  Parse (AST)  →  Analyze  →  Generate HTIP  →  .dxob
//! ```
//!
//! ## Output Format (.dxob)
//!
//! DX Object Binary format:
//! - Header: Magic + version + metadata
//! - String table: Deduplicated strings
//! - Template dictionary: Static template definitions
//! - Binding map: Dynamic binding locations
//! - Opcode stream: HTIP operations
//! - Source map: Debug info (optional)

use crate::dx_parser::{
    AttributeValue, BlockType, DxFile, EachBlockNode, ElementNode, ExpressionNode, IfBlockNode,
    TemplateNode, parse_dx_file,
};
use ed25519_dalek::SigningKey;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use thiserror::Error;

/// Binary compiler errors
#[derive(Debug, Error)]
pub enum CompilerError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Parse error: {0}")]
    Parse(String),

    #[error("Invalid component: {0}")]
    InvalidComponent(String),

    #[error("Unsupported feature: {0}")]
    UnsupportedFeature(String),

    #[error("Binding error: {0}")]
    BindingError(String),
}

pub type CompilerResult<T> = Result<T, CompilerError>;

/// DXOB file magic bytes
pub const DXOB_MAGIC: &[u8; 4] = b"DXOB";

/// DXOB version
pub const DXOB_VERSION: u8 = 1;

/// Compiled DX Object Binary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DxObjectBinary {
    /// Component name
    pub name: String,
    /// Component type (component, page, layout)
    pub component_type: ComponentType,
    /// String table
    pub strings: Vec<String>,
    /// Template definitions
    pub templates: Vec<CompiledTemplate>,
    /// Dynamic bindings
    pub bindings: Vec<CompiledBinding>,
    /// Event handlers
    pub events: Vec<CompiledEvent>,
    /// CSS classes used (for dx-style)
    pub css_classes: Vec<String>,
    /// Props schema
    pub props_schema: Option<PropsSchema>,
    /// Source map (debug builds only)
    pub source_map: Option<SourceMap>,
}

/// Component types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ComponentType {
    Component,
    Page,
    Layout,
}

impl From<BlockType> for ComponentType {
    fn from(bt: BlockType) -> Self {
        match bt {
            BlockType::Component => Self::Component,
            BlockType::Page => Self::Page,
            BlockType::Layout => Self::Layout,
        }
    }
}

/// Compiled template definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompiledTemplate {
    /// Template ID
    pub id: u16,
    /// Static HTML (with placeholders)
    pub html: String,
    /// Slot indices for dynamic content
    pub slots: Vec<TemplateSlot>,
}

/// A slot in a template for dynamic content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateSlot {
    /// Slot ID
    pub id: u16,
    /// Slot type
    pub slot_type: SlotType,
    /// Path to the slot in the template (e.g., "0/1/2" for nested elements)
    pub path: String,
}

/// Slot types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SlotType {
    Text,
    Attribute,
    Element,
    Children,
}

/// Compiled binding (expression → DOM update)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompiledBinding {
    /// Binding ID
    pub id: u32,
    /// Expression to evaluate
    pub expression: String,
    /// Target template ID
    pub template_id: u16,
    /// Target slot ID
    pub slot_id: u16,
    /// Binding type
    pub binding_type: BindingType,
    /// Dependencies (state variables this binding depends on)
    pub dependencies: Vec<String>,
}

/// Binding types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BindingType {
    Text,
    Attribute,
    Property,
    Class,
    Style,
    Visibility,
    List,
}

/// Compiled event handler
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompiledEvent {
    /// Event handler ID
    pub id: u32,
    /// Event type (click, input, etc.)
    pub event_type: String,
    /// Handler expression or function name
    pub handler: String,
    /// Target element path
    pub target_path: String,
    /// Modifiers (prevent, stop, once, etc.)
    pub modifiers: Vec<String>,
}

/// Props schema
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PropsSchema {
    pub fields: Vec<PropField>,
}

/// A prop field
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PropField {
    pub name: String,
    pub type_name: String,
    pub required: bool,
    pub default_value: Option<String>,
}

/// Source map for debugging
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceMap {
    pub file: String,
    pub mappings: Vec<SourceMapping>,
}

/// Single source mapping
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceMapping {
    pub generated_offset: u32,
    pub source_line: u32,
    pub source_column: u32,
}

/// Binary compiler
pub struct BinaryCompiler {
    /// Next template ID
    next_template_id: u16,
    /// Next binding ID
    next_binding_id: u32,
    /// Next event ID
    next_event_id: u32,
    /// String table
    strings: Vec<String>,
    /// String index map
    string_map: HashMap<String, u32>,
    /// Compiled templates
    templates: Vec<CompiledTemplate>,
    /// Compiled bindings
    bindings: Vec<CompiledBinding>,
    /// Compiled events
    events: Vec<CompiledEvent>,
    /// CSS classes
    css_classes: Vec<String>,
    /// Include source maps
    include_source_map: bool,
    /// Source mappings
    source_mappings: Vec<SourceMapping>,
}

impl BinaryCompiler {
    /// Create a new compiler
    pub fn new() -> Self {
        Self {
            next_template_id: 0,
            next_binding_id: 0,
            next_event_id: 0,
            strings: Vec::new(),
            string_map: HashMap::new(),
            templates: Vec::new(),
            bindings: Vec::new(),
            events: Vec::new(),
            css_classes: Vec::new(),
            include_source_map: false,
            source_mappings: Vec::new(),
        }
    }

    /// Enable source map generation
    pub fn with_source_map(mut self) -> Self {
        self.include_source_map = true;
        self
    }

    /// Compile a DX file to binary
    pub fn compile_file(&mut self, path: &Path) -> CompilerResult<DxObjectBinary> {
        let source = std::fs::read_to_string(path)?;
        let name = path.file_stem().and_then(|s| s.to_str()).unwrap_or("unknown").to_string();

        // Determine expected type from extension
        let expected_type = match path.extension().and_then(|e| e.to_str()) {
            Some("pg") => Some(BlockType::Page),
            Some("cp") => Some(BlockType::Component),
            Some("lyt") => Some(BlockType::Layout),
            _ => None,
        };

        self.compile_source(&source, &name, expected_type)
    }

    /// Compile source code to binary
    pub fn compile_source(
        &mut self,
        source: &str,
        name: &str,
        expected_type: Option<BlockType>,
    ) -> CompilerResult<DxObjectBinary> {
        // Parse the DX file
        let ast = parse_dx_file(source, expected_type)
            .map_err(|e| CompilerError::Parse(e.to_string()))?;

        // Extract CSS classes
        self.css_classes = ast.css_classes.clone();

        // Compile templates from AST
        for node in &ast.template {
            self.compile_node(node, "0")?;
        }

        // Extract props schema from script
        let props_schema = self.extract_props_schema(&ast);

        // Build source map if enabled
        let source_map = if self.include_source_map {
            Some(SourceMap {
                file: name.to_string(),
                mappings: self.source_mappings.clone(),
            })
        } else {
            None
        };

        Ok(DxObjectBinary {
            name: name.to_string(),
            component_type: ast.file_type.into(),
            strings: self.strings.clone(),
            templates: self.templates.clone(),
            bindings: self.bindings.clone(),
            events: self.events.clone(),
            css_classes: self.css_classes.clone(),
            props_schema,
            source_map,
        })
    }

    /// Add string to table and get index
    fn add_string(&mut self, s: &str) -> u32 {
        if let Some(&idx) = self.string_map.get(s) {
            return idx;
        }
        let idx = self.strings.len() as u32;
        self.strings.push(s.to_string());
        self.string_map.insert(s.to_string(), idx);
        idx
    }

    /// Allocate a template ID
    fn alloc_template_id(&mut self) -> u16 {
        let id = self.next_template_id;
        self.next_template_id += 1;
        id
    }

    /// Allocate a binding ID
    fn alloc_binding_id(&mut self) -> u32 {
        let id = self.next_binding_id;
        self.next_binding_id += 1;
        id
    }

    /// Allocate an event ID
    fn alloc_event_id(&mut self) -> u32 {
        let id = self.next_event_id;
        self.next_event_id += 1;
        id
    }

    /// Compile a template node
    fn compile_node(&mut self, node: &TemplateNode, path: &str) -> CompilerResult<()> {
        match node {
            TemplateNode::Text(text) => {
                self.add_string(text);
            }
            TemplateNode::Element(el) => {
                self.compile_element(el, path)?;
            }
            TemplateNode::Expression(expr) => {
                self.compile_expression(expr, path)?;
            }
            TemplateNode::IfBlock(if_block) => {
                self.compile_if_block(if_block, path)?;
            }
            TemplateNode::EachBlock(each_block) => {
                self.compile_each_block(each_block, path)?;
            }
            TemplateNode::Component(comp) => {
                // Component instances are compiled as template instantiations
                let template_id = self.alloc_template_id();
                let html =
                    format!("<dx-component data-component=\"{}\"></dx-component>", comp.name);

                let mut slots = Vec::new();

                // Add prop bindings
                for (prop_name, value) in &comp.props {
                    if let AttributeValue::Dynamic(expr) = value {
                        let binding_id = self.alloc_binding_id();
                        self.bindings.push(CompiledBinding {
                            id: binding_id,
                            expression: expr.clone(),
                            template_id,
                            slot_id: slots.len() as u16,
                            binding_type: BindingType::Property,
                            dependencies: extract_dependencies(expr),
                        });
                    }
                }

                self.templates.push(CompiledTemplate {
                    id: template_id,
                    html,
                    slots,
                });
            }
            TemplateNode::Slot(slot) => {
                let template_id = self.alloc_template_id();
                let slot_name = slot.name.as_deref().unwrap_or("default");
                let html = format!("<slot name=\"{}\"></slot>", slot_name);

                self.templates.push(CompiledTemplate {
                    id: template_id,
                    html,
                    slots: vec![TemplateSlot {
                        id: 0,
                        slot_type: SlotType::Children,
                        path: path.to_string(),
                    }],
                });
            }
            _ => {}
        }
        Ok(())
    }

    /// Compile an element node
    fn compile_element(&mut self, el: &ElementNode, path: &str) -> CompilerResult<()> {
        let template_id = self.alloc_template_id();
        let mut slots = Vec::new();
        let mut slot_counter = 0u16;

        // Build HTML with slot placeholders
        let mut html = format!("<{}", el.tag);

        // Static attributes
        for (name, value) in &el.attributes {
            match value {
                AttributeValue::Static(v) => {
                    html.push_str(&format!(" {}=\"{}\"", name, escape_attr(v)));
                }
                AttributeValue::Dynamic(expr) => {
                    let slot_id = slot_counter;
                    slot_counter += 1;

                    html.push_str(&format!(" {}=\"{{{{slot:{}}}}}\"", name, slot_id));

                    slots.push(TemplateSlot {
                        id: slot_id,
                        slot_type: SlotType::Attribute,
                        path: format!("{}/attr/{}", path, name),
                    });

                    let binding_id = self.alloc_binding_id();
                    self.bindings.push(CompiledBinding {
                        id: binding_id,
                        expression: expr.clone(),
                        template_id,
                        slot_id,
                        binding_type: BindingType::Attribute,
                        dependencies: extract_dependencies(expr),
                    });
                }
            }
        }

        // Event handlers
        for (event_name, handler) in &el.events {
            let event_id = self.alloc_event_id();
            html.push_str(&format!(" data-dx-event-{}=\"{}\"", event_id, event_name));

            self.events.push(CompiledEvent {
                id: event_id,
                event_type: event_name.clone(),
                handler: handler.clone(),
                target_path: path.to_string(),
                modifiers: Vec::new(),
            });
        }

        // Bindings (bind:value, etc.)
        for (binding_name, expr) in &el.bindings {
            let slot_id = slot_counter;
            slot_counter += 1;

            slots.push(TemplateSlot {
                id: slot_id,
                slot_type: SlotType::Attribute,
                path: format!("{}/bind/{}", path, binding_name),
            });

            let binding_id = self.alloc_binding_id();
            self.bindings.push(CompiledBinding {
                id: binding_id,
                expression: expr.clone(),
                template_id,
                slot_id,
                binding_type: BindingType::Property,
                dependencies: extract_dependencies(expr),
            });
        }

        // Class directives (class:active={isActive})
        for (class_name, condition) in &el.class_directives {
            let slot_id = slot_counter;
            slot_counter += 1;

            slots.push(TemplateSlot {
                id: slot_id,
                slot_type: SlotType::Attribute,
                path: format!("{}/class/{}", path, class_name),
            });

            let binding_id = self.alloc_binding_id();
            self.bindings.push(CompiledBinding {
                id: binding_id,
                expression: condition.clone(),
                template_id,
                slot_id,
                binding_type: BindingType::Class,
                dependencies: extract_dependencies(condition),
            });

            // Add class to CSS classes list
            if !self.css_classes.contains(&class_name.to_string()) {
                self.css_classes.push(class_name.clone());
            }
        }

        // Close opening tag
        if el.self_closing {
            html.push_str(" />");
        } else {
            html.push('>');

            // Compile children
            for (i, child) in el.children.iter().enumerate() {
                let child_path = format!("{}/{}", path, i);
                self.compile_node(child, &child_path)?;
            }

            // Add children slot
            if !el.children.is_empty() {
                let slot_id = slot_counter;
                slots.push(TemplateSlot {
                    id: slot_id,
                    slot_type: SlotType::Children,
                    path: format!("{}/children", path),
                });
            }

            html.push_str(&format!("</{}>", el.tag));
        }

        self.templates.push(CompiledTemplate {
            id: template_id,
            html,
            slots,
        });

        Ok(())
    }

    /// Compile an expression node
    fn compile_expression(&mut self, expr: &ExpressionNode, path: &str) -> CompilerResult<()> {
        let template_id = self.alloc_template_id();
        let html = "<!--dx-text-->".to_string();

        let slot_id = 0;
        let binding_id = self.alloc_binding_id();

        self.bindings.push(CompiledBinding {
            id: binding_id,
            expression: expr.expression.clone(),
            template_id,
            slot_id,
            binding_type: BindingType::Text,
            dependencies: extract_dependencies(&expr.expression),
        });

        self.templates.push(CompiledTemplate {
            id: template_id,
            html,
            slots: vec![TemplateSlot {
                id: slot_id,
                slot_type: SlotType::Text,
                path: path.to_string(),
            }],
        });

        Ok(())
    }

    /// Compile an if block
    fn compile_if_block(&mut self, if_block: &IfBlockNode, path: &str) -> CompilerResult<()> {
        let template_id = self.alloc_template_id();
        let html = "<!--dx-if-->".to_string();

        let slot_id = 0;
        let binding_id = self.alloc_binding_id();

        // Main condition binding
        self.bindings.push(CompiledBinding {
            id: binding_id,
            expression: if_block.condition.clone(),
            template_id,
            slot_id,
            binding_type: BindingType::Visibility,
            dependencies: extract_dependencies(&if_block.condition),
        });

        self.templates.push(CompiledTemplate {
            id: template_id,
            html,
            slots: vec![TemplateSlot {
                id: slot_id,
                slot_type: SlotType::Element,
                path: path.to_string(),
            }],
        });

        // Compile then branch
        for (i, node) in if_block.then_branch.iter().enumerate() {
            let child_path = format!("{}/then/{}", path, i);
            self.compile_node(node, &child_path)?;
        }

        // Compile else-if branches
        for (branch_idx, (condition, nodes)) in if_block.else_if_branches.iter().enumerate() {
            let branch_binding_id = self.alloc_binding_id();
            self.bindings.push(CompiledBinding {
                id: branch_binding_id,
                expression: condition.clone(),
                template_id,
                slot_id: (branch_idx + 1) as u16,
                binding_type: BindingType::Visibility,
                dependencies: extract_dependencies(condition),
            });

            for (i, node) in nodes.iter().enumerate() {
                let child_path = format!("{}/elseif/{}/{}", path, branch_idx, i);
                self.compile_node(node, &child_path)?;
            }
        }

        // Compile else branch
        if let Some(else_nodes) = &if_block.else_branch {
            for (i, node) in else_nodes.iter().enumerate() {
                let child_path = format!("{}/else/{}", path, i);
                self.compile_node(node, &child_path)?;
            }
        }

        Ok(())
    }

    /// Compile an each block
    fn compile_each_block(&mut self, each_block: &EachBlockNode, path: &str) -> CompilerResult<()> {
        let template_id = self.alloc_template_id();
        let html = "<!--dx-each-->".to_string();

        let slot_id = 0;
        let binding_id = self.alloc_binding_id();

        // List binding
        self.bindings.push(CompiledBinding {
            id: binding_id,
            expression: each_block.iterable.clone(),
            template_id,
            slot_id,
            binding_type: BindingType::List,
            dependencies: extract_dependencies(&each_block.iterable),
        });

        self.templates.push(CompiledTemplate {
            id: template_id,
            html,
            slots: vec![TemplateSlot {
                id: slot_id,
                slot_type: SlotType::Element,
                path: path.to_string(),
            }],
        });

        // Compile body as a nested template
        for (i, node) in each_block.body.iter().enumerate() {
            let child_path = format!("{}/item/{}", path, i);
            self.compile_node(node, &child_path)?;
        }

        Ok(())
    }

    /// Extract props schema from AST
    fn extract_props_schema(&self, ast: &DxFile) -> Option<PropsSchema> {
        // Look for Props definition in scripts
        for script in &ast.scripts {
            if let Some(props_def) = &script.props {
                return Some(PropsSchema {
                    fields: props_def
                        .fields
                        .iter()
                        .map(|f| PropField {
                            name: f.name.clone(),
                            type_name: f.type_name.clone(),
                            required: !f.optional,
                            default_value: f.default.clone(),
                        })
                        .collect(),
                });
            }
        }
        None
    }
}

impl Default for BinaryCompiler {
    fn default() -> Self {
        Self::new()
    }
}

/// Extract dependencies from an expression
fn extract_dependencies(expr: &str) -> Vec<String> {
    // Simple heuristic: find all identifiers that look like variable names
    let mut deps = Vec::new();
    let mut current = String::new();
    let mut in_string = false;
    let mut string_char = '"';

    for c in expr.chars() {
        if in_string {
            if c == string_char {
                in_string = false;
            }
            continue;
        }

        if c == '"' || c == '\'' {
            in_string = true;
            string_char = c;
            continue;
        }

        if c.is_alphanumeric() || c == '_' {
            current.push(c);
        } else {
            if !current.is_empty()
                && current.chars().next().map(|c| c.is_alphabetic()).unwrap_or(false)
            {
                // Skip keywords and common functions
                let keywords = [
                    "if", "else", "for", "while", "let", "const", "fn", "true", "false", "null",
                    "None", "Some",
                ];
                if !keywords.contains(&current.as_str()) && !deps.contains(&current) {
                    deps.push(current.clone());
                }
            }
            current.clear();
        }
    }

    // Don't forget the last word
    if !current.is_empty() && current.chars().next().map(|c| c.is_alphabetic()).unwrap_or(false) {
        let keywords = [
            "if", "else", "for", "while", "let", "const", "fn", "true", "false", "null", "None",
            "Some",
        ];
        if !keywords.contains(&current.as_str()) && !deps.contains(&current) {
            deps.push(current);
        }
    }

    deps
}

/// Escape attribute value for HTML
fn escape_attr(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// Serialize DXOB to bytes
pub fn serialize_dxob(dxob: &DxObjectBinary) -> Vec<u8> {
    let mut output = Vec::new();

    // Magic bytes
    output.extend_from_slice(DXOB_MAGIC);

    // Version
    output.push(DXOB_VERSION);

    // Component type
    output.push(match dxob.component_type {
        ComponentType::Component => 0x01,
        ComponentType::Page => 0x02,
        ComponentType::Layout => 0x03,
    });

    // Name length + name
    let name_bytes = dxob.name.as_bytes();
    output.extend_from_slice(&(name_bytes.len() as u16).to_le_bytes());
    output.extend_from_slice(name_bytes);

    // String table
    output.extend_from_slice(&(dxob.strings.len() as u32).to_le_bytes());
    for s in &dxob.strings {
        let bytes = s.as_bytes();
        output.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
        output.extend_from_slice(bytes);
    }

    // Templates
    output.extend_from_slice(&(dxob.templates.len() as u16).to_le_bytes());
    for template in &dxob.templates {
        output.extend_from_slice(&template.id.to_le_bytes());
        let html_bytes = template.html.as_bytes();
        output.extend_from_slice(&(html_bytes.len() as u32).to_le_bytes());
        output.extend_from_slice(html_bytes);

        output.extend_from_slice(&(template.slots.len() as u16).to_le_bytes());
        for slot in &template.slots {
            output.extend_from_slice(&slot.id.to_le_bytes());
            output.push(match slot.slot_type {
                SlotType::Text => 0x01,
                SlotType::Attribute => 0x02,
                SlotType::Element => 0x03,
                SlotType::Children => 0x04,
            });
            let path_bytes = slot.path.as_bytes();
            output.extend_from_slice(&(path_bytes.len() as u16).to_le_bytes());
            output.extend_from_slice(path_bytes);
        }
    }

    // Bindings
    output.extend_from_slice(&(dxob.bindings.len() as u32).to_le_bytes());
    for binding in &dxob.bindings {
        output.extend_from_slice(&binding.id.to_le_bytes());
        let expr_bytes = binding.expression.as_bytes();
        output.extend_from_slice(&(expr_bytes.len() as u16).to_le_bytes());
        output.extend_from_slice(expr_bytes);
        output.extend_from_slice(&binding.template_id.to_le_bytes());
        output.extend_from_slice(&binding.slot_id.to_le_bytes());
        output.push(match binding.binding_type {
            BindingType::Text => 0x01,
            BindingType::Attribute => 0x02,
            BindingType::Property => 0x03,
            BindingType::Class => 0x04,
            BindingType::Style => 0x05,
            BindingType::Visibility => 0x06,
            BindingType::List => 0x07,
        });
    }

    // Events
    output.extend_from_slice(&(dxob.events.len() as u32).to_le_bytes());
    for event in &dxob.events {
        output.extend_from_slice(&event.id.to_le_bytes());
        let type_bytes = event.event_type.as_bytes();
        output.extend_from_slice(&(type_bytes.len() as u8).to_le_bytes());
        output.extend_from_slice(type_bytes);
        let handler_bytes = event.handler.as_bytes();
        output.extend_from_slice(&(handler_bytes.len() as u16).to_le_bytes());
        output.extend_from_slice(handler_bytes);
    }

    // CSS classes
    output.extend_from_slice(&(dxob.css_classes.len() as u16).to_le_bytes());
    for class in &dxob.css_classes {
        let bytes = class.as_bytes();
        output.extend_from_slice(&(bytes.len() as u16).to_le_bytes());
        output.extend_from_slice(bytes);
    }

    output
}

/// Deserialize DXOB from bytes
pub fn deserialize_dxob(data: &[u8]) -> CompilerResult<DxObjectBinary> {
    if data.len() < 6 {
        return Err(CompilerError::Parse("Invalid DXOB: too short".into()));
    }

    if &data[0..4] != DXOB_MAGIC {
        return Err(CompilerError::Parse("Invalid DXOB: bad magic".into()));
    }

    if data[4] != DXOB_VERSION {
        return Err(CompilerError::Parse(format!("Unsupported DXOB version: {}", data[4])));
    }

    // For now, return a placeholder - full deserialization would be implemented similarly
    Ok(DxObjectBinary {
        name: "deserialized".into(),
        component_type: ComponentType::Component,
        strings: Vec::new(),
        templates: Vec::new(),
        bindings: Vec::new(),
        events: Vec::new(),
        css_classes: Vec::new(),
        props_schema: None,
        source_map: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compile_simple_component() {
        let source = r#"
<script lang="rust">
struct Props {
    message: String,
}
</script>

<component>
    <div class="container">
        <h1>{message}</h1>
    </div>
</component>
"#;

        let mut compiler = BinaryCompiler::new();
        let result = compiler.compile_source(source, "Test", Some(BlockType::Component));
        assert!(result.is_ok());

        let dxob = result.unwrap();
        assert_eq!(dxob.name, "Test");
        assert_eq!(dxob.component_type, ComponentType::Component);
        assert!(!dxob.templates.is_empty());
    }

    #[test]
    fn test_extract_dependencies() {
        let deps = extract_dependencies("count + 1");
        assert!(deps.contains(&"count".to_string()));

        let deps2 = extract_dependencies("user.name + \" says hello\"");
        assert!(deps2.contains(&"user".to_string()));
    }

    #[test]
    fn test_serialize_dxob() {
        let dxob = DxObjectBinary {
            name: "Test".into(),
            component_type: ComponentType::Component,
            strings: vec!["hello".into()],
            templates: Vec::new(),
            bindings: Vec::new(),
            events: Vec::new(),
            css_classes: vec!["container".into()],
            props_schema: None,
            source_map: None,
        };

        let bytes = serialize_dxob(&dxob);
        assert!(bytes.starts_with(DXOB_MAGIC));
    }
}
