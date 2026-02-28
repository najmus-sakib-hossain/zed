//! Jinja2 Template Compatibility
//!
//! Provides compatibility with Jinja2 template engine for Flask applications.
//! Implements template compilation and context rendering.

use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use thiserror::Error;

/// Errors that can occur during template operations
#[derive(Debug, Error)]
pub enum JinjaError {
    #[error("Template syntax error: {0}")]
    SyntaxError(String),

    #[error("Template not found: {0}")]
    NotFound(String),

    #[error("Undefined variable: {0}")]
    UndefinedVariable(String),

    #[error("Filter not found: {0}")]
    FilterNotFound(String),

    #[error("Render error: {0}")]
    RenderError(String),
}

/// Template context value
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum JinjaValue {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    List(Vec<JinjaValue>),
    Dict(HashMap<String, JinjaValue>),
}

impl JinjaValue {
    /// Check if value is truthy (for conditionals)
    pub fn is_truthy(&self) -> bool {
        match self {
            JinjaValue::Null => false,
            JinjaValue::Bool(b) => *b,
            JinjaValue::Int(i) => *i != 0,
            JinjaValue::Float(f) => *f != 0.0,
            JinjaValue::String(s) => !s.is_empty(),
            JinjaValue::List(l) => !l.is_empty(),
            JinjaValue::Dict(d) => !d.is_empty(),
        }
    }

    /// Convert to string for output
    pub fn to_output(&self) -> String {
        match self {
            JinjaValue::Null => String::new(),
            JinjaValue::Bool(b) => if *b { "True" } else { "False" }.to_string(),
            JinjaValue::Int(i) => i.to_string(),
            JinjaValue::Float(f) => f.to_string(),
            JinjaValue::String(s) => s.clone(),
            JinjaValue::List(l) => {
                let items: Vec<String> = l.iter().map(|v| v.to_output()).collect();
                format!("[{}]", items.join(", "))
            }
            JinjaValue::Dict(d) => {
                let items: Vec<String> =
                    d.iter().map(|(k, v)| format!("{}: {}", k, v.to_output())).collect();
                format!("{{{}}}", items.join(", "))
            }
        }
    }

    /// Get a value by key (for dict access)
    pub fn get(&self, key: &str) -> Option<&JinjaValue> {
        match self {
            JinjaValue::Dict(d) => d.get(key),
            _ => None,
        }
    }

    /// Get a value by index (for list access)
    pub fn get_index(&self, index: usize) -> Option<&JinjaValue> {
        match self {
            JinjaValue::List(l) => l.get(index),
            _ => None,
        }
    }
}

impl From<&str> for JinjaValue {
    fn from(s: &str) -> Self {
        JinjaValue::String(s.to_string())
    }
}

impl From<String> for JinjaValue {
    fn from(s: String) -> Self {
        JinjaValue::String(s)
    }
}

impl From<i64> for JinjaValue {
    fn from(i: i64) -> Self {
        JinjaValue::Int(i)
    }
}

impl From<f64> for JinjaValue {
    fn from(f: f64) -> Self {
        JinjaValue::Float(f)
    }
}

impl From<bool> for JinjaValue {
    fn from(b: bool) -> Self {
        JinjaValue::Bool(b)
    }
}

impl<T: Into<JinjaValue>> From<Vec<T>> for JinjaValue {
    fn from(v: Vec<T>) -> Self {
        JinjaValue::List(v.into_iter().map(|x| x.into()).collect())
    }
}

impl From<JsonValue> for JinjaValue {
    fn from(v: JsonValue) -> Self {
        match v {
            JsonValue::Null => JinjaValue::Null,
            JsonValue::Bool(b) => JinjaValue::Bool(b),
            JsonValue::Number(n) => {
                if let Some(i) = n.as_i64() {
                    JinjaValue::Int(i)
                } else if let Some(f) = n.as_f64() {
                    JinjaValue::Float(f)
                } else {
                    JinjaValue::Null
                }
            }
            JsonValue::String(s) => JinjaValue::String(s),
            JsonValue::Array(a) => JinjaValue::List(a.into_iter().map(JinjaValue::from).collect()),
            JsonValue::Object(o) => {
                JinjaValue::Dict(o.into_iter().map(|(k, v)| (k, JinjaValue::from(v))).collect())
            }
        }
    }
}

