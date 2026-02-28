//! Editor Layout Component
//!
//! Combines file tree, code viewer, and minimap into a unified editor interface.

use std::any::Any;
use std::path::PathBuf;

use crate::ui::components::{
    Bounds, ComponentResult, DxComponent, KeyEvent, MouseEvent, RenderContext,
};
use crate::ui::editor::{
    CodeViewer, EditorKeybindings as Keybindings, FileTree, Minimap, MinimapConfig,
};

/// Split pane orientation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SplitOrientation {
    /// Horizontal split (panes side by side)
    Horizontal,
    /// Vertical split (panes stacked)
    Vertical,
}

/// A pane in the editor (either a viewer or a split)
#[derive(Debug)]
pub enum EditorPane {
    /// Single code viewer
    Viewer(CodeViewer),
    /// Split into two panes
    Split {
        orientation: SplitOrientation,
        first: Box<EditorPane>,
        second: Box<EditorPane>,
        /// Position of the split (0.0 to 1.0)
        split_pos: f32,
    },
}

impl EditorPane {
    /// Create a new viewer pane
    pub fn viewer(viewer: CodeViewer) -> Self {
        EditorPane::Viewer(viewer)
    }

    /// Split this pane horizontally
    pub fn split_horizontal(self, new_viewer: CodeViewer) -> Self {
        EditorPane::Split {
            orientation: SplitOrientation::Horizontal,
            first: Box::new(self),
            second: Box::new(EditorPane::Viewer(new_viewer)),
            split_pos: 0.5,
        }
    }

    /// Split this pane vertically
    pub fn split_vertical(self, new_viewer: CodeViewer) -> Self {
        EditorPane::Split {
            orientation: SplitOrientation::Vertical,
            first: Box::new(self),
            second: Box::new(EditorPane::Viewer(new_viewer)),
            split_pos: 0.5,
        }
    }
}

/// Main editor layout combining file tree and code viewers
pub struct EditorLayout {
    /// File tree (left sidebar)
    file_tree: Option<FileTree>,
    /// Main editor pane(s)
    pane: EditorPane,
    /// Minimap (right sidebar)
    minimap: Option<Minimap>,
    /// Active pane index
    active_pane: usize,
    /// Show file tree
    show_tree: bool,
    /// Show minimap
    show_minimap: bool,
    /// File tree width (percentage)
    tree_width: f32,
    /// Minimap width (characters)
    minimap_width: u16,
    /// Current keybindings
    keybindings: Keybindings,
    /// Component bounds
    bounds: Bounds,
    /// Component ID
    id: Option<String>,
    /// Whether focused
    focused: bool,
    /// Focus target (0 = tree, 1+ = panes)
    focus_target: usize,
}

impl Default for EditorLayout {
    fn default() -> Self {
        Self::new()
    }
}

impl EditorLayout {
    /// Create a new editor layout
    pub fn new() -> Self {
        Self {
            file_tree: None,
            pane: EditorPane::Viewer(CodeViewer::new()),
            minimap: Some(Minimap::new()),
            active_pane: 0,
            show_tree: true,
            show_minimap: true,
            tree_width: 0.25,
            minimap_width: 15,
            keybindings: Keybindings::default(),
            bounds: Bounds::default(),
            id: None,
            focused: false,
            focus_target: 0,
        }
    }

    /// Set the file tree
    pub fn file_tree(mut self, tree: FileTree) -> Self {
        self.file_tree = Some(tree);
        self
    }

    /// Set the main viewer
    pub fn viewer(mut self, viewer: CodeViewer) -> Self {
        self.pane = EditorPane::Viewer(viewer);
        self
    }

    /// Set the keybindings
    pub fn keybindings(mut self, keybindings: Keybindings) -> Self {
        self.keybindings = keybindings;
        self
    }

    /// Toggle file tree visibility
    pub fn toggle_tree(&mut self) {
        self.show_tree = !self.show_tree;
    }

    /// Toggle minimap visibility
    pub fn toggle_minimap(&mut self) {
        self.show_minimap = !self.show_minimap;
    }

    /// Set minimap configuration
    pub fn minimap_config(mut self, config: MinimapConfig) -> Self {
        if let Some(ref mut minimap) = self.minimap {
            *minimap = minimap.clone().config(config);
        }
        self
    }

