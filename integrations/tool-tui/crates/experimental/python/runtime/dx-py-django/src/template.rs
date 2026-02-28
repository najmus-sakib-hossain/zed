//! Template engine compatibility for Django
//!
//! Provides compatibility layers for:
//! - Jinja2 C extensions (markupsafe)
//! - Django template compilation

use std::collections::HashMap;
use thiserror::Error;

/// Errors that can occur during template operations
#[derive(Debug, Error)]
pub enum TemplateError {
    #[error("Template syntax error: {0}")]
    SyntaxError(String),

    #[error("Template not found: {0}")]
    NotFound(String),

    #[error("Variable not found: {0}")]
    VariableNotFound(String),

    #[error("Filter error: {0}")]
    FilterError(String),

    #[error("Render error: {0}")]
    RenderError(String),
}

/// MarkupSafe compatible string type
/// Represents HTML-safe strings that won't be escaped again
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Markup(String);

impl Markup {
    /// Create a new Markup from a string (marks it as safe)
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }

    /// Get the inner string
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Consume and return the inner string
    pub fn into_string(self) -> String {
        self.0
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Get length
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Concatenate with another Markup
    pub fn concat(&self, other: &Markup) -> Markup {
        Markup(format!("{}{}", self.0, other.0))
    }
}

impl std::fmt::Display for Markup {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<String> for Markup {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&str> for Markup {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

/// Escape HTML special characters (markupsafe.escape)
pub fn escape_html(s: &str) -> Markup {
    let mut result = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => result.push_str("&amp;"),
            '<' => result.push_str("&lt;"),
            '>' => result.push_str("&gt;"),
            '"' => result.push_str("&#34;"),
            '\'' => result.push_str("&#39;"),
            _ => result.push(c),
        }
    }
    Markup(result)
}

/// Escape HTML but preserve Markup objects
pub fn escape_silent(s: &str) -> Markup {
    escape_html(s)
}

/// Soft string conversion (markupsafe.soft_str)
pub fn soft_str(value: &str) -> String {
    value.to_string()
}

/// Template context value
#[derive(Debug, Clone)]
pub enum ContextValue {
    None,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    Markup(Markup),
    List(Vec<ContextValue>),
    Dict(HashMap<String, ContextValue>),
}

impl ContextValue {
    pub fn as_bool(&self) -> bool {
        match self {
            ContextValue::None => false,
            ContextValue::Bool(b) => *b,
            ContextValue::Int(i) => *i != 0,
            ContextValue::Float(f) => *f != 0.0,
            ContextValue::String(s) => !s.is_empty(),
            ContextValue::Markup(m) => !m.is_empty(),
            ContextValue::List(l) => !l.is_empty(),
            ContextValue::Dict(d) => !d.is_empty(),
        }
    }

    pub fn as_string(&self) -> String {
        match self {
            ContextValue::None => "".to_string(),
            ContextValue::Bool(b) => if *b { "True" } else { "False" }.to_string(),
            ContextValue::Int(i) => i.to_string(),
            ContextValue::Float(f) => f.to_string(),
            ContextValue::String(s) => s.clone(),
            ContextValue::Markup(m) => m.to_string(),
            ContextValue::List(_) => "[...]".to_string(),
            ContextValue::Dict(_) => "{...}".to_string(),
        }
    }
}

/// Template context
#[derive(Debug, Clone, Default)]
pub struct Context {
    values: HashMap<String, ContextValue>,
    autoescape: bool,
}

impl Context {
    pub fn new() -> Self {
        Self {
            values: HashMap::new(),
            autoescape: true,
        }
    }

    pub fn with_autoescape(mut self, autoescape: bool) -> Self {
        self.autoescape = autoescape;
        self
    }

    pub fn set(&mut self, key: impl Into<String>, value: ContextValue) {
        self.values.insert(key.into(), value);
    }

    pub fn get(&self, key: &str) -> Option<&ContextValue> {
        self.values.get(key)
    }

