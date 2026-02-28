//! Command Registry Module
//!
//! This module provides a dynamic, thread-safe command registry that supports
//! runtime command registration, hot-reloading, and multiple handler types.
//!
//! # Architecture
//!
//! The registry uses a `DashMap` for lock-free concurrent access and supports:
//!
//! - **Built-in commands**: Native Rust function handlers
//! - **WASM plugins**: WebAssembly-based commands
//! - **Native plugins**: Dynamically loaded shared libraries
//! - **Script commands**: Shell/interpreter-based commands
//!
//! # Example
//!
//! ```rust,ignore
//! use dx_cli::registry::{CommandRegistry, CommandEntry, HandlerType, DynamicExecutor};
//!
//! let registry = CommandRegistry::new();
//!
//! // Register a built-in command
//! registry.register(CommandEntry {
//!     name: "greet".into(),
//!     description: "Say hello".into(),
//!     handler: HandlerType::BuiltIn(Box::new(|args| {
//!         println!("Hello, {}!", args.first().unwrap_or(&"World".to_string()));
//!         Ok(())
//!     })),
//!     ..Default::default()
//! });
//!
//! // Execute via dynamic executor
//! let executor = DynamicExecutor::new(registry);
//! executor.execute("greet", &["Alice".to_string()]).await?;
//! ```

mod bridge;
mod executor;
mod loader;
mod types;
mod watcher;

#[cfg(test)]
mod migration_tests;

pub use bridge::register_builtin_commands;
pub use executor::DynamicExecutor;
pub use types::*;

use dashmap::DashMap;
use std::cmp::Ordering;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Thread-safe command registry
#[derive(Clone)]
pub struct CommandRegistry {
    /// Registered commands (name -> entry)
    commands: Arc<DashMap<String, CommandEntry>>,
    /// Command aliases (alias -> canonical name)
    aliases: Arc<DashMap<String, String>>,
    /// Registry version for change detection
    version: Arc<RwLock<u64>>,
}

impl Default for CommandRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl CommandRegistry {
    /// Create a new empty command registry
    pub fn new() -> Self {
        Self {
            commands: Arc::new(DashMap::new()),
            aliases: Arc::new(DashMap::new()),
            version: Arc::new(RwLock::new(0)),
        }
    }

    /// Register a new command
    pub fn register(&self, entry: CommandEntry) {
        let name = entry.name.clone();

        if let Some(existing) = self.commands.get(&name) {
            if !should_replace_command(&existing, &entry) {
                return;
            }

            for alias in &existing.aliases {
                self.aliases.remove(alias);
            }
        }

        // Register aliases
        for alias in &entry.aliases {
            self.aliases.insert(alias.clone(), name.clone());
        }

        // Register the command
        self.commands.insert(name, entry);

        // Increment version
        if let Ok(mut v) = self.version.try_write() {
            *v += 1;
        }
    }

    /// Register a new command, overriding any existing entry
    pub fn register_override(&self, entry: CommandEntry) {
        let name = entry.name.clone();

        if let Some(existing) = self.commands.get(&name) {
            for alias in &existing.aliases {
                self.aliases.remove(alias);
            }
        }

        for alias in &entry.aliases {
            self.aliases.insert(alias.clone(), name.clone());
        }

        self.commands.insert(name, entry);

        if let Ok(mut v) = self.version.try_write() {
            *v += 1;
        }
    }

    /// Unregister a command by name
    pub fn unregister(&self, name: &str) -> Option<CommandEntry> {
        // Remove aliases first
        if let Some((_, entry)) = self.commands.remove(name) {
            for alias in &entry.aliases {
                self.aliases.remove(alias);
            }

            // Increment version
            if let Ok(mut v) = self.version.try_write() {
                *v += 1;
            }

            return Some(entry);
        }
        None
    }

    /// Look up a command by name or alias
    pub fn get(&self, name: &str) -> Option<CommandEntry> {
        // Try direct lookup first
        if let Some(entry) = self.commands.get(name) {
            return Some(entry.clone());
        }

        // Try alias lookup
        if let Some(canonical) = self.aliases.get(name) {
            if let Some(entry) = self.commands.get(canonical.value()) {
                return Some(entry.clone());
            }
        }

        None
    }

    /// Check if a command exists
    pub fn contains(&self, name: &str) -> bool {
        self.commands.contains_key(name) || self.aliases.contains_key(name)
    }

    /// Get all registered command names
    pub fn command_names(&self) -> Vec<String> {
        self.commands.iter().map(|r| r.key().clone()).collect()
    }