    /// Set the component ID
    pub fn with_id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }

    /// Open a file in the active viewer
    pub fn open_file(&mut self, path: PathBuf) -> anyhow::Result<()> {
        if let EditorPane::Viewer(ref mut viewer) = self.pane {
            viewer.load_file(path)?;

            // Update minimap with new content
            if let Some(ref mut minimap) = self.minimap {
                let (start, end) = viewer.viewport_info();
                minimap.update_content(viewer.lines(), start, end, viewer.syntax_index());
            }
        }
        Ok(())
    }

    /// Move focus to next element
    fn focus_next(&mut self) {
        let max_targets = if self.show_tree && self.file_tree.is_some() {
            2 // tree + pane
        } else {
            1 // just pane
        };
        self.focus_target = (self.focus_target + 1) % max_targets;
    }

    /// Move focus to previous element
    fn focus_prev(&mut self) {
        let max_targets = if self.show_tree && self.file_tree.is_some() {
            2
        } else {
            1
        };
        self.focus_target = if self.focus_target == 0 {
            max_targets - 1
        } else {
            self.focus_target - 1
        };
    }
}

impl DxComponent for EditorLayout {
    fn handle_key(&mut self, key: KeyEvent) -> ComponentResult {
        // Global keybindings first
        match &key {
            KeyEvent::Ctrl('b') => {
                self.toggle_tree();
                return ComponentResult::Redraw;
            }
            KeyEvent::Ctrl('m') => {
                self.toggle_minimap();
                return ComponentResult::Redraw;
            }
            KeyEvent::Tab => {
                self.focus_next();
                return ComponentResult::Redraw;
            }
            KeyEvent::BackTab => {
                self.focus_prev();
                return ComponentResult::Redraw;
            }
            _ => {}
        }

        // Delegate to focused component
        if self.focus_target == 0 && self.show_tree {
            if let Some(ref mut tree) = self.file_tree {
                let result = tree.handle_key(key);
                if let ComponentResult::ActionWithData(action, path) = result {
                    if action == "select_file" {
                        if let Ok(path) = PathBuf::try_from(path.as_str()) {
                            let _ = self.open_file(path);
                        }
                    }
                    return ComponentResult::Redraw;
                }
                return result;
            }
        }

        // Forward to active pane
        if let EditorPane::Viewer(ref mut viewer) = self.pane {
            let result = viewer.handle_key(key);

            // Update minimap after viewer changes
            if let Some(ref mut minimap) = self.minimap {
                let (start, end) = viewer.viewport_info();
                minimap.update_content(viewer.lines(), start, end, viewer.syntax_index());
            }

            return result;
        }

        ComponentResult::Ignored
    }

    fn handle_mouse(&mut self, event: MouseEvent) -> ComponentResult {
        let (x, _y) = event.position();
        let tree_width = if self.show_tree && self.file_tree.is_some() {
            (self.bounds.width as f32 * self.tree_width) as u16
        } else {
            0
        };

        let minimap_width = if self.show_minimap && self.minimap.is_some() {
            self.minimap_width
        } else {
            0
        };

        let minimap_start = self.bounds.width.saturating_sub(minimap_width);

        // Click on minimap area
        if x >= minimap_start && self.show_minimap {
            if let Some(ref mut minimap) = self.minimap {
                let result = minimap.handle_mouse(event);
                if let ComponentResult::ActionWithData(action, data) = result {
                    if action == "jump_to_line" {
                        if let Ok(line) = data.parse::<usize>() {
                            if let EditorPane::Viewer(ref mut viewer) = self.pane {
                                viewer.jump_to_line(line);
                            }
                        }
                    }
                    return ComponentResult::Redraw;
                }
                return result;
            }
        }

        // Click on tree area
        if x < tree_width {
            self.focus_target = 0;
            if let Some(ref mut tree) = self.file_tree {
                return tree.handle_mouse(event);
            }
        } else {
            // Click on editor area
            self.focus_target = 1;
            if let EditorPane::Viewer(ref mut viewer) = self.pane {
                return viewer.handle_mouse(event);
            }
        }

        ComponentResult::Consumed
    }

