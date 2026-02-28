//! Dynamic command executor using the command registry
//!
//! This module provides dynamic command resolution and execution
//! through the registry system, replacing static enum-based dispatch.
//!
//! # Architecture
//!
//! The executor supports three dispatch modes:
//! 1. **Built-in**: Static commands defined in Rust code
//! 2. **Dynamic**: Commands loaded from .sr configuration files
//! 3. **Hybrid**: Built-in commands with dynamic overrides
//!
//! # Example
//!
//! ```rust,ignore
//! use dx_cli::registry::{DynamicExecutor, CommandRegistry};
//!
//! let registry = CommandRegistry::new();
//! let executor = DynamicExecutor::new(registry);
//!
//! // Execute a command dynamically
//! executor.execute("build", &["--release"]).await?;
//! ```

use super::{CommandEntry, CommandError, CommandRegistry, CommandResult, HandlerType};
use std::sync::Arc;

/// Dynamic command executor
#[derive(Clone)]
pub struct DynamicExecutor {
    /// Command registry
    registry: Arc<CommandRegistry>,
    /// Whether to show suggestions on unknown commands
    suggest_on_unknown: bool,
    /// Maximum suggestions to show
    max_suggestions: usize,
}

impl DynamicExecutor {
    /// Create a new dynamic executor
    pub fn new(registry: CommandRegistry) -> Self {
        Self {
            registry: Arc::new(registry),
            suggest_on_unknown: true,
            max_suggestions: 3,
        }
    }

    /// Create from an existing Arc<CommandRegistry>
    pub fn with_registry(registry: Arc<CommandRegistry>) -> Self {
        Self {
            registry,
            suggest_on_unknown: true,
            max_suggestions: 3,
        }
    }

    /// Set whether to show suggestions on unknown commands
    pub fn suggest_on_unknown(mut self, enable: bool) -> Self {
        self.suggest_on_unknown = enable;
        self
    }

    /// Set maximum number of suggestions
    pub fn max_suggestions(mut self, max: usize) -> Self {
        self.max_suggestions = max;
        self
    }

    /// Get the underlying registry
    pub fn registry(&self) -> &CommandRegistry {
        &self.registry
    }

    /// Execute a command by name with arguments
    pub async fn execute(
        &self,
        name: &str,
        args: &[String],
    ) -> Result<CommandResult, CommandError> {
        // Resolve the command (handles aliases)
        let entry = self.resolve(name)?;

        // Check if command is enabled
        if !entry.enabled {
            return Err(CommandError::Disabled {
                name: name.to_string(),
            });
        }

        // Execute based on handler type
        self.execute_handler(&entry, args).await
    }

    /// Resolve a command name to its entry
    pub fn resolve(&self, name: &str) -> Result<CommandEntry, CommandError> {
        self.registry.get(name).ok_or_else(|| {
            let suggestions = if self.suggest_on_unknown {
                self.find_suggestions(name)
            } else {
                Vec::new()
            };

            CommandError::NotFound {
                name: name.to_string(),
                suggestions,
            }
        })
    }

    /// Find similar command names for suggestions
    fn find_suggestions(&self, name: &str) -> Vec<String> {
        let all_names = self.registry.command_names();
        let mut scored: Vec<(String, usize)> = all_names
            .into_iter()
            .filter_map(|cmd| {
                let distance = levenshtein_distance(name, &cmd);
                // Include if distance is small or if it contains the search term
                if distance <= 2 || cmd.contains(name) || name.contains(&cmd) {
                    Some((cmd, distance))
                } else {
                    None
                }
            })
            .collect();

        // Sort by distance, then alphabetically
        scored.sort_by(|a, b| a.1.cmp(&b.1).then_with(|| a.0.cmp(&b.0)));

        scored.into_iter().take(self.max_suggestions).map(|(cmd, _)| cmd).collect()
    }

    /// Execute a handler
    async fn execute_handler(
        &self,
        entry: &CommandEntry,
        args: &[String],
    ) -> Result<CommandResult, CommandError> {
        match &entry.handler {
            HandlerType::BuiltIn(handler) => {
                handler(args.to_vec()).map_err(|e| CommandError::ExecutionFailed {
                    name: entry.name.clone(),
                    reason: e,
                })
            }
            HandlerType::Async(handler) => {
                handler(args.to_vec()).await.map_err(|e| CommandError::ExecutionFailed {
                    name: entry.name.clone(),
                    reason: e,
                })
            }
            HandlerType::Wasm {
                module_path,
                function,
            } => {
                // WASM execution not yet implemented
                Err(CommandError::ExecutionFailed {
                    name: entry.name.clone(),
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
                // Native plugin execution not yet implemented
                Err(CommandError::ExecutionFailed {
                    name: entry.name.clone(),
                    reason: format!(
                        "Native plugin execution not yet implemented: {}::{}",
                        library_path, symbol
                    ),
                })
            }
            HandlerType::Script {
                interpreter,
                script,
            } => self.execute_script(interpreter, script, &entry.name, args).await,
        }
    }

    /// Execute a script command
    async fn execute_script(
        &self,
        interpreter: &str,
        script: &str,
        name: &str,
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
                name: name.to_string(),
                reason: e.to_string(),
            })?;

        if output.status.success() {
            Ok(CommandResult {
                exit_code: 0,
                stdout: String::from_utf8_lossy(&output.stdout).to_string(),
                stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            })
        } else {
            let code = output.status.code().unwrap_or(1);
            Err(CommandError::ExecutionFailed {
                name: name.to_string(),
                reason: format!(
                    "Script exited with code {}: {}",
                    code,
                    String::from_utf8_lossy(&output.stderr)
                ),
            })
        }
    }

