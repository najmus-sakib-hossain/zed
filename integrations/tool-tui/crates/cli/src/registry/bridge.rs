//! Bridge module for migrating from static enum dispatch to dynamic registry
//!
//! This module registers all existing CLI commands into the registry system,
//! allowing gradual migration from the old executor to the new one.

use super::{CommandEntry, CommandRegistry, HandlerType};
use crate::ui::theme::Theme;
use std::sync::Arc;

/// Register all built-in commands into the registry
pub fn register_builtin_commands(registry: &CommandRegistry, theme: Arc<Theme>) {
    // Project Commands
    register_project_commands(registry, theme.clone());

    // Code Quality
    register_quality_commands(registry, theme.clone());

    // Asset Tools
    register_asset_commands(registry, theme.clone());

    // Infrastructure
    register_infrastructure_commands(registry, theme.clone());

    // Development
    register_development_commands(registry, theme.clone());

    // Utility
    register_utility_commands(registry, theme.clone());

    // Interactive & UI
    register_interactive_commands(registry, theme.clone());
}

fn register_project_commands(registry: &CommandRegistry, theme: Arc<Theme>) {
    registry.register(CommandEntry {
        name: "init".to_string(),
        description: "Initialize a new DX project".to_string(),
        category: Some("Project".to_string()),
        handler: HandlerType::Async(Arc::new(move |_args| {
            let _theme = theme.clone();
            Box::pin(async move {
                // This is a placeholder - actual implementation would parse args
                Err("Command execution via registry not yet fully implemented".to_string())
            })
        })),
        ..Default::default()
    });

    registry.register(CommandEntry {
        name: "dev".to_string(),
        description: "Start development server".to_string(),
        category: Some("Project".to_string()),
        handler: HandlerType::Async(Arc::new(move |_args| {
            Box::pin(async move {
                Err("Command execution via registry not yet fully implemented".to_string())
            })
        })),
        ..Default::default()
    });

    registry.register(CommandEntry {
        name: "build".to_string(),
        aliases: vec!["b".to_string()],
        description: "Build the project".to_string(),
        category: Some("Project".to_string()),
        handler: HandlerType::Async(Arc::new(move |_args| {
            Box::pin(async move {
                Err("Command execution via registry not yet fully implemented".to_string())
            })
        })),
        ..Default::default()
    });

    registry.register(CommandEntry {
        name: "run".to_string(),
        aliases: vec!["r".to_string()],
        description: "Run the project".to_string(),
        category: Some("Project".to_string()),
        handler: HandlerType::Async(Arc::new(move |_args| {
            Box::pin(async move {
                Err("Command execution via registry not yet fully implemented".to_string())
            })
        })),
        ..Default::default()
    });

    registry.register(CommandEntry {
        name: "test".to_string(),
        aliases: vec!["t".to_string()],
        description: "Run tests".to_string(),
        category: Some("Project".to_string()),
        handler: HandlerType::Async(Arc::new(move |_args| {
            Box::pin(async move {
                Err("Command execution via registry not yet fully implemented".to_string())
            })
        })),
        ..Default::default()
    });

    registry.register(CommandEntry {
        name: "deploy".to_string(),
        description: "Deploy the project".to_string(),
        category: Some("Project".to_string()),
        handler: HandlerType::Async(Arc::new(move |_args| {
            Box::pin(async move {
                Err("Command execution via registry not yet fully implemented".to_string())
            })
        })),
        ..Default::default()
    });
}

fn register_quality_commands(registry: &CommandRegistry, _theme: Arc<Theme>) {
    registry.register(CommandEntry {
        name: "check".to_string(),
        description: "Run code quality checks".to_string(),
        category: Some("Quality".to_string()),
        handler: HandlerType::Async(Arc::new(move |_args| {
            Box::pin(async move {
                Err("Command execution via registry not yet fully implemented".to_string())
            })
        })),
        ..Default::default()
    });
}

fn register_asset_commands(registry: &CommandRegistry, _theme: Arc<Theme>) {
    registry.register(CommandEntry {
        name: "style".to_string(),
        description: "Process stylesheets".to_string(),
        category: Some("Assets".to_string()),
        handler: HandlerType::Async(Arc::new(move |_args| {
            Box::pin(async move {
                Err("Command execution via registry not yet fully implemented".to_string())
            })
        })),
        ..Default::default()
    });

    registry.register(CommandEntry {
        name: "media".to_string(),
        description: "Process media files".to_string(),
        category: Some("Assets".to_string()),
        handler: HandlerType::Async(Arc::new(move |_args| {
            Box::pin(async move {
                Err("Command execution via registry not yet fully implemented".to_string())
            })
        })),
        ..Default::default()
    });

    registry.register(CommandEntry {
        name: "font".to_string(),
        description: "Process fonts".to_string(),
        category: Some("Assets".to_string()),
        handler: HandlerType::Async(Arc::new(move |_args| {
            Box::pin(async move {
                Err("Command execution via registry not yet fully implemented".to_string())
            })
        })),
        ..Default::default()
    });

    registry.register(CommandEntry {
        name: "icon".to_string(),
        description: "Process icons".to_string(),
        category: Some("Assets".to_string()),
        handler: HandlerType::Async(Arc::new(move |_args| {
            Box::pin(async move {
                Err("Command execution via registry not yet fully implemented".to_string())
            })
        })),
        ..Default::default()
    });
}

