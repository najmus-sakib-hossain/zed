//! # dx-compat-macro
//!
//! Compile-time macros compatibility layer.
//!
//! This crate provides functionality for executing code at compile time and
//! inlining the results as literals, similar to Bun's macro system.
//!
//! ## Features
//!
//! - Execute functions at compile time
//! - File system access in macros
//! - Environment variable access in macros
//! - Value serialization and inlining
//! - Isolated execution context
//!
//! ## Example
//!
//! ```rust,ignore
//! use dx_compat_macro::{MacroContext, MacroValue};
//!
//! let ctx = MacroContext::new();
//!
//! // Execute a macro that reads a file
//! let result = ctx.execute(|| {
//!     let content = std::fs::read_to_string("config.json")?;
//!     Ok(MacroValue::String(content))
//! })?;
//! ```

#![warn(missing_docs)]

mod error;

pub use error::{MacroError, MacroResult};

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Value types that can be returned from macros.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MacroValue {
    /// Null value
    Null,
    /// Boolean value
    Bool(bool),
    /// Integer value
    Integer(i64),
    /// Float value
    Float(f64),
    /// String value
    String(String),
    /// Array of values
    Array(Vec<MacroValue>),
    /// Object/map of values
    Object(HashMap<String, MacroValue>),
}

impl MacroValue {
    /// Create a null value.
    pub fn null() -> Self {
        MacroValue::Null
    }

    /// Check if value is null.
    pub fn is_null(&self) -> bool {
        matches!(self, MacroValue::Null)
    }

    /// Get as boolean.
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            MacroValue::Bool(b) => Some(*b),
            _ => None,
        }
    }

    /// Get as integer.
    pub fn as_i64(&self) -> Option<i64> {
        match self {
            MacroValue::Integer(i) => Some(*i),
            _ => None,
        }
    }

    /// Get as float.
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            MacroValue::Float(f) => Some(*f),
            MacroValue::Integer(i) => Some(*i as f64),
            _ => None,
        }
    }

    /// Get as string.
    pub fn as_str(&self) -> Option<&str> {
        match self {
            MacroValue::String(s) => Some(s),
            _ => None,
        }
    }

    /// Get as array.
    pub fn as_array(&self) -> Option<&Vec<MacroValue>> {
        match self {
            MacroValue::Array(arr) => Some(arr),
            _ => None,
        }
    }

    /// Get as object.
    pub fn as_object(&self) -> Option<&HashMap<String, MacroValue>> {
        match self {
            MacroValue::Object(obj) => Some(obj),
            _ => None,
        }
    }

    /// Convert to JSON string.
    pub fn to_json(&self) -> MacroResult<String> {
        serde_json::to_string(self).map_err(|e| MacroError::Serialization(e.to_string()))
    }

    /// Convert to pretty JSON string.
    pub fn to_json_pretty(&self) -> MacroResult<String> {
        serde_json::to_string_pretty(self).map_err(|e| MacroError::Serialization(e.to_string()))
    }

    /// Parse from JSON string.
    pub fn from_json(json: &str) -> MacroResult<Self> {
        serde_json::from_str(json).map_err(|e| MacroError::Serialization(e.to_string()))
    }

    /// Generate JavaScript literal representation.
    pub fn to_js_literal(&self) -> String {
        match self {
            MacroValue::Null => "null".to_string(),
            MacroValue::Bool(b) => b.to_string(),
            MacroValue::Integer(i) => i.to_string(),
            MacroValue::Float(f) => {
                if f.is_nan() {
                    "NaN".to_string()
                } else if f.is_infinite() {
                    if *f > 0.0 {
                        "Infinity".to_string()
                    } else {
                        "-Infinity".to_string()
                    }
                } else {
                    f.to_string()
                }
            }
            MacroValue::String(s) => format!("{:?}", s),
            MacroValue::Array(arr) => {
                let items: Vec<String> = arr.iter().map(|v| v.to_js_literal()).collect();
                format!("[{}]", items.join(", "))
            }
            MacroValue::Object(obj) => {
                let items: Vec<String> =
                    obj.iter().map(|(k, v)| format!("{:?}: {}", k, v.to_js_literal())).collect();
                format!("{{{}}}", items.join(", "))
            }
        }
    }
}