/// Template context for rendering
#[derive(Debug, Clone, Default)]
pub struct JinjaContext {
    values: HashMap<String, JinjaValue>,
}

impl JinjaContext {
    /// Create a new empty context
    pub fn new() -> Self {
        Self {
            values: HashMap::new(),
        }
    }

    /// Set a value in the context
    pub fn set(&mut self, key: impl Into<String>, value: impl Into<JinjaValue>) {
        self.values.insert(key.into(), value.into());
    }

    /// Get a value from the context
    pub fn get(&self, key: &str) -> Option<&JinjaValue> {
        self.values.get(key)
    }

    /// Check if a key exists
    pub fn contains(&self, key: &str) -> bool {
        self.values.contains_key(key)
    }

    /// Create context from JSON value
    pub fn from_json(value: JsonValue) -> Self {
        let mut ctx = Self::new();
        if let JsonValue::Object(obj) = value {
            for (k, v) in obj {
                ctx.set(k, JinjaValue::from(v));
            }
        }
        ctx
    }
}

/// A compiled Jinja2 template
#[derive(Debug, Clone)]
pub struct JinjaTemplate {
    /// Template name
    pub name: String,
    /// Template source
    source: String,
    /// Parsed template nodes
    nodes: Vec<TemplateNode>,
}

/// Template AST node
#[derive(Debug, Clone)]
enum TemplateNode {
    /// Raw text
    Text(String),
    /// Variable output: {{ expr }}
    Output(String),
    /// If statement: {% if expr %}...{% endif %}
    If {
        condition: String,
        then_nodes: Vec<TemplateNode>,
        else_nodes: Vec<TemplateNode>,
    },
    /// For loop: {% for item in items %}...{% endfor %}
    For {
        var_name: String,
        iterable: String,
        body: Vec<TemplateNode>,
    },
    /// Block: {% block name %}...{% endblock %}
    Block {
        #[allow(dead_code)]
        name: String,
        content: Vec<TemplateNode>,
    },
}

impl JinjaTemplate {
    /// Parse a template from source
    pub fn parse(name: impl Into<String>, source: impl Into<String>) -> Result<Self, JinjaError> {
        let name = name.into();
        let source = source.into();
        let nodes = Self::parse_nodes(&source)?;

        Ok(Self {
            name,
            source,
            nodes,
        })
    }

