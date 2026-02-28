//! File Tree Component
//!
//! A high-performance file tree with lazy directory loading, Git status
//! indicators, and keyboard navigation.
//!
//! # Features
//!
//! - Lazy directory loading (only loads when expanded)
//! - Git status indicators (M, A, D, ?)
//! - Nerd Font icons for file types
//! - Vim-style keyboard navigation
//! - Zero-copy path handling with indices
//!
//! # Example
//!
//! ```rust,ignore
//! use dx_cli::ui::editor::FileTree;
//!
//! let mut tree = FileTree::new("/path/to/project")?;
//! tree.expand_selected();
//! ```

use std::any::Any;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::ui::components::traits::{
    Bounds, ComponentResult, DxComponent, KeyEvent, MouseEvent, RenderContext,
};
use crate::ui::theme::DxTheme;

/// Git status for a file entry
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GitStatus {
    /// File is not tracked by git
    #[default]
    Untracked,
    /// File is unmodified
    Clean,
    /// File has been modified
    Modified,
    /// File has been added/staged
    Added,
    /// File has been deleted
    Deleted,
    /// File has been renamed
    Renamed,
    /// File has conflicts
    Conflict,
    /// File is ignored
    Ignored,
}

impl GitStatus {
    /// Get the indicator character for this status
    #[inline]
    pub const fn indicator(&self) -> char {
        match self {
            Self::Untracked => '?',
            Self::Clean => ' ',
            Self::Modified => 'M',
            Self::Added => 'A',
            Self::Deleted => 'D',
            Self::Renamed => 'R',
            Self::Conflict => 'C',
            Self::Ignored => '!',
        }
    }

    /// Get the color index for this status (maps to theme)
    #[inline]
    pub const fn color_index(&self) -> u8 {
        match self {
            Self::Untracked => 3, // yellow
            Self::Clean => 7,    // white
            Self::Modified => 3, // yellow
            Self::Added => 2,    // green
            Self::Deleted => 1,  // red
            Self::Renamed => 6,  // cyan
            Self::Conflict => 1, // red
            Self::Ignored => 8,  // bright black
        }
    }
}

/// Kind of file entry
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileEntryKind {
    /// Directory
    Directory,
    /// Regular file
    File,
    /// Symbolic link
    Symlink,
}

/// A single entry in the file tree
#[derive(Debug, Clone)]
pub struct FileEntry {
    /// File name (not full path)
    pub name: String,
    /// Entry kind
    pub kind: FileEntryKind,
    /// Git status
    pub git_status: GitStatus,
    /// Depth in tree (0 = root)
    pub depth: u16,
    /// Whether this directory is expanded
    pub expanded: bool,
    /// Whether children have been loaded
    pub loaded: bool,
    /// Index of parent entry (u32::MAX = root)
    pub parent_idx: u32,
    /// Number of direct children (when loaded)
    pub child_count: u32,
    /// Index of first child in entries vec
    pub first_child_idx: u32,
}

impl FileEntry {
    /// Get the Nerd Font icon for this entry
    pub fn icon(&self) -> &'static str {
        match self.kind {
            FileEntryKind::Directory => {
                if self.expanded {
                    ""  // nf-fa-folder_open
                } else {
                    ""  // nf-fa-folder
                }
            }
            FileEntryKind::Symlink => "",  // nf-fa-link
            FileEntryKind::File => self.file_icon(),
        }
    }

    /// Get file-type specific icon based on extension
    fn file_icon(&self) -> &'static str {
        let ext = self.name.rsplit('.').next().unwrap_or("");
        match ext.to_lowercase().as_str() {
            // Rust
            "rs" => "",  // nf-dev-rust
            // JavaScript/TypeScript
            "js" => "",  // nf-dev-javascript
            "jsx" => "",
            "ts" => "",  // nf-seti-typescript
            "tsx" => "",
            // Web
            "html" | "htm" => "",  // nf-fa-html5
            "css" => "",  // nf-dev-css3
            "scss" | "sass" => "",
            // Data
            "json" => "",  // nf-seti-json
            "yaml" | "yml" => "",
            "toml" => "",
            "xml" => "",
            "sr" => "",  // DX Serializer format
            // Config
            "md" | "markdown" => "",  // nf-dev-markdown
            "txt" => "",  // nf-fa-file_text
            "gitignore" | "dockerignore" => "",
            // Images
            "png" | "jpg" | "jpeg" | "gif" | "svg" | "webp" => "",  // nf-fa-file_image
            // Binary/compiled
            "wasm" => "",
            "exe" | "dll" | "so" | "dylib" => "",
            // Archives
            "zip" | "tar" | "gz" | "bz2" | "xz" => "",  // nf-fa-file_archive
            // Lock files
            "lock" => "",  // nf-fa-lock
            // Default
            _ => "",  // nf-fa-file
        }
    }
}

