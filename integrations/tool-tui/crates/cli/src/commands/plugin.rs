//! Plugin Command Arguments
//!
//! Clap argument definitions for plugin subcommands.

use clap::{Args, Subcommand, ValueEnum};
use std::path::PathBuf;

/// Plugin management commands
#[derive(Args)]
pub struct PluginArgs {
    #[command(subcommand)]
    pub command: PluginCommands,
}

#[derive(Subcommand)]
pub enum PluginCommands {
    /// List installed plugins
    #[command(visible_alias = "ls")]
    List {
        /// Show detailed info
        #[arg(short, long)]
        verbose: bool,
        /// Filter by type (wasm, native)
        #[arg(long)]
        plugin_type: Option<PluginTypeFilter>,
        /// Output format
        #[arg(short, long, default_value = "human")]
        format: OutputFormat,
    },

    /// Install a plugin
    #[command(visible_alias = "add")]
    Install {
        /// Plugin source (path, URL, or registry name)
        source: String,
        /// Force reinstall
        #[arg(long)]
        force: bool,
        /// Skip signature verification (dangerous)
        #[arg(long)]
        no_verify: bool,
    },

    /// Remove a plugin
    #[command(visible_alias = "rm")]
    Remove {
        /// Plugin name
        name: String,
        /// Force removal
        #[arg(long)]
        force: bool,
    },

    /// Update plugins
    Update {
        /// Specific plugin to update (all if not specified)
        name: Option<String>,
        /// Check for updates only
        #[arg(long)]
        check: bool,
    },

    /// Show plugin info
    Info {
        /// Plugin name
        name: String,
    },

    /// Run a plugin
    Run {
        /// Plugin name
        name: String,
        /// Arguments to pass to plugin
        #[arg(trailing_var_arg = true)]
        args: Vec<String>,
    },

    /// Create a new plugin from template
    Create {
        /// Plugin name
        name: String,
        /// Plugin type (wasm, native)
        #[arg(short = 't', long, default_value = "wasm")]
        plugin_type: PluginTypeFilter,
        /// Output directory
        #[arg(short, long)]
        output: Option<PathBuf>,
        /// Programming language (for WASM: rust, js, python, go)
        #[arg(short, long, default_value = "rust")]
        lang: String,
    },

    /// Build a plugin
    Build {
        /// Plugin directory (default: current)
        #[arg(index = 1)]
        path: Option<PathBuf>,
        /// Release mode
        #[arg(long)]
        release: bool,
        /// Optimize WASM output
        #[arg(long)]
        optimize: bool,
    },

    /// Publish a plugin to registry
    Publish {
        /// Plugin directory
        #[arg(index = 1)]
        path: Option<PathBuf>,
        /// Skip validation
        #[arg(long)]
        no_verify: bool,
    },

    /// Search plugin registry
    Search {
        /// Search query
        query: String,
        /// Limit results
        #[arg(short = 'n', long, default_value = "20")]
        limit: usize,
    },

    /// Enable a plugin
    Enable {
        /// Plugin name
        name: String,
    },

    /// Disable a plugin
    Disable {
        /// Plugin name
        name: String,
    },
}

#[derive(ValueEnum, Clone, Copy, Default)]
pub enum PluginTypeFilter {
    #[default]
    All,
    Wasm,
    Native,
}

#[derive(ValueEnum, Clone, Copy, Default)]
pub enum OutputFormat {
    #[default]
    Human,
    Json,
    Llm,
}