fn register_infrastructure_commands(registry: &CommandRegistry, _theme: Arc<Theme>) {
    registry.register(CommandEntry {
        name: "forge".to_string(),
        description: "Build orchestration".to_string(),
        category: Some("Infrastructure".to_string()),
        handler: HandlerType::Async(Arc::new(move |_args| {
            Box::pin(async move {
                Err("Command execution via registry not yet fully implemented".to_string())
            })
        })),
        ..Default::default()
    });

    registry.register(CommandEntry {
        name: "serializer".to_string(),
        description: "DX Serializer operations".to_string(),
        category: Some("Infrastructure".to_string()),
        handler: HandlerType::Async(Arc::new(move |_args| {
            Box::pin(async move {
                Err("Command execution via registry not yet fully implemented".to_string())
            })
        })),
        ..Default::default()
    });

    registry.register(CommandEntry {
        name: "markdown".to_string(),
        description: "Markdown processing".to_string(),
        category: Some("Infrastructure".to_string()),
        handler: HandlerType::Async(Arc::new(move |_args| {
            Box::pin(async move {
                Err("Command execution via registry not yet fully implemented".to_string())
            })
        })),
        ..Default::default()
    });
}

fn register_development_commands(registry: &CommandRegistry, _theme: Arc<Theme>) {
    registry.register(CommandEntry {
        name: "driven".to_string(),
        description: "AI-assisted development".to_string(),
        category: Some("Development".to_string()),
        handler: HandlerType::Async(Arc::new(move |_args| {
            Box::pin(async move {
                Err("Command execution via registry not yet fully implemented".to_string())
            })
        })),
        ..Default::default()
    });

    registry.register(CommandEntry {
        name: "generator".to_string(),
        description: "Code generation".to_string(),
        category: Some("Development".to_string()),
        handler: HandlerType::Async(Arc::new(move |_args| {
            Box::pin(async move {
                Err("Command execution via registry not yet fully implemented".to_string())
            })
        })),
        ..Default::default()
    });

    registry.register(CommandEntry {
        name: "workspace".to_string(),
        description: "Workspace management".to_string(),
        category: Some("Development".to_string()),
        handler: HandlerType::Async(Arc::new(move |_args| {
            Box::pin(async move {
                Err("Command execution via registry not yet fully implemented".to_string())
            })
        })),
        ..Default::default()
    });
}

fn register_utility_commands(registry: &CommandRegistry, _theme: Arc<Theme>) {
    registry.register(CommandEntry {
        name: "system".to_string(),
        description: "Show system information".to_string(),
        category: Some("Utility".to_string()),
        handler: HandlerType::Async(Arc::new(move |_args| {
            Box::pin(async move {
                Err("Command execution via registry not yet fully implemented".to_string())
            })
        })),
        ..Default::default()
    });

    registry.register(CommandEntry {
        name: "logo".to_string(),
        description: "Show DX logo".to_string(),
        category: Some("Utility".to_string()),
        handler: HandlerType::Async(Arc::new(move |_args| {
            Box::pin(async move {
                Err("Command execution via registry not yet fully implemented".to_string())
            })
        })),
        ..Default::default()
    });

    registry.register(CommandEntry {
        name: "doctor".to_string(),
        description: "Diagnose system issues".to_string(),
        category: Some("Utility".to_string()),
        handler: HandlerType::Async(Arc::new(move |_args| {
            Box::pin(async move {
                Err("Command execution via registry not yet fully implemented".to_string())
            })
        })),
        ..Default::default()
    });
}

fn register_interactive_commands(registry: &CommandRegistry, _theme: Arc<Theme>) {
    registry.register(CommandEntry {
        name: "chat".to_string(),
        description: "Interactive chat interface".to_string(),
        category: Some("Interactive".to_string()),
        handler: HandlerType::Async(Arc::new(move |_args| {
            Box::pin(async move {
                Err("Command execution via registry not yet fully implemented".to_string())
            })
        })),
        ..Default::default()
    });

    registry.register(CommandEntry {
        name: "triple".to_string(),
        description: "Triple layout demo".to_string(),
        category: Some("Interactive".to_string()),
        hidden: true,
        handler: HandlerType::Async(Arc::new(move |_args| {
            Box::pin(async move {
                Err("Command execution via registry not yet fully implemented".to_string())
            })
        })),
        ..Default::default()
    });
}
