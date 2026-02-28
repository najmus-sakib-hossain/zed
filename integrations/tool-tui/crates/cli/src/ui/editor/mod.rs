//! Code Editor Module
//!
//! This module provides a complete TUI-based code editor with:
//! - File tree navigation with lazy loading and Git status
//! - Syntax highlighting (100+ languages via syntect)
//! - Vim/Emacs/Arrow key bindings with full motion support
//! - Line numbers (absolute, relative, hybrid)
//! - Search with regex support
//! - Minimap preview with viewport indicator
//! - Split pane editing
//!
//! # Architecture
//!
//! The editor is composed of several sub-components:
//! - [`FileTree`](tree::FileTree) - Directory tree navigation with Git integration
//! - [`CodeViewer`](viewer::CodeViewer) - Syntax-highlighted code display
//! - [`Minimap`](minimap::Minimap) - File overview with viewport indicator
//! - [`EditorKeybindings`](keybindings::EditorKeybindings) - Configurable key mappings
//! - [`EditorLayout`](layout::EditorLayout) - Main layout combining all components
//!
//! # Example
//!
//! ```rust,ignore
//! use dx_cli::ui::editor::{FileTree, CodeViewer, EditorLayout};
//!
//! // Create file tree for current directory
//! let tree = FileTree::new("./src")?;
//!
//! // Create code viewer
//! let viewer = CodeViewer::new()
//!     .line_numbers(LineNumberMode::Relative)
//!     .theme("Dracula");
//!
//! // Compose into editor layout with minimap
//! let editor = EditorLayout::new()
//!     .file_tree(tree)
//!     .viewer(viewer);
//! ```

pub mod keybindings;
pub mod layout;
pub mod minimap;
pub mod search;
pub mod tree;
pub mod viewer;

// Re-export keybinding types
pub use keybindings::{
    EditorKeybindings, EditorMode, KeyAction, KeybindingConfig, KeybindingMode,
    PendingOperator, VimMotion,
};

// Re-export layout types
pub use layout::{EditorLayout, EditorPane, SplitOrientation};

// Re-export minimap types
pub use minimap::{Minimap, MinimapConfig};

// Re-export search types
pub use search::{SearchDirection, SearchEngine, SearchMatch, SearchMode};

// Re-export tree types
pub use tree::{FileEntry, FileEntryKind, FileTree, GitStatus};

// Re-export viewer types
pub use viewer::{CodeViewer, LineNumberMode, ViewerConfig};

// Legacy aliases for backward compatibility
pub type Keybindings = EditorKeybindings;
pub type FileTreeItem = FileEntry;
pub type FileTreeConfig = ViewerConfig;