impl From<bool> for MacroValue {
    fn from(b: bool) -> Self {
        MacroValue::Bool(b)
    }
}

impl From<i64> for MacroValue {
    fn from(i: i64) -> Self {
        MacroValue::Integer(i)
    }
}

impl From<i32> for MacroValue {
    fn from(i: i32) -> Self {
        MacroValue::Integer(i as i64)
    }
}

impl From<f64> for MacroValue {
    fn from(f: f64) -> Self {
        MacroValue::Float(f)
    }
}

impl From<String> for MacroValue {
    fn from(s: String) -> Self {
        MacroValue::String(s)
    }
}

impl From<&str> for MacroValue {
    fn from(s: &str) -> Self {
        MacroValue::String(s.to_string())
    }
}

impl<T: Into<MacroValue>> From<Vec<T>> for MacroValue {
    fn from(v: Vec<T>) -> Self {
        MacroValue::Array(v.into_iter().map(Into::into).collect())
    }
}

impl<T: Into<MacroValue>> From<Option<T>> for MacroValue {
    fn from(opt: Option<T>) -> Self {
        match opt {
            Some(v) => v.into(),
            None => MacroValue::Null,
        }
    }
}

/// Configuration for macro execution.
#[derive(Debug, Clone)]
pub struct MacroConfig {
    /// Working directory for file operations.
    pub cwd: PathBuf,
    /// Environment variables available to macros.
    pub env: HashMap<String, String>,
    /// Timeout in milliseconds (0 = no timeout).
    pub timeout_ms: u64,
    /// Allow file system access.
    pub allow_fs: bool,
    /// Allow environment variable access.
    pub allow_env: bool,
    /// Allowed file paths (empty = all allowed if allow_fs is true).
    pub allowed_paths: Vec<PathBuf>,
}

impl Default for MacroConfig {
    fn default() -> Self {
        Self {
            cwd: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            env: std::env::vars().collect(),
            timeout_ms: 30000, // 30 seconds default
            allow_fs: true,
            allow_env: true,
            allowed_paths: Vec::new(),
        }
    }
}

impl MacroConfig {
    /// Create a new config with default settings.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set working directory.
    pub fn cwd(mut self, path: impl AsRef<Path>) -> Self {
        self.cwd = path.as_ref().to_path_buf();
        self
    }

    /// Set timeout in milliseconds.
    pub fn timeout(mut self, ms: u64) -> Self {
        self.timeout_ms = ms;
        self
    }

    /// Enable/disable file system access.
    pub fn allow_fs(mut self, allow: bool) -> Self {
        self.allow_fs = allow;
        self
    }

    /// Enable/disable environment variable access.
    pub fn allow_env(mut self, allow: bool) -> Self {
        self.allow_env = allow;
        self
    }

    /// Set allowed file paths.
    pub fn allowed_paths(mut self, paths: Vec<PathBuf>) -> Self {
        self.allowed_paths = paths;
        self
    }

    /// Add an environment variable.
    pub fn env_var(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.env.insert(key.into(), value.into());
        self
    }
}

/// Macro execution context providing isolated runtime for compile-time execution.
pub struct MacroContext {
    config: MacroConfig,
    cache: Arc<RwLock<HashMap<String, MacroValue>>>,
}

impl Default for MacroContext {
    fn default() -> Self {
        Self::new()
    }
}

impl MacroContext {
    /// Create a new macro context with default configuration.
    pub fn new() -> Self {
        Self::with_config(MacroConfig::default())
    }

