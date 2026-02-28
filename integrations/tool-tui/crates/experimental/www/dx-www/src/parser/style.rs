//! # Style Parser
//!
//! Parses the `<style>` section of component files.
//! Supports scoped styles, atomic CSS classes, and global styles.

#![allow(missing_docs)]

use std::path::Path;

use crate::error::DxResult;

// =============================================================================
// Parsed Style
// =============================================================================

/// A parsed style section.
#[derive(Debug, Clone)]
pub struct ParsedStyle {
    /// Source CSS
    pub source: String,

    /// Whether styles are scoped
    pub scoped: bool,

    /// Parsed CSS rules
    pub rules: Vec<CssRule>,

    /// Detected atomic classes
    pub atomic_classes: Vec<String>,

    /// CSS custom properties (variables)
    pub custom_properties: Vec<CustomProperty>,

    /// Import statements
    pub imports: Vec<CssImport>,
}

/// A CSS rule.
#[derive(Debug, Clone)]
pub struct CssRule {
    /// Selector
    pub selector: String,

    /// Declarations
    pub declarations: Vec<CssDeclaration>,

    /// Whether this is an at-rule
    pub at_rule: Option<AtRule>,
}

/// A CSS declaration (property: value).
#[derive(Debug, Clone)]
pub struct CssDeclaration {
    /// Property name
    pub property: String,

    /// Property value
    pub value: String,

    /// Whether this is important
    pub important: bool,
}

/// An at-rule.
#[derive(Debug, Clone)]
pub enum AtRule {
    /// @media query
    Media { query: String },
    /// @keyframes animation
    Keyframes { name: String },
    /// @import
    Import { url: String },
    /// @supports
    Supports { condition: String },
    /// @layer
    Layer { name: Option<String> },
}

/// A CSS custom property.
#[derive(Debug, Clone)]
pub struct CustomProperty {
    /// Property name (e.g., "--primary-color")
    pub name: String,
    /// Property value
    pub value: String,
}

/// A CSS import statement.
#[derive(Debug, Clone)]
pub struct CssImport {
    /// Import URL
    pub url: String,
    /// Media query (if any)
    pub media: Option<String>,
}

// =============================================================================
// Style Parser
// =============================================================================

/// Parser for style sections.
#[derive(Debug, Default)]
pub struct StyleParser {
    // Configuration can be added here
}

impl StyleParser {
    /// Create a new style parser.
    pub fn new() -> Self {
        Self::default()
    }

    /// Parse a style section.
    pub fn parse(&self, source: &str, scoped: bool, _path: &Path) -> DxResult<ParsedStyle> {
        let source = source.trim().to_string();
        let mut rules = Vec::new();
        let mut atomic_classes = Vec::new();
        let mut custom_properties = Vec::new();
        let mut imports = Vec::new();

        // Parse the CSS
        self.parse_css(
            &source,
            &mut rules,
            &mut atomic_classes,
            &mut custom_properties,
            &mut imports,
        );

        Ok(ParsedStyle {
            source,
            scoped,
            rules,
            atomic_classes,
            custom_properties,
            imports,
        })
    }

    /// Parse CSS content.
    fn parse_css(
        &self,
        source: &str,
        rules: &mut Vec<CssRule>,
        atomic_classes: &mut Vec<String>,
        custom_properties: &mut Vec<CustomProperty>,
        imports: &mut Vec<CssImport>,
    ) {
        let chars: Vec<char> = source.chars().collect();
        let mut pos = 0;

        while pos < chars.len() {
            // Skip whitespace and comments
            pos = self.skip_whitespace_and_comments(&chars, pos);
            if pos >= chars.len() {
                break;
            }

            // Check for at-rule
            if chars[pos] == '@' {
                let (rule, end) =
                    self.parse_at_rule(&chars, pos, atomic_classes, custom_properties, imports);
                if let Some(rule) = rule {
                    rules.push(rule);
                }
                pos = end;
                continue;
            }

            // Parse regular rule
            let (rule, end) = self.parse_rule(&chars, pos, atomic_classes, custom_properties);
            if let Some(rule) = rule {
                rules.push(rule);
            }
            pos = end;
        }
    }

