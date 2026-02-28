//! .env file loading support
//!
//! Implements loading of environment variables from .env files:
//! - .env - Base environment file
//! - .env.local - Local overrides (not committed to git)
//! - .env.development - Development environment
//! - .env.production - Production environment
//! - .env.test - Test environment

use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// Load environment variables from .env files
///
/// Files are loaded in order of precedence (later files override earlier):
/// 1. .env - Base environment
/// 2. .env.local - Local overrides
/// 3. .env.{NODE_ENV} - Environment-specific (development, production, test)
/// 4. .env.{NODE_ENV}.local - Environment-specific local overrides
pub fn load_dotenv(base_path: Option<&Path>) -> HashMap<String, String> {
    let mut env_vars = HashMap::new();

    // Determine base path (current directory if not specified)
    let base = base_path
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());

    // Get NODE_ENV from system environment
    let node_env = std::env::var("NODE_ENV").unwrap_or_else(|_| "development".to_string());

    // Load files in order of precedence
    let files = [
        ".env".to_string(),
        ".env.local".to_string(),
        format!(".env.{}", node_env),
        format!(".env.{}.local", node_env),
    ];

    for file in &files {
        let path = base.join(file);
        if let Ok(contents) = fs::read_to_string(&path) {
            parse_dotenv(&contents, &mut env_vars);
        }
    }

    env_vars
}

/// Parse a .env file contents and add to the environment map
fn parse_dotenv(contents: &str, env_vars: &mut HashMap<String, String>) {
    for line in contents.lines() {
        let line = line.trim();

        // Skip empty lines and comments
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        // Parse KEY=VALUE format
        if let Some((key, value)) = parse_env_line(line) {
            env_vars.insert(key, value);
        }
    }
}

/// Parse a single environment variable line
fn parse_env_line(line: &str) -> Option<(String, String)> {
    // Find the first '=' that's not inside quotes
    let mut in_quotes = false;
    let mut quote_char = ' ';
    let mut equals_pos = None;

    for (i, ch) in line.char_indices() {
        if !in_quotes {
            if ch == '"' || ch == '\'' {
                in_quotes = true;
                quote_char = ch;
            } else if ch == '=' && equals_pos.is_none() {
                equals_pos = Some(i);
                break;
            }
        } else if ch == quote_char {
            in_quotes = false;
        }
    }

    let equals_pos = equals_pos?;

    let key = line[..equals_pos].trim().to_string();
    let value = line[equals_pos + 1..].trim();

    // Handle quoted values
    let value = if (value.starts_with('"') && value.ends_with('"'))
        || (value.starts_with('\'') && value.ends_with('\''))
    {
        // Remove quotes and handle escape sequences
        let inner = &value[1..value.len() - 1];
        unescape_value(inner)
    } else {
        // Unquoted value - remove inline comments
        let value = if let Some(comment_pos) = value.find(" #") {
            value[..comment_pos].trim()
        } else {
            value
        };
        value.to_string()
    };

    if key.is_empty() {
        return None;
    }

    Some((key, value))
}

/// Unescape special characters in a quoted value
fn unescape_value(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '\\' {
            if let Some(&next) = chars.peek() {
                match next {
                    'n' => {
                        result.push('\n');
                        chars.next();
                    }
                    'r' => {
                        result.push('\r');
                        chars.next();
                    }
                    't' => {
                        result.push('\t');
                        chars.next();
                    }
                    '\\' => {
                        result.push('\\');
                        chars.next();
                    }
                    '"' => {
                        result.push('"');
                        chars.next();
                    }
                    '\'' => {
                        result.push('\'');
                        chars.next();
                    }
                    _ => {
                        result.push(ch);
                    }
                }
            } else {
                result.push(ch);
            }
        } else {
            result.push(ch);
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple() {
        let mut env = HashMap::new();
        parse_dotenv("KEY=value", &mut env);
        assert_eq!(env.get("KEY"), Some(&"value".to_string()));
    }

    #[test]
    fn test_parse_quoted() {
        let mut env = HashMap::new();
        parse_dotenv("KEY=\"hello world\"", &mut env);
        assert_eq!(env.get("KEY"), Some(&"hello world".to_string()));
    }

    #[test]
    fn test_parse_single_quoted() {
        let mut env = HashMap::new();
        parse_dotenv("KEY='hello world'", &mut env);
        assert_eq!(env.get("KEY"), Some(&"hello world".to_string()));
    }

    #[test]
    fn test_parse_escape_sequences() {
        let mut env = HashMap::new();
        parse_dotenv("KEY=\"line1\\nline2\"", &mut env);
        assert_eq!(env.get("KEY"), Some(&"line1\nline2".to_string()));
    }

    #[test]
    fn test_parse_comments() {
        let mut env = HashMap::new();
        parse_dotenv("# This is a comment\nKEY=value", &mut env);
        assert_eq!(env.get("KEY"), Some(&"value".to_string()));
        assert_eq!(env.len(), 1);
    }

    #[test]
    fn test_parse_inline_comments() {
        let mut env = HashMap::new();
        parse_dotenv("KEY=value # inline comment", &mut env);
        assert_eq!(env.get("KEY"), Some(&"value".to_string()));
    }

    #[test]
    fn test_parse_empty_value() {
        let mut env = HashMap::new();
        parse_dotenv("KEY=", &mut env);
        assert_eq!(env.get("KEY"), Some(&"".to_string()));
    }

    #[test]
    fn test_parse_multiple() {
        let mut env = HashMap::new();
        parse_dotenv("KEY1=value1\nKEY2=value2\nKEY3=value3", &mut env);
        assert_eq!(env.get("KEY1"), Some(&"value1".to_string()));
        assert_eq!(env.get("KEY2"), Some(&"value2".to_string()));
        assert_eq!(env.get("KEY3"), Some(&"value3".to_string()));
    }
}