    /// Get all commands grouped by category
    pub fn commands_by_category(&self) -> std::collections::HashMap<String, Vec<CommandEntry>> {
        let mut result: std::collections::HashMap<String, Vec<CommandEntry>> =
            std::collections::HashMap::new();

        for entry in self.commands.iter() {
            let category = entry.category.clone().unwrap_or_else(|| "Other".to_string());
            result.entry(category).or_default().push(entry.clone());
        }

        // Sort commands within each category
        for commands in result.values_mut() {
            commands.sort_by(|a, b| a.name.cmp(&b.name));
        }

        result
    }

    /// Execute a command by name
    pub async fn execute(
        &self,
        name: &str,
        args: &[String],
    ) -> Result<CommandResult, CommandError> {
        let entry = self.get(name).ok_or_else(|| CommandError::NotFound {
            name: name.to_string(),
            suggestions: self.suggest_similar(name),
        })?;

        // Check if command is enabled
        if !entry.enabled {
            return Err(CommandError::Disabled {
                name: name.to_string(),
            });
        }

        // Validate required capabilities
        // (In a real implementation, this would check against granted capabilities)

        // Execute based on handler type
        match &entry.handler {
            HandlerType::BuiltIn(handler) => {
                handler(args.to_vec()).map_err(|e| CommandError::ExecutionFailed {
                    name: name.to_string(),
                    reason: e,
                })
            }
            HandlerType::Async(handler) => {
                handler(args.to_vec()).await.map_err(|e| CommandError::ExecutionFailed {
                    name: name.to_string(),
                    reason: e,
                })
            }
            HandlerType::Wasm {
                module_path,
                function,
            } => {
                // Placeholder for WASM execution
                Err(CommandError::ExecutionFailed {
                    name: name.to_string(),
                    reason: format!(
                        "WASM execution not yet implemented: {}::{}",
                        module_path, function
                    ),
                })
            }
            HandlerType::Native {
                library_path,
                symbol,
            } => {
                // Placeholder for native plugin execution
                Err(CommandError::ExecutionFailed {
                    name: name.to_string(),
                    reason: format!(
                        "Native plugin execution not yet implemented: {}::{}",
                        library_path, symbol
                    ),
                })
            }
            HandlerType::Script {
                interpreter,
                script,
            } => self.execute_script(interpreter, script, args).await,
        }
    }

    /// Execute a script command
    async fn execute_script(
        &self,
        interpreter: &str,
        script: &str,
        args: &[String],
    ) -> Result<CommandResult, CommandError> {
        use tokio::process::Command;

        let output = Command::new(interpreter)
            .arg("-c")
            .arg(script)
            .args(args)
            .output()
            .await
            .map_err(|e| CommandError::ExecutionFailed {
                name: script.to_string(),
                reason: e.to_string(),
            })?;

        if output.status.success() {
            Ok(CommandResult {
                exit_code: 0,
                stdout: String::from_utf8_lossy(&output.stdout).to_string(),
                stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            })
        } else {
            Err(CommandError::ExecutionFailed {
                name: script.to_string(),
                reason: String::from_utf8_lossy(&output.stderr).to_string(),
            })
        }
    }

    /// Suggest similar command names for typos
    fn suggest_similar(&self, name: &str) -> Vec<String> {
        self.command_names()
            .into_iter()
            .filter(|cmd| {
                // Simple Levenshtein-like similarity check
                let distance = Self::levenshtein_distance(name, cmd);
                distance <= 2 || cmd.contains(name) || name.contains(cmd.as_str())
            })
            .take(3)
            .collect()
    }

    /// Calculate Levenshtein distance between two strings
    fn levenshtein_distance(a: &str, b: &str) -> usize {
        let a_len = a.chars().count();
        let b_len = b.chars().count();

        if a_len == 0 {
            return b_len;
        }
        if b_len == 0 {
            return a_len;
        }

        let mut matrix = vec![vec![0usize; b_len + 1]; a_len + 1];

        for i in 0..=a_len {
            matrix[i][0] = i;
        }
        for j in 0..=b_len {
            matrix[0][j] = j;
        }

        for (i, ca) in a.chars().enumerate() {
            for (j, cb) in b.chars().enumerate() {
                let cost = if ca == cb { 0 } else { 1 };
                matrix[i + 1][j + 1] =
                    (matrix[i][j + 1] + 1).min(matrix[i + 1][j] + 1).min(matrix[i][j] + cost);
            }
        }

        matrix[a_len][b_len]
    }

    /// Get current registry version
    pub async fn version(&self) -> u64 {
        *self.version.read().await
    }

    /// Clear all registered commands
    pub fn clear(&self) {
        self.commands.clear();
        self.aliases.clear();
        if let Ok(mut v) = self.version.try_write() {
            *v += 1;
        }
    }

    /// Get the number of registered commands
    pub fn len(&self) -> usize {
        self.commands.len()
    }

    /// Check if the registry is empty
    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }
}