    /// Skip whitespace and comments.
    fn skip_whitespace_and_comments(&self, chars: &[char], start: usize) -> usize {
        let mut pos = start;

        while pos < chars.len() {
            // Skip whitespace
            while pos < chars.len() && chars[pos].is_whitespace() {
                pos += 1;
            }

            // Check for comment
            if pos + 1 < chars.len() && chars[pos] == '/' && chars[pos + 1] == '*' {
                pos += 2;
                while pos + 1 < chars.len() && !(chars[pos] == '*' && chars[pos + 1] == '/') {
                    pos += 1;
                }
                if pos + 1 < chars.len() {
                    pos += 2;
                }
            } else {
                break;
            }
        }

        pos
    }

    /// Parse an at-rule.
    fn parse_at_rule(
        &self,
        chars: &[char],
        start: usize,
        atomic_classes: &mut Vec<String>,
        custom_properties: &mut Vec<CustomProperty>,
        imports: &mut Vec<CssImport>,
    ) -> (Option<CssRule>, usize) {
        let mut pos = start + 1; // Skip '@'

        // Parse at-rule name
        let mut name = String::new();
        while pos < chars.len() && chars[pos].is_alphabetic() {
            name.push(chars[pos]);
            pos += 1;
        }

        // Skip whitespace
        while pos < chars.len() && chars[pos].is_whitespace() {
            pos += 1;
        }

        match name.as_str() {
            "import" => {
                let (import, end) = self.parse_import(chars, pos);
                imports.push(import);
                return (None, end);
            }
            "media" => {
                let (rule, end) =
                    self.parse_media_rule(chars, pos, atomic_classes, custom_properties);
                return (rule, end);
            }
            "keyframes" => {
                let (rule, end) = self.parse_keyframes(chars, pos);
                return (rule, end);
            }
            "apply" => {
                // Tailwind-style @apply directive
                let (classes, end) = self.parse_apply(chars, pos);
                atomic_classes.extend(classes);
                return (None, end);
            }
            _ => {}
        }

        // Skip to end of rule
        let mut depth = 0;
        while pos < chars.len() {
            if chars[pos] == '{' {
                depth += 1;
            } else if chars[pos] == '}' {
                depth -= 1;
                if depth == 0 {
                    pos += 1;
                    break;
                }
            } else if chars[pos] == ';' && depth == 0 {
                pos += 1;
                break;
            }
            pos += 1;
        }

        (None, pos)
    }

    /// Parse an @import rule.
    fn parse_import(&self, chars: &[char], start: usize) -> (CssImport, usize) {
        let mut pos = start;
        let mut url = String::new();
        let mut media = None;

        // Parse URL
        if pos < chars.len() && (chars[pos] == '"' || chars[pos] == '\'') {
            let quote = chars[pos];
            pos += 1;
            while pos < chars.len() && chars[pos] != quote {
                url.push(chars[pos]);
                pos += 1;
            }
            pos += 1;
        } else if self.starts_with(chars, pos, "url(") {
            pos += 4;
            while pos < chars.len() && chars[pos] != ')' {
                if chars[pos] != '"' && chars[pos] != '\'' {
                    url.push(chars[pos]);
                }
                pos += 1;
            }
            pos += 1;
        }

        // Skip whitespace
        while pos < chars.len() && chars[pos].is_whitespace() {
            pos += 1;
        }

        // Check for media query
        if pos < chars.len() && chars[pos] != ';' {
            let mut media_query = String::new();
            while pos < chars.len() && chars[pos] != ';' {
                media_query.push(chars[pos]);
                pos += 1;
            }
            if !media_query.trim().is_empty() {
                media = Some(media_query.trim().to_string());
            }
        }

        // Skip semicolon
        if pos < chars.len() && chars[pos] == ';' {
            pos += 1;
        }

        (CssImport { url, media }, pos)
    }

