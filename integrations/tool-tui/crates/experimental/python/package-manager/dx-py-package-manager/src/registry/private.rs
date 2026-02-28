//! Private Registry Support
//!
//! Implements authentication and configuration for private Python package registries.
//! Supports:
//! - Extra index URLs (--extra-index-url)
//! - Authentication via keyring, netrc, and environment variables
//! - SSL certificate configuration
//! - Registry priority ordering
//!
//! Requirements: 2.5.1-2.5.5

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::Result;

/// Authentication credentials for a registry
#[derive(Debug, Clone)]
pub struct RegistryCredentials {
    /// Username
    pub username: Option<String>,
    /// Password or token
    pub password: Option<String>,
    /// Bearer token (alternative to username/password)
    pub token: Option<String>,
}

impl RegistryCredentials {
    /// Create empty credentials
    pub fn empty() -> Self {
        Self {
            username: None,
            password: None,
            token: None,
        }
    }

    /// Create credentials with username and password
    pub fn basic(username: &str, password: &str) -> Self {
        Self {
            username: Some(username.to_string()),
            password: Some(password.to_string()),
            token: None,
        }
    }

    /// Create credentials with a bearer token
    pub fn bearer(token: &str) -> Self {
        Self {
            username: None,
            password: None,
            token: Some(token.to_string()),
        }
    }

    /// Check if credentials are available
    pub fn is_available(&self) -> bool {
        self.token.is_some() || (self.username.is_some() && self.password.is_some())
    }

    /// Get the authorization header value
    pub fn authorization_header(&self) -> Option<String> {
        if let Some(ref token) = self.token {
            Some(format!("Bearer {}", token))
        } else if let (Some(ref user), Some(ref pass)) = (&self.username, &self.password) {
            use base64::Engine;
            let encoded =
                base64::engine::general_purpose::STANDARD.encode(format!("{}:{}", user, pass));
            Some(format!("Basic {}", encoded))
        } else {
            None
        }
    }
}

/// SSL/TLS configuration for a registry
#[derive(Debug, Clone, Default)]
pub struct SslConfig {
    /// Path to CA certificate bundle
    pub ca_bundle: Option<PathBuf>,
    /// Path to client certificate
    pub client_cert: Option<PathBuf>,
    /// Path to client key
    pub client_key: Option<PathBuf>,
    /// Whether to verify SSL certificates
    pub verify: bool,
}

impl SslConfig {
    /// Create default SSL config (verify enabled)
    pub fn default_verify() -> Self {
        Self {
            ca_bundle: None,
            client_cert: None,
            client_key: None,
            verify: true,
        }
    }

    /// Create SSL config that skips verification (insecure)
    pub fn insecure() -> Self {
        Self {
            ca_bundle: None,
            client_cert: None,
            client_key: None,
            verify: false,
        }
    }
}

/// Configuration for a single registry
#[derive(Debug, Clone)]
pub struct RegistryConfig {
    /// Registry URL
    pub url: String,
    /// Registry name (for display)
    pub name: String,
    /// Priority (lower = higher priority)
    pub priority: u32,
    /// Authentication credentials
    pub credentials: RegistryCredentials,
    /// SSL configuration
    pub ssl: SslConfig,
    /// Whether this registry is enabled
    pub enabled: bool,
}

impl RegistryConfig {
    /// Create a new registry config
    pub fn new(url: &str, name: &str) -> Self {
        Self {
            url: url.trim_end_matches('/').to_string(),
            name: name.to_string(),
            priority: 100,
            credentials: RegistryCredentials::empty(),
            ssl: SslConfig::default_verify(),
            enabled: true,
        }
    }

    /// Set priority
    pub fn with_priority(mut self, priority: u32) -> Self {
        self.priority = priority;
        self
    }

    /// Set credentials
    pub fn with_credentials(mut self, credentials: RegistryCredentials) -> Self {
        self.credentials = credentials;
        self
    }

    /// Set SSL config
    pub fn with_ssl(mut self, ssl: SslConfig) -> Self {
        self.ssl = ssl;
        self
    }

    /// Get the simple API URL for this registry
    pub fn simple_url(&self) -> String {
        if self.url.ends_with("/simple") {
            self.url.clone()
        } else {
            format!("{}/simple", self.url)
        }
    }
}

/// Manager for multiple registries with authentication
pub struct RegistryManager {
    /// Configured registries (sorted by priority)
    registries: Vec<RegistryConfig>,
    /// Credential providers
    credential_providers: Vec<Box<dyn CredentialProvider>>,
}

impl Default for RegistryManager {
    fn default() -> Self {
        Self::new()
    }
}

