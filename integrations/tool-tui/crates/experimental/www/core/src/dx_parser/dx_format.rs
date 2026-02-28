//! # DX Format Parser
//!
//! Parser for DX-WWW component (.cp) and page (.pg) files.
//!
//! ## File Format
//!
//! ```dx
//! <script lang="rust">
//! struct Props {
//!     message: String,
//!     count: i32,
//! }
//!
//! let mut count = 0;
//!
//! fn handleClick() {
//!     count += 1;
//! }
//! </script>
//!
//! <component>
//!     <div class="container mx-auto p-4">
//!         <h1 class="text-2xl font-bold">{message}</h1>
//!         <p class="text-gray-600">Count: {count}</p>
//!         {#if count > 5}
//!             <span>High count!</span>
//!         {:else}
//!             <span>Keep clicking</span>
//!         {/if}
//!         <button onClick={handleClick}>Click me</button>
//!     </div>
//! </component>
//! ```

use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

/// DX format parsing errors
#[derive(Debug, Error)]
pub enum DxFormatError {
    #[error("Missing <{0}> block")]
    MissingBlock(String),

    #[error("Invalid script language: {0}")]
    InvalidLanguage(String),

    #[error("Parse error at line {line}: {message}")]
    ParseError { line: usize, message: String },

    #[error("Unclosed directive: {0}")]
    UnclosedDirective(String),

    #[error("Invalid expression: {0}")]
    InvalidExpression(String),
}

pub type DxResult<T> = Result<T, DxFormatError>;

/// Supported script languages
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ScriptLanguage {
    Rust,
    Python,
    JavaScript,
    TypeScript,
    Go,
}

impl ScriptLanguage {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "rust" | "rs" => Some(Self::Rust),
            "python" | "py" => Some(Self::Python),
            "javascript" | "js" => Some(Self::JavaScript),
            "typescript" | "ts" => Some(Self::TypeScript),
            "go" => Some(Self::Go),
            _ => None,
        }
    }

    pub fn extension(&self) -> &'static str {
        match self {
            Self::Rust => "rs",
            Self::Python => "py",
            Self::JavaScript => "js",
            Self::TypeScript => "ts",
            Self::Go => "go",
        }
    }
}

/// A script block from a DX file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScriptBlock {
    /// The language of this script
    pub language: ScriptLanguage,
    /// The raw script content
    pub content: String,
    /// Extracted props definition (if any)
    pub props: Option<PropsDefinition>,
    /// Extracted reactive state variables
    pub state_vars: Vec<StateVariable>,
    /// Extracted functions
    pub functions: Vec<FunctionDef>,
}

/// Props definition extracted from script
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PropsDefinition {
    /// Props struct/interface name
    pub name: String,
    /// Individual prop fields
    pub fields: Vec<PropField>,
}

/// A single prop field
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PropField {
    pub name: String,
    pub type_name: String,
    pub optional: bool,
    pub default: Option<String>,
}

/// A reactive state variable
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateVariable {
    pub name: String,
    pub type_name: Option<String>,
    pub initial_value: Option<String>,
    pub is_reactive: bool,
}

/// A function definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionDef {
    pub name: String,
    pub params: Vec<(String, String)>, // (name, type)
    pub return_type: Option<String>,
    pub body: String,
    pub is_async: bool,
}

/// Type of component block
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BlockType {
    Component,
    Page,
    Layout,
}

impl BlockType {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "component" => Some(Self::Component),
            "page" => Some(Self::Page),
            "layout" => Some(Self::Layout),
            _ => None,
        }
    }

    pub fn tag_name(&self) -> &'static str {
        match self {
            Self::Component => "component",
            Self::Page => "page",
            Self::Layout => "layout",
        }
    }
}

/// A template node in the AST
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TemplateNode {
    /// Static text content
    Text(String),
    /// HTML element
    Element(ElementNode),
    /// Expression interpolation: {expression}
    Expression(ExpressionNode),
    /// If directive: {#if condition}...{/if}
    IfBlock(IfBlockNode),
    /// Each directive: {#each items as item}...{/each}
    EachBlock(EachBlockNode),
    /// Await directive: {#await promise}...{/await}
    AwaitBlock(AwaitBlockNode),
    /// Key directive: {#key value}...{/key}
    KeyBlock(KeyBlockNode),
    /// Slot element
    Slot(SlotNode),
    /// Component instance
    Component(ComponentInstance),
}

/// An HTML element node
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElementNode {
    /// Tag name (div, span, button, etc.)
    pub tag: String,
    /// Static attributes
    pub attributes: HashMap<String, AttributeValue>,
    /// Event handlers
    pub events: HashMap<String, String>,
    /// Bindings (bind:value, etc.)
    pub bindings: HashMap<String, String>,
    /// Class directives (class:active={isActive})
    pub class_directives: HashMap<String, String>,
    /// Use directives (use:action)
    pub use_directives: Vec<String>,
    /// Transition directive
    pub transition: Option<String>,
    /// Child nodes
    pub children: Vec<TemplateNode>,
    /// Is self-closing?
    pub self_closing: bool,
}