    /// Parse template source into nodes
    fn parse_nodes(source: &str) -> Result<Vec<TemplateNode>, JinjaError> {
        let mut nodes = Vec::new();
        let mut pos = 0;
        let chars: Vec<char> = source.chars().collect();

        while pos < chars.len() {
            // Look for template tags
            if pos + 1 < chars.len() {
                let two_chars: String = chars[pos..pos + 2].iter().collect();

                match two_chars.as_str() {
                    "{{" => {
                        // Variable output
                        if let Some(end) = Self::find_closing(&chars, pos + 2, "}}") {
                            let expr: String = chars[pos + 2..end].iter().collect();
                            nodes.push(TemplateNode::Output(expr.trim().to_string()));
                            pos = end + 2;
                            continue;
                        }
                    }
                    "{%" => {
                        // Control tag
                        if let Some(end) = Self::find_closing(&chars, pos + 2, "%}") {
                            let tag: String = chars[pos + 2..end].iter().collect();
                            let tag = tag.trim();

                            if tag.starts_with("if ") {
                                let condition = tag.strip_prefix("if ").unwrap().trim().to_string();
                                let (then_nodes, else_nodes, new_pos) =
                                    Self::parse_if_block(&chars, end + 2)?;
                                nodes.push(TemplateNode::If {
                                    condition,
                                    then_nodes,
                                    else_nodes,
                                });
                                pos = new_pos;
                                continue;
                            } else if tag.starts_with("for ") {
                                let parts: Vec<&str> =
                                    tag.strip_prefix("for ").unwrap().split(" in ").collect();
                                if parts.len() == 2 {
                                    let var_name = parts[0].trim().to_string();
                                    let iterable = parts[1].trim().to_string();
                                    let (body, new_pos) = Self::parse_for_block(&chars, end + 2)?;
                                    nodes.push(TemplateNode::For {
                                        var_name,
                                        iterable,
                                        body,
                                    });
                                    pos = new_pos;
                                    continue;
                                }
                            } else if tag.starts_with("block ") {
                                let block_name =
                                    tag.strip_prefix("block ").unwrap().trim().to_string();
                                let (content, new_pos) = Self::parse_block(&chars, end + 2)?;
                                nodes.push(TemplateNode::Block {
                                    name: block_name,
                                    content,
                                });
                                pos = new_pos;
                                continue;
                            }

                            pos = end + 2;
                            continue;
                        }
                    }
                    "{#" => {
                        // Comment - skip
                        if let Some(end) = Self::find_closing(&chars, pos + 2, "#}") {
                            pos = end + 2;
                            continue;
                        }
                    }
                    _ => {}
                }
            }

            // Regular text
            let mut text = String::new();
            while pos < chars.len() {
                if pos + 1 < chars.len() {
                    let two: String = chars[pos..pos + 2].iter().collect();
                    if two == "{{" || two == "{%" || two == "{#" {
                        break;
                    }
                }
                text.push(chars[pos]);
                pos += 1;
            }
            if !text.is_empty() {
                nodes.push(TemplateNode::Text(text));
            }
        }

        Ok(nodes)
    }

    /// Find closing tag
    fn find_closing(chars: &[char], start: usize, closing: &str) -> Option<usize> {
        let closing_chars: Vec<char> = closing.chars().collect();
        (start..chars.len() - closing_chars.len() + 1)
            .find(|&i| chars[i..i + closing_chars.len()] == closing_chars[..])
    }

    /// Parse if block - collects content until {% endif %}
    fn parse_if_block(
        chars: &[char],
        start: usize,
    ) -> Result<(Vec<TemplateNode>, Vec<TemplateNode>, usize), JinjaError> {
        let mut then_nodes = Vec::new();
        let mut else_nodes = Vec::new();
        let mut pos = start;
        let mut in_else = false;

        while pos < chars.len() {
            // Check for control tags
            if pos + 1 < chars.len() {
                let two: String = chars[pos..pos + 2].iter().collect();
                if two == "{%" {
                    if let Some(end) = Self::find_closing(chars, pos + 2, "%}") {
                        let tag: String = chars[pos + 2..end].iter().collect();
                        let tag = tag.trim();

                        if tag == "endif" {
                            return Ok((then_nodes, else_nodes, end + 2));
                        } else if tag == "else" {
                            in_else = true;
                            pos = end + 2;
                            continue;
                        }
                    }
                } else if two == "{{" {
                    // Variable output
                    if let Some(end) = Self::find_closing(chars, pos + 2, "}}") {
                        let expr: String = chars[pos + 2..end].iter().collect();
                        let node = TemplateNode::Output(expr.trim().to_string());
                        if in_else {
                            else_nodes.push(node);
                        } else {
                            then_nodes.push(node);
                        }
                        pos = end + 2;
                        continue;
                    }
                }
            }

            // Collect text until next tag
            let mut text = String::new();
            while pos < chars.len() {
                if pos + 1 < chars.len() {
                    let two: String = chars[pos..pos + 2].iter().collect();
                    if two == "{{" || two == "{%" {
                        break;
                    }
                }
                text.push(chars[pos]);
                pos += 1;
            }
            if !text.is_empty() {
                let node = TemplateNode::Text(text);
                if in_else {
                    else_nodes.push(node);
                } else {
                    then_nodes.push(node);
                }
            }
        }

        Ok((then_nodes, else_nodes, pos))
    }

