//! Security Command Arguments
//!
//! Clap argument definitions for security subcommands.

use clap::{Args, Subcommand, ValueEnum};
use std::path::PathBuf;

/// Security and audit commands
#[derive(Args)]
pub struct SecurityArgs {
    #[command(subcommand)]
    pub command: SecurityCommands,
}

#[derive(Subcommand)]
pub enum SecurityCommands {
    /// Run security audit on the project
    #[command(visible_alias = "a")]
    Audit {
        /// Path to audit (default: current directory)
        #[arg(index = 1)]
        path: Option<PathBuf>,
        /// Output format
        #[arg(short, long, default_value = "human")]
        format: OutputFormat,
        /// Run deep scan (slower but more thorough)
        #[arg(long)]
        deep: bool,
        /// Check dependencies only
        #[arg(long)]
        deps_only: bool,
    },

    /// Manage secrets
    Secrets {
        #[command(subcommand)]
        command: SecretsSubcommands,
    },

    /// Manage permissions
    Permissions {
        #[command(subcommand)]
        command: PermissionsSubcommands,
    },

    /// View audit logs
    Logs {
        /// Number of entries to show
        #[arg(short = 'n', long, default_value = "50")]
        count: usize,
        /// Filter by action
        #[arg(long)]
        action: Option<String>,
        /// Export format
        #[arg(long)]
        export: Option<ExportFormat>,
        /// Output file
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Sandbox management
    Sandbox {
        #[command(subcommand)]
        command: SandboxSubcommands,
    },

    /// Set trust level for a context
    Trust {
        /// Context (project, plugin, channel)
        #[arg(index = 1)]
        context: String,
        /// Trust level (untrusted, basic, standard, extended, full)
        #[arg(index = 2)]
        level: String,
    },
}

#[derive(Subcommand)]
pub enum SecretsSubcommands {
    /// List stored secrets (names only)
    List,
    /// Set a secret
    Set {
        /// Secret name
        name: String,
        /// Secret value (prompt if not provided)
        value: Option<String>,
    },
    /// Remove a secret
    Remove {
        /// Secret name
        name: String,
    },
    /// Rotate master key
    Rotate {
        /// Force rotation
        #[arg(long)]
        force: bool,
    },
    /// Export secrets (encrypted)
    Export {
        /// Output file
        output: PathBuf,
    },
    /// Import secrets
    Import {
        /// Input file
        input: PathBuf,
    },
}

#[derive(Subcommand)]
pub enum PermissionsSubcommands {
    /// List current permissions
    List {
        /// Filter by context
        #[arg(long)]
        context: Option<String>,
    },
    /// Grant a permission
    Grant {
        /// Permission to grant
        permission: String,
        /// Context to grant to
        context: String,
    },
    /// Revoke a permission
    Revoke {
        /// Permission to revoke
        permission: String,
        /// Context to revoke from
        context: String,
    },
    /// Reset all permissions
    Reset {
        /// Force reset
        #[arg(long)]
        force: bool,
    },
}

#[derive(Subcommand)]
pub enum SandboxSubcommands {
    /// Show sandbox status
    Status,
    /// List running sandboxes
    List,
    /// Configure sandbox limits
    Config {
        /// Memory limit (e.g., 256MB)
        #[arg(long)]
        memory: Option<String>,
        /// CPU time limit (e.g., 30s)
        #[arg(long)]
        cpu: Option<String>,
        /// Network access
        #[arg(long)]
        network: Option<bool>,
        /// File system access
        #[arg(long)]
        filesystem: Option<bool>,
    },
    /// Stop a running sandbox
    Stop {
        /// Sandbox ID
        id: String,
    },
}

#[derive(ValueEnum, Clone, Copy, Default)]
pub enum OutputFormat {
    #[default]
    Human,
    Json,
    Llm,
}

#[derive(ValueEnum, Clone, Copy)]
pub enum ExportFormat {
    Json,
    Csv,
    Llm,
}
