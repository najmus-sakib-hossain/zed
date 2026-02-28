//! Environment variable loading from .env files
//!
//! Supports loading environment variables from:
//! - .env (base environment)
//! - .env.local (local overrides, not committed)
//! - .env.development / .env.production / .env.test (environment-specific)
//! - .env.development.local / .env.production.local (environment-specific local overrides)

use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// Environment loader that handles .env files
pub struct EnvLoader {
    /// Loaded environment variables
    vars: HashMap<String, String>,
    /// Whether to override existing env vars
    override_existing: bool,
}

impl EnvLoader {
    /// Create a new environment loader
    pub fn new() -> Self {
        Self {
            vars: HashMap::new(),
            override_existing: false,
        }
    }

    /// Set whether to override existing environment variables
    pub fn override_existing(mut self, override_existing: bool) -> Self {
        self.override_existing = override_existing;
        self
    }

    /// Load environment variables from the standard .env files
    ///
    /// Loading order (later files override earlier):
    /// 1. .env
    /// 2. .env.local
    /// 3. .env.{NODE_ENV} (e.g., .env.development, .env.production)
    /// 4. .env.{NODE_ENV}.local
    pub fn load(&mut self, dir: &Path) -> &mut Self {
        // Get NODE_ENV or default to "development"
        let node_env = std::env::var("NODE_ENV").unwrap_or_else(|_| "development".to_string());

        // Load files in order (later overrides earlier)
        let files = [
            ".env".to_string(),
            ".env.local".to_string(),
            format!(".env.{}", node_env),
            format!(".env.{}.local", node_env),
        ];

        for file in &files {
            let path = dir.join(file);
            if path.exists() {
                self.load_file(&path);
            }
        }

        self
    }

    /// Load environment variables from a specific file
    pub fn load_file(&mut self, path: &Path) -> &mut Self {
        if let Ok(content) = fs::read_to_string(path) {
            self.parse_env_content(&content);
        }
        self
    }

    /// Parse .env file content
    fn parse_env_content(&mut self, content: &str) {
        for line in content.lines() {
            let line = line.trim();

            // Skip empty lines and comments
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            // Parse KEY=VALUE
            if let Some((key, value)) = parse_env_line(line) {
                self.vars.insert(key, value);
            }
        }
    }

    /// Apply loaded variables to the process environment
    pub fn apply(&self) {
        for (key, value) in &self.vars {
            if self.override_existing || std::env::var(key).is_err() {
                std::env::set_var(key, value);
            }
        }
    }

    /// Get all loaded variables
    pub fn vars(&self) -> &HashMap<String, String> {
        &self.vars
    }

    /// Get a specific variable
    pub fn get(&self, key: &str) -> Option<&String> {
        self.vars.get(key)
    }
}

impl Default for EnvLoader {
    fn default() -> Self {
        Self::new()
    }
}

/// Parse a single .env line into key-value pair
fn parse_env_line(line: &str) -> Option<(String, String)> {
    // Find the first '=' that's not inside quotes
    let mut in_single_quote = false;
    let mut in_double_quote = false;
    let mut equals_pos = None;

    for (i, ch) in line.char_indices() {
        match ch {
            '\'' if !in_double_quote => in_single_quote = !in_single_quote,
            '"' if !in_single_quote => in_double_quote = !in_double_quote,
            '=' if !in_single_quote && !in_double_quote && equals_pos.is_none() => {
                equals_pos = Some(i);
            }
            _ => {}
        }
    }

    let equals_pos = equals_pos?;
    let key = line[..equals_pos].trim().to_string();
    let value = parse_env_value(&line[equals_pos + 1..]);

    if key.is_empty() {
        return None;
    }

    Some((key, value))
}

/// Parse the value part of an env line, handling quotes and escapes
fn parse_env_value(value: &str) -> String {
    let value = value.trim();

    // Handle quoted values
    if (value.starts_with('"') && value.ends_with('"'))
        || (value.starts_with('\'') && value.ends_with('\''))
    {
        let inner = &value[1..value.len() - 1];
        return unescape_env_value(inner, value.starts_with('"'));
    }

    // Handle unquoted values (strip inline comments)
    if let Some(comment_pos) = value.find(" #") {
        return value[..comment_pos].trim().to_string();
    }

    value.to_string()
}

/// Unescape special characters in env values
fn unescape_env_value(value: &str, is_double_quoted: bool) -> String {
    let mut result = String::with_capacity(value.len());
    let mut chars = value.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '\\' && is_double_quoted {
            match chars.peek() {
                Some('n') => {
                    result.push('\n');
                    chars.next();
                }
                Some('r') => {
                    result.push('\r');
                    chars.next();
                }
                Some('t') => {
                    result.push('\t');
                    chars.next();
                }
                Some('\\') => {
                    result.push('\\');
                    chars.next();
                }
                Some('"') => {
                    result.push('"');
                    chars.next();
                }
                Some('$') => {
                    result.push('$');
                    chars.next();
                }
                _ => result.push(ch),
            }
        } else {
            result.push(ch);
        }
    }

    result
}

/// Load .env files from the current directory and apply to process environment
pub fn load_dotenv() {
    let cwd = std::env::current_dir().unwrap_or_default();
    let mut loader = EnvLoader::new();
    loader.load(&cwd).apply();
}

/// Load .env files from a specific directory and apply to process environment
pub fn load_dotenv_from(dir: &Path) {
    let mut loader = EnvLoader::new();
    loader.load(dir).apply();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_line() {
        let (key, value) = parse_env_line("FOO=bar").unwrap();
        assert_eq!(key, "FOO");
        assert_eq!(value, "bar");
    }

    #[test]
    fn test_parse_quoted_value() {
        let (key, value) = parse_env_line("FOO=\"bar baz\"").unwrap();
        assert_eq!(key, "FOO");
        assert_eq!(value, "bar baz");
    }

    #[test]
    fn test_parse_single_quoted_value() {
        let (key, value) = parse_env_line("FOO='bar baz'").unwrap();
        assert_eq!(key, "FOO");
        assert_eq!(value, "bar baz");
    }

    #[test]
    fn test_parse_escaped_newline() {
        let (key, value) = parse_env_line("FOO=\"bar\\nbaz\"").unwrap();
        assert_eq!(key, "FOO");
        assert_eq!(value, "bar\nbaz");
    }

    #[test]
    fn test_parse_inline_comment() {
        let (key, value) = parse_env_line("FOO=bar # this is a comment").unwrap();
        assert_eq!(key, "FOO");
        assert_eq!(value, "bar");
    }

    #[test]
    fn test_skip_comment_line() {
        assert!(parse_env_line("# this is a comment").is_none());
    }

    #[test]
    fn test_skip_empty_line() {
        assert!(parse_env_line("").is_none());
    }

    #[test]
    fn test_parse_value_with_equals() {
        let (key, value) = parse_env_line("DATABASE_URL=postgres://user:pass@host/db").unwrap();
        assert_eq!(key, "DATABASE_URL");
        assert_eq!(value, "postgres://user:pass@host/db");
    }

    #[test]
    fn test_env_loader() {
        let mut loader = EnvLoader::new();
        loader.parse_env_content(
            r#"
# Database config
DATABASE_URL=postgres://localhost/test
API_KEY="secret-key-123"
DEBUG=true
"#,
        );

        assert_eq!(loader.get("DATABASE_URL"), Some(&"postgres://localhost/test".to_string()));
        assert_eq!(loader.get("API_KEY"), Some(&"secret-key-123".to_string()));
        assert_eq!(loader.get("DEBUG"), Some(&"true".to_string()));
    }
}