/// Attribute value types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AttributeValue {
    /// Static string value
    Static(String),
    /// Dynamic expression
    Dynamic(String),
}

/// Expression node: {expression}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExpressionNode {
    pub expression: String,
    /// Is this HTML-escaped?
    pub escaped: bool,
}

/// If block: {#if}...{:else if}...{:else}...{/if}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IfBlockNode {
    pub condition: String,
    pub then_branch: Vec<TemplateNode>,
    pub else_if_branches: Vec<(String, Vec<TemplateNode>)>,
    pub else_branch: Option<Vec<TemplateNode>>,
}

/// Each block: {#each items as item, index}...{/each}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EachBlockNode {
    pub iterable: String,
    pub item_name: String,
    pub index_name: Option<String>,
    pub key: Option<String>,
    pub body: Vec<TemplateNode>,
    pub empty_branch: Option<Vec<TemplateNode>>,
}

/// Await block: {#await promise}...{:then}...{:catch}...{/await}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AwaitBlockNode {
    pub promise: String,
    pub pending_branch: Vec<TemplateNode>,
    pub then_name: Option<String>,
    pub then_branch: Vec<TemplateNode>,
    pub catch_name: Option<String>,
    pub catch_branch: Vec<TemplateNode>,
}

/// Key block: {#key value}...{/key}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyBlockNode {
    pub key: String,
    pub body: Vec<TemplateNode>,
}

/// Slot node
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlotNode {
    pub name: Option<String>,
    pub fallback: Vec<TemplateNode>,
}

/// Component instance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentInstance {
    pub name: String,
    pub props: HashMap<String, AttributeValue>,
    pub events: HashMap<String, String>,
    pub slots: HashMap<String, Vec<TemplateNode>>,
    pub children: Vec<TemplateNode>,
}

/// Parsed DX file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DxFile {
    /// File type (component, page, layout)
    pub file_type: BlockType,
    /// Script blocks (can have multiple for different languages)
    pub scripts: Vec<ScriptBlock>,
    /// Template AST
    pub template: Vec<TemplateNode>,
    /// Extracted CSS class names (for dx-style processing)
    pub css_classes: Vec<String>,
    /// Component imports
    pub imports: Vec<ComponentImport>,
}

/// Component import
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentImport {
    pub name: String,
    pub path: String,
}

/// Parse a DX file (.pg, .cp, .lyt)
pub fn parse_dx_file(source: &str, expected_type: Option<BlockType>) -> DxResult<DxFile> {
    let scripts = extract_script_blocks(source)?;
    let (file_type, template_content) = extract_template_block(source, expected_type)?;
    let template = parse_template(&template_content)?;
    let css_classes = extract_css_classes(&template);
    let imports = extract_imports(&scripts);

    Ok(DxFile {
        file_type,
        scripts,
        template,
        css_classes,
        imports,
    })
}