/// File tree component with lazy loading
pub struct FileTree {
    /// Root directory path
    root: PathBuf,
    /// All entries (flat list with tree structure via indices)
    entries: Vec<FileEntry>,
    /// Currently selected entry index
    selected: u32,
    /// Scroll offset for display
    scroll_offset: u32,
    /// Visible height in rows
    visible_height: u16,
    /// Whether the component is focused
    focused: bool,
    /// Component bounds
    bounds: Bounds,
    /// Git status cache (path hash -> status)
    git_cache: HashMap<u64, GitStatus>,
    /// Whether git status is available
    git_available: bool,
}

impl FileTree {
    /// Create a new file tree rooted at the given path
    ///
    /// # Errors
    ///
    /// Returns an error if the path doesn't exist or isn't a directory
    pub fn new<P: AsRef<Path>>(root: P) -> std::io::Result<Self> {
        let root = root.as_ref().to_path_buf();
        if !root.is_dir() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotADirectory,
                "Root path must be a directory",
            ));
        }

        let mut tree = Self {
            root: root.clone(),
            entries: Vec::with_capacity(256),
            selected: 0,
            scroll_offset: 0,
            visible_height: 20,
            focused: false,
            bounds: Bounds::default(),
            git_cache: HashMap::new(),
            git_available: Self::detect_git(&root),
        };

        // Add root entry
        let root_name = root
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(".")
            .to_string();

        tree.entries.push(FileEntry {
            name: root_name,
            kind: FileEntryKind::Directory,
            git_status: GitStatus::Clean,
            depth: 0,
            expanded: true,
            loaded: false,
            parent_idx: u32::MAX,
            child_count: 0,
            first_child_idx: 1,
        });

        // Load root children
        tree.load_children(0)?;

        Ok(tree)
    }

    /// Detect if git is available in the directory
    fn detect_git(path: &Path) -> bool {
        let mut current = Some(path);
        while let Some(dir) = current {
            if dir.join(".git").exists() {
                return true;
            }
            current = dir.parent();
        }
        false
    }

    /// Load children for a directory entry
    fn load_children(&mut self, parent_idx: u32) -> std::io::Result<()> {
        let parent_depth = self.entries[parent_idx as usize].depth;
        let parent_path = self.get_full_path(parent_idx);

        // Read directory entries
        let mut children: Vec<_> = fs::read_dir(&parent_path)?
            .filter_map(|e| e.ok())
            .filter(|e| {
                // Skip hidden files unless they're important
                let name = e.file_name();
                let name_str = name.to_string_lossy();
                !name_str.starts_with('.')
                    || name_str == ".git"
                    || name_str == ".gitignore"
                    || name_str == ".dx"
            })
            .collect();

        // Sort: directories first, then alphabetically
        children.sort_by(|a, b| {
            let a_is_dir = a.file_type().map(|t| t.is_dir()).unwrap_or(false);
            let b_is_dir = b.file_type().map(|t| t.is_dir()).unwrap_or(false);
            match (a_is_dir, b_is_dir) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => a.file_name().cmp(&b.file_name()),
            }
        });

        let first_child_idx = self.entries.len() as u32;
        let child_count = children.len() as u32;

        // Update parent entry
        self.entries[parent_idx as usize].loaded = true;
        self.entries[parent_idx as usize].child_count = child_count;
        self.entries[parent_idx as usize].first_child_idx = first_child_idx;

        // Add children
        for entry in children {
            let name = entry.file_name().to_string_lossy().to_string();
            let file_type = entry.file_type().ok();
            let kind = match file_type {
                Some(ft) if ft.is_dir() => FileEntryKind::Directory,
                Some(ft) if ft.is_symlink() => FileEntryKind::Symlink,
                _ => FileEntryKind::File,
            };

            let git_status = if self.git_available {
                self.get_git_status(&entry.path())
            } else {
                GitStatus::Clean
            };

            self.entries.push(FileEntry {
                name,
                kind,
                git_status,
                depth: parent_depth + 1,
                expanded: false,
                loaded: false,
                parent_idx,
                child_count: 0,
                first_child_idx: 0,
            });
        }

        Ok(())
    }

    /// Get the full path for an entry by walking up the tree
    pub fn get_full_path(&self, idx: u32) -> PathBuf {
        let mut parts = Vec::new();
        let mut current = idx;

        while current != u32::MAX {
            parts.push(&self.entries[current as usize].name);
            current = self.entries[current as usize].parent_idx;
        }

        parts.reverse();

        // Skip root entry name since we have the full root path
        let mut path = self.root.clone();
        for part in parts.iter().skip(1) {
            path.push(part);
        }
        path
    }

    /// Get git status for a path (cached)
    fn get_git_status(&mut self, path: &Path) -> GitStatus {
        // Simple hash for caching
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        path.hash(&mut hasher);
        let hash = hasher.finish();

        if let Some(&status) = self.git_cache.get(&hash) {
            return status;
        }

        // For now, return clean - full git integration would run `git status`
        // In production, this would use git2 or spawn git status
        let status = GitStatus::Clean;
        self.git_cache.insert(hash, status);
        status
    }

    /// Expand the selected directory
    pub fn expand_selected(&mut self) {
        let idx = self.selected;
        if idx >= self.entries.len() as u32 {
            return;
        }

        let entry = &self.entries[idx as usize];
        if entry.kind != FileEntryKind::Directory {
            return;
        }

        if !entry.loaded {
            let _ = self.load_children(idx);
        }

        self.entries[idx as usize].expanded = true;
    }

    /// Collapse the selected directory
    pub fn collapse_selected(&mut self) {
        let idx = self.selected;
        if idx >= self.entries.len() as u32 {
            return;
        }

        if self.entries[idx as usize].kind == FileEntryKind::Directory {
            self.entries[idx as usize].expanded = false;
        } else {
            // If on a file, collapse parent
            let parent_idx = self.entries[idx as usize].parent_idx;
            if parent_idx != u32::MAX {
                self.entries[parent_idx as usize].expanded = false;
                self.selected = parent_idx;
            }
        }
    }

    /// Get visible entries (respecting collapsed directories)
    fn visible_entries(&self) -> Vec<u32> {
        let mut visible = Vec::with_capacity(self.entries.len());
        self.collect_visible(0, &mut visible);
        visible
    }

    fn collect_visible(&self, idx: u32, visible: &mut Vec<u32>) {
        if idx >= self.entries.len() as u32 {
            return;
        }

        visible.push(idx);

        let entry = &self.entries[idx as usize];
        if entry.kind == FileEntryKind::Directory && entry.expanded && entry.loaded {
            for i in 0..entry.child_count {
                self.collect_visible(entry.first_child_idx + i, visible);
            }
        }
    }

    /// Move selection up
    pub fn move_up(&mut self) {
        let visible = self.visible_entries();
        if let Some(pos) = visible.iter().position(|&i| i == self.selected) {
            if pos > 0 {
                self.selected = visible[pos - 1];
                self.ensure_visible();
            }
        }
    }

    /// Move selection down
    pub fn move_down(&mut self) {
        let visible = self.visible_entries();
        if let Some(pos) = visible.iter().position(|&i| i == self.selected) {
            if pos + 1 < visible.len() {
                self.selected = visible[pos + 1];
                self.ensure_visible();
            }
        }
    }

    /// Ensure selected item is visible
    fn ensure_visible(&mut self) {
        let visible = self.visible_entries();
        if let Some(pos) = visible.iter().position(|&i| i == self.selected) {
            let pos = pos as u32;
            if pos < self.scroll_offset {
                self.scroll_offset = pos;
            } else if pos >= self.scroll_offset + self.visible_height as u32 {
                self.scroll_offset = pos - self.visible_height as u32 + 1;
            }
        }
    }

    /// Get the currently selected entry
    pub fn selected_entry(&self) -> Option<&FileEntry> {
        self.entries.get(self.selected as usize)
    }

    /// Get the full path of the selected entry
    pub fn selected_path(&self) -> PathBuf {
        self.get_full_path(self.selected)
    }

    /// Refresh git status
    pub fn refresh_git_status(&mut self) {
        self.git_cache.clear();
        self.git_available = Self::detect_git(&self.root);
    }
}

