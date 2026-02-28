//! .npmrc Configuration Parser
//!
//! Parses npm configuration files (.npmrc) to support:
//! - Custom registry URLs
//! - Authentication tokens
//! - Scoped registry configurations
//!
//! Configuration is loaded from multiple locations with precedence:
//! 1. Project-level: ./.npmrc
//! 2. User-level: ~/.npmrc
//! 3. Global: /etc/npmrc (Unix) or %APPDATA%/npm/etc/npmrc (Windows)

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::{Error, Result};

/// Registry configuration parsed from .npmrc files
#[derive(Debug, Clone, Default)]
pub struct NpmrcConfig {
    /// Default registry URL (defaults to https://registry.npmjs.org)
    pub registry: String,
    /// Scoped registries (@scope -> registry URL)
    pub scoped_registries: HashMap<String, String>,
    /// Auth tokens (registry URL -> token)
    pub auth_tokens: HashMap<String, AuthToken>,
    /// Always-auth flag per registry
    pub always_auth: HashMap<String, bool>,
    /// Proxy settings
    pub proxy: Option<String>,
    pub https_proxy: Option<String>,
    /// Strict SSL verification
    pub strict_ssl: bool,
    /// CA certificate path
    pub cafile: Option<PathBuf>,
}

/// Authentication token types
#[derive(Debug, Clone)]
pub enum AuthToken {
    /// Bearer token (npm token)
    Bearer(String),
    /// Basic auth (base64 encoded username:password)
    Basic(String),
    /// Username and password (will be encoded to basic auth)
    Credentials { username: String, password: String },
}

impl NpmrcConfig {
    /// Create a new config with default values
    pub fn new() -> Self {
        Self {
            registry: "https://registry.npmjs.org".to_string(),
            scoped_registries: HashMap::new(),
            auth_tokens: HashMap::new(),
            always_auth: HashMap::new(),
            proxy: None,
            https_proxy: None,
            strict_ssl: true,
            cafile: None,
        }
    }

    /// Load configuration from all standard locations
    /// Merges configs with project-level taking highest precedence
    pub fn load() -> Result<Self> {
        let mut config = Self::new();

        // Load in order of increasing precedence
        // 1. Global config
        if let Some(global_path) = Self::global_npmrc_path()
            && global_path.exists()
        {
            config.merge_from_file(&global_path)?;
        }

        // 2. User config (~/.npmrc)
        if let Some(user_path) = Self::user_npmrc_path()
            && user_path.exists()
        {
            config.merge_from_file(&user_path)?;
        }

        // 3. Project config (./.npmrc)
        let project_path = PathBuf::from(".npmrc");
        if project_path.exists() {
            config.merge_from_file(&project_path)?;
        }

        Ok(config)
    }

    /// Load configuration from a specific file
    pub fn from_file(path: &Path) -> Result<Self> {
        let mut config = Self::new();
        config.merge_from_file(path)?;
        Ok(config)
    }

    /// Parse and merge configuration from a file
    pub fn merge_from_file(&mut self, path: &Path) -> Result<()> {
        let content = fs::read_to_string(path)
            .map_err(|e| Error::IoError(format!("Failed to read {}: {}", path.display(), e)))?;

        self.merge_from_string(&content)
    }

    /// Parse and merge configuration from a string
    pub fn merge_from_string(&mut self, content: &str) -> Result<()> {
        for (line_num, line) in content.lines().enumerate() {
            let line = line.trim();

            // Skip empty lines and comments
            if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
                continue;
            }

            // Parse key=value pairs
            if let Some((key, value)) = Self::parse_line(line) {
                self.apply_setting(&key, &value, line_num + 1)?;
            }
        }