impl RegistryManager {
    /// Create a new registry manager with default PyPI
    pub fn new() -> Self {
        let mut manager = Self {
            registries: Vec::new(),
            credential_providers: Vec::new(),
        };

        // Add default PyPI
        manager.add_registry(RegistryConfig::new("https://pypi.org", "pypi").with_priority(1000));

        // Add default credential providers
        manager.add_credential_provider(Box::new(EnvironmentCredentialProvider));
        manager.add_credential_provider(Box::new(NetrcCredentialProvider::new()));

        manager
    }

    /// Add a registry
    pub fn add_registry(&mut self, config: RegistryConfig) {
        self.registries.push(config);
        self.registries.sort_by_key(|r| r.priority);
    }

    /// Add an extra index URL
    pub fn add_extra_index(&mut self, url: &str) {
        let name = url_to_name(url);
        let priority = self.registries.len() as u32 + 1;
        self.add_registry(RegistryConfig::new(url, &name).with_priority(priority));
    }

    /// Add a credential provider
    pub fn add_credential_provider(&mut self, provider: Box<dyn CredentialProvider>) {
        self.credential_providers.push(provider);
    }

    /// Get all enabled registries
    pub fn registries(&self) -> impl Iterator<Item = &RegistryConfig> {
        self.registries.iter().filter(|r| r.enabled)
    }

    /// Get credentials for a URL
    pub fn get_credentials(&self, url: &str) -> RegistryCredentials {
        // First check if the registry has explicit credentials
        for registry in &self.registries {
            if url.starts_with(&registry.url) && registry.credentials.is_available() {
                return registry.credentials.clone();
            }
        }

        // Try credential providers
        for provider in &self.credential_providers {
            if let Some(creds) = provider.get_credentials(url) {
                return creds;
            }
        }

        RegistryCredentials::empty()
    }

    /// Load configuration from pip.conf
    pub fn load_pip_conf(&mut self, path: &Path) -> Result<()> {
        if !path.exists() {
            return Ok(());
        }

        let content = std::fs::read_to_string(path)?;
        self.parse_pip_conf(&content)
    }

    /// Parse pip.conf content
    fn parse_pip_conf(&mut self, content: &str) -> Result<()> {
        let mut current_section = String::new();
        let mut global_index_url: Option<String> = None;
        let mut extra_index_urls: Vec<String> = Vec::new();
        let mut trusted_hosts: Vec<String> = Vec::new();

        for line in content.lines() {
            let line = line.trim();

            // Skip comments and empty lines
            if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
                continue;
            }

            // Section header
            if line.starts_with('[') && line.ends_with(']') {
                current_section = line[1..line.len() - 1].to_lowercase();
                continue;
            }

            // Key-value pair
            if let Some(eq_idx) = line.find('=') {
                let key = line[..eq_idx].trim().to_lowercase();
                let value = line[eq_idx + 1..].trim();

                match (current_section.as_str(), key.as_str()) {
                    ("global", "index-url") | ("install", "index-url") => {
                        global_index_url = Some(value.to_string());
                    }
                    ("global", "extra-index-url") | ("install", "extra-index-url") => {
                        // Can be multiple URLs separated by newlines or spaces
                        for url in value.split_whitespace() {
                            extra_index_urls.push(url.to_string());
                        }
                    }
                    ("global", "trusted-host") | ("install", "trusted-host") => {
                        for host in value.split_whitespace() {
                            trusted_hosts.push(host.to_string());
                        }
                    }
                    _ => {}
                }
            } else if !line.is_empty() && !line.starts_with('[') {
                // Continuation line (indented URL for extra-index-url)
                if line.starts_with("http://") || line.starts_with("https://") {
                    extra_index_urls.push(line.to_string());
                }
            }
        }

        // Apply configuration
        if let Some(url) = global_index_url {
            // Replace default PyPI with custom index
            self.registries.retain(|r| r.name != "pypi");
            self.add_registry(RegistryConfig::new(&url, "primary").with_priority(0));
        }

        for url in extra_index_urls {
            self.add_extra_index(&url);
        }

        // Mark trusted hosts as not requiring SSL verification
        for host in trusted_hosts {
            for registry in &mut self.registries {
                if registry.url.contains(&host) {
                    registry.ssl.verify = false;
                }
            }
        }