impl DxComponent for FileTree {
    fn handle_key(&mut self, key: KeyEvent) -> ComponentResult {
        match key {
            // Vim navigation
            KeyEvent::Char('j') | KeyEvent::Down => {
                self.move_down();
                ComponentResult::Consumed
            }
            KeyEvent::Char('k') | KeyEvent::Up => {
                self.move_up();
                ComponentResult::Consumed
            }
            KeyEvent::Char('l') | KeyEvent::Right | KeyEvent::Enter => {
                let entry = &self.entries[self.selected as usize];
                if entry.kind == FileEntryKind::Directory {
                    if entry.expanded {
                        // Move into directory
                        if entry.child_count > 0 {
                            self.selected = entry.first_child_idx;
                            self.ensure_visible();
                        }
                    } else {
                        self.expand_selected();
                    }
                    ComponentResult::Consumed
                } else {
                    // Open file
                    let path = self.selected_path();
                    ComponentResult::ActionWithData(
                        "open_file".to_string(),
                        path.display().to_string(),
                    )
                }
            }
            KeyEvent::Char('h') | KeyEvent::Left => {
                self.collapse_selected();
                ComponentResult::Consumed
            }
            KeyEvent::Char('g') => {
                // Go to top
                self.selected = 0;
                self.scroll_offset = 0;
                ComponentResult::Consumed
            }
            KeyEvent::Char('G') => {
                // Go to bottom
                let visible = self.visible_entries();
                if !visible.is_empty() {
                    self.selected = *visible.last().unwrap();
                    self.ensure_visible();
                }
                ComponentResult::Consumed
            }
            KeyEvent::Escape => ComponentResult::FocusNext,
            KeyEvent::Tab => ComponentResult::FocusNext,
            KeyEvent::BackTab => ComponentResult::FocusPrev,
            _ => ComponentResult::Ignored,
        }
    }