/// Extract all script blocks from source
fn extract_script_blocks(source: &str) -> DxResult<Vec<ScriptBlock>> {
    let script_regex = Regex::new(r#"<script(?:\s+lang="([^"]*)")?\s*>([\s\S]*?)</script>"#)
        .expect("Invalid regex");

    let mut scripts = Vec::new();

    for cap in script_regex.captures_iter(source) {
        let lang_str = cap.get(1).map(|m| m.as_str()).unwrap_or("rust");
        let content = cap.get(2).map(|m| m.as_str()).unwrap_or("");

        let language = ScriptLanguage::from_str(lang_str)
            .ok_or_else(|| DxFormatError::InvalidLanguage(lang_str.to_string()))?;

        let props = extract_props_definition(content, language);
        let state_vars = extract_state_variables(content, language);
        let functions = extract_functions(content, language);

        scripts.push(ScriptBlock {
            language,
            content: content.to_string(),
            props,
            state_vars,
            functions,
        });
    }

    // Default to Rust if no script block
    if scripts.is_empty() {
        scripts.push(ScriptBlock {
            language: ScriptLanguage::Rust,
            content: String::new(),
            props: None,
            state_vars: Vec::new(),
            functions: Vec::new(),
        });
    }

    Ok(scripts)
}

/// Extract the template block (<component>, <page>, or <layout>)
fn extract_template_block(
    source: &str,
    expected_type: Option<BlockType>,
) -> DxResult<(BlockType, String)> {
    // Try each block type
    for block_type in [BlockType::Component, BlockType::Page, BlockType::Layout] {
        let tag = block_type.tag_name();
        let pattern = format!(r"<{}>([\s\S]*?)</{}>", tag, tag);
        let regex = Regex::new(&pattern).expect("Invalid regex");

        if let Some(cap) = regex.captures(source) {
            if let Some(expected) = expected_type {
                if block_type != expected {
                    continue;
                }
            }
            let content = cap.get(1).map(|m| m.as_str()).unwrap_or("");
            return Ok((block_type, content.to_string()));
        }
    }

    let expected_tag = expected_type.map(|t| t.tag_name()).unwrap_or("component|page|layout");
    Err(DxFormatError::MissingBlock(expected_tag.to_string()))
}

/// Parse template content into AST
fn parse_template(content: &str) -> DxResult<Vec<TemplateNode>> {
    let mut nodes = Vec::new();
    let mut chars: Vec<char> = content.chars().collect();
    let mut pos = 0;

    while pos < chars.len() {
        // Skip whitespace at start
        while pos < chars.len() && chars[pos].is_whitespace() {
            pos += 1;
        }
        if pos >= chars.len() {
            break;
        }

        if chars[pos] == '{' && pos + 1 < chars.len() && chars[pos + 1] == '#' {
            // Directive block: {#if}, {#each}, etc.
            let node = parse_directive_block(&chars, &mut pos)?;
            nodes.push(node);
        } else if chars[pos] == '{' {
            // Expression: {expression}
            let node = parse_expression(&chars, &mut pos)?;
            nodes.push(node);
        } else if chars[pos] == '<' {
            // Element or component
            let node = parse_element(&chars, &mut pos)?;
            nodes.push(node);
        } else {
            // Text content
            let text = parse_text(&chars, &mut pos);
            if !text.trim().is_empty() {
                nodes.push(TemplateNode::Text(text));
            }
        }
    }

    Ok(nodes)
}

/// Parse a directive block ({#if}, {#each}, etc.)
fn parse_directive_block(chars: &[char], pos: &mut usize) -> DxResult<TemplateNode> {
    // Skip {#
    *pos += 2;

    // Read directive name
    let mut name = String::new();
    while *pos < chars.len() && chars[*pos].is_alphabetic() {
        name.push(chars[*pos]);
        *pos += 1;
    }

    // Skip whitespace
    while *pos < chars.len() && chars[*pos].is_whitespace() {
        *pos += 1;
    }

    // Read condition/expression until }
    let mut expr = String::new();
    let mut brace_depth = 1;
    while *pos < chars.len() && brace_depth > 0 {
        if chars[*pos] == '{' {
            brace_depth += 1;
        } else if chars[*pos] == '}' {
            brace_depth -= 1;
            if brace_depth == 0 {
                break;
            }
        }
        expr.push(chars[*pos]);
        *pos += 1;
    }
    *pos += 1; // Skip closing }

    let expr = expr.trim().to_string();

    match name.as_str() {
        "if" => parse_if_block(chars, pos, expr),
        "each" => parse_each_block(chars, pos, expr),
        "await" => parse_await_block(chars, pos, expr),
        "key" => parse_key_block(chars, pos, expr),
        _ => Err(DxFormatError::UnclosedDirective(name)),
    }
}

/// Parse {#if condition}...{:else if}...{:else}...{/if}
fn parse_if_block(chars: &[char], pos: &mut usize, condition: String) -> DxResult<TemplateNode> {
    let mut then_branch = Vec::new();
    let mut else_if_branches = Vec::new();
    let mut else_branch = None;

    loop {
        // Parse content until we hit {:else if}, {:else}, or {/if}
        let (nodes, terminator) = parse_until_terminator(chars, pos, &["else if", "else", "/if"])?;

        match terminator.as_str() {
            "/if" => {
                if else_branch.is_some() || !else_if_branches.is_empty() {
                    // This is the last else branch content
                    if else_branch.is_none() {
                        else_branch = Some(nodes);
                    }
                } else {
                    then_branch = nodes;
                }
                break;
            }
            "else if" => {
                if then_branch.is_empty() && else_if_branches.is_empty() {
                    then_branch = nodes;
                } else if let Some(ref mut eb) = else_branch {
                    eb.extend(nodes);
                } else {
                    else_if_branches.push((condition.clone(), nodes));
                }
                // Read the else-if condition
                let cond = parse_directive_expression(chars, pos)?;
                else_if_branches.push((cond, Vec::new()));
            }
            "else" => {
                if then_branch.is_empty() {
                    then_branch = nodes;
                }
                else_branch = Some(Vec::new());
            }
            _ => break,
        }
    }

    Ok(TemplateNode::IfBlock(IfBlockNode {
        condition,
        then_branch,
        else_if_branches,
        else_branch,
    }))
}

/// Parse {#each items as item, index}...{/each}
fn parse_each_block(chars: &[char], pos: &mut usize, expr: String) -> DxResult<TemplateNode> {
    // Parse "items as item, index" or "items as item (key)"
    let parts: Vec<&str> = expr.splitn(2, " as ").collect();
    let iterable = parts.first().unwrap_or(&"").trim().to_string();

    let (item_name, index_name, key) = if parts.len() > 1 {
        let binding = parts[1].trim();
        // Check for (key) at end
        let (binding, key) = if binding.contains('(') {
            let key_start = binding.find('(').unwrap();
            let key_end = binding.find(')').unwrap_or(binding.len());
            let key = binding[key_start + 1..key_end].trim().to_string();
            (binding[..key_start].trim(), Some(key))
        } else {
            (binding, None)
        };

        // Check for index
        let names: Vec<&str> = binding.split(',').collect();
        let item = names.first().unwrap_or(&"item").trim().to_string();
        let index = names.get(1).map(|s| s.trim().to_string());
        (item, index, key)
    } else {
        ("item".to_string(), None, None)
    };

    let (body, _) = parse_until_terminator(chars, pos, &["/each"])?;

    Ok(TemplateNode::EachBlock(EachBlockNode {
        iterable,
        item_name,
        index_name,
        key,
        body,
        empty_branch: None,
    }))
}

/// Parse {#await promise}...{:then}...{:catch}...{/await}
fn parse_await_block(chars: &[char], pos: &mut usize, promise: String) -> DxResult<TemplateNode> {
    let (pending, _) = parse_until_terminator(chars, pos, &["then", "/await"])?;
    let (then_branch, term) = if pending.is_empty() {
        parse_until_terminator(chars, pos, &["catch", "/await"])?
    } else {
        (Vec::new(), "".to_string())
    };
    let catch_branch = if term == "catch" {
        let (cb, _) = parse_until_terminator(chars, pos, &["/await"])?;
        cb
    } else {
        Vec::new()
    };

    Ok(TemplateNode::AwaitBlock(AwaitBlockNode {
        promise,
        pending_branch: pending,
        then_name: None,
        then_branch,
        catch_name: None,
        catch_branch,
    }))
}

/// Parse {#key value}...{/key}
fn parse_key_block(chars: &[char], pos: &mut usize, key: String) -> DxResult<TemplateNode> {
    let (body, _) = parse_until_terminator(chars, pos, &["/key"])?;
    Ok(TemplateNode::KeyBlock(KeyBlockNode { key, body }))
}

/// Parse until we hit one of the terminators
fn parse_until_terminator(
    chars: &[char],
    pos: &mut usize,
    terminators: &[&str],
) -> DxResult<(Vec<TemplateNode>, String)> {
    let mut nodes = Vec::new();

    while *pos < chars.len() {
        // Check for terminator
        if chars[*pos] == '{' && *pos + 1 < chars.len() && chars[*pos + 1] == ':' {
            // {:else} or {:else if} or {:then} or {:catch}
            *pos += 2;
            let mut term = String::new();
            while *pos < chars.len() && chars[*pos] != '}' {
                term.push(chars[*pos]);
                *pos += 1;
            }
            *pos += 1; // Skip }
            let term = term.trim();
            for t in terminators {
                if term == *t || term.starts_with(t) {
                    return Ok((nodes, term.to_string()));
                }
            }
        } else if chars[*pos] == '{' && *pos + 1 < chars.len() && chars[*pos + 1] == '/' {
            // {/if}, {/each}, etc.
            *pos += 2;
            let mut term = String::new();
            while *pos < chars.len() && chars[*pos] != '}' {
                term.push(chars[*pos]);
                *pos += 1;
            }
            *pos += 1; // Skip }
            let term = format!("/{}", term.trim());
            for t in terminators {
                if term == *t {
                    return Ok((nodes, term));
                }
            }
        } else if chars[*pos] == '{' && *pos + 1 < chars.len() && chars[*pos + 1] == '#' {
            // Nested directive
            let node = parse_directive_block(chars, pos)?;
            nodes.push(node);
        } else if chars[*pos] == '{' {
            // Expression
            let node = parse_expression(chars, pos)?;
            nodes.push(node);
        } else if chars[*pos] == '<' {
            // Element
            let node = parse_element(chars, pos)?;
            nodes.push(node);
        } else {
            // Text
            let text = parse_text_until_special(chars, pos);
            if !text.trim().is_empty() {
                nodes.push(TemplateNode::Text(text));
            }
        }
    }

    Err(DxFormatError::UnclosedDirective(terminators.first().unwrap_or(&"").to_string()))
}

/// Parse a directive expression (after :else if, etc.)
fn parse_directive_expression(chars: &[char], pos: &mut usize) -> DxResult<String> {
    // Skip to expression
    while *pos < chars.len() && chars[*pos].is_whitespace() {
        *pos += 1;
    }

    let mut expr = String::new();
    let mut brace_depth = 0;

    while *pos < chars.len() {
        if chars[*pos] == '{' {
            brace_depth += 1;
        } else if chars[*pos] == '}' {
            if brace_depth == 0 {
                *pos += 1;
                break;
            }
            brace_depth -= 1;
        }
        expr.push(chars[*pos]);
        *pos += 1;
    }

    Ok(expr.trim().to_string())
}

/// Parse an expression: {expression}
fn parse_expression(chars: &[char], pos: &mut usize) -> DxResult<TemplateNode> {
    *pos += 1; // Skip {

    let mut expr = String::new();
    let mut brace_depth = 1;

    while *pos < chars.len() && brace_depth > 0 {
        if chars[*pos] == '{' {
            brace_depth += 1;
        } else if chars[*pos] == '}' {
            brace_depth -= 1;
            if brace_depth == 0 {
                break;
            }
        }
        expr.push(chars[*pos]);
        *pos += 1;
    }
    *pos += 1; // Skip }

    Ok(TemplateNode::Expression(ExpressionNode {
        expression: expr.trim().to_string(),
        escaped: true,
    }))
}

/// Parse an HTML element
fn parse_element(chars: &[char], pos: &mut usize) -> DxResult<TemplateNode> {
    *pos += 1; // Skip <

    // Parse tag name
    let mut tag = String::new();
    while *pos < chars.len()
        && !chars[*pos].is_whitespace()
        && chars[*pos] != '>'
        && chars[*pos] != '/'
    {
        tag.push(chars[*pos]);
        *pos += 1;
    }

    // Check if it's a component (PascalCase) or slot
    let is_component = tag.chars().next().map(|c| c.is_uppercase()).unwrap_or(false);
    let is_slot = tag == "slot";

    let mut attributes = HashMap::new();
    let mut events = HashMap::new();
    let mut bindings = HashMap::new();
    let mut class_directives = HashMap::new();
    let mut use_directives = Vec::new();
    let mut transition = None;

    // Parse attributes
    loop {
        // Skip whitespace
        while *pos < chars.len() && chars[*pos].is_whitespace() {
            *pos += 1;
        }

        if *pos >= chars.len() {
            break;
        }

        // Check for self-closing or end
        if chars[*pos] == '/' {
            *pos += 1; // Skip /
            while *pos < chars.len() && chars[*pos] != '>' {
                *pos += 1;
            }
            *pos += 1; // Skip >

            if is_slot {
                return Ok(TemplateNode::Slot(SlotNode {
                    name: attributes.get("name").and_then(|v| match v {
                        AttributeValue::Static(s) => Some(s.clone()),
                        _ => None,
                    }),
                    fallback: Vec::new(),
                }));
            }

            if is_component {
                return Ok(TemplateNode::Component(ComponentInstance {
                    name: tag,
                    props: attributes,
                    events,
                    slots: HashMap::new(),
                    children: Vec::new(),
                }));
            }

            return Ok(TemplateNode::Element(ElementNode {
                tag,
                attributes,
                events,
                bindings,
                class_directives,
                use_directives,
                transition,
                children: Vec::new(),
                self_closing: true,
            }));
        }

        if chars[*pos] == '>' {
            *pos += 1;
            break;
        }

        // Parse attribute name
        let mut attr_name = String::new();
        while *pos < chars.len()
            && !chars[*pos].is_whitespace()
            && chars[*pos] != '='
            && chars[*pos] != '>'
            && chars[*pos] != '/'
        {
            attr_name.push(chars[*pos]);
            *pos += 1;
        }

        if attr_name.is_empty() {
            continue;
        }

        // Check for value
        while *pos < chars.len() && chars[*pos].is_whitespace() {
            *pos += 1;
        }

        let value = if *pos < chars.len() && chars[*pos] == '=' {
            *pos += 1; // Skip =
            while *pos < chars.len() && chars[*pos].is_whitespace() {
                *pos += 1;
            }

            if *pos < chars.len() && chars[*pos] == '"' {
                // Static string value
                *pos += 1;
                let mut val = String::new();
                while *pos < chars.len() && chars[*pos] != '"' {
                    val.push(chars[*pos]);
                    *pos += 1;
                }
                *pos += 1; // Skip "
                AttributeValue::Static(val)
            } else if *pos < chars.len() && chars[*pos] == '{' {
                // Dynamic value
                *pos += 1;
                let mut val = String::new();
                let mut brace_depth = 1;
                while *pos < chars.len() && brace_depth > 0 {
                    if chars[*pos] == '{' {
                        brace_depth += 1;
                    } else if chars[*pos] == '}' {
                        brace_depth -= 1;
                        if brace_depth == 0 {
                            break;
                        }
                    }
                    val.push(chars[*pos]);
                    *pos += 1;
                }
                *pos += 1; // Skip }
                AttributeValue::Dynamic(val.trim().to_string())
            } else {
                AttributeValue::Static(String::new())
            }
        } else {
            // Boolean attribute
            AttributeValue::Static("true".to_string())
        };

        // Categorize attribute
        if attr_name.starts_with("on:") || attr_name.starts_with("on") {
            let event_name = if attr_name.starts_with("on:") {
                attr_name[3..].to_string()
            } else {
                // onClick -> click
                let name = attr_name[2..].to_string();
                name[0..1].to_lowercase() + &name[1..]
            };
            if let AttributeValue::Dynamic(handler) = value {
                events.insert(event_name, handler);
            }
        } else if attr_name.starts_with("bind:") {
            let binding_name = attr_name[5..].to_string();
            if let AttributeValue::Dynamic(expr) = value {
                bindings.insert(binding_name, expr);
            }
        } else if attr_name.starts_with("class:") {
            let class_name = attr_name[6..].to_string();
            if let AttributeValue::Dynamic(expr) = value {
                class_directives.insert(class_name, expr);
            }
        } else if attr_name.starts_with("use:") {
            use_directives.push(attr_name[4..].to_string());
        } else if attr_name.starts_with("transition:") {
            transition = Some(attr_name[11..].to_string());
        } else {
            attributes.insert(attr_name, value);
        }
    }

    // Parse children
    let children = parse_element_children(chars, pos, &tag)?;

    if is_slot {
        return Ok(TemplateNode::Slot(SlotNode {
            name: attributes.get("name").and_then(|v| match v {
                AttributeValue::Static(s) => Some(s.clone()),
                _ => None,
            }),
            fallback: children,
        }));
    }

    if is_component {
        return Ok(TemplateNode::Component(ComponentInstance {
            name: tag,
            props: attributes,
            events,
            slots: HashMap::new(),
            children,
        }));
    }

    Ok(TemplateNode::Element(ElementNode {
        tag,
        attributes,
        events,
        bindings,
        class_directives,
        use_directives,
        transition,
        children,
        self_closing: false,
    }))
}

/// Parse element children until closing tag
fn parse_element_children(
    chars: &[char],
    pos: &mut usize,
    tag: &str,
) -> DxResult<Vec<TemplateNode>> {
    let mut children = Vec::new();
    let close_tag = format!("</{}>", tag);

    while *pos < chars.len() {
        // Check for closing tag
        let remaining: String = chars[*pos..].iter().collect();
        if remaining.starts_with(&close_tag) {
            *pos += close_tag.len();
            break;
        }

        if chars[*pos] == '{' && *pos + 1 < chars.len() && chars[*pos + 1] == '#' {
            let node = parse_directive_block(chars, pos)?;
            children.push(node);
        } else if chars[*pos] == '{' {
            let node = parse_expression(chars, pos)?;
            children.push(node);
        } else if chars[*pos] == '<' {
            // Check if it's a closing tag for parent (shouldn't happen but handle gracefully)
            if *pos + 1 < chars.len() && chars[*pos + 1] == '/' {
                break;
            }
            let node = parse_element(chars, pos)?;
            children.push(node);
        } else {
            let text = parse_text_until_special(chars, pos);
            if !text.trim().is_empty() {
                children.push(TemplateNode::Text(text));
            }
        }
    }

    Ok(children)
}

/// Parse text content
fn parse_text(chars: &[char], pos: &mut usize) -> String {
    let mut text = String::new();
    while *pos < chars.len() && chars[*pos] != '<' && chars[*pos] != '{' {
        text.push(chars[*pos]);
        *pos += 1;
    }
    text
}

/// Parse text until special character or element
fn parse_text_until_special(chars: &[char], pos: &mut usize) -> String {
    let mut text = String::new();
    while *pos < chars.len() {
        if chars[*pos] == '<' || chars[*pos] == '{' {
            break;
        }
        text.push(chars[*pos]);
        *pos += 1;
    }
    text
}

/// Extract props definition from script (language-specific)
fn extract_props_definition(content: &str, language: ScriptLanguage) -> Option<PropsDefinition> {
    match language {
        ScriptLanguage::Rust => {
            // Look for struct Props { ... } or #[derive(Props)] struct ...
            let props_regex = Regex::new(r"(?:pub\s+)?struct\s+Props\s*\{([^}]*)\}").ok()?;
            if let Some(cap) = props_regex.captures(content) {
                let body = cap.get(1)?.as_str();
                let mut fields = Vec::new();

                // Parse field: name: Type,
                let field_regex = Regex::new(r"(\w+)\s*:\s*([^,\n]+)").ok()?;
                for field_cap in field_regex.captures_iter(body) {
                    let name = field_cap.get(1)?.as_str().to_string();
                    let mut type_name = field_cap.get(2)?.as_str().trim().to_string();

                    let optional = type_name.starts_with("Option<");
                    if optional {
                        type_name = type_name
                            .trim_start_matches("Option<")
                            .trim_end_matches('>')
                            .to_string();
                    }

                    fields.push(PropField {
                        name,
                        type_name,
                        optional,
                        default: None,
                    });
                }

                return Some(PropsDefinition {
                    name: "Props".to_string(),
                    fields,
                });
            }
            None
        }
        ScriptLanguage::TypeScript | ScriptLanguage::JavaScript => {
            // Look for interface Props { ... } or type Props = { ... }
            let props_regex =
                Regex::new(r"(?:interface|type)\s+Props\s*(?:=\s*)?\{([^}]*)\}").ok()?;
            if let Some(cap) = props_regex.captures(content) {
                let body = cap.get(1)?.as_str();
                let mut fields = Vec::new();

                // Parse field: name?: type;
                let field_regex = Regex::new(r"(\w+)(\?)?:\s*([^;\n]+)").ok()?;
                for field_cap in field_regex.captures_iter(body) {
                    let name = field_cap.get(1)?.as_str().to_string();
                    let optional = field_cap.get(2).is_some();
                    let type_name = field_cap.get(3)?.as_str().trim().to_string();

                    fields.push(PropField {
                        name,
                        type_name,
                        optional,
                        default: None,
                    });
                }

                return Some(PropsDefinition {
                    name: "Props".to_string(),
                    fields,
                });
            }
            None
        }
        _ => None,
    }
}

/// Extract reactive state variables from script
fn extract_state_variables(content: &str, language: ScriptLanguage) -> Vec<StateVariable> {
    let mut vars = Vec::new();

    match language {
        ScriptLanguage::Rust => {
            // let mut count = 0;
            let let_regex = Regex::new(r"let\s+(mut\s+)?(\w+)(?:\s*:\s*(\w+))?\s*=\s*([^;]+)").ok();
            if let Some(regex) = let_regex {
                for cap in regex.captures_iter(content) {
                    let is_mutable = cap.get(1).is_some();
                    let name = cap.get(2).map(|m| m.as_str().to_string()).unwrap_or_default();
                    let type_name = cap.get(3).map(|m| m.as_str().to_string());
                    let initial = cap.get(4).map(|m| m.as_str().trim().to_string());

                    vars.push(StateVariable {
                        name,
                        type_name,
                        initial_value: initial,
                        is_reactive: is_mutable,
                    });
                }
            }
        }
        ScriptLanguage::JavaScript | ScriptLanguage::TypeScript => {
            // let count = 0; or const count = 0;
            let let_regex =
                Regex::new(r"(?:let|const|var)\s+(\w+)(?:\s*:\s*(\w+))?\s*=\s*([^;\n]+)").ok();
            if let Some(regex) = let_regex {
                for cap in regex.captures_iter(content) {
                    let name = cap.get(1).map(|m| m.as_str().to_string()).unwrap_or_default();
                    let type_name = cap.get(2).map(|m| m.as_str().to_string());
                    let initial = cap.get(3).map(|m| m.as_str().trim().to_string());

                    vars.push(StateVariable {
                        name,
                        type_name,
                        initial_value: initial,
                        is_reactive: true,
                    });
                }
            }
        }
        _ => {}
    }

    vars
}

/// Extract function definitions from script
fn extract_functions(content: &str, language: ScriptLanguage) -> Vec<FunctionDef> {
    let mut funcs = Vec::new();

    match language {
        ScriptLanguage::Rust => {
            // fn name(params) -> ReturnType { body }
            let fn_regex = Regex::new(
                r"(?:pub\s+)?(?:async\s+)?fn\s+(\w+)\s*\(([^)]*)\)(?:\s*->\s*(\w+))?\s*\{",
            )
            .ok();
            if let Some(regex) = fn_regex {
                for cap in regex.captures_iter(content) {
                    let name = cap.get(1).map(|m| m.as_str().to_string()).unwrap_or_default();
                    let _params_str = cap.get(2).map(|m| m.as_str()).unwrap_or("");
                    let return_type = cap.get(3).map(|m| m.as_str().to_string());
                    let is_async = content[..cap.get(0).unwrap().start()].ends_with("async ");

                    funcs.push(FunctionDef {
                        name,
                        params: Vec::new(), // TODO: parse params
                        return_type,
                        body: String::new(), // TODO: extract body
                        is_async,
                    });
                }
            }
        }
        ScriptLanguage::JavaScript | ScriptLanguage::TypeScript => {
            // function name(params) { } or async function name() { } or const name = () => { }
            let fn_regex = Regex::new(r"(?:async\s+)?function\s+(\w+)\s*\(([^)]*)\)").ok();
            if let Some(regex) = fn_regex {
                for cap in regex.captures_iter(content) {
                    let name = cap.get(1).map(|m| m.as_str().to_string()).unwrap_or_default();
                    let is_async = content[..cap.get(0).unwrap().start()].contains("async");

                    funcs.push(FunctionDef {
                        name,
                        params: Vec::new(),
                        return_type: None,
                        body: String::new(),
                        is_async,
                    });
                }
            }
        }
        _ => {}
    }

    funcs
}

/// Extract all CSS class names from template
fn extract_css_classes(nodes: &[TemplateNode]) -> Vec<String> {
    let mut classes = Vec::new();

    fn extract_from_node(node: &TemplateNode, classes: &mut Vec<String>) {
        match node {
            TemplateNode::Element(el) => {
                if let Some(class_attr) = el.attributes.get("class") {
                    if let AttributeValue::Static(class_str) = class_attr {
                        for class in class_str.split_whitespace() {
                            if !classes.contains(&class.to_string()) {
                                classes.push(class.to_string());
                            }
                        }
                    }
                }
                for child in &el.children {
                    extract_from_node(child, classes);
                }
            }
            TemplateNode::Component(comp) => {
                for child in &comp.children {
                    extract_from_node(child, classes);
                }
            }
            TemplateNode::IfBlock(ib) => {
                for child in &ib.then_branch {
                    extract_from_node(child, classes);
                }
                for (_, branch) in &ib.else_if_branches {
                    for child in branch {
                        extract_from_node(child, classes);
                    }
                }
                if let Some(ref else_branch) = ib.else_branch {
                    for child in else_branch {
                        extract_from_node(child, classes);
                    }
                }
            }
            TemplateNode::EachBlock(eb) => {
                for child in &eb.body {
                    extract_from_node(child, classes);
                }
            }
            _ => {}
        }
    }

    for node in nodes {
        extract_from_node(node, &mut classes);
    }

    classes
}

/// Extract component imports from scripts
fn extract_imports(scripts: &[ScriptBlock]) -> Vec<ComponentImport> {
    let mut imports = Vec::new();

    for script in scripts {
        // Rust: use crate::components::Button;
        // JS: import Button from './Button.cp';

        match script.language {
            ScriptLanguage::Rust => {
                let import_regex =
                    Regex::new(r"use\s+(?:crate|super)::(?:components::)?(\w+)").ok();
                if let Some(regex) = import_regex {
                    for cap in regex.captures_iter(&script.content) {
                        if let Some(name) = cap.get(1) {
                            imports.push(ComponentImport {
                                name: name.as_str().to_string(),
                                path: format!("components/{}.cp", name.as_str()),
                            });
                        }
                    }
                }
            }
            ScriptLanguage::JavaScript | ScriptLanguage::TypeScript => {
                let import_regex = Regex::new(r#"import\s+(\w+)\s+from\s+['"]([^'"]+)['"]"#).ok();
                if let Some(regex) = import_regex {
                    for cap in regex.captures_iter(&script.content) {
                        if let (Some(name), Some(path)) = (cap.get(1), cap.get(2)) {
                            imports.push(ComponentImport {
                                name: name.as_str().to_string(),
                                path: path.as_str().to_string(),
                            });
                        }
                    }
                }
            }
            _ => {}
        }
    }

    imports
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_component() {
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

        let result = parse_dx_file(source, Some(BlockType::Component)).unwrap();
        assert_eq!(result.file_type, BlockType::Component);
        assert_eq!(result.scripts.len(), 1);
        assert_eq!(result.scripts[0].language, ScriptLanguage::Rust);
        assert!(result.scripts[0].props.is_some());
    }

    #[test]
    fn test_parse_if_directive() {
        let source = r#"
<component>
    {#if count > 0}
        <p>Positive</p>
    {:else}
        <p>Zero or negative</p>
    {/if}
</component>
"#;

        let result = parse_dx_file(source, Some(BlockType::Component)).unwrap();
        assert!(!result.template.is_empty());
    }

    #[test]
    fn test_parse_each_directive() {
        let source = r#"
<component>
    {#each items as item, index}
        <li>{index}: {item.name}</li>
    {/each}
</component>
"#;

        let result = parse_dx_file(source, Some(BlockType::Component)).unwrap();
        assert!(!result.template.is_empty());
    }

    #[test]
    fn test_extract_css_classes() {
        let source = r#"
<component>
    <div class="container mx-auto p-4">
        <button class="bg-blue-500 text-white px-4 py-2">Click</button>
    </div>
</component>
"#;

        let result = parse_dx_file(source, Some(BlockType::Component)).unwrap();
        assert!(result.css_classes.contains(&"container".to_string()));
        assert!(result.css_classes.contains(&"mx-auto".to_string()));
        assert!(result.css_classes.contains(&"bg-blue-500".to_string()));
    }
}