    /// Check if a command exists
    pub fn has_command(&self, name: &str) -> bool {
        self.registry.contains(name)
    }

    /// List all available commands
    pub fn list_commands(&self) -> Vec<CommandEntry> {
        self.registry
            .command_names()
            .into_iter()
            .filter_map(|name| self.registry.get(&name))
            .filter(|entry| !entry.hidden)
            .collect()
    }

    /// List commands by category
    pub fn list_by_category(&self) -> std::collections::HashMap<String, Vec<CommandEntry>> {
        self.registry.commands_by_category()
    }

    /// Format an unknown command error message
    pub fn format_unknown_error(&self, name: &str) -> String {
        let suggestions = self.find_suggestions(name);

        if suggestions.is_empty() {
            format!("Unknown command: '{}'\n\nRun 'dx --help' to see available commands.", name)
        } else {
            format!(
                "Unknown command: '{}'\n\nDid you mean:\n{}\n\nRun 'dx --help' to see available commands.",
                name,
                suggestions.iter().map(|s| format!("  - {}", s)).collect::<Vec<_>>().join("\n")
            )
        }
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::{CommandEntry, HandlerType};
    use std::sync::Arc;

    fn create_test_registry() -> CommandRegistry {
        let registry = CommandRegistry::new();

        registry.register(CommandEntry {
            name: "build".to_string(),
            description: "Build the project".to_string(),
            aliases: vec!["b".to_string()],
            handler: HandlerType::BuiltIn(Arc::new(|args| {
                Ok(CommandResult::success(format!("Built with args: {:?}", args)))
            })),
            ..Default::default()
        });

        registry.register(CommandEntry {
            name: "test".to_string(),
            description: "Run tests".to_string(),
            aliases: vec!["t".to_string()],
            handler: HandlerType::BuiltIn(Arc::new(|_| Ok(CommandResult::success("Tests passed")))),
            ..Default::default()
        });

        registry.register(CommandEntry {
            name: "deploy".to_string(),
            description: "Deploy the project".to_string(),
            enabled: false,
            ..Default::default()
        });

        registry
    }

    #[tokio::test]
    async fn test_execute_command() {
        let executor = DynamicExecutor::new(create_test_registry());
        let result = executor.execute("build", &["--release".to_string()]).await.unwrap();

        assert!(result.stdout.contains("--release"));
    }

    #[tokio::test]
    async fn test_execute_via_alias() {
        let executor = DynamicExecutor::new(create_test_registry());
        let result = executor.execute("b", &[]).await.unwrap();

        assert!(result.stdout.contains("Built"));
    }

    #[tokio::test]
    async fn test_unknown_command() {
        let executor = DynamicExecutor::new(create_test_registry());
        let result = executor.execute("buidl", &[]).await;

        match result {
            Err(CommandError::NotFound { name, suggestions }) => {
                assert_eq!(name, "buidl");
                assert!(suggestions.contains(&"build".to_string()));
            }
            _ => panic!("Expected NotFound error"),
        }
    }

    #[tokio::test]
    async fn test_disabled_command() {
        let executor = DynamicExecutor::new(create_test_registry());
        let result = executor.execute("deploy", &[]).await;

        match result {
            Err(CommandError::Disabled { name }) => {
                assert_eq!(name, "deploy");
            }
            _ => panic!("Expected Disabled error"),
        }
    }

    #[test]
    fn test_levenshtein_distance() {
        assert_eq!(levenshtein_distance("hello", "hello"), 0);
        assert_eq!(levenshtein_distance("hello", "helo"), 1);
        assert_eq!(levenshtein_distance("build", "buidl"), 2);
        assert_eq!(levenshtein_distance("test", "tests"), 1);
    }

    #[test]
    fn test_suggestions() {
        let executor = DynamicExecutor::new(create_test_registry());

        let suggestions = executor.find_suggestions("buidl");
        assert!(suggestions.contains(&"build".to_string()));

        let suggestions = executor.find_suggestions("tes");
        assert!(suggestions.contains(&"test".to_string()));
    }

    #[test]
    fn test_format_unknown_error() {
        let executor = DynamicExecutor::new(create_test_registry());

        let error = executor.format_unknown_error("buidl");
        assert!(error.contains("Unknown command: 'buidl'"));
        assert!(error.contains("build"));
    }

    #[test]
    fn test_list_commands() {
        let executor = DynamicExecutor::new(create_test_registry());
        let commands = executor.list_commands();

        assert_eq!(commands.len(), 3);
    }

    #[test]
    fn test_has_command() {
        let executor = DynamicExecutor::new(create_test_registry());

        assert!(executor.has_command("build"));
        assert!(executor.has_command("b")); // alias
        assert!(!executor.has_command("unknown"));
    }
}