    fn handle_mouse(&mut self, event: MouseEvent) -> ComponentResult {
        match event {
            MouseEvent::Click { x: _, y } => {
                let relative_y = y.saturating_sub(self.bounds.y);
                let visible = self.visible_entries();
                let idx = self.scroll_offset as usize + relative_y as usize;
                if idx < visible.len() {
                    self.selected = visible[idx];
                    ComponentResult::Consumed
                } else {
                    ComponentResult::Ignored
                }
            }
            MouseEvent::DoubleClick { x: _, y } => {
                let relative_y = y.saturating_sub(self.bounds.y);
                let visible = self.visible_entries();
                let idx = self.scroll_offset as usize + relative_y as usize;
                if idx < visible.len() {
                    self.selected = visible[idx];
                    let entry = &self.entries[self.selected as usize];
                    if entry.kind == FileEntryKind::Directory {
                        if entry.expanded {
                            self.collapse_selected();
                        } else {
                            self.expand_selected();
                        }
                        ComponentResult::Consumed
                    } else {
                        let path = self.selected_path();
                        ComponentResult::ActionWithData(
                            "open_file".to_string(),
                            path.display().to_string(),
                        )
                    }
                } else {
                    ComponentResult::Ignored
                }
            }
            MouseEvent::ScrollUp { .. } => {
                self.scroll_offset = self.scroll_offset.saturating_sub(3);
                ComponentResult::Consumed
            }
            MouseEvent::ScrollDown { .. } => {
                let visible = self.visible_entries();
                let max_offset = visible.len().saturating_sub(self.visible_height as usize) as u32;
                self.scroll_offset = (self.scroll_offset + 3).min(max_offset);
                ComponentResult::Consumed
            }
            _ => ComponentResult::Ignored,
        }
    }

