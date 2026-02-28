//! Git Command Arguments
//!
//! Clap argument definitions for git subcommands.

use clap::{Args, Subcommand};
use std::path::PathBuf;

/// Git integration commands
#[derive(Args)]
pub struct GitArgs {
    #[command(subcommand)]
    pub command: GitCommands,
}

#[derive(Subcommand)]
pub enum GitCommands {
    /// Show interactive status with staging
    #[command(visible_alias = "s")]
    Status {
        /// Show short format
        #[arg(short, long)]
        short: bool,
        /// Show verbose output
        #[arg(short, long)]
        verbose: bool,
    },

    /// Commit staged changes
    #[command(visible_alias = "c")]
    Commit {
        /// Commit message
        #[arg(short, long)]
        message: Option<String>,
        /// Amend the last commit
        #[arg(long)]
        amend: bool,
        /// Add all tracked files
        #[arg(short, long)]
        all: bool,
    },

    /// Show diff between commits, branches, or files
    #[command(visible_alias = "d")]
    Diff {
        /// Compare against this ref
        #[arg(index = 1)]
        target: Option<String>,
        /// Show staged changes
        #[arg(long)]
        staged: bool,
        /// Side-by-side view
        #[arg(long)]
        side_by_side: bool,
        /// Show inline diff
        #[arg(long)]
        inline: bool,
    },

    /// Branch management
    #[command(visible_alias = "br")]
    Branch {
        #[command(subcommand)]
        command: Option<BranchSubcommands>,
    },

    /// Stash operations
    Stash {
        #[command(subcommand)]
        command: Option<StashSubcommands>,
    },

    /// Interactive staging (add files interactively)
    Add {
        /// Files to add (interactive if none)
        files: Vec<PathBuf>,
        /// Interactive patch mode
        #[arg(short, long)]
        patch: bool,
    },

    /// Show commit log
    Log {
        /// Number of commits to show
        #[arg(short = 'n', long, default_value = "10")]
        count: usize,
        /// One line per commit
        #[arg(long)]
        oneline: bool,
        /// Show graph
        #[arg(long)]
        graph: bool,
    },
}

#[derive(Subcommand)]
pub enum BranchSubcommands {
    /// List branches
    List {
        /// Show all branches (including remotes)
        #[arg(short, long)]
        all: bool,
        /// Show remote branches only
        #[arg(short, long)]
        remote: bool,
    },
    /// Create a new branch
    Create {
        /// Branch name
        name: String,
        /// Start point
        #[arg(index = 2)]
        start_point: Option<String>,
    },
    /// Switch to a branch
    Switch {
        /// Branch name
        name: String,
        /// Create branch if it doesn't exist
        #[arg(short, long)]
        create: bool,
    },
    /// Delete a branch
    Delete {
        /// Branch name
        name: String,
        /// Force delete
        #[arg(short, long)]
        force: bool,
    },
    /// Merge a branch
    Merge {
        /// Branch to merge
        branch: String,
        /// Don't create merge commit
        #[arg(long)]
        no_commit: bool,
    },
}

#[derive(Subcommand)]
pub enum StashSubcommands {
    /// Save changes to stash
    Save {
        /// Stash message
        message: Option<String>,
        /// Include untracked files
        #[arg(short, long)]
        include_untracked: bool,
    },
    /// List all stashes
    List,
    /// Pop the latest stash
    Pop {
        /// Stash index
        #[arg(index = 1)]
        stash: Option<usize>,
    },
    /// Apply a stash without removing it
    Apply {
        /// Stash index
        #[arg(index = 1)]
        stash: Option<usize>,
    },
    /// Drop a stash
    Drop {
        /// Stash index
        #[arg(index = 1)]
        stash: Option<usize>,
    },
    /// Show stash contents
    Show {
        /// Stash index
        #[arg(index = 1)]
        stash: Option<usize>,
    },
}