    /// Create a new macro context with custom configuration.
    pub fn with_config(config: MacroConfig) -> Self {
        Self {
            config,
            cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Get the working directory.
    pub fn cwd(&self) -> &Path {
        &self.config.cwd
    }

    /// Execute a macro function and return the result.
    pub fn execute<F>(&self, func: F) -> MacroResult<MacroValue>
    where
        F: FnOnce(&MacroContext) -> MacroResult<MacroValue>,
    {
        func(self)
    }

    /// Execute a macro with caching.
    pub fn execute_cached<F>(&self, key: &str, func: F) -> MacroResult<MacroValue>
    where
        F: FnOnce(&MacroContext) -> MacroResult<MacroValue>,
    {
        // Check cache first
        {
            let cache = self.cache.read();
            if let Some(value) = cache.get(key) {
                return Ok(value.clone());
            }
        }

        // Execute and cache
        let result = func(self)?;
        {
            let mut cache = self.cache.write();
            cache.insert(key.to_string(), result.clone());
        }

        Ok(result)
    }

    /// Read a file as string.
    pub fn read_file(&self, path: impl AsRef<Path>) -> MacroResult<String> {
        if !self.config.allow_fs {
            return Err(MacroError::FileAccess("File system access is disabled".to_string()));
        }

        let path = self.resolve_path(path.as_ref())?;
        std::fs::read_to_string(&path).map_err(|e| MacroError::FileAccess(e.to_string()))
    }

    /// Read a file as bytes.
    pub fn read_file_bytes(&self, path: impl AsRef<Path>) -> MacroResult<Vec<u8>> {
        if !self.config.allow_fs {
            return Err(MacroError::FileAccess("File system access is disabled".to_string()));
        }

        let path = self.resolve_path(path.as_ref())?;
        std::fs::read(&path).map_err(|e| MacroError::FileAccess(e.to_string()))
    }

    /// Read a JSON file and parse it.
    pub fn read_json(&self, path: impl AsRef<Path>) -> MacroResult<MacroValue> {
        let content = self.read_file(path)?;
        MacroValue::from_json(&content)
    }

    /// Check if a file exists.
    pub fn file_exists(&self, path: impl AsRef<Path>) -> MacroResult<bool> {
        if !self.config.allow_fs {
            return Err(MacroError::FileAccess("File system access is disabled".to_string()));
        }

        let path = self.resolve_path(path.as_ref())?;
        Ok(path.exists())
    }

    /// Get an environment variable.
    pub fn env(&self, key: &str) -> MacroResult<Option<String>> {
        if !self.config.allow_env {
            return Err(MacroError::EnvVar("Environment variable access is disabled".to_string()));
        }

        Ok(self.config.env.get(key).cloned())
    }

    /// Get an environment variable or return a default.
    pub fn env_or(&self, key: &str, default: &str) -> MacroResult<String> {
        Ok(self.env(key)?.unwrap_or_else(|| default.to_string()))
    }

    /// Get all environment variables.
    pub fn env_all(&self) -> MacroResult<HashMap<String, String>> {
        if !self.config.allow_env {
            return Err(MacroError::EnvVar("Environment variable access is disabled".to_string()));
        }

        Ok(self.config.env.clone())
    }

    /// Resolve a path relative to the working directory.
    fn resolve_path(&self, path: &Path) -> MacroResult<PathBuf> {
        let resolved = if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.config.cwd.join(path)
        };

        // Check allowed paths if configured
        if !self.config.allowed_paths.is_empty() {
            let canonical =
                resolved.canonicalize().map_err(|e| MacroError::FileAccess(e.to_string()))?;

            let allowed = self.config.allowed_paths.iter().any(|allowed| {
                if let Ok(allowed_canonical) = allowed.canonicalize() {
                    canonical.starts_with(&allowed_canonical)
                } else {
                    false
                }
            });

            if !allowed {
                return Err(MacroError::FileAccess(format!(
                    "Path not in allowed paths: {}",
                    path.display()
                )));
            }
        }

        Ok(resolved)
    }

    /// Clear the cache.
    pub fn clear_cache(&self) {
        let mut cache = self.cache.write();
        cache.clear();
    }
}

/// Macro definition for registration.
#[derive(Debug, Clone)]
pub struct MacroDefinition {
    /// Macro name.
    pub name: String,
    /// Module path where the macro is defined.
    pub module: String,
    /// Function name to call.
    pub function: String,
    /// Description of the macro.
    pub description: Option<String>,
}

impl MacroDefinition {
    /// Create a new macro definition.
    pub fn new(
        name: impl Into<String>,
        module: impl Into<String>,
        function: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            module: module.into(),
            function: function.into(),
            description: None,
        }
    }

