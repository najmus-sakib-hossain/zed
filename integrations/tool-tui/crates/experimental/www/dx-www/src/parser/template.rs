//! # Template Parser
//!
//! Parses the `<template>` section of component files.
//! Supports HTML with Svelte-style directives and React-style bindings.

#![allow(missing_docs)]

use std::path::Path;

use crate::error::DxResult;

// =============================================================================
// Parsed Template
// =============================================================================

/// A parsed template section.
#[derive(Debug, Clone)]
pub struct ParsedTemplate {
    /// Source HTML
    pub source: String,

    /// Root nodes
    pub nodes: Vec<TemplateNode>,

    /// All bindings found in the template
    pub bindings: Vec<Binding>,

    /// All event handlers found
    pub event_handlers: Vec<EventHandler>,

    /// All directives found
    pub directives: Vec<TemplateDirective>,
}

/// A node in the template AST.
#[derive(Debug, Clone)]
pub enum TemplateNode {
    /// An HTML element
    Element(ElementNode),
    /// A text node
    Text(String),
    /// An interpolation expression
    Interpolation(String),
    /// A directive block
    Directive(DirectiveNode),
    /// A component reference
    Component(ComponentRef),
    /// A comment
    Comment(String),
}

/// An HTML element node.
#[derive(Debug, Clone)]
pub struct ElementNode {
    /// Tag name
    pub tag: String,
    /// Attributes
    pub attributes: Vec<Attribute>,
    /// Child nodes
    pub children: Vec<TemplateNode>,
    /// Whether this is a self-closing tag
    pub self_closing: bool,
}

/// An attribute on an element.
#[derive(Debug, Clone)]
pub struct Attribute {
    /// Attribute name
    pub name: String,
    /// Attribute value (None for boolean attributes)
    pub value: Option<AttributeValue>,
}

/// An attribute value.
#[derive(Debug, Clone)]
pub enum AttributeValue {
    /// Static string value
    Static(String),
    /// Dynamic expression
    Dynamic(String),
}

/// A directive block.
#[derive(Debug, Clone)]
pub struct DirectiveNode {
    /// Directive type
    pub directive: TemplateDirective,
    /// Child nodes
    pub children: Vec<TemplateNode>,
    /// Else branch (for if directives)
    pub else_branch: Option<Vec<TemplateNode>>,
}

/// Template directive types.
#[derive(Debug, Clone)]
pub enum TemplateDirective {
    /// Conditional: {#if condition}
    If { condition: String },
    /// Iteration: {#each items as item}
    Each {
        items: String,
        item: String,
        index: Option<String>,
    },
    /// Async: {#await promise}
    Await {
        promise: String,
        then_var: Option<String>,
        catch_var: Option<String>,
    },
    /// Key: {#key expression}
    Key { expression: String },
}

/// A component reference.
#[derive(Debug, Clone)]
pub struct ComponentRef {
    /// Component name
    pub name: String,
    /// Props passed to the component
    pub props: Vec<Attribute>,
    /// Children (slot content)
    pub children: Vec<TemplateNode>,
}

/// A data binding in the template.
#[derive(Debug, Clone)]
pub struct Binding {
    /// The expression being bound
    pub expression: String,
    /// The attribute or content being bound to
    pub target: BindingTarget,
}

/// Target of a binding.
#[derive(Debug, Clone)]
pub enum BindingTarget {
    /// Text content
    Content,
    /// An attribute
    Attribute { element: String, attribute: String },
    /// A class binding
    Class { element: String },
    /// A style binding
    Style { element: String },
}

/// An event handler in the template.
#[derive(Debug, Clone)]
pub struct EventHandler {
    /// Event name (e.g., "click", "submit")
    pub event: String,
    /// Handler expression
    pub handler: String,
    /// Element the handler is attached to
    pub element: String,
    /// Event modifiers
    pub modifiers: Vec<EventModifier>,
}

/// Event handler modifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventModifier {
    /// preventDefault
    Prevent,
    /// stopPropagation
    Stop,
    /// Use capture phase
    Capture,
    /// Only fire once
    Once,
    /// Passive listener
    Passive,
    /// Self-only (not from children)
    Self_,
}

// =============================================================================
// Template Parser
// =============================================================================

/// Parser for template sections.
#[derive(Debug, Default)]
pub struct TemplateParser {
    // Configuration can be added here
}

