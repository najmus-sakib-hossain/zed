//! # Config Module - The God Mode
//!
//! Parses the `dx` configuration file and enforces security capabilities.
//!
//! ## Capabilities
//! - `net`: Network access (fetch, websocket)
//! - `fs`: File system access
//! - `env`: Environment variables
//! - `crypto`: Cryptographic operations
//!
//! ## Example dx config
//! ```dx
//! project "my-app"
//! version "1.0.0"
//!
//! capabilities {
//!     net: ["api.example.com", "cdn.example.com"]
//!     fs: read
//! }
//! ```

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

/// Project configuration parsed from the `dx` God File
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DxConfig {
    /// Project name
    pub name: String,
    /// Project version
    pub version: String,
    /// Security capabilities
    pub capabilities: Capabilities,
    /// Build configuration
    pub build: BuildConfig,
    /// PWA configuration
    pub pwa: Option<PwaConfig>,
}

/// Security capabilities - what the app is allowed to do
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Capabilities {
    /// Allowed network domains (empty = no network access)
    pub net: Vec<String>,
    /// File system access level
    pub fs: FsAccess,
    /// Environment variable access
    pub env: bool,
    /// Crypto API access
    pub crypto: bool,
}

/// File system access levels
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub enum FsAccess {
    #[default]
    None,
    Read,
    Write,
    ReadWrite,
}

/// Build configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BuildConfig {
    /// Output directory
    pub output: String,
    /// Enable minification
    pub minify: bool,
    /// Enable source maps
    pub sourcemap: bool,
}

/// PWA configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PwaConfig {
    pub name: String,
    pub short_name: String,
    pub theme_color: String,
    pub background_color: String,
}

impl DxConfig {
    pub fn new() -> Self {
        Self::default()
    }
}

/// Parse the `dx` God File from the project root
pub fn parse_god_file(root: &Path, verbose: bool) -> Result<DxConfig> {
    let config_path = root.join("dx");

    if !config_path.exists() {
        if verbose {
            println!("  ⚙️ No dx config found, using defaults");
        }
        return Ok(DxConfig::default());
    }

    if verbose {
        println!("  ⚙️ Parsing dx config...");
    }

    let source = fs::read_to_string(&config_path)
        .with_context(|| format!("Failed to read dx config: {}", config_path.display()))?;

    parse_config(&source, verbose)
}

/// Parse the config content
fn parse_config(source: &str, verbose: bool) -> Result<DxConfig> {
    let mut config = DxConfig::default();

    // Simple line-by-line parser for dx config format
    // Production would use a proper parser
    for line in source.lines() {
        let line = line.trim();

        // Skip comments and empty lines
        if line.is_empty() || line.starts_with("//") || line.starts_with('#') {
            continue;
        }

        // Parse project name: project "name"
        if line.starts_with("project") {
            if let Some(name) = extract_quoted_value(line) {
                config.name = name;
            }
        }

        // Parse version: version "1.0.0"
        if line.starts_with("version") {
            if let Some(version) = extract_quoted_value(line) {
                config.version = version;
            }
        }

        // Parse net capability: net: ["domain1", "domain2"]
        if line.starts_with("net:") {
            config.capabilities.net = extract_string_array(line);
        }

        // Parse fs capability: fs: read | write | none
        if line.starts_with("fs:") {
            let value = line.trim_start_matches("fs:").trim();
            config.capabilities.fs = match value.to_lowercase().as_str() {
                "read" => FsAccess::Read,
                "write" => FsAccess::Write,
                "readwrite" | "read-write" => FsAccess::ReadWrite,
                _ => FsAccess::None,
            };
        }

        // Parse env capability: env: true | false
        if line.starts_with("env:") {
            let value = line.trim_start_matches("env:").trim();
            config.capabilities.env = value == "true";
        }
    }

    if verbose && !config.name.is_empty() {
        println!("    Project: {} v{}", config.name, config.version);
        println!("    Net: {:?}", config.capabilities.net);
        println!("    FS: {:?}", config.capabilities.fs);
    }

    Ok(config)
}

/// Extract a quoted value from a line like: project "my-app"
fn extract_quoted_value(line: &str) -> Option<String> {
    let start = line.find('"')?;
    let rest = &line[start + 1..];
    let end = rest.find('"')?;
    Some(rest[..end].to_string())
}