    /// Set description.
    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }
}

/// Registry for macro definitions.
pub struct MacroRegistry {
    macros: HashMap<String, MacroDefinition>,
}

impl Default for MacroRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl MacroRegistry {
    /// Create a new empty registry.
    pub fn new() -> Self {
        Self {
            macros: HashMap::new(),
        }
    }

    /// Register a macro.
    pub fn register(&mut self, definition: MacroDefinition) {
        self.macros.insert(definition.name.clone(), definition);
    }

    /// Get a macro by name.
    pub fn get(&self, name: &str) -> Option<&MacroDefinition> {
        self.macros.get(name)
    }

    /// Check if a macro exists.
    pub fn has(&self, name: &str) -> bool {
        self.macros.contains_key(name)
    }

    /// List all registered macros.
    pub fn list(&self) -> Vec<&MacroDefinition> {
        self.macros.values().collect()
    }

    /// Remove a macro.
    pub fn remove(&mut self, name: &str) -> Option<MacroDefinition> {
        self.macros.remove(name)
    }
}

/// Result of macro expansion.
#[derive(Debug, Clone)]
pub struct MacroExpansion {
    /// The expanded value.
    pub value: MacroValue,
    /// JavaScript literal representation.
    pub js_literal: String,
    /// Original macro call location (if available).
    pub location: Option<MacroLocation>,
}

/// Location of a macro call in source code.
#[derive(Debug, Clone)]
pub struct MacroLocation {
    /// File path.
    pub file: String,
    /// Line number (1-indexed).
    pub line: u32,
    /// Column number (1-indexed).
    pub column: u32,
}

impl MacroExpansion {
    /// Create a new macro expansion.
    pub fn new(value: MacroValue) -> Self {
        let js_literal = value.to_js_literal();
        Self {
            value,
            js_literal,
            location: None,
        }
    }

    /// Set the location.
    pub fn with_location(mut self, file: impl Into<String>, line: u32, column: u32) -> Self {
        self.location = Some(MacroLocation {
            file: file.into(),
            line,
            column,
        });
        self
    }
}

// ============================================================================
// Built-in macro helpers
// ============================================================================

/// Built-in macro helpers for common operations.
pub mod builtins {
    use super::*;

    /// Read and inline a file's contents as a string.
    pub fn include_str(ctx: &MacroContext, path: &str) -> MacroResult<MacroValue> {
        let content = ctx.read_file(path)?;
        Ok(MacroValue::String(content))
    }

    /// Read and inline a file's contents as bytes (base64 encoded).
    pub fn include_bytes(ctx: &MacroContext, path: &str) -> MacroResult<MacroValue> {
        let bytes = ctx.read_file_bytes(path)?;
        let encoded = crate::base64_impl::encode(&bytes);
        Ok(MacroValue::String(encoded))
    }

    /// Read and parse a JSON file.
    pub fn include_json(ctx: &MacroContext, path: &str) -> MacroResult<MacroValue> {
        ctx.read_json(path)
    }

    /// Get an environment variable.
    pub fn env(ctx: &MacroContext, key: &str) -> MacroResult<MacroValue> {
        match ctx.env(key)? {
            Some(value) => Ok(MacroValue::String(value)),
            None => Ok(MacroValue::Null),
        }
    }

    /// Get an environment variable with default.
    pub fn env_or(ctx: &MacroContext, key: &str, default: &str) -> MacroResult<MacroValue> {
        let value = ctx.env_or(key, default)?;
        Ok(MacroValue::String(value))
    }