        Ok(())
    }

    /// Load configuration from environment variables
    pub fn load_from_env(&mut self) {
        // PIP_INDEX_URL
        if let Ok(url) = std::env::var("PIP_INDEX_URL") {
            self.registries.retain(|r| r.name != "pypi");
            self.add_registry(RegistryConfig::new(&url, "primary").with_priority(0));
        }

        // PIP_EXTRA_INDEX_URL (space-separated)
        if let Ok(urls) = std::env::var("PIP_EXTRA_INDEX_URL") {
            for url in urls.split_whitespace() {
                self.add_extra_index(url);
            }
        }

        // PIP_TRUSTED_HOST (space-separated)
        if let Ok(hosts) = std::env::var("PIP_TRUSTED_HOST") {
            for host in hosts.split_whitespace() {
                for registry in &mut self.registries {
                    if registry.url.contains(host) {
                        registry.ssl.verify = false;
                    }
                }
            }
        }
    }
}

/// Trait for credential providers
pub trait CredentialProvider: Send + Sync {
    /// Get credentials for a URL
    fn get_credentials(&self, url: &str) -> Option<RegistryCredentials>;
}

/// Credential provider that reads from environment variables
pub struct EnvironmentCredentialProvider;

impl CredentialProvider for EnvironmentCredentialProvider {
    fn get_credentials(&self, url: &str) -> Option<RegistryCredentials> {
        // Check for URL-specific credentials
        // Format: DX_PY_<HOST>_USERNAME, DX_PY_<HOST>_PASSWORD
        let host = extract_host(url)?;
        let host_upper = host.replace(['.', '-'], "_").to_uppercase();

        let username = std::env::var(format!("DX_PY_{}_USERNAME", host_upper)).ok();
        let password = std::env::var(format!("DX_PY_{}_PASSWORD", host_upper)).ok();
        let token = std::env::var(format!("DX_PY_{}_TOKEN", host_upper)).ok();

        if token.is_some() {
            return Some(RegistryCredentials {
                username: None,
                password: None,
                token,
            });
        }

        if username.is_some() && password.is_some() {
            return Some(RegistryCredentials {
                username,
                password,
                token: None,
            });
        }

        // Check for generic PyPI token
        if url.contains("pypi.org") {
            if let Ok(token) = std::env::var("PYPI_TOKEN") {
                return Some(RegistryCredentials::bearer(&token));
            }
        }

        None
    }
}

/// Credential provider that reads from .netrc file
pub struct NetrcCredentialProvider {
    entries: HashMap<String, (String, String)>,
}

impl NetrcCredentialProvider {
    /// Create a new netrc credential provider
    pub fn new() -> Self {
        let mut provider = Self {
            entries: HashMap::new(),
        };
        provider.load_netrc();
        provider
    }

    /// Load credentials from .netrc file
    fn load_netrc(&mut self) {
        let netrc_path = if cfg!(windows) {
            dirs::home_dir().map(|h| h.join("_netrc"))
        } else {
            dirs::home_dir().map(|h| h.join(".netrc"))
        };

        if let Some(path) = netrc_path {
            if let Ok(content) = std::fs::read_to_string(&path) {
                self.parse_netrc(&content);
            }
        }
    }

    /// Parse netrc content
    fn parse_netrc(&mut self, content: &str) {
        let mut current_machine: Option<String> = None;
        let mut current_login: Option<String> = None;
        let mut current_password: Option<String> = None;

        for token in content.split_whitespace() {
            match token {
                "machine" => {
                    // Save previous entry
                    if let (Some(machine), Some(login), Some(password)) =
                        (&current_machine, &current_login, &current_password)
                    {
                        self.entries.insert(machine.clone(), (login.clone(), password.clone()));
                    }
                    current_machine = None;
                    current_login = None;
                    current_password = None;
                }
                "login" | "user" => {}
                "password" | "passwd" => {}
                "default" => {
                    current_machine = Some("default".to_string());
                }
                _ => {
                    if current_machine.is_none() {
                        current_machine = Some(token.to_string());
                    } else if current_login.is_none() {
                        current_login = Some(token.to_string());
                    } else if current_password.is_none() {
                        current_password = Some(token.to_string());
                    }
                }
            }
        }

        // Save last entry
        if let (Some(machine), Some(login), Some(password)) =
            (current_machine, current_login, current_password)
        {
            self.entries.insert(machine, (login, password));
        }
    }
}

impl Default for NetrcCredentialProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl CredentialProvider for NetrcCredentialProvider {
    fn get_credentials(&self, url: &str) -> Option<RegistryCredentials> {
        let host = extract_host(url)?;

        // Try exact match first
        if let Some((login, password)) = self.entries.get(&host) {
            return Some(RegistryCredentials::basic(login, password));
        }

        // Try default
        if let Some((login, password)) = self.entries.get("default") {
            return Some(RegistryCredentials::basic(login, password));
        }

        None
    }
}