    fn render(&self, ctx: &RenderContext<'_>) -> Vec<String> {
        let mut lines = Vec::new();
        let width = ctx.bounds.width as usize;
        let height = ctx.bounds.height as usize;

        let tree_width = if self.show_tree && self.file_tree.is_some() {
            (width as f32 * self.tree_width) as usize
        } else {
            0
        };

        let minimap_width = if self.show_minimap && self.minimap.is_some() {
            self.minimap_width as usize
        } else {
            0
        };

        let editor_width = width.saturating_sub(tree_width).saturating_sub(minimap_width);

        // Create contexts for sub-components
        let tree_ctx = RenderContext {
            bounds: Bounds::new(ctx.bounds.x, ctx.bounds.y, tree_width as u16, ctx.bounds.height),
            focused: self.focused && self.focus_target == 0,
            theme: ctx.theme,
            frame: ctx.frame,
        };

        let editor_ctx = RenderContext {
            bounds: Bounds::new(
                ctx.bounds.x + tree_width as u16,
                ctx.bounds.y,
                editor_width as u16,
                ctx.bounds.height,
            ),
            focused: self.focused && self.focus_target == 1,
            theme: ctx.theme,
            frame: ctx.frame,
        };

        let minimap_ctx = RenderContext {
            bounds: Bounds::new(
                ctx.bounds.x + tree_width as u16 + editor_width as u16,
                ctx.bounds.y,
                minimap_width as u16,
                ctx.bounds.height,
            ),
            focused: false,
            theme: ctx.theme,
            frame: ctx.frame,
        };

        // Render tree, editor, and minimap
        let tree_lines = if self.show_tree {
            self.file_tree
                .as_ref()
                .map(|t| t.render(&tree_ctx))
                .unwrap_or_default()
        } else {
            Vec::new()
        };

        let editor_lines = if let EditorPane::Viewer(ref viewer) = self.pane {
            viewer.render(&editor_ctx)
        } else {
            Vec::new()
        };

        let minimap_lines = if self.show_minimap {
            self.minimap
                .as_ref()
                .map(|m| m.render(&minimap_ctx))
                .unwrap_or_default()
        } else {
            Vec::new()
        };

        // Combine lines horizontally
        for i in 0..height {
            let tree_line = tree_lines.get(i).map(|s| s.as_str()).unwrap_or("");
            let editor_line = editor_lines.get(i).map(|s| s.as_str()).unwrap_or("");
            let minimap_line = minimap_lines.get(i).map(|s| s.as_str()).unwrap_or("");

            let mut combined = String::new();

            if self.show_tree && tree_width > 0 {
                combined.push_str(&format!(
                    "{:tree_width$}│",
                    tree_line,
                    tree_width = tree_width.saturating_sub(1)
                ));
            }

            combined.push_str(&format!(
                "{:editor_width$}",
                editor_line,
                editor_width = editor_width
            ));

            if self.show_minimap && minimap_width > 0 {
                combined.push_str(minimap_line);
            }

            lines.push(combined);
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
        self.id.as_deref()
    }

    fn bounds(&self) -> Bounds {
        self.bounds
    }

    fn set_bounds(&mut self, bounds: Bounds) {
        self.bounds = bounds;
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

    #[test]
    fn test_editor_layout_creation() {
        let editor = EditorLayout::new();
        assert!(editor.show_tree);
        assert!(editor.show_minimap);
        assert_eq!(editor.focus_target, 0);
    }

    #[test]
    fn test_toggle_tree() {
        let mut editor = EditorLayout::new();
        assert!(editor.show_tree);
        editor.toggle_tree();
        assert!(!editor.show_tree);
        editor.toggle_tree();
        assert!(editor.show_tree);
    }

    #[test]
    fn test_toggle_minimap() {
        let mut editor = EditorLayout::new();
        assert!(editor.show_minimap);
        editor.toggle_minimap();
        assert!(!editor.show_minimap);
        editor.toggle_minimap();
        assert!(editor.show_minimap);
    }

    #[test]
    fn test_split_pane() {
        let viewer = CodeViewer::new();
        let pane = EditorPane::viewer(viewer);

        let split = pane.split_horizontal(CodeViewer::new());
        if let EditorPane::Split { orientation, .. } = split {
            assert_eq!(orientation, SplitOrientation::Horizontal);
        } else {
            panic!("Expected split pane");
        }
    }
}