    pub fn autoescape(&self) -> bool {
        self.autoescape
    }
}

/// Template node types
#[derive(Debug, Clone)]
pub enum TemplateNode {
    Text(String),
    Variable {
        name: String,
        filters: Vec<String>,
    },
    If {
        condition: String,
        then_nodes: Vec<TemplateNode>,
        else_nodes: Vec<TemplateNode>,
    },
    For {
        var: String,
        iterable: String,
        nodes: Vec<TemplateNode>,
    },
    Block {
        name: String,
        nodes: Vec<TemplateNode>,
    },
    Extends(String),
    Include(String),
    Comment,
}

/// Compiled template
#[derive(Debug, Clone)]
pub struct CompiledTemplate {
    nodes: Vec<TemplateNode>,
    name: String,
}

impl CompiledTemplate {
    pub fn new(name: impl Into<String>, nodes: Vec<TemplateNode>) -> Self {
        Self {
            nodes,
            name: name.into(),
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn nodes(&self) -> &[TemplateNode] {
        &self.nodes
    }
}

/// Template compiler (Django/Jinja2 compatible)
pub struct TemplateCompiler {
    autoescape: bool,
}

impl Default for TemplateCompiler {
    fn default() -> Self {
        Self::new()
    }
}

impl TemplateCompiler {
    pub fn new() -> Self {
        Self { autoescape: true }
    }

    pub fn with_autoescape(mut self, autoescape: bool) -> Self {
        self.autoescape = autoescape;
        self
    }

    /// Compile a template string
    pub fn compile(&self, name: &str, source: &str) -> Result<CompiledTemplate, TemplateError> {
        let nodes = self.parse(source)?;
        Ok(CompiledTemplate::new(name, nodes))
    }

    fn parse(&self, source: &str) -> Result<Vec<TemplateNode>, TemplateError> {
        let mut nodes = Vec::new();
        let mut chars = source.chars().peekable();
        let mut text_buf = String::new();

        while let Some(c) = chars.next() {
            if c == '{' {
                if let Some(&next) = chars.peek() {
                    match next {
                        '{' => {
                            // Variable: {{ var }}
                            if !text_buf.is_empty() {
                                nodes.push(TemplateNode::Text(std::mem::take(&mut text_buf)));
                            }
                            chars.next(); // consume second {
                            let var_content = self.read_until(&mut chars, "}}")?;
                            let (name, filters) = self.parse_variable(&var_content);
                            nodes.push(TemplateNode::Variable { name, filters });
                        }
                        '%' => {
                            // Tag: {% tag %}
                            if !text_buf.is_empty() {
                                nodes.push(TemplateNode::Text(std::mem::take(&mut text_buf)));
                            }
                            chars.next(); // consume %
                            let tag_content = self.read_until(&mut chars, "%}")?;
                            if let Some(node) = self.parse_tag(&tag_content)? {
                                nodes.push(node);
                            }
                        }
                        '#' => {
                            // Comment: {# comment #}
                            if !text_buf.is_empty() {
                                nodes.push(TemplateNode::Text(std::mem::take(&mut text_buf)));
                            }
                            chars.next(); // consume #
                            self.read_until(&mut chars, "#}")?;
                            nodes.push(TemplateNode::Comment);
                        }
                        _ => text_buf.push(c),
                    }
                } else {
                    text_buf.push(c);
                }
            } else {
                text_buf.push(c);
            }
        }

        if !text_buf.is_empty() {
            nodes.push(TemplateNode::Text(text_buf));
        }

        Ok(nodes)
    }

    fn read_until(
        &self,
        chars: &mut std::iter::Peekable<std::str::Chars>,
        end: &str,
    ) -> Result<String, TemplateError> {
        let mut content = String::new();
        let _end_chars: Vec<char> = end.chars().collect();

        for c in chars.by_ref() {
            content.push(c);
            if content.ends_with(end) {
                content.truncate(content.len() - end.len());
                return Ok(content.trim().to_string());
            }
        }

        Err(TemplateError::SyntaxError(format!("Unclosed tag, expected '{}'", end)))
    }

    fn parse_variable(&self, content: &str) -> (String, Vec<String>) {
        let parts: Vec<&str> = content.split('|').collect();
        let name = parts[0].trim().to_string();
        let filters = parts[1..].iter().map(|s| s.trim().to_string()).collect();
        (name, filters)
    }

    fn parse_tag(&self, content: &str) -> Result<Option<TemplateNode>, TemplateError> {
        let parts: Vec<&str> = content.split_whitespace().collect();
        if parts.is_empty() {
            return Ok(None);
        }

        match parts[0] {
            "if" => {
                let condition = parts[1..].join(" ");
                Ok(Some(TemplateNode::If {
                    condition,
                    then_nodes: Vec::new(),
                    else_nodes: Vec::new(),
                }))
            }
            "for" => {
                if parts.len() >= 4 && parts[2] == "in" {
                    Ok(Some(TemplateNode::For {
                        var: parts[1].to_string(),
                        iterable: parts[3].to_string(),
                        nodes: Vec::new(),
                    }))
                } else {
                    Err(TemplateError::SyntaxError("Invalid for syntax".into()))
                }
            }
            "block" => {
                let name = parts.get(1).unwrap_or(&"").to_string();
                Ok(Some(TemplateNode::Block {
                    name,
                    nodes: Vec::new(),
                }))
            }
            "extends" => {
                let parent =
                    parts.get(1).unwrap_or(&"").trim_matches('"').trim_matches('\'').to_string();
                Ok(Some(TemplateNode::Extends(parent)))
            }
            "include" => {
                let template =
                    parts.get(1).unwrap_or(&"").trim_matches('"').trim_matches('\'').to_string();
                Ok(Some(TemplateNode::Include(template)))
            }
            "endif" | "endfor" | "endblock" | "else" => Ok(None),
            _ => Ok(None),
        }
    }
}

/// Filter function type alias
type FilterFn = Box<dyn Fn(&str) -> String + Send + Sync>;

/// Template renderer
pub struct TemplateRenderer {
    filters: HashMap<String, FilterFn>,
}

impl Default for TemplateRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl TemplateRenderer {
    pub fn new() -> Self {
        let mut renderer = Self {
            filters: HashMap::new(),
        };
        renderer.register_builtin_filters();
        renderer
    }

    fn register_builtin_filters(&mut self) {
        self.filters.insert("upper".into(), Box::new(|s| s.to_uppercase()));
        self.filters.insert("lower".into(), Box::new(|s| s.to_lowercase()));
        self.filters.insert(
            "title".into(),
            Box::new(|s| {
                s.split_whitespace()
                    .map(|w| {
                        let mut c = w.chars();
                        match c.next() {
                            None => String::new(),
                            Some(f) => f.to_uppercase().chain(c).collect(),
                        }
                    })
                    .collect::<Vec<_>>()
                    .join(" ")
            }),
        );
        self.filters.insert("trim".into(), Box::new(|s| s.trim().to_string()));
        self.filters.insert("length".into(), Box::new(|s| s.len().to_string()));
        self.filters.insert(
            "default".into(),
            Box::new(|s| {
                if s.is_empty() {
                    "".to_string()
                } else {
                    s.to_string()
                }
            }),
        );
        self.filters.insert("safe".into(), Box::new(|s| s.to_string()));
        self.filters.insert("escape".into(), Box::new(|s| escape_html(s).to_string()));
    }

    pub fn register_filter<F>(&mut self, name: impl Into<String>, filter: F)
    where
        F: Fn(&str) -> String + Send + Sync + 'static,
    {
        self.filters.insert(name.into(), Box::new(filter));
    }

    pub fn render(
        &self,
        template: &CompiledTemplate,
        context: &Context,
    ) -> Result<String, TemplateError> {
        let mut output = String::new();
        for node in template.nodes() {
            output.push_str(&self.render_node(node, context)?);
        }
        Ok(output)
    }

    fn render_node(&self, node: &TemplateNode, context: &Context) -> Result<String, TemplateError> {
        match node {
            TemplateNode::Text(text) => Ok(text.clone()),
            TemplateNode::Variable { name, filters } => {
                let value = context.get(name).map(|v| v.as_string()).unwrap_or_default();

                let mut result = value;
                for filter_name in filters {
                    if let Some(filter) = self.filters.get(filter_name) {
                        result = filter(&result);
                    }
                }

                if context.autoescape() && !filters.contains(&"safe".to_string()) {
                    Ok(escape_html(&result).to_string())
                } else {
                    Ok(result)
                }
            }
            TemplateNode::If {
                condition,
                then_nodes,
                else_nodes,
            } => {
                let cond_value = context.get(condition).map(|v| v.as_bool()).unwrap_or(false);
                let nodes = if cond_value { then_nodes } else { else_nodes };
                let mut output = String::new();
                for n in nodes {
                    output.push_str(&self.render_node(n, context)?);
                }
                Ok(output)
            }
            TemplateNode::For {
                var,
                iterable,
                nodes,
            } => {
                let mut output = String::new();
                if let Some(ContextValue::List(items)) = context.get(iterable) {
                    for item in items {
                        let mut loop_context = context.clone();
                        loop_context.set(var.clone(), item.clone());
                        for n in nodes {
                            output.push_str(&self.render_node(n, &loop_context)?);
                        }
                    }
                }
                Ok(output)
            }
            TemplateNode::Block { nodes, .. } => {
                let mut output = String::new();
                for n in nodes {
                    output.push_str(&self.render_node(n, context)?);
                }
                Ok(output)
            }
            TemplateNode::Comment => Ok(String::new()),
            TemplateNode::Extends(_) | TemplateNode::Include(_) => Ok(String::new()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_escape_html() {
        assert_eq!(escape_html("<script>").as_str(), "&lt;script&gt;");
        assert_eq!(escape_html("a & b").as_str(), "a &amp; b");
        assert_eq!(escape_html("\"quoted\"").as_str(), "&#34;quoted&#34;");
        assert_eq!(escape_html("normal").as_str(), "normal");
    }

    #[test]
    fn test_markup() {
        let m = Markup::new("<b>bold</b>");
        assert_eq!(m.as_str(), "<b>bold</b>");
        assert!(!m.is_empty());
        assert_eq!(m.len(), 11);
    }

    #[test]
    fn test_context() {
        let mut ctx = Context::new();
        ctx.set("name", ContextValue::String("World".into()));
        ctx.set("count", ContextValue::Int(42));

        assert_eq!(ctx.get("name").unwrap().as_string(), "World");
        assert_eq!(ctx.get("count").unwrap().as_string(), "42");
    }

    #[test]
    fn test_compile_simple() {
        let compiler = TemplateCompiler::new();
        let template = compiler.compile("test", "Hello, {{ name }}!").unwrap();
        assert_eq!(template.name(), "test");
        assert_eq!(template.nodes().len(), 3);
    }

    #[test]
    fn test_render_variable() {
        let compiler = TemplateCompiler::new();
        let renderer = TemplateRenderer::new();
        let template = compiler.compile("test", "Hello, {{ name }}!").unwrap();

        let mut ctx = Context::new().with_autoescape(false);
        ctx.set("name", ContextValue::String("World".into()));

        let result = renderer.render(&template, &ctx).unwrap();
        assert_eq!(result, "Hello, World!");
    }

    #[test]
    fn test_render_with_filter() {
        let compiler = TemplateCompiler::new();
        let renderer = TemplateRenderer::new();
        let template = compiler.compile("test", "{{ name|upper }}").unwrap();

        let mut ctx = Context::new().with_autoescape(false);
        ctx.set("name", ContextValue::String("hello".into()));

        let result = renderer.render(&template, &ctx).unwrap();
        assert_eq!(result, "HELLO");
    }

    #[test]
    fn test_autoescape() {
        let compiler = TemplateCompiler::new();
        let renderer = TemplateRenderer::new();
        let template = compiler.compile("test", "{{ html }}").unwrap();

        let mut ctx = Context::new(); // autoescape on by default
        ctx.set("html", ContextValue::String("<script>".into()));

        let result = renderer.render(&template, &ctx).unwrap();
        assert_eq!(result, "&lt;script&gt;");
    }

    #[test]
    fn test_safe_filter() {
        let compiler = TemplateCompiler::new();
        let renderer = TemplateRenderer::new();
        let template = compiler.compile("test", "{{ html|safe }}").unwrap();

        let mut ctx = Context::new();
        ctx.set("html", ContextValue::String("<b>bold</b>".into()));

        let result = renderer.render(&template, &ctx).unwrap();
        assert_eq!(result, "<b>bold</b>");
    }

    #[test]
    fn test_comment() {
        let compiler = TemplateCompiler::new();
        let template = compiler.compile("test", "Hello{# comment #}World").unwrap();
        assert_eq!(template.nodes().len(), 3);
    }
}