        Ok(())
    }

    /// Parse a single line into key-value pair
    fn parse_line(line: &str) -> Option<(String, String)> {
        let parts: Vec<&str> = line.splitn(2, '=').collect();
        if parts.len() == 2 {
            let key = parts[0].trim().to_string();
            let value = parts[1].trim().to_string();
            Some((key, value))
        } else {
            None
        }
    }

    /// Apply a single configuration setting
    fn apply_setting(&mut self, key: &str, value: &str, _line_num: usize) -> Result<()> {
        match key {
            // Default registry
            "registry" => {
                self.registry = Self::normalize_registry_url(value);
            }

            // Proxy settings
            "proxy" => {
                self.proxy = if value.is_empty() {
                    None
                } else {
                    Some(value.to_string())
                };
            }
            "https-proxy" | "https_proxy" => {
                self.https_proxy = if value.is_empty() {
                    None
                } else {
                    Some(value.to_string())
                };
            }

            // SSL settings
            "strict-ssl" | "strict_ssl" => {
                self.strict_ssl = value.to_lowercase() != "false";
            }
            "cafile" => {
                self.cafile = if value.is_empty() {
                    None
                } else {
                    Some(PathBuf::from(value))
                };
            }

            // Scoped registry: @scope:registry=url
            key if key.starts_with('@') && key.contains(":registry") => {
                let scope = key.split(':').next().unwrap_or("").to_string();
                self.scoped_registries.insert(scope, Self::normalize_registry_url(value));
            }

            // Auth token for registry: //registry.example.com/:_authToken=token
            key if key.contains(":_authToken") || key.contains(":_auth") => {
                let registry_url = Self::extract_registry_from_auth_key(key);
                if key.contains(":_authToken") {
                    self.auth_tokens.insert(registry_url, AuthToken::Bearer(value.to_string()));
                } else {
                    self.auth_tokens.insert(registry_url, AuthToken::Basic(value.to_string()));
                }
            }

            // Username/password auth
            key if key.contains(":username") => {
                let registry_url = Self::extract_registry_from_auth_key(key);
                self.update_credentials(&registry_url, Some(value), None);
            }
            key if key.contains(":_password") => {
                let registry_url = Self::extract_registry_from_auth_key(key);
                // Password is base64 encoded in .npmrc
                let decoded = Self::decode_base64(value).unwrap_or_else(|_| value.to_string());
                self.update_credentials(&registry_url, None, Some(&decoded));
            }

            // Always-auth flag
            key if key.contains(":always-auth") => {
                let registry_url = Self::extract_registry_from_auth_key(key);
                self.always_auth.insert(registry_url, value.to_lowercase() == "true");
            }
            "always-auth" | "always_auth" => {
                self.always_auth.insert(self.registry.clone(), value.to_lowercase() == "true");
            }

            // Ignore unknown keys
            _ => {}
        }

        Ok(())
    }

    /// Update credentials for a registry
    fn update_credentials(
        &mut self,
        registry_url: &str,
        username: Option<&str>,
        password: Option<&str>,
    ) {
        let existing = self.auth_tokens.get(registry_url);

        let (existing_user, existing_pass) = match existing {
            Some(AuthToken::Credentials { username, password }) => {
                (Some(username.clone()), Some(password.clone()))
            }
            _ => (None, None),
        };

        let new_user = username.map(|s| s.to_string()).or(existing_user);
        let new_pass = password.map(|s| s.to_string()).or(existing_pass);

        if let (Some(u), Some(p)) = (new_user, new_pass) {
            self.auth_tokens.insert(
                registry_url.to_string(),
                AuthToken::Credentials {
                    username: u,
                    password: p,
                },
            );
        }
    }

    /// Extract registry URL from auth key like //registry.example.com/:_authToken
    fn extract_registry_from_auth_key(key: &str) -> String {
        // Remove leading // and trailing :_authToken, :_auth, :username, :_password, :always-auth
        let key = key.trim_start_matches("//");

        // Find the position of /: which separates the host from the auth key
        if let Some(pos) = key.find("/:") {
            let host = &key[..pos];
            format!("https://{}", host)
        } else {
            // Fallback: split on first : that's followed by _
            let key = key.split(":_").next().unwrap_or(key);
            let key = key.split(":always").next().unwrap_or(key);
            let key = key.split(":username").next().unwrap_or(key);
            format!("https://{}", key)
        }
    }

    /// Normalize registry URL (ensure https:// prefix, remove trailing slash)
    fn normalize_registry_url(url: &str) -> String {
        let url = url.trim();
        let url = if !url.starts_with("http://") && !url.starts_with("https://") {
            format!("https://{}", url)
        } else {
            url.to_string()
        };
        url.trim_end_matches('/').to_string()
    }

    /// Decode base64 string
    fn decode_base64(encoded: &str) -> Result<String> {
        use base64::{Engine as _, engine::general_purpose::STANDARD};
        let bytes = STANDARD
            .decode(encoded)
            .map_err(|e| Error::ParseError(format!("Invalid base64: {}", e)))?;
        String::from_utf8(bytes)
            .map_err(|e| Error::ParseError(format!("Invalid UTF-8 in base64: {}", e)))
    }

    /// Get the path to user-level .npmrc (~/.npmrc)
    pub fn user_npmrc_path() -> Option<PathBuf> {
        dirs::home_dir().map(|h| h.join(".npmrc"))
    }

    /// Get the path to global .npmrc
    pub fn global_npmrc_path() -> Option<PathBuf> {
        #[cfg(unix)]
        {
            Some(PathBuf::from("/etc/npmrc"))
        }
        #[cfg(windows)]
        {
            std::env::var("APPDATA")
                .ok()
                .map(|appdata| PathBuf::from(appdata).join("npm").join("etc").join("npmrc"))
        }
        #[cfg(not(any(unix, windows)))]
        {
            None
        }
    }

    /// Get the registry URL for a package (considering scoped registries)
    pub fn get_registry_for_package(&self, package_name: &str) -> &str {
        // Check if it's a scoped package
        if package_name.starts_with('@')
            && let Some(scope) = package_name.split('/').next()
            && let Some(registry) = self.scoped_registries.get(scope)
        {
            return registry;
        }
        &self.registry
    }

    /// Get auth token for a registry URL
    pub fn get_auth_for_registry(&self, registry_url: &str) -> Option<&AuthToken> {
        // Try exact match first
        if let Some(token) = self.auth_tokens.get(registry_url) {
            return Some(token);
        }

        // Try matching by host
        let normalized = Self::normalize_registry_url(registry_url);
        self.auth_tokens.get(&normalized)
    }

    /// Get the Authorization header value for a registry
    pub fn get_auth_header(&self, registry_url: &str) -> Option<String> {
        self.get_auth_for_registry(registry_url).map(|token| match token {
            AuthToken::Bearer(t) => format!("Bearer {}", t),
            AuthToken::Basic(b) => format!("Basic {}", b),
            AuthToken::Credentials { username, password } => {
                use base64::{Engine as _, engine::general_purpose::STANDARD};
                let credentials = format!("{}:{}", username, password);
                let encoded = STANDARD.encode(credentials.as_bytes());
                format!("Basic {}", encoded)
            }
        })
    }

    /// Check if always-auth is enabled for a registry
    pub fn requires_auth(&self, registry_url: &str) -> bool {
        self.always_auth.get(registry_url).copied().unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = NpmrcConfig::new();
        assert_eq!(config.registry, "https://registry.npmjs.org");
        assert!(config.scoped_registries.is_empty());
        assert!(config.auth_tokens.is_empty());
        assert!(config.strict_ssl);
    }

    #[test]
    fn test_parse_registry() {
        let mut config = NpmrcConfig::new();
        config.merge_from_string("registry=https://custom.registry.com").unwrap();
        assert_eq!(config.registry, "https://custom.registry.com");
    }

    #[test]
    fn test_parse_scoped_registry() {
        let mut config = NpmrcConfig::new();
        config
            .merge_from_string("@mycompany:registry=https://npm.mycompany.com")
            .unwrap();
        assert_eq!(
            config.scoped_registries.get("@mycompany"),
            Some(&"https://npm.mycompany.com".to_string())
        );
    }

    #[test]
    fn test_parse_auth_token() {
        let mut config = NpmrcConfig::new();
        config
            .merge_from_string("//npm.mycompany.com/:_authToken=secret-token-123")
            .unwrap();

        let auth = config.get_auth_for_registry("https://npm.mycompany.com");
        assert!(auth.is_some());
        match auth.unwrap() {
            AuthToken::Bearer(token) => assert_eq!(token, "secret-token-123"),
            _ => panic!("Expected Bearer token"),
        }
    }

    #[test]
    fn test_parse_basic_auth() {
        let mut config = NpmrcConfig::new();
        config.merge_from_string("//npm.mycompany.com/:_auth=dXNlcjpwYXNz").unwrap();

        let auth = config.get_auth_for_registry("https://npm.mycompany.com");
        assert!(auth.is_some());
        match auth.unwrap() {
            AuthToken::Basic(encoded) => assert_eq!(encoded, "dXNlcjpwYXNz"),
            _ => panic!("Expected Basic auth"),
        }
    }

    #[test]
    fn test_get_registry_for_scoped_package() {
        let mut config = NpmrcConfig::new();
        config
            .merge_from_string(
                r#"
            registry=https://registry.npmjs.org
            @mycompany:registry=https://npm.mycompany.com
        "#,
            )
            .unwrap();

        assert_eq!(
            config.get_registry_for_package("@mycompany/my-package"),
            "https://npm.mycompany.com"
        );
        assert_eq!(config.get_registry_for_package("lodash"), "https://registry.npmjs.org");
    }

    #[test]
    fn test_get_auth_header_bearer() {
        let mut config = NpmrcConfig::new();
        config.merge_from_string("//npm.mycompany.com/:_authToken=my-token").unwrap();

        let header = config.get_auth_header("https://npm.mycompany.com");
        assert_eq!(header, Some("Bearer my-token".to_string()));
    }

    #[test]
    fn test_parse_comments_and_empty_lines() {
        let mut config = NpmrcConfig::new();
        config
            .merge_from_string(
                r#"
            # This is a comment
            ; This is also a comment
            
            registry=https://custom.registry.com
            
            # Another comment
        "#,
            )
            .unwrap();

        assert_eq!(config.registry, "https://custom.registry.com");
    }

    #[test]
    fn test_parse_proxy_settings() {
        let mut config = NpmrcConfig::new();
        config
            .merge_from_string(
                r#"
            proxy=http://proxy.example.com:8080
            https-proxy=http://proxy.example.com:8080
        "#,
            )
            .unwrap();

        assert_eq!(config.proxy, Some("http://proxy.example.com:8080".to_string()));
        assert_eq!(config.https_proxy, Some("http://proxy.example.com:8080".to_string()));
    }

    #[test]
    fn test_parse_strict_ssl() {
        let mut config = NpmrcConfig::new();
        config.merge_from_string("strict-ssl=false").unwrap();
        assert!(!config.strict_ssl);
    }

    #[test]
    fn test_normalize_registry_url() {
        assert_eq!(
            NpmrcConfig::normalize_registry_url("registry.npmjs.org"),
            "https://registry.npmjs.org"
        );
        assert_eq!(
            NpmrcConfig::normalize_registry_url("https://registry.npmjs.org/"),
            "https://registry.npmjs.org"
        );
        assert_eq!(
            NpmrcConfig::normalize_registry_url("http://localhost:4873/"),
            "http://localhost:4873"
        );
    }

    #[test]
    fn test_always_auth() {
        let mut config = NpmrcConfig::new();
        config
            .merge_from_string(
                r#"
            //npm.mycompany.com/:always-auth=true
            always-auth=false
        "#,
            )
            .unwrap();

        assert!(config.requires_auth("https://npm.mycompany.com"));
        assert!(!config.requires_auth("https://registry.npmjs.org"));
    }

    #[test]
    fn test_multiple_scoped_registries() {
        let mut config = NpmrcConfig::new();
        config
            .merge_from_string(
                r#"
            @company-a:registry=https://npm.company-a.com
            @company-b:registry=https://npm.company-b.com
            //npm.company-a.com/:_authToken=token-a
            //npm.company-b.com/:_authToken=token-b
        "#,
            )
            .unwrap();

        assert_eq!(config.get_registry_for_package("@company-a/pkg"), "https://npm.company-a.com");
        assert_eq!(config.get_registry_for_package("@company-b/pkg"), "https://npm.company-b.com");

        assert_eq!(
            config.get_auth_header("https://npm.company-a.com"),
            Some("Bearer token-a".to_string())
        );
        assert_eq!(
            config.get_auth_header("https://npm.company-b.com"),
            Some("Bearer token-b".to_string())
        );
    }
}