/// Extract a string array from a line like: net: ["a.com", "b.com"]
fn extract_string_array(line: &str) -> Vec<String> {
    let mut results = Vec::new();

    // Find content between brackets
    if let (Some(start), Some(end)) = (line.find('['), line.rfind(']')) {
        let content = &line[start + 1..end];
        for item in content.split(',') {
            let item = item.trim().trim_matches('"').trim_matches('\'');
            if !item.is_empty() {
                results.push(item.to_string());
            }
        }
    }

    results
}

/// Capability violation error
#[derive(Debug, Clone)]
pub struct CapabilityViolation {
    pub capability: String,
    pub attempted: String,
    pub file: String,
    pub line: usize,
}

/// Enforce capabilities by scanning the parsed code
pub fn enforce_capabilities(
    config: &DxConfig,
    source: &str,
    file_path: &str,
    verbose: bool,
) -> Result<Vec<CapabilityViolation>> {
    let mut violations = Vec::new();

    if verbose {
        println!("  🔒 Enforcing capabilities for {}", file_path);
    }

    // Check for network access
    for (line_num, line) in source.lines().enumerate() {
        // Check for fetch() calls
        if line.contains("fetch(") {
            // Extract the URL being fetched
            if let Some(url) = extract_fetch_url(line) {
                if !is_allowed_domain(&url, &config.capabilities.net) {
                    violations.push(CapabilityViolation {
                        capability: "net".to_string(),
                        attempted: url,
                        file: file_path.to_string(),
                        line: line_num + 1,
                    });
                }
            }
        }

        // Check for fs access (in server code)
        if (line.contains("fs.read") || line.contains("fs.write") || line.contains("Deno.read"))
            && config.capabilities.fs == FsAccess::None
        {
            violations.push(CapabilityViolation {
                capability: "fs".to_string(),
                attempted: "file system access".to_string(),
                file: file_path.to_string(),
                line: line_num + 1,
            });
        }
    }

    if verbose && !violations.is_empty() {
        println!("    ⚠️ Found {} capability violations", violations.len());
    }

    Ok(violations)
}

/// Extract URL from a fetch() call
fn extract_fetch_url(line: &str) -> Option<String> {
    // Simple extraction: fetch("https://example.com")
    let start = line.find("fetch(")? + 6;
    let rest = &line[start..];

    // Find the quoted URL
    let quote_char = if rest.starts_with('"') {
        '"'
    } else if rest.starts_with('\'') {
        '\''
    } else if rest.starts_with('`') {
        '`'
    } else {
        return None;
    };

    let rest = &rest[1..];
    let end = rest.find(quote_char)?;
    Some(rest[..end].to_string())
}

/// Check if a URL's domain is in the allowed list
fn is_allowed_domain(url: &str, allowed: &[String]) -> bool {
    if allowed.is_empty() {
        return false;
    }

    // Extract domain from URL
    let domain = if url.starts_with("http://") || url.starts_with("https://") {
        url.split("//").nth(1).and_then(|s| s.split('/').next())
    } else {
        url.split('/').next()
    };

    if let Some(domain) = domain {
        allowed.iter().any(|a| domain == a || domain.ends_with(&format!(".{}", a)))
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_quoted_value() {
        assert_eq!(extract_quoted_value(r#"project "my-app""#), Some("my-app".to_string()));
        assert_eq!(extract_quoted_value(r#"version "1.0.0""#), Some("1.0.0".to_string()));
    }

    #[test]
    fn test_extract_string_array() {
        let arr = extract_string_array(r#"net: ["api.com", "cdn.com"]"#);
        assert_eq!(arr, vec!["api.com", "cdn.com"]);
    }

    #[test]
    fn test_is_allowed_domain() {
        let allowed = vec!["api.example.com".to_string(), "cdn.example.com".to_string()];
        assert!(is_allowed_domain("https://api.example.com/users", &allowed));
        assert!(is_allowed_domain("https://cdn.example.com/image.png", &allowed));
        assert!(!is_allowed_domain("https://evil.com/steal", &allowed));
    }

    #[test]
    fn test_extract_fetch_url() {
        assert_eq!(
            extract_fetch_url(r#"const res = await fetch("https://api.com/data");"#),
            Some("https://api.com/data".to_string())
        );
    }
}