    /// Parse a @media rule.
    fn parse_media_rule(
        &self,
        chars: &[char],
        start: usize,
        atomic_classes: &mut Vec<String>,
        custom_properties: &mut Vec<CustomProperty>,
    ) -> (Option<CssRule>, usize) {
        let mut pos = start;

        // Parse media query
        let mut query = String::new();
        while pos < chars.len() && chars[pos] != '{' {
            query.push(chars[pos]);
            pos += 1;
        }

        // Skip opening brace
        if pos < chars.len() {
            pos += 1;
        }

        // Parse nested rules (simplified)
        let declarations = Vec::new();
        let mut depth = 1;
        let content_start = pos;

        while pos < chars.len() && depth > 0 {
            if chars[pos] == '{' {
                depth += 1;
            } else if chars[pos] == '}' {
                depth -= 1;
            }
            pos += 1;
        }

        let content: String = chars[content_start..pos - 1].iter().collect();
        self.parse_css(
            &content,
            &mut Vec::new(),
            atomic_classes,
            custom_properties,
            &mut Vec::new(),
        );

        (
            Some(CssRule {
                selector: format!("@media {}", query.trim()),
                declarations,
                at_rule: Some(AtRule::Media {
                    query: query.trim().to_string(),
                }),
            }),
            pos,
        )
    }

    /// Parse a @keyframes rule.
    fn parse_keyframes(&self, chars: &[char], start: usize) -> (Option<CssRule>, usize) {
        let mut pos = start;

        // Parse animation name
        let mut name = String::new();
        while pos < chars.len() && chars[pos] != '{' && !chars[pos].is_whitespace() {
            name.push(chars[pos]);
            pos += 1;
        }

        // Skip to end of rule
        let mut depth = 0;
        while pos < chars.len() {
            if chars[pos] == '{' {
                depth += 1;
            } else if chars[pos] == '}' {
                depth -= 1;
                if depth == 0 {
                    pos += 1;
                    break;
                }
            }
            pos += 1;
        }

        (
            Some(CssRule {
                selector: format!("@keyframes {name}"),
                declarations: Vec::new(),
                at_rule: Some(AtRule::Keyframes {
                    name: name.trim().to_string(),
                }),
            }),
            pos,
        )
    }

    /// Parse @apply directive (Tailwind-style).
    fn parse_apply(&self, chars: &[char], start: usize) -> (Vec<String>, usize) {
        let mut pos = start;
        let mut classes = Vec::new();

        while pos < chars.len() && chars[pos] != ';' {
            // Skip whitespace
            while pos < chars.len() && chars[pos].is_whitespace() {
                pos += 1;
            }

            // Parse class name
            let mut class = String::new();
            while pos < chars.len() && !chars[pos].is_whitespace() && chars[pos] != ';' {
                class.push(chars[pos]);
                pos += 1;
            }

            if !class.is_empty() {
                classes.push(class);
            }
        }

        // Skip semicolon
        if pos < chars.len() && chars[pos] == ';' {
            pos += 1;
        }

        (classes, pos)
    }

    /// Parse a regular CSS rule.
    fn parse_rule(
        &self,
        chars: &[char],
        start: usize,
        atomic_classes: &mut Vec<String>,
        custom_properties: &mut Vec<CustomProperty>,
    ) -> (Option<CssRule>, usize) {
        let mut pos = start;

        // Parse selector
        let mut selector = String::new();
        while pos < chars.len() && chars[pos] != '{' {
            selector.push(chars[pos]);
            pos += 1;
        }

        let selector = selector.trim().to_string();
        if selector.is_empty() {
            return (None, pos);
        }

        // Extract class names from selector
        for word in selector.split_whitespace() {
            if word.starts_with('.') {
                let class = word.trim_start_matches('.').split(|c| c == ':' || c == '[').next();
                if let Some(class) = class {
                    if self.is_atomic_class(class) {
                        atomic_classes.push(class.to_string());
                    }
                }
            }
        }

        // Skip opening brace
        if pos < chars.len() {
            pos += 1;
        }

        // Parse declarations
        let mut declarations = Vec::new();
        while pos < chars.len() && chars[pos] != '}' {
            // Skip whitespace
            pos = self.skip_whitespace_and_comments(chars, pos);
            if pos >= chars.len() || chars[pos] == '}' {
                break;
            }

            // Parse property
            let mut property = String::new();
            while pos < chars.len() && chars[pos] != ':' && chars[pos] != '}' {
                property.push(chars[pos]);
                pos += 1;
            }

            let property = property.trim().to_string();
            if property.is_empty() || pos >= chars.len() || chars[pos] == '}' {
                break;
            }

            // Skip colon
            pos += 1;

            // Parse value
            let mut value = String::new();
            let mut important = false;
            while pos < chars.len() && chars[pos] != ';' && chars[pos] != '}' {
                value.push(chars[pos]);
                pos += 1;
            }

            let value = value.trim().to_string();
            if value.ends_with("!important") {
                important = true;
            }
            let value = value.trim_end_matches("!important").trim().to_string();

            // Check for custom property
            if property.starts_with("--") {
                custom_properties.push(CustomProperty {
                    name: property.clone(),
                    value: value.clone(),
                });
            }

            declarations.push(CssDeclaration {
                property,
                value,
                important,
            });

            // Skip semicolon
            if pos < chars.len() && chars[pos] == ';' {
                pos += 1;
            }
        }

        // Skip closing brace
        if pos < chars.len() && chars[pos] == '}' {
            pos += 1;
        }

        (
            Some(CssRule {
                selector,
                declarations,
                at_rule: None,
            }),
            pos,
        )
    }

