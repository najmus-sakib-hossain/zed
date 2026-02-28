//! Driven CLI - AI Development Orchestrator
//!
//! Command-line interface for managing AI coding rules across editors.

use clap::{Parser, Subcommand};
use std::path::PathBuf;

/// AI Development Orchestrator
#[derive(Parser)]
#[command(name = "driven")]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Verbose output
    #[arg(short, long, action = clap::ArgAction::Count)]
    verbose: u8,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize Driven in the current project
    Init {
        /// Run in interactive mode
        #[arg(short, long)]
        interactive: bool,
    },
    /// Synchronize rules to all editors
    Sync {
        /// Watch for changes
        #[arg(short, long)]
        watch: bool,
    },
    /// Convert rules between formats
    Convert {
        /// Input file
        input: PathBuf,
        /// Output file
        output: PathBuf,
        /// Target editor format
        #[arg(short, long)]
        editor: Option<String>,
    },
    /// Manage templates
    Template {
        #[command(subcommand)]
        action: TemplateAction,
    },
    /// Analyze project for context
    Analyze {
        /// Generate context rules
        #[arg(short, long)]
        context: bool,
        /// Index the codebase
        #[arg(short, long)]
        index: bool,
    },
    /// Validate rules
    Validate {
        /// Rules file to validate
        #[arg(default_value = ".driven/rules.md")]
        file: PathBuf,
        /// Strict mode (fail on warnings)
        #[arg(short, long)]
        strict: bool,
    },
    /// Manage agent hooks
    Hook {
        #[command(subcommand)]
        action: HookAction,
    },
    /// Manage agent steering rules
    Steer {
        #[command(subcommand)]
        action: SteerAction,
    },
}

#[derive(Subcommand)]
enum TemplateAction {
    /// List available templates
    List,
    /// Search templates
    Search {
        /// Search query
        query: String,
    },
    /// Apply a template
    Apply {
        /// Template name
        name: String,
    },
}

#[derive(Subcommand)]
enum HookAction {
    /// List all hooks
    List,
    /// Add a new hook
    Add {
        /// Hook ID (unique identifier)
        id: String,
        /// Trigger type (file, git, build, test, manual, scheduled)
        #[arg(short, long)]
        trigger: String,
        /// Trigger value (patterns, operations, events, or command)
        #[arg(short = 'v', long)]
        trigger_value: String,
        /// Agent to invoke
        #[arg(short, long)]
        agent: String,
        /// Message to send to the agent
        #[arg(short, long)]
        message: String,
        /// Hook name (defaults to ID)
        #[arg(short, long)]
        name: Option<String>,
        /// Workflow to run
        #[arg(short, long)]
        workflow: Option<String>,
        /// Condition expression
        #[arg(short, long)]
        condition: Option<String>,
        /// Disable the hook initially
        #[arg(long)]
        disabled: bool,
    },
    /// Remove a hook
    Remove {
        /// Hook ID to remove
        id: String,
    },
    /// Manually trigger a hook
    Trigger {
        /// Command name to trigger
        command: String,
    },
    /// Enable a hook
    Enable {
        /// Hook ID to enable
        id: String,
    },
    /// Disable a hook
    Disable {
        /// Hook ID to disable
        id: String,
    },
    /// Show hook details
    Show {
        /// Hook ID to show
        id: String,
    },
}

#[derive(Subcommand)]
enum SteerAction {
    /// List all steering rules
    List,
    /// Add a new steering rule
    Add {
        /// Rule ID (unique identifier)
        id: String,
        /// Inclusion type (always, fileMatch, manual)
        #[arg(short, long, default_value = "always")]
        inclusion: String,
        /// Pattern (for fileMatch) or key (for manual)
        #[arg(short, long)]
        pattern: Option<String>,
        /// Rule content (markdown)
        #[arg(short, long)]
        content: String,
        /// Rule name (defaults to ID)
        #[arg(short, long)]
        name: Option<String>,
        /// Priority (lower = higher priority)
        #[arg(long)]
        priority: Option<u8>,
    },
    /// Remove a steering rule
    Remove {
        /// Rule ID to remove
        id: String,
    },
    /// Test which rules apply to a file
    Test {
        /// File path to test
        file: PathBuf,
        /// Manual keys to include
        #[arg(short, long)]
        keys: Vec<String>,
    },
    /// Show steering rule details
    Show {
        /// Rule ID to show
        id: String,
    },
    /// Get combined steering content for a context
    Inject {
        /// File path (optional)
        #[arg(short, long)]
        file: Option<PathBuf>,
        /// Manual keys to include
        #[arg(short, long)]
        keys: Vec<String>,
    },
}

fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .init();

    let cli = Cli::parse();
    let project_root = std::env::current_dir()?;

    match cli.command {
        Commands::Init { interactive } => {
            driven::cli::InitCommand::run(&project_root, interactive)?;
        }
        Commands::Sync { watch } => {
            if watch {
                driven::cli::SyncCommand::watch(&project_root)?;
            } else {
                driven::cli::SyncCommand::run(&project_root)?;
            }
        }
        Commands::Convert {
            input,
            output,
            editor,
        } => {
            let editor = editor.and_then(|e| match e.to_lowercase().as_str() {
                "cursor" => Some(driven::Editor::Cursor),
                "copilot" => Some(driven::Editor::Copilot),
                "windsurf" => Some(driven::Editor::Windsurf),
                "claude" => Some(driven::Editor::Claude),
                "aider" => Some(driven::Editor::Aider),
                "cline" => Some(driven::Editor::Cline),
                _ => None,
            });
            driven::cli::ConvertCommand::run(&input, &output, editor)?;
        }
        Commands::Template { action } => match action {
            TemplateAction::List => {
                driven::cli::TemplateCommand::list()?;
            }
            TemplateAction::Search { query } => {
                driven::cli::TemplateCommand::search(&query)?;
            }
            TemplateAction::Apply { name } => {
                driven::cli::TemplateCommand::apply(&project_root, &name)?;
            }
        },
        Commands::Analyze { context, index } => {
            if context {
                let output = project_root.join(".driven/context.md");
                driven::cli::AnalyzeCommand::generate_context(&project_root, &output)?;
            } else if index {
                driven::cli::AnalyzeCommand::index(&project_root)?;
            } else {
                driven::cli::AnalyzeCommand::run(&project_root)?;
            }
        }
        Commands::Validate { file, strict } => {
            if strict {
                driven::cli::ValidateCommand::run_strict(&file)?;
            } else {
                driven::cli::ValidateCommand::run(&file)?;
            }
        }
        Commands::Hook { action } => match action {
            HookAction::List => {
                let hooks = driven::cli::HookCommand::list(&project_root)?;
                driven::cli::print_hooks_table(&hooks);
            }
            HookAction::Add {
                id,
                trigger,
                trigger_value,
                agent,
                message,
                name,
                workflow,
                condition,
                disabled,
            } => {
                driven::cli::HookCommand::add(
                    &project_root,
                    &id,
                    name.as_deref(),
                    &trigger,
                    &trigger_value,
                    &agent,
                    &message,
                    workflow.as_deref(),
                    condition.as_deref(),
                    !disabled,
                )?;
            }
            HookAction::Remove { id } => {
                driven::cli::HookCommand::remove(&project_root, &id)?;
            }
            HookAction::Trigger { command } => {
                driven::cli::HookCommand::trigger(&project_root, &command)?;
            }
            HookAction::Enable { id } => {
                driven::cli::HookCommand::enable(&project_root, &id)?;
            }
            HookAction::Disable { id } => {
                driven::cli::HookCommand::disable(&project_root, &id)?;
            }
            HookAction::Show { id } => {
                let hook = driven::cli::HookCommand::show(&project_root, &id)?;
                driven::cli::print_hook_details(&hook);
            }
        },
        Commands::Steer { action } => match action {
            SteerAction::List => {
                let rules = driven::cli::SteerCommand::list(&project_root)?;
                driven::cli::print_steering_table(&rules);
            }
            SteerAction::Add {
                id,
                inclusion,
                pattern,
                content,
                name,
                priority,
            } => {
                driven::cli::SteerCommand::add(
                    &project_root,
                    &id,
                    name.as_deref(),
                    &inclusion,
                    pattern.as_deref(),
                    &content,
                    priority,
                )?;
            }
            SteerAction::Remove { id } => {
                driven::cli::SteerCommand::remove(&project_root, &id)?;
            }
            SteerAction::Test { file, keys } => {
                let rules = driven::cli::SteerCommand::test(&project_root, &file, &keys)?;
                if rules.is_empty() {
                    println!("No steering rules apply to this file.");
                } else {
                    println!("Applicable steering rules:");
                    driven::cli::print_steering_table(&rules);
                }
            }
            SteerAction::Show { id } => {
                let rule = driven::cli::SteerCommand::show(&project_root, &id)?;
                driven::cli::print_steering_details(&rule);
            }
            SteerAction::Inject { file, keys } => {
                let content =
                    driven::cli::SteerCommand::inject(&project_root, file.as_deref(), &keys)?;
                println!("{}", content);
            }
        },
    }

    Ok(())
}