    /// Get current timestamp.
    pub fn timestamp(_ctx: &MacroContext) -> MacroResult<MacroValue> {
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        Ok(MacroValue::Integer(ts as i64))
    }

    /// Get build date as ISO string.
    pub fn build_date(_ctx: &MacroContext) -> MacroResult<MacroValue> {
        let now = std::time::SystemTime::now();
        let datetime = chrono_lite::format_iso8601(now);
        Ok(MacroValue::String(datetime))
    }
}

// Simple chrono-lite implementation for build date
mod chrono_lite {
    use std::time::SystemTime;

    pub fn format_iso8601(time: SystemTime) -> String {
        let duration = time.duration_since(std::time::UNIX_EPOCH).unwrap_or_default();
        let secs = duration.as_secs();

        // Simple calculation (not accounting for leap seconds)
        let days = secs / 86400;
        let remaining = secs % 86400;
        let hours = remaining / 3600;
        let minutes = (remaining % 3600) / 60;
        let seconds = remaining % 60;

        // Calculate year, month, day from days since epoch
        let (year, month, day) = days_to_ymd(days as i64);

        format!(
            "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
            year, month, day, hours, minutes, seconds
        )
    }

    fn days_to_ymd(days: i64) -> (i32, u32, u32) {
        // Days since 1970-01-01
        let mut remaining_days = days;
        let mut year = 1970i32;

        loop {
            let days_in_year = if is_leap_year(year) { 366 } else { 365 };
            if remaining_days < days_in_year {
                break;
            }
            remaining_days -= days_in_year;
            year += 1;
        }

        let leap = is_leap_year(year);
        let days_in_months: [i64; 12] = if leap {
            [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
        } else {
            [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
        };

        let mut month = 1u32;
        for days_in_month in days_in_months.iter() {
            if remaining_days < *days_in_month {
                break;
            }
            remaining_days -= days_in_month;
            month += 1;
        }

        let day = remaining_days as u32 + 1;
        (year, month, day)
    }

    fn is_leap_year(year: i32) -> bool {
        (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
    }
}

// Simple base64 encoding implementation
mod base64_impl {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

    pub fn encode(data: &[u8]) -> String {
        let mut result = String::new();
        let mut i = 0;

        while i < data.len() {
            let b0 = data[i] as usize;
            let b1 = if i + 1 < data.len() {
                data[i + 1] as usize
            } else {
                0
            };
            let b2 = if i + 2 < data.len() {
                data[i + 2] as usize
            } else {
                0
            };

            result.push(ALPHABET[b0 >> 2] as char);
            result.push(ALPHABET[((b0 & 0x03) << 4) | (b1 >> 4)] as char);

            if i + 1 < data.len() {
                result.push(ALPHABET[((b1 & 0x0f) << 2) | (b2 >> 6)] as char);
            } else {
                result.push('=');
            }

            if i + 2 < data.len() {
                result.push(ALPHABET[b2 & 0x3f] as char);
            } else {
                result.push('=');
            }

            i += 3;
        }

        result
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_macro_value_primitives() {
        assert!(MacroValue::Null.is_null());
        assert_eq!(MacroValue::Bool(true).as_bool(), Some(true));
        assert_eq!(MacroValue::Integer(42).as_i64(), Some(42));
        assert_eq!(MacroValue::Float(3.14).as_f64(), Some(3.14));
        assert_eq!(MacroValue::String("hello".into()).as_str(), Some("hello"));
    }

    #[test]
    fn test_macro_value_from() {
        let v: MacroValue = true.into();
        assert_eq!(v.as_bool(), Some(true));

        let v: MacroValue = 42i64.into();
        assert_eq!(v.as_i64(), Some(42));

        let v: MacroValue = "hello".into();
        assert_eq!(v.as_str(), Some("hello"));

        let v: MacroValue = vec![1i32, 2, 3].into();
        assert!(v.as_array().is_some());
    }

    #[test]
    fn test_macro_value_json() {
        let value = MacroValue::Object({
            let mut map = HashMap::new();
            map.insert("name".to_string(), MacroValue::String("test".to_string()));
            map.insert("count".to_string(), MacroValue::Integer(42));
            map
        });

        let json = value.to_json().unwrap();
        let parsed = MacroValue::from_json(&json).unwrap();

        assert!(parsed.as_object().is_some());
    }

    #[test]
    fn test_macro_value_js_literal() {
        assert_eq!(MacroValue::Null.to_js_literal(), "null");
        assert_eq!(MacroValue::Bool(true).to_js_literal(), "true");
        assert_eq!(MacroValue::Integer(42).to_js_literal(), "42");
        assert_eq!(MacroValue::String("hello".into()).to_js_literal(), "\"hello\"");
    }

    #[test]
    fn test_macro_context_env() {
        let ctx = MacroContext::with_config(
            MacroConfig::new().env_var("TEST_VAR", "test_value").allow_env(true),
        );

        assert_eq!(ctx.env("TEST_VAR").unwrap(), Some("test_value".to_string()));
        assert_eq!(ctx.env("NONEXISTENT").unwrap(), None);
        assert_eq!(ctx.env_or("NONEXISTENT", "default").unwrap(), "default");
    }

    #[test]
    fn test_macro_context_env_disabled() {
        let ctx = MacroContext::with_config(MacroConfig::new().allow_env(false));

        assert!(ctx.env("TEST").is_err());
    }

    #[test]
    fn test_macro_context_execute() {
        let ctx = MacroContext::new();

        let result = ctx.execute(|_| Ok(MacroValue::Integer(42))).unwrap();
        assert_eq!(result.as_i64(), Some(42));
    }

    #[test]
    fn test_macro_context_cached() {
        let ctx = MacroContext::new();
        let counter = std::sync::atomic::AtomicU32::new(0);

        let result1 = ctx
            .execute_cached("test", |_| {
                counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                Ok(MacroValue::Integer(42))
            })
            .unwrap();

        let result2 = ctx
            .execute_cached("test", |_| {
                counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                Ok(MacroValue::Integer(100))
            })
            .unwrap();

        assert_eq!(result1.as_i64(), Some(42));
        assert_eq!(result2.as_i64(), Some(42)); // Cached value
        assert_eq!(counter.load(std::sync::atomic::Ordering::SeqCst), 1);
    }

    #[test]
    fn test_macro_registry() {
        let mut registry = MacroRegistry::new();

        registry.register(
            MacroDefinition::new("test", "test_module", "test_fn").description("A test macro"),
        );

        assert!(registry.has("test"));
        assert!(!registry.has("nonexistent"));

        let def = registry.get("test").unwrap();
        assert_eq!(def.name, "test");
        assert_eq!(def.module, "test_module");
        assert_eq!(def.function, "test_fn");
    }

    #[test]
    fn test_macro_expansion() {
        let value = MacroValue::String("hello".into());
        let expansion = MacroExpansion::new(value.clone()).with_location("test.ts", 10, 5);

        assert_eq!(expansion.js_literal, "\"hello\"");
        assert!(expansion.location.is_some());
        let loc = expansion.location.unwrap();
        assert_eq!(loc.file, "test.ts");
        assert_eq!(loc.line, 10);
        assert_eq!(loc.column, 5);
    }

    #[test]
    fn test_builtin_timestamp() {
        let ctx = MacroContext::new();
        let result = builtins::timestamp(&ctx).unwrap();
        assert!(result.as_i64().is_some());
        assert!(result.as_i64().unwrap() > 0);
    }

    #[test]
    fn test_builtin_build_date() {
        let ctx = MacroContext::new();
        let result = builtins::build_date(&ctx).unwrap();
        let date = result.as_str().unwrap();
        assert!(date.contains("T"));
        assert!(date.ends_with("Z"));
    }
}