impl TemplateParser {
    /// Create a new template parser.
    pub fn new() -> Self {
        Self::default()
    }

    /// Parse a template section.
    pub fn parse(&self, source: &str, _path: &Path) -> DxResult<ParsedTemplate> {
        let source = source.trim().to_string();
        let mut bindings = Vec::new();
        let mut event_handlers = Vec::new();
        let mut directives = Vec::new();

        // Parse the template
        let nodes = self.parse_nodes(&source, &mut bindings, &mut event_handlers, &mut directives);

        Ok(ParsedTemplate {
            source,
            nodes,
            bindings,
            event_handlers,
            directives,
        })
    }

    /// Parse template nodes from source.
    fn parse_nodes(
        &self,
        source: &str,
        bindings: &mut Vec<Binding>,
        event_handlers: &mut Vec<EventHandler>,
        directives: &mut Vec<TemplateDirective>,
    ) -> Vec<TemplateNode> {
        let mut nodes = Vec::new();
        let mut pos = 0;
        let chars: Vec<char> = source.chars().collect();

        while pos < chars.len() {
            // Check for directive
            if pos + 1 < chars.len() && chars[pos] == '{' && chars[pos + 1] == '#' {
                let (directive_node, end_pos) =
                    self.parse_directive(&chars, pos, bindings, event_handlers, directives);
                nodes.push(TemplateNode::Directive(directive_node));
                pos = end_pos;
                continue;
            }

            // Check for interpolation
            if chars[pos] == '{' && (pos + 1 >= chars.len() || chars[pos + 1] != '#') {
                let (expr, end_pos) = self.parse_interpolation(&chars, pos);
                bindings.push(Binding {
                    expression: expr.clone(),
                    target: BindingTarget::Content,
                });
                nodes.push(TemplateNode::Interpolation(expr));
                pos = end_pos;
                continue;
            }

            // Check for comment
            if self.starts_with(&chars, pos, "<!--") {
                let (comment, end_pos) = self.parse_comment(&chars, pos);
                nodes.push(TemplateNode::Comment(comment));
                pos = end_pos;
                continue;
            }

            // Check for element
            if chars[pos] == '<' {
                let (element, end_pos) =
                    self.parse_element(&chars, pos, bindings, event_handlers, directives);
                if let Some(elem) = element {
                    nodes.push(elem);
                }
                pos = end_pos;
                continue;
            }

            // Text content
            let (text, end_pos) = self.parse_text(&chars, pos);
            if !text.trim().is_empty() {
                nodes.push(TemplateNode::Text(text));
            }
            pos = end_pos;
        }

        nodes
    }

    /// Parse an interpolation expression.
    fn parse_interpolation(&self, chars: &[char], start: usize) -> (String, usize) {
        let mut pos = start + 1; // Skip opening brace
        let mut depth = 1;
        let mut expr = String::new();

        while pos < chars.len() && depth > 0 {
            match chars[pos] {
                '{' => {
                    depth += 1;
                    expr.push('{');
                }
                '}' => {
                    depth -= 1;
                    if depth > 0 {
                        expr.push('}');
                    }
                }
                c => expr.push(c),
            }
            pos += 1;
        }

        (expr.trim().to_string(), pos)
    }

    /// Parse a directive block.
    fn parse_directive(
        &self,
        chars: &[char],
        start: usize,
        bindings: &mut Vec<Binding>,
        event_handlers: &mut Vec<EventHandler>,
        directives: &mut Vec<TemplateDirective>,
    ) -> (DirectiveNode, usize) {
        // Find the directive expression
        let mut pos = start + 2; // Skip "{#"
        let mut directive_content = String::new();

        while pos < chars.len() && chars[pos] != '}' {
            directive_content.push(chars[pos]);
            pos += 1;
        }
        pos += 1; // Skip closing brace

        // Parse directive type
        let directive = self.parse_directive_type(&directive_content);
        directives.push(directive.clone());

        // Find matching closing tag and parse children
        let directive_name = self.get_directive_name(&directive);
        let (children, else_branch, end_pos) = self.parse_directive_body(
            chars,
            pos,
            &directive_name,
            bindings,
            event_handlers,
            directives,
        );

        (
            DirectiveNode {
                directive,
                children,
                else_branch,
            },
            end_pos,
        )
    }