    fn render(&self, ctx: &RenderContext<'_>) -> Vec<String> {
        use owo_colors::OwoColorize;

        let visible = self.visible_entries();
        let mut lines = Vec::with_capacity(ctx.bounds.height as usize);

        let start = self.scroll_offset as usize;
        let end = (start + ctx.bounds.height as usize).min(visible.len());

        for idx in start..end {
            let entry_idx = visible[idx];
            let entry = &self.entries[entry_idx as usize];
            let is_selected = entry_idx == self.selected;

            // Build the line
            let indent = "  ".repeat(entry.depth as usize);
            let icon = entry.icon();
            let git_indicator = if entry.git_status != GitStatus::Clean {
                format!(" {}", entry.git_status.indicator())
            } else {
                String::new()
            };

            let line = format!("{}{} {}{}", indent, icon, entry.name, git_indicator);

            // Apply styling
            let styled_line = if is_selected && ctx.focused {
                format!("{}", line.on_bright_blue().white().bold())
            } else if is_selected {
                format!("{}", line.on_bright_black().white())
            } else {
                match entry.kind {
                    FileEntryKind::Directory => format!("{}", line.bright_blue().bold()),
                    FileEntryKind::Symlink => format!("{}", line.cyan()),
                    FileEntryKind::File => match entry.git_status {
                        GitStatus::Modified => format!("{}", line.yellow()),
                        GitStatus::Added => format!("{}", line.green()),
                        GitStatus::Deleted => format!("{}", line.red()),
                        GitStatus::Untracked => format!("{}", line.bright_black()),
                        _ => line,
                    },
                }
            };

            lines.push(styled_line);
        }

        // Pad to full height
        while lines.len() < ctx.bounds.height as usize {
            lines.push(String::new());
        }

        lines
    }

    fn is_focusable(&self) -> bool {
        true
    }

    fn set_focused(&mut self, focused: bool) {
        self.focused = focused;
    }

    fn is_focused(&self) -> bool {
        self.focused
    }

    fn id(&self) -> Option<&str> {
        Some("file_tree")
    }

    fn bounds(&self) -> Bounds {
        self.bounds
    }

    fn set_bounds(&mut self, bounds: Bounds) {
        self.bounds = bounds;
        self.visible_height = bounds.height;
    }

    fn min_size(&self) -> (u16, u16) {
        (20, 5)
    }

    fn preferred_size(&self) -> (u16, u16) {
        (30, 20)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{self, File};
    use tempfile::TempDir;

    fn create_test_tree() -> (TempDir, FileTree) {
        let temp = TempDir::new().unwrap();
        let root = temp.path();

        // Create directory structure
        fs::create_dir_all(root.join("src")).unwrap();
        fs::create_dir_all(root.join("tests")).unwrap();
        File::create(root.join("Cargo.toml")).unwrap();
        File::create(root.join("src/main.rs")).unwrap();
        File::create(root.join("src/lib.rs")).unwrap();
        File::create(root.join("tests/test.rs")).unwrap();

        let tree = FileTree::new(root).unwrap();
        (temp, tree)
    }

    #[test]
    fn test_tree_creation() {
        let (_temp, tree) = create_test_tree();
        assert!(!tree.entries.is_empty());
        assert_eq!(tree.entries[0].kind, FileEntryKind::Directory);
    }

    #[test]
    fn test_navigation() {
        let (_temp, mut tree) = create_test_tree();

        tree.move_down();
        assert!(tree.selected > 0);

        tree.move_up();
        assert_eq!(tree.selected, 0);
    }

    #[test]
    fn test_expand_collapse() {
        let (_temp, mut tree) = create_test_tree();

        // Find src directory
        let src_idx = tree
            .entries
            .iter()
            .position(|e| e.name == "src")
            .map(|i| i as u32);

        if let Some(idx) = src_idx {
            tree.selected = idx;
            tree.expand_selected();
            assert!(tree.entries[idx as usize].expanded);

            tree.collapse_selected();
            assert!(!tree.entries[idx as usize].expanded);
        }
    }

    #[test]
    fn test_git_status_indicator() {
        assert_eq!(GitStatus::Modified.indicator(), 'M');
        assert_eq!(GitStatus::Added.indicator(), 'A');
        assert_eq!(GitStatus::Deleted.indicator(), 'D');
        assert_eq!(GitStatus::Untracked.indicator(), '?');
    }

    #[test]
    fn test_file_icons() {
        let entry = FileEntry {
            name: "main.rs".to_string(),
            kind: FileEntryKind::File,
            git_status: GitStatus::Clean,
            depth: 0,
            expanded: false,
            loaded: false,
            parent_idx: u32::MAX,
            child_count: 0,
            first_child_idx: 0,
        };
        assert_eq!(entry.icon(), "");

        let dir_entry = FileEntry {
            name: "src".to_string(),
            kind: FileEntryKind::Directory,
            git_status: GitStatus::Clean,
            depth: 0,
            expanded: false,
            loaded: false,
            parent_idx: u32::MAX,
            child_count: 0,
            first_child_idx: 0,
        };
        assert_eq!(dir_entry.icon(), "");
    }
}
