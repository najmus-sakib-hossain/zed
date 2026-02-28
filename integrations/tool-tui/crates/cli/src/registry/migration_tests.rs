//! Migration tests for old static executor to new dynamic dispatch
//!
//! These tests verify that the new registry-based dynamic dispatch system
//! maintains compatibility with the old static enum-based execution.

#[cfg(test)]
mod tests {
    use crate::registry::{
        CommandEntry, CommandError, CommandRegistry, CommandResult, DynamicExecutor, HandlerType,
        RegistryLoader,
    };
    use std::sync::Arc;

    /// Test that built-in commands work identically in both systems
    #[tokio::test]
    async fn test_builtin_command_compatibility() {
        let registry = CommandRegistry::new();

        // Register a command similar to the old static executor
        registry.register(CommandEntry {
            name: "echo".to_string(),
            description: "Echo arguments".to_string(),
            aliases: vec!["e".to_string()],
            handler: HandlerType::BuiltIn(Arc::new(|args| {
                Ok(CommandResult::success(args.join(" ")))
            })),
            category: Some("Utility".to_string()),
            ..Default::default()
        });

        let executor = DynamicExecutor::new(registry);

        // Test direct execution
        let result = executor.execute("echo", &["hello".to_string(), "world".to_string()]).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().stdout, "hello world");

