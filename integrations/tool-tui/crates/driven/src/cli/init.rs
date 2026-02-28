//! Init command - initialize driven in a project

use crate::{DrivenConfig, Result};
use dialoguer::{Confirm, MultiSelect, Select, theme::ColorfulTheme};
use std::path::Path;

/// Init command handler
#[derive(Debug)]
pub struct InitCommand;

impl InitCommand {
    /// Run the init command
    pub fn run(project_root: &Path, interactive: bool) -> Result<()> {
        if interactive {
            Self::run_interactive(project_root)
        } else {
            Self::run_default(project_root)
        }
    }

    fn run_interactive(project_root: &Path) -> Result<()> {
        let theme = ColorfulTheme::default();

        println!();
        println!("🚀 Welcome to Driven - AI-Assisted Development Orchestrator");
        println!();

        // Select editors
        let editor_items = &[
            "Cursor (.cursor/rules/)",
            "GitHub Copilot (.github/copilot-instructions.md)",
            "Windsurf (.windsurf/rules/)",
            "Claude (CLAUDE.md)",
            "Aider (.aider.conf.yml)",
            "Cline (.clinerules)",
        ];

        let selected = MultiSelect::with_theme(&theme)
            .with_prompt("Which AI editors do you use?")
            .items(editor_items)
            .defaults(&[true, true, false, false, false, false])
            .interact()
            .map_err(|e| crate::DrivenError::Cli(e.to_string()))?;

        let mut config = DrivenConfig::default();
        config.editors.cursor = selected.contains(&0);
        config.editors.copilot = selected.contains(&1);
        config.editors.windsurf = selected.contains(&2);
        config.editors.claude = selected.contains(&3);
        config.editors.aider = selected.contains(&4);
        config.editors.cline = selected.contains(&5);

        // Select template
        let template_items = &[
            "Rust Workspace",
            "TypeScript/JavaScript",
            "Full-Stack (Rust + TypeScript)",
            "CLI Tool",
            "Library",
            "Custom (start empty)",
        ];

        let template_idx = Select::with_theme(&theme)
            .with_prompt("What type of project is this?")
            .items(template_items)
            .default(0)
            .interact()
            .map_err(|e| crate::DrivenError::Cli(e.to_string()))?;

        config.templates.project = Some(match template_idx {
            0 => "rust-workspace".to_string(),
            1 => "typescript-monorepo".to_string(),
            2 => "fullstack".to_string(),
            3 => "cli-tool".to_string(),
            4 => "library".to_string(),
            _ => "custom".to_string(),
        });

        // Enable auto-sync?
        config.sync.watch = Confirm::with_theme(&theme)
            .with_prompt("Enable auto-sync when rules change?")
            .default(true)
            .interact()
            .map_err(|e| crate::DrivenError::Cli(e.to_string()))?;

        // Create config
        Self::create_config(project_root, &config)?;

        // Create initial rules
        Self::create_initial_rules(project_root, &config)?;

        super::print_success("Driven initialized successfully!");
        println!();

        // Show next steps
        println!("📋 Next steps:");
        println!("   1. Edit .driven/rules.drv to customize your AI rules");
        println!("   2. Run 'driven sync' to propagate rules to all editors");
        println!("   3. Run 'driven analyze' to generate context from your codebase");
        println!();

        Ok(())
    }

    fn run_default(project_root: &Path) -> Result<()> {
        let config = DrivenConfig::default();
        Self::create_config(project_root, &config)?;
        Self::create_initial_rules(project_root, &config)?;

        super::print_success("Driven initialized with default settings");
        super::print_info("Run 'driven init -i' for interactive setup");

        Ok(())
    }

    fn create_config(project_root: &Path, config: &DrivenConfig) -> Result<()> {
        let config_dir = project_root.join(".driven");
        std::fs::create_dir_all(&config_dir).map_err(|e| {
            crate::DrivenError::Config(format!("Failed to create .driven directory: {}", e))
        })?;

        let config_path = config_dir.join("config.toml");
        let toml = toml::to_string_pretty(config).map_err(|e| {
            crate::DrivenError::Config(format!("Failed to serialize config: {}", e))
        })?;

        std::fs::write(&config_path, toml)
            .map_err(|e| crate::DrivenError::Config(format!("Failed to write config: {}", e)))?;

        Ok(())
    }

    fn create_initial_rules(project_root: &Path, config: &DrivenConfig) -> Result<()> {
        let rules_path = project_root.join(&config.sync.source_of_truth);

        // Ensure parent directory exists
        if let Some(parent) = rules_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                crate::DrivenError::Config(format!("Failed to create rules directory: {}", e))
            })?;
        }

        // Create initial markdown rules (can be converted to binary later)
        let initial_rules = r#"# AI Development Rules

## Persona

You are an expert software engineer with deep knowledge of the project's tech stack.

### Traits
- Precise and detail-oriented
- Proactive about edge cases
- Explains reasoning clearly

### Principles
- Write idiomatic code
- Prefer explicit over implicit
- Test thoroughly

## Standards

### Style
- Follow project's existing code style
- Use consistent formatting

### Naming
- Use descriptive, meaningful names
- Follow language conventions

### Error Handling
- Handle all error cases explicitly
- Provide helpful error messages

### Testing
- Write tests for all new features
- Maintain existing test coverage

## Context

### Focus
- src/
- tests/

### Exclude
- target/
- node_modules/
- .git/
"#;

        let rules_md_path = rules_path.with_extension("md");
        std::fs::write(&rules_md_path, initial_rules)
            .map_err(|e| crate::DrivenError::Config(format!("Failed to write rules: {}", e)))?;

        Ok(())
    }
}