    /// Parse for block
    fn parse_for_block(
        chars: &[char],
        start: usize,
    ) -> Result<(Vec<TemplateNode>, usize), JinjaError> {
        let mut body = Vec::new();
        let mut pos = start;

        while pos < chars.len() {
            if pos + 1 < chars.len() {
                let two: String = chars[pos..pos + 2].iter().collect();
                if two == "{%" {
                    if let Some(end) = Self::find_closing(chars, pos + 2, "%}") {
                        let tag: String = chars[pos + 2..end].iter().collect();
                        if tag.trim() == "endfor" {
                            return Ok((body, end + 2));
                        }
                    }
                }
            }

            // Collect text until next tag
            let mut text = String::new();
            while pos < chars.len() {
                if pos + 1 < chars.len() {
                    let two: String = chars[pos..pos + 2].iter().collect();
                    if two == "{{" || two == "{%" {
                        break;
                    }
                }
                text.push(chars[pos]);
                pos += 1;
            }
            if !text.is_empty() {
                body.push(TemplateNode::Text(text));
            }

            // Handle variable output
            if pos + 1 < chars.len() {
                let two: String = chars[pos..pos + 2].iter().collect();
                if two == "{{" {
                    if let Some(end) = Self::find_closing(chars, pos + 2, "}}") {
                        let expr: String = chars[pos + 2..end].iter().collect();
                        body.push(TemplateNode::Output(expr.trim().to_string()));
                        pos = end + 2;
                    }
                }
            }
        }

        Ok((body, pos))
    }

    /// Parse block
    fn parse_block(chars: &[char], start: usize) -> Result<(Vec<TemplateNode>, usize), JinjaError> {
        let mut content = Vec::new();
        let mut pos = start;

        while pos < chars.len() {
            if pos + 1 < chars.len() {
                let two: String = chars[pos..pos + 2].iter().collect();
                if two == "{%" {
                    if let Some(end) = Self::find_closing(chars, pos + 2, "%}") {
                        let tag: String = chars[pos + 2..end].iter().collect();
                        if tag.trim() == "endblock" || tag.trim().starts_with("endblock ") {
                            return Ok((content, end + 2));
                        }
                    }
                }
            }

            let mut text = String::new();
            while pos < chars.len() {
                if pos + 1 < chars.len() {
                    let two: String = chars[pos..pos + 2].iter().collect();
                    if two == "{{" || two == "{%" {
                        break;
                    }
                }
                text.push(chars[pos]);
                pos += 1;
            }
            if !text.is_empty() {
                content.push(TemplateNode::Text(text));
            }
        }

        Ok((content, pos))
    }

    /// Render the template with context
    pub fn render(&self, context: &JinjaContext) -> Result<String, JinjaError> {
        self.render_nodes(&self.nodes, context)
    }

    /// Render a list of nodes
    fn render_nodes(
        &self,
        nodes: &[TemplateNode],
        context: &JinjaContext,
    ) -> Result<String, JinjaError> {
        let mut output = String::new();

        for node in nodes {
            match node {
                TemplateNode::Text(text) => {
                    output.push_str(text);
                }
                TemplateNode::Output(expr) => {
                    let value = self.evaluate_expr(expr, context)?;
                    output.push_str(&Self::escape_html(&value.to_output()));
                }
                TemplateNode::If {
                    condition,
                    then_nodes,
                    else_nodes,
                } => {
                    let value = self.evaluate_expr(condition, context)?;
                    if value.is_truthy() {
                        output.push_str(&self.render_nodes(then_nodes, context)?);
                    } else {
                        output.push_str(&self.render_nodes(else_nodes, context)?);
                    }
                }
                TemplateNode::For {
                    var_name,
                    iterable,
                    body,
                } => {
                    let items = self.evaluate_expr(iterable, context)?;
                    if let JinjaValue::List(list) = items {
                        for item in list {
                            let mut loop_context = context.clone();
                            loop_context.set(var_name.clone(), item);
                            output.push_str(&self.render_nodes(body, &loop_context)?);
                        }
                    }
                }
                TemplateNode::Block { content, .. } => {
                    output.push_str(&self.render_nodes(content, context)?);
                }
            }
        }

        Ok(output)
    }