        // Test alias execution
        let result = executor.execute("e", &["test".to_string()]).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().stdout, "test");
    }

    /// Test that async handlers work correctly
    #[tokio::test]
    async fn test_async_handler_migration() {
        let registry = CommandRegistry::new();

        registry.register(CommandEntry {
            name: "async-cmd".to_string(),
            description: "Async command".to_string(),
            handler: HandlerType::Async(Arc::new(|args| {
                Box::pin(async move {
                    // Simulate async work
                    tokio::time::sleep(std::time::Duration::from_millis(1)).await;
                    Ok(CommandResult::success(format!("Async result: {:?}", args)))
                })
            })),
            ..Default::default()
        });

        let executor = DynamicExecutor::new(registry);
        let result = executor.execute("async-cmd", &["arg1".to_string()]).await;

        assert!(result.is_ok());
        assert!(result.unwrap().stdout.contains("arg1"));
    }

    /// Test that unknown command errors are formatted correctly
    #[tokio::test]
    async fn test_unknown_command_error_format() {
        let registry = CommandRegistry::new();
        registry.register(CommandEntry {
            name: "build".to_string(),
            description: "Build project".to_string(),
            ..Default::default()
        });

        let executor = DynamicExecutor::new(registry);
        let result = executor.execute("buidl", &[]).await;

        match result {
            Err(CommandError::NotFound { name, suggestions }) => {
                assert_eq!(name, "buidl");
                assert!(suggestions.contains(&"build".to_string()));
            }
            _ => panic!("Expected NotFound error with suggestions"),
        }

        // Test error formatting
        let error_msg = executor.format_unknown_error("buidl");
        assert!(error_msg.contains("Unknown command"));
        assert!(error_msg.contains("build"));
    }

    /// Test that disabled commands return proper errors
    #[tokio::test]
    async fn test_disabled_command_migration() {
        let registry = CommandRegistry::new();
        registry.register(CommandEntry {
            name: "deprecated".to_string(),
            description: "Old command".to_string(),
            enabled: false,
            ..Default::default()
        });

        let executor = DynamicExecutor::new(registry);
        let result = executor.execute("deprecated", &[]).await;

        match result {
            Err(CommandError::Disabled { name }) => {
                assert_eq!(name, "deprecated");
            }
            _ => panic!("Expected Disabled error"),
        }
    }

    /// Test that command categories are preserved
    #[test]
    fn test_category_migration() {
        let registry = CommandRegistry::new();

        registry.register(CommandEntry {
            name: "build".to_string(),
            category: Some("Project".to_string()),
            ..Default::default()
        });

        registry.register(CommandEntry {
            name: "test".to_string(),
            category: Some("Project".to_string()),
            ..Default::default()
        });

        registry.register(CommandEntry {
            name: "format".to_string(),
            category: Some("Code Quality".to_string()),
            ..Default::default()
        });

        let categories = registry.commands_by_category();

        assert!(categories.contains_key("Project"));
        assert_eq!(categories.get("Project").map(|v| v.len()), Some(2));
        assert!(categories.contains_key("Code Quality"));
    }

    /// Test that script handlers work
    #[tokio::test]
    #[cfg(unix)]
    async fn test_script_handler_migration() {
        let registry = CommandRegistry::new();

        registry.register(CommandEntry {
            name: "hello-script".to_string(),
            description: "Script command".to_string(),
            handler: HandlerType::Script {
                interpreter: "sh".to_string(),
                script: "echo 'Hello from script'".to_string(),
            },
            ..Default::default()
        });

        let executor = DynamicExecutor::new(registry);
        let result = executor.execute("hello-script", &[]).await;

        assert!(result.is_ok());
        assert!(result.unwrap().stdout.contains("Hello from script"));
    }

    /// Test loading from .sr configuration
    #[test]
    fn test_sr_config_loading() {
        let sr_content = r#"
[[command]]
name = "custom"
description = "A custom command"
aliases = "c, cust"
category = "Custom"
enabled = true
script = "echo custom"

[[command]]
name = "another"
description = "Another command"
category = "Custom"
"#;

        let registry = CommandRegistry::new();
        let count = RegistryLoader::load_from_str(sr_content, &registry).unwrap();

        assert_eq!(count, 2);
        assert!(registry.contains("custom"));
        assert!(registry.contains("c")); // alias
        assert!(registry.contains("cust")); // alias
        assert!(registry.contains("another"));

        let entry = registry.get("custom").unwrap();
        assert_eq!(entry.category, Some("Custom".to_string()));
    }

    /// Test that registry hot-reload preserves state correctly
    #[test]
    fn test_hot_reload_state() {
        let registry = CommandRegistry::new();

        // Initial load
        registry.register(CommandEntry {
            name: "cmd".to_string(),
            description: "v1".to_string(),
            version: Some("1.0.0".to_string()),
            ..Default::default()
        });

        assert_eq!(registry.get("cmd").unwrap().description, "v1");

        // Hot reload with newer version
        registry.register(CommandEntry {
            name: "cmd".to_string(),
            description: "v2".to_string(),
            version: Some("2.0.0".to_string()),
            ..Default::default()
        });

        assert_eq!(registry.get("cmd").unwrap().description, "v2");

        // Hot reload with older version should not replace
        registry.register(CommandEntry {
            name: "cmd".to_string(),
            description: "v1-old".to_string(),
            version: Some("1.0.0".to_string()),
            ..Default::default()
        });

        // Should still be v2
        assert_eq!(registry.get("cmd").unwrap().description, "v2");
    }

    /// Test executor with shared registry reference
    #[tokio::test]
    async fn test_shared_registry() {
        let registry = Arc::new(CommandRegistry::new());

        registry.register(CommandEntry {
            name: "shared".to_string(),
            description: "Shared command".to_string(),
            handler: HandlerType::BuiltIn(Arc::new(|_| Ok(CommandResult::success("shared")))),
            ..Default::default()
        });

        // Create multiple executors sharing the same registry
        let executor1 = DynamicExecutor::with_registry(Arc::clone(&registry));
        let executor2 = DynamicExecutor::with_registry(Arc::clone(&registry));

        // Both should see the same commands
        assert!(executor1.has_command("shared"));
        assert!(executor2.has_command("shared"));

        // Register a new command via registry
        registry.register(CommandEntry {
            name: "new".to_string(),
            description: "New command".to_string(),
            ..Default::default()
        });

        // Both executors should see the new command
        assert!(executor1.has_command("new"));
        assert!(executor2.has_command("new"));
    }

    /// Test that all old command patterns are supported
    #[test]
    fn test_command_patterns_compatibility() {
        let registry = CommandRegistry::new();

        // Pattern 1: Simple command
        registry.register(CommandEntry {
            name: "simple".to_string(),
            ..Default::default()
        });

        // Pattern 2: Command with aliases
        registry.register(CommandEntry {
            name: "aliased".to_string(),
            aliases: vec!["a".to_string(), "al".to_string()],
            ..Default::default()
        });

        // Pattern 3: Hidden command
        registry.register(CommandEntry {
            name: "hidden".to_string(),
            hidden: true,
            ..Default::default()
        });

        // Pattern 4: Command with arguments
        registry.register(CommandEntry {
            name: "with-args".to_string(),
            arguments: vec![
                crate::registry::ArgumentDef {
                    name: "input".to_string(),
                    required: true,
                    positional: true,
                    ..Default::default()
                },
                crate::registry::ArgumentDef {
                    name: "verbose".to_string(),
                    short: Some('v'),
                    long: Some("verbose".to_string()),
                    ..Default::default()
                },
            ],
            ..Default::default()
        });

        // All should be registered
        assert!(registry.contains("simple"));
        assert!(registry.contains("aliased"));
        assert!(registry.contains("a"));
        assert!(registry.contains("hidden"));
        assert!(registry.contains("with-args"));

        // Hidden commands should still be accessible but not listed
        let executor = DynamicExecutor::new(registry);
        let visible_commands = executor.list_commands();
        assert!(!visible_commands.iter().any(|c| c.name == "hidden"));
    }
}
