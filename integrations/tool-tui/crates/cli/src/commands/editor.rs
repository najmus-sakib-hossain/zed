//! Editor Command Arguments
//!
//! Clap argument definitions for the built-in code editor.

use clap::{Args, Subcommand, ValueEnum};
use std::path::PathBuf;

/// Built-in code editor commands
#[derive(Args)]
pub struct EditorArgs {
    #[command(subcommand)]
    pub command: Option<EditorCommands>,

    /// File or directory to open
    #[arg(index = 1)]
    pub path: Option<PathBuf>,
}

#[derive(Subcommand)]
pub enum EditorCommands {
    /// Open file(s) in the editor
    Open {
        /// Files to open
        #[arg(index = 1)]
        files: Vec<PathBuf>,
        /// Open at specific line
        #[arg(short, long)]
        line: Option<usize>,
        /// Open at specific column
        #[arg(short, long)]
        column: Option<usize>,
        /// Open in read-only mode
        #[arg(long)]
        readonly: bool,
    },

    /// Show file tree
    Tree {
        /// Root directory
        #[arg(index = 1)]
        path: Option<PathBuf>,
        /// Show hidden files
        #[arg(short, long)]
        all: bool,
        /// Max depth
        #[arg(short, long)]
        depth: Option<usize>,
    },

    /// Configure editor settings
    Config {
        /// Set a config value (key=value)
        #[arg(long)]
        set: Option<String>,
        /// Get a config value
        #[arg(long)]
        get: Option<String>,
        /// Show all config
        #[arg(long)]
        show: bool,
        /// Reset to defaults
        #[arg(long)]
        reset: bool,
    },

    /// Keybinding management
    Keys {
        /// Show keybinding for action
        #[arg(long)]
        action: Option<String>,
        /// List all keybindings
        #[arg(long)]
        list: bool,
        /// Keybinding preset (vim, emacs, vscode, default)
        #[arg(long)]
        preset: Option<KeyPreset>,
    },

    /// Syntax highlighting
    Syntax {
        /// File to show
        file: PathBuf,
        /// Theme name
        #[arg(short, long)]
        theme: Option<String>,
        /// Start line
        #[arg(long)]
        start: Option<usize>,
        /// End line
        #[arg(long)]
        end: Option<usize>,
    },

    /// Search in files
    Search {
        /// Search pattern
        pattern: String,
        /// Directory to search
        #[arg(index = 2)]
        path: Option<PathBuf>,
        /// Case sensitive
        #[arg(short = 'i', long)]
        case_sensitive: bool,
        /// Use regex
        #[arg(short, long)]
        regex: bool,
        /// File pattern to include
        #[arg(short, long)]
        include: Option<String>,
        /// File pattern to exclude
        #[arg(short, long)]
        exclude: Option<String>,
    },

    /// Compare two files (diff)
    Diff {
        /// First file
        file1: PathBuf,
        /// Second file
        file2: PathBuf,
        /// Side-by-side view
        #[arg(long)]
        side_by_side: bool,
        /// Show only changes
        #[arg(long)]
        minimal: bool,
    },

    /// Minimap view
    Minimap {
        /// File to show
        file: PathBuf,
        /// Width of minimap
        #[arg(short, long, default_value = "20")]
        width: u16,
    },

    /// List supported languages
    Languages,

    /// List available themes
    Themes,

    /// Preview a file with syntax highlighting
    Preview {
        /// File to preview
        file: PathBuf,
        /// Pager mode (less-like navigation)
        #[arg(long)]
        pager: bool,
    },

    /// Start interactive session
    Interactive {
        /// Working directory
        #[arg(index = 1)]
        path: Option<PathBuf>,
    },
}

#[derive(Debug, ValueEnum, Clone, Copy, Default)]
pub enum KeyPreset {
    #[default]
    Default,
    Vim,
    Emacs,
    Vscode,
}