    /// Check if a class name looks like an atomic class.
    fn is_atomic_class(&self, class: &str) -> bool {
        // Common atomic class patterns
        let patterns = [
            "flex",
            "grid",
            "block",
            "inline",
            "hidden",
            "w-",
            "h-",
            "m-",
            "p-",
            "gap-",
            "text-",
            "font-",
            "bg-",
            "border-",
            "rounded",
            "shadow",
            "opacity",
            "justify-",
            "items-",
            "self-",
            "absolute",
            "relative",
            "fixed",
            "sticky",
            "top-",
            "right-",
            "bottom-",
            "left-",
            "z-",
            "overflow-",
        ];

        patterns.iter().any(|p| class.starts_with(p) || class == *p)
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
    fn test_parse_simple_style() {
        let parser = StyleParser::new();
        let source = r#"
            .container {
                max-width: 1200px;
                margin: 0 auto;
            }
        "#;

        let result = parser.parse(source, false, &PathBuf::from("test.pg"));
        assert!(result.is_ok());

        let style = result.unwrap();
        assert_eq!(style.rules.len(), 1);
        assert_eq!(style.rules[0].declarations.len(), 2);
    }

    #[test]
    fn test_parse_scoped_style() {
        let parser = StyleParser::new();
        let source = "h1 { color: blue; }";

        let result = parser.parse(source, true, &PathBuf::from("test.pg"));
        assert!(result.is_ok());

        let style = result.unwrap();
        assert!(style.scoped);
    }

    #[test]
    fn test_parse_import() {
        let parser = StyleParser::new();
        let source = r#"
            @import "other.css";
            .test { color: red; }
        "#;

        let result = parser.parse(source, false, &PathBuf::from("test.pg"));
        assert!(result.is_ok());

        let style = result.unwrap();
        assert_eq!(style.imports.len(), 1);
        assert_eq!(style.imports[0].url, "other.css");
    }

    #[test]
    fn test_detect_atomic_classes() {
        let parser = StyleParser::new();
        let source = r#"
            .flex { display: flex; }
            .w-full { width: 100%; }
            .bg-blue-500 { background-color: blue; }
        "#;

        let result = parser.parse(source, false, &PathBuf::from("test.pg"));
        assert!(result.is_ok());

        let style = result.unwrap();
        assert!(style.atomic_classes.contains(&"flex".to_string()));
        assert!(style.atomic_classes.contains(&"w-full".to_string()));
        assert!(style.atomic_classes.contains(&"bg-blue-500".to_string()));
    }

    #[test]
    fn test_parse_custom_properties() {
        let parser = StyleParser::new();
        let source = r#"
            :root {
                --primary-color: blue;
                --spacing: 1rem;
            }
        "#;

        let result = parser.parse(source, false, &PathBuf::from("test.pg"));
        assert!(result.is_ok());

        let style = result.unwrap();
        assert_eq!(style.custom_properties.len(), 2);
    }
}