fn should_replace_command(existing: &CommandEntry, incoming: &CommandEntry) -> bool {
    match (&existing.version, &incoming.version) {
        (Some(old), Some(new)) => compare_versions(new, old) == Ordering::Greater,
        (None, Some(_)) => true,
        (Some(_), None) => false,
        (None, None) => true,
    }
}

fn compare_versions(a: &str, b: &str) -> Ordering {
    let a_parts = parse_version_parts(a);
    let b_parts = parse_version_parts(b);
    let max_len = a_parts.len().max(b_parts.len());

    for idx in 0..max_len {
        let a_val = *a_parts.get(idx).unwrap_or(&0);
        let b_val = *b_parts.get(idx).unwrap_or(&0);

        match a_val.cmp(&b_val) {
            Ordering::Equal => continue,
            other => return other,
        }
    }

    Ordering::Equal
}

fn parse_version_parts(version: &str) -> Vec<u32> {
    version
        .split('.')
        .map(|part| {
            let digits: String = part.chars().take_while(|c| c.is_ascii_digit()).collect();
            digits.parse::<u32>().unwrap_or(0)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compare_versions() {
        assert_eq!(compare_versions("1.2.0", "1.1.9"), Ordering::Greater);
        assert_eq!(compare_versions("1.2", "1.2.0"), Ordering::Equal);
        assert_eq!(compare_versions("2.0.0", "10.0.0"), Ordering::Less);
        assert_eq!(compare_versions("1.2.3-alpha", "1.2.3"), Ordering::Equal);
    }

    #[test]
    fn test_register_versioned_command() {
        let registry = CommandRegistry::new();

        let mut v1 = CommandEntry::default();
        v1.name = "build".to_string();
        v1.version = Some("1.0.0".to_string());

        let mut v2 = CommandEntry::default();
        v2.name = "build".to_string();
        v2.version = Some("1.2.0".to_string());

        let mut v0 = CommandEntry::default();
        v0.name = "build".to_string();
        v0.version = Some("0.9.0".to_string());

        registry.register(v1);
        registry.register(v2);
        registry.register(v0);

        let current = registry.get("build").unwrap();
        assert_eq!(current.version.as_deref(), Some("1.2.0"));
    }

    #[test]
    fn test_register_override_wins() {
        let registry = CommandRegistry::new();

        let mut built_in = CommandEntry::default();
        built_in.name = "doctor".to_string();
        built_in.description = "Built-in".to_string();
        built_in.version = Some("2.0.0".to_string());

        let mut user = CommandEntry::default();
        user.name = "doctor".to_string();
        user.description = "User override".to_string();
        user.version = Some("1.0.0".to_string());

        registry.register(built_in);
        registry.register_override(user);

        let current = registry.get("doctor").unwrap();
        assert_eq!(current.description, "User override");
        assert_eq!(current.version.as_deref(), Some("1.0.0"));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_and_get() {
        let registry = CommandRegistry::new();

        registry.register(CommandEntry {
            name: "test".to_string(),
            description: "A test command".to_string(),
            ..Default::default()
        });

        assert!(registry.contains("test"));
        let entry = registry.get("test").unwrap();
        assert_eq!(entry.name, "test");
    }

    #[test]
    fn test_aliases() {
        let registry = CommandRegistry::new();

        registry.register(CommandEntry {
            name: "hello".to_string(),
            aliases: vec!["hi".to_string(), "hey".to_string()],
            ..Default::default()
        });

        assert!(registry.contains("hello"));
        assert!(registry.contains("hi"));
        assert!(registry.contains("hey"));

        let entry = registry.get("hi").unwrap();
        assert_eq!(entry.name, "hello");
    }

    #[test]
    fn test_unregister() {
        let registry = CommandRegistry::new();

        registry.register(CommandEntry {
            name: "temp".to_string(),
            aliases: vec!["t".to_string()],
            ..Default::default()
        });

        assert!(registry.contains("temp"));
        registry.unregister("temp");
        assert!(!registry.contains("temp"));
        assert!(!registry.contains("t"));
    }

    #[test]
    fn test_levenshtein() {
        assert_eq!(CommandRegistry::levenshtein_distance("hello", "hello"), 0);
        assert_eq!(CommandRegistry::levenshtein_distance("hello", "helo"), 1);
        assert_eq!(CommandRegistry::levenshtein_distance("hello", "world"), 4);
    }

    #[tokio::test]
    async fn test_builtin_execution() {
        let registry = CommandRegistry::new();

        registry.register(CommandEntry {
            name: "echo".to_string(),
            handler: HandlerType::BuiltIn(Arc::new(|args: Vec<String>| {
                Ok(CommandResult {
                    exit_code: 0,
                    stdout: args.join(" "),
                    stderr: String::new(),
                })
            })),
            ..Default::default()
        });

        let result = registry
            .execute("echo", &["Hello".to_string(), "World".to_string()])
            .await
            .unwrap();

        assert_eq!(result.stdout, "Hello World");
    }
}