/// Extract host from URL
fn extract_host(url: &str) -> Option<String> {
    let url = url.strip_prefix("https://").or_else(|| url.strip_prefix("http://"))?;
    let host_part = url.split('/').next()?;

    // Handle user:pass@host format
    let host = if let Some(at_idx) = host_part.find('@') {
        &host_part[at_idx + 1..]
    } else {
        host_part
    };

    // Remove port (but be careful with IPv6)
    let host = if host.starts_with('[') {
        // IPv6 address
        host.split(']').next().map(|s| &s[1..]).unwrap_or(host)
    } else {
        host.split(':').next().unwrap_or(host)
    };

    Some(host.to_string())
}

/// Convert URL to a registry name
fn url_to_name(url: &str) -> String {
    extract_host(url).unwrap_or_else(|| "unknown".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_config() {
        let config = RegistryConfig::new("https://pypi.org", "pypi")
            .with_priority(10)
            .with_credentials(RegistryCredentials::basic("user", "pass"));

        assert_eq!(config.url, "https://pypi.org");
        assert_eq!(config.name, "pypi");
        assert_eq!(config.priority, 10);
        assert!(config.credentials.is_available());
    }

    #[test]
    fn test_credentials_basic() {
        let creds = RegistryCredentials::basic("user", "pass");
        assert!(creds.is_available());

        let header = creds.authorization_header().unwrap();
        assert!(header.starts_with("Basic "));
    }

    #[test]
    fn test_credentials_bearer() {
        let creds = RegistryCredentials::bearer("token123");
        assert!(creds.is_available());

        let header = creds.authorization_header().unwrap();
        assert_eq!(header, "Bearer token123");
    }

    #[test]
    fn test_registry_manager_default() {
        let manager = RegistryManager::new();
        let registries: Vec<_> = manager.registries().collect();

        assert!(!registries.is_empty());
        assert!(registries.iter().any(|r| r.url.contains("pypi.org")));
    }

    #[test]
    fn test_registry_manager_extra_index() {
        let mut manager = RegistryManager::new();
        manager.add_extra_index("https://private.example.com/simple");

        let registries: Vec<_> = manager.registries().collect();
        assert!(registries.iter().any(|r| r.url.contains("private.example.com")));
    }

    #[test]
    fn test_pip_conf_parsing() {
        let content = r#"
[global]
index-url = https://custom.pypi.org/simple
extra-index-url = https://extra1.example.com/simple
    https://extra2.example.com/simple
trusted-host = extra1.example.com
"#;

        let mut manager = RegistryManager::new();
        manager.parse_pip_conf(content).unwrap();

        let registries: Vec<_> = manager.registries().collect();

        // Should have custom index instead of pypi
        assert!(registries.iter().any(|r| r.url.contains("custom.pypi.org")));
        assert!(!registries.iter().any(|r| r.name == "pypi"));

        // Should have extra indexes
        assert!(registries.iter().any(|r| r.url.contains("extra1.example.com")));
        assert!(registries.iter().any(|r| r.url.contains("extra2.example.com")));

        // Trusted host should have SSL verification disabled
        let extra1 = registries.iter().find(|r| r.url.contains("extra1.example.com")).unwrap();
        assert!(!extra1.ssl.verify);
    }

    #[test]
    fn test_netrc_parsing() {
        let mut provider = NetrcCredentialProvider {
            entries: HashMap::new(),
        };

        provider.parse_netrc(
            r#"
machine pypi.org
login myuser
password mypass

machine private.example.com
login otheruser
password otherpass
"#,
        );

        let creds = provider.get_credentials("https://pypi.org/simple").unwrap();
        assert_eq!(creds.username, Some("myuser".to_string()));
        assert_eq!(creds.password, Some("mypass".to_string()));

        let creds = provider.get_credentials("https://private.example.com/simple").unwrap();
        assert_eq!(creds.username, Some("otheruser".to_string()));
    }

    #[test]
    fn test_extract_host() {
        assert_eq!(extract_host("https://pypi.org/simple"), Some("pypi.org".to_string()));
        assert_eq!(extract_host("http://localhost:8080/simple"), Some("localhost".to_string()));
        assert_eq!(extract_host("https://user:pass@example.com/"), Some("example.com".to_string()));
    }

    #[test]
    fn test_simple_url() {
        let config = RegistryConfig::new("https://pypi.org", "pypi");
        assert_eq!(config.simple_url(), "https://pypi.org/simple");

        let config = RegistryConfig::new("https://pypi.org/simple", "pypi");
        assert_eq!(config.simple_url(), "https://pypi.org/simple");
    }
}
