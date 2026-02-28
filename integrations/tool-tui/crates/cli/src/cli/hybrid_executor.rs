//! Hybrid command executor supporting both legacy and registry-based dispatch
//!
//! This module provides a transition path from static enum-based command
//! execution to dynamic registry-based execution.

use anyhow::Result;
use std::sync::Arc;

use crate::registry::{CommandRegistry, DynamicExecutor};
use crate::ui::theme::Theme;

use super::commands::Commands;

#[allow(deprecated)]
use super::executor::execute_command as legacy_execute;

/// Hybrid executor that supports both legacy and registry-based dispatch
pub struct HybridExecutor {
    registry: Arc<CommandRegistry>,
    executor: DynamicExecutor,
    theme: Arc<Theme>,
    /// Whether to prefer registry over legacy (for gradual migration)
    prefer_registry: bool,
}

impl HybridExecutor {
    /// Create a new hybrid executor
    pub fn new(theme: Theme) -> Self {
        let registry = CommandRegistry::new();
        let theme_arc = Arc::new(theme);

        // Register all built-in commands
        crate::registry::register_builtin_commands(&registry, theme_arc.clone());

        let executor = DynamicExecutor::new(registry.clone());

        Self {
            registry: Arc::new(registry),
            executor,
            theme: theme_arc,
            prefer_registry: false, // Start with legacy by default
        }
    }

    /// Enable registry-first execution (for testing migration)
    pub fn prefer_registry(mut self, enable: bool) -> Self {
        self.prefer_registry = enable;
        self
    }

    /// Execute a command using the appropriate dispatcher
    pub async fn execute(&self, command: Commands) -> Result<()> {
        if self.prefer_registry {
            // Try registry first, fall back to legacy
            match self.try_registry_execute(&command).await {
                Ok(()) => Ok(()),
                Err(_) => {
                    // Fall back to legacy
                    #[allow(deprecated)]
                    legacy_execute(command, &self.theme).await
                }
            }
        } else {
            // Use legacy by default (current behavior)
            #[allow(deprecated)]
            legacy_execute(command, &self.theme).await
        }
    }

    /// Try to execute via registry
    async fn try_registry_execute(&self, command: &Commands) -> Result<()> {
        // Map Commands enum to registry command names
        let (name, args) = self.command_to_registry_call(command)?;

        self.executor
            .execute(&name, &args)
            .await
            .map(|_| ())
            .map_err(|e| anyhow::anyhow!("Registry execution failed: {}", e))
    }

    /// Convert Commands enum to registry command name and args
    fn command_to_registry_call(&self, command: &Commands) -> Result<(String, Vec<String>)> {
        // This is a simplified mapping - full implementation would parse all args
        let name = match command {
            Commands::Init(_) => "init",
            Commands::Dev(_) => "dev",
            Commands::Build(_) => "build",
            Commands::Run(_) => "run",
            Commands::Test(_) => "test",
            Commands::Deploy(_) => "deploy",
            Commands::Check(_) => "check",
            Commands::Style(_) => "style",
            Commands::Media(_) => "media",
            Commands::Font(_) => "font",
            Commands::Icon(_) => "icon",
            Commands::Forge(_) => "forge",
            Commands::Serializer(_) => "serializer",
            Commands::Markdown(_) => "markdown",
            Commands::Driven(_) => "driven",
            Commands::Generator(_) => "generator",
            Commands::Workspace(_) => "workspace",
            Commands::System => "system",
            Commands::Logo => "logo",
            Commands::Doctor(_) => "doctor",
            Commands::Chat(_) => "chat",
            Commands::Triple => "triple",
            _ => return Err(anyhow::anyhow!("Command not yet mapped to registry")),
        };

        Ok((name.to_string(), vec![]))
    }

    /// Get the underlying registry
    pub fn registry(&self) -> &CommandRegistry {
        &self.registry
    }

    /// Get the dynamic executor
    pub fn executor(&self) -> &DynamicExecutor {
        &self.executor
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::theme::ColorMode;

    #[test]
    fn test_hybrid_executor_creation() {
        let theme = Theme::with_color_mode(ColorMode::Never);
        let executor = HybridExecutor::new(theme);

        // Verify registry has commands
        assert!(executor.registry().len() > 0);

        // Verify some key commands exist
        assert!(executor.registry().contains("build"));
        assert!(executor.registry().contains("test"));
        assert!(executor.registry().contains("check"));
    }

    #[test]
    fn test_command_aliases() {
        let theme = Theme::with_color_mode(ColorMode::Never);
        let executor = HybridExecutor::new(theme);

        // Test aliases work
        assert!(executor.registry().contains("b")); // build alias
        assert!(executor.registry().contains("r")); // run alias
        assert!(executor.registry().contains("t")); // test alias
    }
}