    /// Evaluate an expression
    fn evaluate_expr(&self, expr: &str, context: &JinjaContext) -> Result<JinjaValue, JinjaError> {
        let expr = expr.trim();

        // Check for filter: expr|filter
        if let Some(pipe_pos) = expr.find('|') {
            let base_expr = &expr[..pipe_pos].trim();
            let filter = &expr[pipe_pos + 1..].trim();
            let value = self.evaluate_expr(base_expr, context)?;
            return self.apply_filter(&value, filter);
        }

        // Check for attribute access: obj.attr
        if expr.contains('.') {
            let parts: Vec<&str> = expr.splitn(2, '.').collect();
            let base = self.evaluate_expr(parts[0], context)?;
            if let Some(value) = base.get(parts[1]) {
                return Ok(value.clone());
            }
            return Err(JinjaError::UndefinedVariable(expr.to_string()));
        }

        // String literal
        if (expr.starts_with('"') && expr.ends_with('"'))
            || (expr.starts_with('\'') && expr.ends_with('\''))
        {
            return Ok(JinjaValue::String(expr[1..expr.len() - 1].to_string()));
        }

        // Number literal
        if let Ok(i) = expr.parse::<i64>() {
            return Ok(JinjaValue::Int(i));
        }
        if let Ok(f) = expr.parse::<f64>() {
            return Ok(JinjaValue::Float(f));
        }

        // Boolean literal
        if expr == "true" || expr == "True" {
            return Ok(JinjaValue::Bool(true));
        }
        if expr == "false" || expr == "False" {
            return Ok(JinjaValue::Bool(false));
        }

        // Variable lookup
        context
            .get(expr)
            .cloned()
            .ok_or_else(|| JinjaError::UndefinedVariable(expr.to_string()))
    }

    /// Apply a filter to a value
    fn apply_filter(&self, value: &JinjaValue, filter: &str) -> Result<JinjaValue, JinjaError> {
        match filter {
            "upper" => Ok(JinjaValue::String(value.to_output().to_uppercase())),
            "lower" => Ok(JinjaValue::String(value.to_output().to_lowercase())),
            "title" => {
                let s = value.to_output();
                let titled: String = s
                    .split_whitespace()
                    .map(|word| {
                        let mut chars = word.chars();
                        match chars.next() {
                            None => String::new(),
                            Some(c) => c.to_uppercase().chain(chars).collect(),
                        }
                    })
                    .collect::<Vec<_>>()
                    .join(" ");
                Ok(JinjaValue::String(titled))
            }
            "length" => {
                let len = match value {
                    JinjaValue::String(s) => s.len(),
                    JinjaValue::List(l) => l.len(),
                    JinjaValue::Dict(d) => d.len(),
                    _ => 0,
                };
                Ok(JinjaValue::Int(len as i64))
            }
            "safe" => {
                // Return as-is (no escaping)
                Ok(value.clone())
            }
            _ => Err(JinjaError::FilterNotFound(filter.to_string())),
        }
    }

    /// Escape HTML special characters
    fn escape_html(s: &str) -> String {
        s.replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;")
            .replace('\'', "&#x27;")
    }

    /// Get template source
    pub fn source(&self) -> &str {
        &self.source
    }
}

/// Jinja2 template engine
pub struct JinjaEngine {
    templates: HashMap<String, JinjaTemplate>,
    auto_escape: bool,
}

impl JinjaEngine {
    /// Create a new template engine
    pub fn new() -> Self {
        Self {
            templates: HashMap::new(),
            auto_escape: true,
        }
    }