    /// Get the name of a directive for matching closing tags.
    fn get_directive_name(&self, directive: &TemplateDirective) -> &'static str {
        match directive {
            TemplateDirective::If { .. } => "if",
            TemplateDirective::Each { .. } => "each",
            TemplateDirective::Await { .. } => "await",
            TemplateDirective::Key { .. } => "key",
        }
    }

    /// Parse directive type from content.
    fn parse_directive_type(&self, content: &str) -> TemplateDirective {
        let content = content.trim();

        if content.starts_with("if ") {
            TemplateDirective::If {
                condition: content[3..].trim().to_string(),
            }
        } else if content.starts_with("each ") {
            let rest = &content[5..];
            if let Some(as_pos) = rest.find(" as ") {
                let items = rest[..as_pos].trim().to_string();
                let after_as = &rest[as_pos + 4..];

                // Check for index
                if let Some(comma_pos) = after_as.find(',') {
                    let item = after_as[..comma_pos].trim().to_string();
                    let index = after_as[comma_pos + 1..].trim().to_string();
                    TemplateDirective::Each {
                        items,
                        item,
                        index: Some(index),
                    }
                } else {
                    TemplateDirective::Each {
                        items,
                        item: after_as.trim().to_string(),
                        index: None,
                    }
                }
            } else {
                TemplateDirective::Each {
                    items: rest.to_string(),
                    item: "item".to_string(),
                    index: None,
                }
            }
        } else if content.starts_with("await ") {
            TemplateDirective::Await {
                promise: content[6..].trim().to_string(),
                then_var: None,
                catch_var: None,
            }
        } else if content.starts_with("key ") {
            TemplateDirective::Key {
                expression: content[4..].trim().to_string(),
            }
        } else {
            // Default to if
            TemplateDirective::If {
                condition: content.to_string(),
            }
        }
    }

    /// Parse directive body including children and else branch.
    fn parse_directive_body(
        &self,
        chars: &[char],
        start: usize,
        directive_name: &str,
        bindings: &mut Vec<Binding>,
        event_handlers: &mut Vec<EventHandler>,
        directives: &mut Vec<TemplateDirective>,
    ) -> (Vec<TemplateNode>, Option<Vec<TemplateNode>>, usize) {
        let closing_tag = format!("{{/{directive_name}}}");
        let else_tag = "{:else}";

        let mut pos = start;
        let mut content = String::new();
        let mut else_content = String::new();
        let mut in_else = false;

        while pos < chars.len() {
            // Check for else tag
            if self.starts_with(chars, pos, else_tag) {
                in_else = true;
                pos += else_tag.len();
                continue;
            }

            // Check for closing tag
            if self.starts_with(chars, pos, &closing_tag) {
                pos += closing_tag.len();
                break;
            }

            if in_else {
                else_content.push(chars[pos]);
            } else {
                content.push(chars[pos]);
            }
            pos += 1;
        }

        let children = self.parse_nodes(&content, bindings, event_handlers, directives);
        let else_branch = if !else_content.is_empty() {
            Some(self.parse_nodes(&else_content, bindings, event_handlers, directives))
        } else {
            None
        };

        (children, else_branch, pos)
    }

    /// Parse an HTML element.
    fn parse_element(
        &self,
        chars: &[char],
        start: usize,
        bindings: &mut Vec<Binding>,
        event_handlers: &mut Vec<EventHandler>,
        directives: &mut Vec<TemplateDirective>,
    ) -> (Option<TemplateNode>, usize) {
        let mut pos = start + 1; // Skip '<'

        // Check for closing tag
        if pos < chars.len() && chars[pos] == '/' {
            // Skip closing tag
            while pos < chars.len() && chars[pos] != '>' {
                pos += 1;
            }
            return (None, pos + 1);
        }

        // Parse tag name
        let mut tag = String::new();
        while pos < chars.len() && chars[pos].is_alphanumeric()
            || chars[pos] == '-'
            || chars[pos] == '_'
        {
            tag.push(chars[pos]);
            pos += 1;
        }

        if tag.is_empty() {
            return (None, pos);
        }

        // Check if this is a component (PascalCase)
        let is_component = tag.chars().next().map(|c| c.is_uppercase()).unwrap_or(false);

        // Parse attributes
        let (attributes, self_closing, end_pos) =
            self.parse_attributes(chars, pos, &tag, bindings, event_handlers);
        pos = end_pos;

        // Parse children if not self-closing
        let children = if self_closing {
            Vec::new()
        } else {
            // Find matching closing tag and parse content
            let (content, end) = self.find_element_content(chars, pos, &tag);
            pos = end;
            self.parse_nodes(&content, bindings, event_handlers, directives)
        };

        if is_component {
            (
                Some(TemplateNode::Component(ComponentRef {
                    name: tag,
                    props: attributes,
                    children,
                })),
                pos,
            )
        } else {
            (
                Some(TemplateNode::Element(ElementNode {
                    tag,
                    attributes,
                    children,
                    self_closing,
                })),
                pos,
            )
        }
    }

    /// Parse element attributes.
    fn parse_attributes(
        &self,
        chars: &[char],
        start: usize,
        element_name: &str,
        bindings: &mut Vec<Binding>,
        event_handlers: &mut Vec<EventHandler>,
    ) -> (Vec<Attribute>, bool, usize) {
        let mut attributes = Vec::new();
        let mut pos = start;
        let mut self_closing = false;

        // Skip whitespace
        while pos < chars.len() && chars[pos].is_whitespace() {
            pos += 1;
        }

        while pos < chars.len() && chars[pos] != '>' && chars[pos] != '/' {
            // Parse attribute name
            let mut name = String::new();
            while pos < chars.len()
                && !chars[pos].is_whitespace()
                && chars[pos] != '='
                && chars[pos] != '>'
                && chars[pos] != '/'
            {
                name.push(chars[pos]);
                pos += 1;
            }

            if name.is_empty() {
                pos += 1;
                continue;
            }

            // Skip whitespace
            while pos < chars.len() && chars[pos].is_whitespace() {
                pos += 1;
            }

            // Check for value
            let value = if pos < chars.len() && chars[pos] == '=' {
                pos += 1;

                // Skip whitespace
                while pos < chars.len() && chars[pos].is_whitespace() {
                    pos += 1;
                }

                // Parse value
                if pos < chars.len() {
                    let (val, end) = self.parse_attribute_value(chars, pos);
                    pos = end;
                    Some(val)
                } else {
                    None
                }
            } else {
                None
            };

            // Handle event handlers
            if name.starts_with("on") || name.starts_with("@") {
                let event_name = if name.starts_with('@') {
                    name[1..].to_string()
                } else {
                    // Convert onClick to click
                    let mut event = name[2..].to_string();
                    event = event.chars().next().unwrap().to_lowercase().to_string() + &event[1..];
                    event
                };

                if let Some(AttributeValue::Dynamic(handler)) = &value {
                    event_handlers.push(EventHandler {
                        event: event_name,
                        handler: handler.clone(),
                        element: element_name.to_string(),
                        modifiers: Vec::new(),
                    });
                }
            }

            // Handle dynamic bindings
            if let Some(AttributeValue::Dynamic(expr)) = &value {
                bindings.push(Binding {
                    expression: expr.clone(),
                    target: BindingTarget::Attribute {
                        element: element_name.to_string(),
                        attribute: name.clone(),
                    },
                });
            }

            attributes.push(Attribute { name, value });

            // Skip whitespace
            while pos < chars.len() && chars[pos].is_whitespace() {
                pos += 1;
            }
        }

        // Check for self-closing
        if pos < chars.len() && chars[pos] == '/' {
            self_closing = true;
            pos += 1;
        }

        // Skip closing '>'
        if pos < chars.len() && chars[pos] == '>' {
            pos += 1;
        }

        (attributes, self_closing, pos)
    }

    /// Parse an attribute value.
    fn parse_attribute_value(&self, chars: &[char], start: usize) -> (AttributeValue, usize) {
        let mut pos = start;

        // Check for quoted value
        if chars[pos] == '"' || chars[pos] == '\'' {
            let quote = chars[pos];
            pos += 1;
            let mut value = String::new();
            while pos < chars.len() && chars[pos] != quote {
                value.push(chars[pos]);
                pos += 1;
            }
            pos += 1; // Skip closing quote
            return (AttributeValue::Static(value), pos);
        }

        // Check for dynamic value
        if chars[pos] == '{' {
            let (expr, end) = self.parse_interpolation(chars, pos);
            return (AttributeValue::Dynamic(expr), end);
        }

        // Unquoted value
        let mut value = String::new();
        while pos < chars.len() && !chars[pos].is_whitespace() && chars[pos] != '>' {
            value.push(chars[pos]);
            pos += 1;
        }

        (AttributeValue::Static(value), pos)
    }

    /// Find element content until closing tag.
    fn find_element_content(&self, chars: &[char], start: usize, tag: &str) -> (String, usize) {
        let closing = format!("</{tag}>");
        let mut pos = start;
        let mut content = String::new();
        let mut depth = 1;

        while pos < chars.len() && depth > 0 {
            // Check for closing tag
            if self.starts_with(chars, pos, &closing) {
                depth -= 1;
                if depth == 0 {
                    pos += closing.len();
                    break;
                }
            }

            // Check for opening tag
            let opening = format!("<{tag}");
            if self.starts_with(chars, pos, &opening)
                && (pos + opening.len() >= chars.len()
                    || chars[pos + opening.len()].is_whitespace()
                    || chars[pos + opening.len()] == '>')
            {
                depth += 1;
            }

            content.push(chars[pos]);
            pos += 1;
        }

        (content, pos)
    }

    /// Parse text content.
    fn parse_text(&self, chars: &[char], start: usize) -> (String, usize) {
        let mut pos = start;
        let mut text = String::new();

        while pos < chars.len() && chars[pos] != '<' && chars[pos] != '{' {
            text.push(chars[pos]);
            pos += 1;
        }

        (text, pos)
    }

    /// Parse an HTML comment.
    fn parse_comment(&self, chars: &[char], start: usize) -> (String, usize) {
        let mut pos = start + 4; // Skip "<!--"
        let mut comment = String::new();

        while pos + 2 < chars.len() {
            if chars[pos] == '-' && chars[pos + 1] == '-' && chars[pos + 2] == '>' {
                pos += 3;
                break;
            }
            comment.push(chars[pos]);
            pos += 1;
        }

        (comment.trim().to_string(), pos)
    }

    /// Check if chars starting at pos match the pattern.
    fn starts_with(&self, chars: &[char], pos: usize, pattern: &str) -> bool {
        let pattern_chars: Vec<char> = pattern.chars().collect();
        if pos + pattern_chars.len() > chars.len() {
            return false;
        }
        for (i, &c) in pattern_chars.iter().enumerate() {
            if chars[pos + i] != c {
                return false;
            }
        }
        true
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_parse_simple_template() {
        let parser = TemplateParser::new();
        let source = "<h1>Hello</h1>";

        let result = parser.parse(source, &PathBuf::from("test.pg"));
        assert!(result.is_ok());

        let template = result.unwrap();
        assert_eq!(template.nodes.len(), 1);
    }

    #[test]
    fn test_parse_interpolation() {
        let parser = TemplateParser::new();
        let source = "<p>{message}</p>";

        let result = parser.parse(source, &PathBuf::from("test.pg"));
        assert!(result.is_ok());

        let template = result.unwrap();
        assert!(!template.bindings.is_empty());
    }

    #[test]
    fn test_parse_if_directive() {
        let parser = TemplateParser::new();
        let source = "{#if show}<p>Visible</p>{/if}";

        let result = parser.parse(source, &PathBuf::from("test.pg"));
        assert!(result.is_ok());

        let template = result.unwrap();
        assert!(!template.directives.is_empty());
        assert!(matches!(template.directives[0], TemplateDirective::If { .. }));
    }

    #[test]
    fn test_parse_each_directive() {
        let parser = TemplateParser::new();
        let source = "{#each items as item}<p>{item}</p>{/each}";

        let result = parser.parse(source, &PathBuf::from("test.pg"));
        assert!(result.is_ok());

        let template = result.unwrap();
        assert!(!template.directives.is_empty());
        assert!(matches!(template.directives[0], TemplateDirective::Each { .. }));
    }

    #[test]
    fn test_parse_event_handler() {
        let parser = TemplateParser::new();
        let source = r#"<button onClick={handleClick}>Click</button>"#;

        let result = parser.parse(source, &PathBuf::from("test.pg"));
        assert!(result.is_ok());

        let template = result.unwrap();
        assert!(!template.event_handlers.is_empty());
        assert_eq!(template.event_handlers[0].event, "click");
    }
}