    /// Disable auto-escaping
    pub fn with_auto_escape(mut self, enabled: bool) -> Self {
        self.auto_escape = enabled;
        self
    }

    /// Add a template
    pub fn add_template(
        &mut self,
        name: impl Into<String>,
        source: impl Into<String>,
    ) -> Result<(), JinjaError> {
        let name = name.into();
        let template = JinjaTemplate::parse(&name, source)?;
        self.templates.insert(name, template);
        Ok(())
    }

    /// Get a template by name
    pub fn get_template(&self, name: &str) -> Option<&JinjaTemplate> {
        self.templates.get(name)
    }

    /// Render a template by name
    pub fn render(&self, name: &str, context: &JinjaContext) -> Result<String, JinjaError> {
        let template =
            self.templates.get(name).ok_or_else(|| JinjaError::NotFound(name.to_string()))?;
        template.render(context)
    }

    /// Render a template string directly
    pub fn render_string(
        &self,
        source: &str,
        context: &JinjaContext,
    ) -> Result<String, JinjaError> {
        let template = JinjaTemplate::parse("_inline_", source)?;
        template.render(context)
    }
}

impl Default for JinjaEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_variable() {
        let template = JinjaTemplate::parse("test", "Hello, {{ name }}!").unwrap();
        let mut ctx = JinjaContext::new();
        ctx.set("name", "World");
        let result = template.render(&ctx).unwrap();
        assert_eq!(result, "Hello, World!");
    }

    #[test]
    fn test_if_statement() {
        let template = JinjaTemplate::parse("test", "{% if show %}Visible{% endif %}").unwrap();

        let mut ctx = JinjaContext::new();
        ctx.set("show", true);
        assert_eq!(template.render(&ctx).unwrap(), "Visible");

        ctx.set("show", false);
        assert_eq!(template.render(&ctx).unwrap(), "");
    }

    #[test]
    fn test_for_loop() {
        let template =
            JinjaTemplate::parse("test", "{% for item in items %}{{ item }}{% endfor %}").unwrap();
        let mut ctx = JinjaContext::new();
        ctx.set(
            "items",
            vec![
                JinjaValue::String("a".to_string()),
                JinjaValue::String("b".to_string()),
                JinjaValue::String("c".to_string()),
            ],
        );
        let result = template.render(&ctx).unwrap();
        assert_eq!(result, "abc");
    }

    #[test]
    fn test_filter_upper() {
        let template = JinjaTemplate::parse("test", "{{ name|upper }}").unwrap();
        let mut ctx = JinjaContext::new();
        ctx.set("name", "hello");
        let result = template.render(&ctx).unwrap();
        assert_eq!(result, "HELLO");
    }

    #[test]
    fn test_html_escaping() {
        let template = JinjaTemplate::parse("test", "{{ html }}").unwrap();
        let mut ctx = JinjaContext::new();
        ctx.set("html", "<script>alert('xss')</script>");
        let result = template.render(&ctx).unwrap();
        assert!(result.contains("&lt;script&gt;"));
    }

    #[test]
    fn test_engine() {
        let mut engine = JinjaEngine::new();
        engine.add_template("hello", "Hello, {{ name }}!").unwrap();

        let mut ctx = JinjaContext::new();
        ctx.set("name", "World");

        let result = engine.render("hello", &ctx).unwrap();
        assert_eq!(result, "Hello, World!");
    }

    #[test]
    fn test_jinja_value_truthy() {
        assert!(!JinjaValue::Null.is_truthy());
        assert!(!JinjaValue::Bool(false).is_truthy());
        assert!(JinjaValue::Bool(true).is_truthy());
        assert!(!JinjaValue::Int(0).is_truthy());
        assert!(JinjaValue::Int(1).is_truthy());
        assert!(!JinjaValue::String(String::new()).is_truthy());
        assert!(JinjaValue::String("hello".to_string()).is_truthy());
    }
}
