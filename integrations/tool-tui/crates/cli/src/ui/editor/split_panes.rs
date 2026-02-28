//! Split panes implementation for the code editor
//!
//! Provides horizontal and vertical split pane layouts for
//! viewing multiple files simultaneously.

use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, StatefulWidget, Widget},
};
use std::sync::Arc;

/// Direction of split
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SplitDirection {
    Horizontal,
    Vertical,
}

/// A single pane in the split layout
#[derive(Debug, Clone)]
pub struct Pane {
    /// Unique pane identifier
    pub id: usize,
    /// File path being displayed (if any)
    pub file_path: Option<String>,
    /// Scroll position
    pub scroll: (u16, u16),
    /// Cursor position
    pub cursor: (usize, usize),
    /// Is this pane focused?
    pub focused: bool,
}

impl Pane {
    pub fn new(id: usize) -> Self {
        Self {
            id,
            file_path: None,
            scroll: (0, 0),
            cursor: (0, 0),
            focused: false,
        }
    }

    pub fn with_file(mut self, path: impl Into<String>) -> Self {
        self.file_path = Some(path.into());
        self
    }
}

/// Split pane node - either a leaf pane or a split containing children
#[derive(Debug, Clone)]
pub enum SplitNode {
    /// A leaf pane
    Pane(Pane),
    /// A split containing two children
    Split {
        direction: SplitDirection,
        /// Ratio of first child (0.0 to 1.0)
        ratio: f32,
        first: Box<SplitNode>,
        second: Box<SplitNode>,
    },
}

impl SplitNode {
    /// Create a new leaf pane
    pub fn pane(pane: Pane) -> Self {
        SplitNode::Pane(pane)
    }

    /// Create a horizontal split
    pub fn horizontal(first: SplitNode, second: SplitNode, ratio: f32) -> Self {
        SplitNode::Split {
            direction: SplitDirection::Horizontal,
            ratio,
            first: Box::new(first),
            second: Box::new(second),
        }
    }

    /// Create a vertical split
    pub fn vertical(first: SplitNode, second: SplitNode, ratio: f32) -> Self {
        SplitNode::Split {
            direction: SplitDirection::Vertical,
            ratio,
            first: Box::new(first),
            second: Box::new(second),
        }
    }

    /// Get all panes in this node
    pub fn panes(&self) -> Vec<&Pane> {
        match self {
            SplitNode::Pane(pane) => vec![pane],
            SplitNode::Split { first, second, .. } => {
                let mut panes = first.panes();
                panes.extend(second.panes());
                panes
            }
        }
    }

    /// Get all panes mutably
    pub fn panes_mut(&mut self) -> Vec<&mut Pane> {
        match self {
            SplitNode::Pane(pane) => vec![pane],
            SplitNode::Split { first, second, .. } => {
                let mut panes = first.panes_mut();
                panes.extend(second.panes_mut());
                panes
            }
        }
    }

    /// Find the focused pane
    pub fn focused_pane(&self) -> Option<&Pane> {
        self.panes().into_iter().find(|p| p.focused)
    }

    /// Find the focused pane mutably
    pub fn focused_pane_mut(&mut self) -> Option<&mut Pane> {
        self.panes_mut().into_iter().find(|p| p.focused)
    }

    /// Count total panes
    pub fn pane_count(&self) -> usize {
        match self {
            SplitNode::Pane(_) => 1,
            SplitNode::Split { first, second, .. } => {
                first.pane_count() + second.pane_count()
            }
        }
    }
}

/// Split pane layout manager
pub struct SplitPanes {
    /// Root node of the split tree
    root: SplitNode,
    /// Next pane ID to assign
    next_id: usize,
    /// Minimum pane size
    min_size: u16,
    /// Border style
    border_style: Style,
    /// Focused border style
    focused_border_style: Style,
    /// Show pane numbers
    show_numbers: bool,
}

impl SplitPanes {
    /// Create a new split panes layout with a single pane
    pub fn new() -> Self {
        Self {
            root: SplitNode::pane(Pane::new(0)),
            next_id: 1,
            min_size: 5,
            border_style: Style::default().fg(Color::DarkGray),
            focused_border_style: Style::default().fg(Color::Cyan),
            show_numbers: true,
        }
    }

    /// Set border style
    pub fn border_style(mut self, style: Style) -> Self {
        self.border_style = style;
        self
    }

    /// Set focused border style
    pub fn focused_border_style(mut self, style: Style) -> Self {
        self.focused_border_style = style;
        self
    }

    /// Show pane numbers
    pub fn show_numbers(mut self, show: bool) -> Self {
        self.show_numbers = show;
        self
    }

    /// Get the root node
    pub fn root(&self) -> &SplitNode {
        &self.root
    }

    /// Get the root node mutably
    pub fn root_mut(&mut self) -> &mut SplitNode {
        &mut self.root
    }

    /// Split the focused pane horizontally
    pub fn split_horizontal(&mut self) -> Option<usize> {
        self.split(SplitDirection::Horizontal)
    }

    /// Split the focused pane vertically
    pub fn split_vertical(&mut self) -> Option<usize> {
        self.split(SplitDirection::Vertical)
    }

    /// Split the focused pane in the given direction
    fn split(&mut self, direction: SplitDirection) -> Option<usize> {
        if let Some(focused) = self.find_focused_node_mut() {
            if let SplitNode::Pane(pane) = focused {
                let new_id = self.next_id;
                self.next_id += 1;

                let existing_pane = pane.clone();
                let new_pane = Pane::new(new_id);

                let new_node = match direction {
                    SplitDirection::Horizontal => {
                        SplitNode::horizontal(
                            SplitNode::pane(existing_pane),
                            SplitNode::pane(new_pane),
                            0.5,
                        )
                    }
                    SplitDirection::Vertical => {
                        SplitNode::vertical(
                            SplitNode::pane(existing_pane),
                            SplitNode::pane(new_pane),
                            0.5,
                        )
                    }
                };

                *focused = new_node;
                return Some(new_id);
            }
        }
        None
    }

    /// Find the focused node (for splitting)
    fn find_focused_node_mut(&mut self) -> Option<&mut SplitNode> {
        fn find_in_node(node: &mut SplitNode) -> Option<&mut SplitNode> {
            match node {
                SplitNode::Pane(pane) if pane.focused => Some(node),
                SplitNode::Split { first, second, .. } => {
                    find_in_node(first).or_else(|| find_in_node(second))
                }
                _ => None,
            }
        }
        find_in_node(&mut self.root)
    }

    /// Close the focused pane
    pub fn close_focused(&mut self) -> bool {
        // Don't close if only one pane
        if self.root.pane_count() <= 1 {
            return false;
        }

        fn close_in_node(node: &mut SplitNode) -> Option<SplitNode> {
            match node {
                SplitNode::Pane(pane) if pane.focused => {
                    // This pane should be removed
                    None
                }
                SplitNode::Split { first, second, .. } => {
                    if let SplitNode::Pane(p) = first.as_ref() {
                        if p.focused {
                            // Replace this split with the second child
                            return Some(second.as_ref().clone());
                        }
                    }
                    if let SplitNode::Pane(p) = second.as_ref() {
                        if p.focused {
                            // Replace this split with the first child
                            return Some(first.as_ref().clone());
                        }
                    }

                    // Recursively check children
                    if let Some(replacement) = close_in_node(first) {
                        *first = Box::new(replacement);
                    }
                    if let Some(replacement) = close_in_node(second) {
                        *second = Box::new(replacement);
                    }

                    None
                }
                _ => None,
            }
        }

        if let Some(replacement) = close_in_node(&mut self.root) {
            self.root = replacement;
            // Focus the first pane
            if let Some(pane) = self.root.panes_mut().first_mut() {
                pane.focused = true;
            }
            true
        } else {
            false
        }
    }

    /// Focus the next pane
    pub fn focus_next(&mut self) {
        let panes = self.root.panes_mut();
        let count = panes.len();
        if count == 0 {
            return;
        }

        let current = panes.iter().position(|p| p.focused).unwrap_or(0);
        let next = (current + 1) % count;

        // Unfocus all, then focus next
        for (i, pane) in panes.into_iter().enumerate() {
            pane.focused = i == next;
        }
    }

    /// Focus the previous pane
    pub fn focus_prev(&mut self) {
        let panes = self.root.panes_mut();
        let count = panes.len();
        if count == 0 {
            return;
        }

        let current = panes.iter().position(|p| p.focused).unwrap_or(0);
        let prev = if current == 0 { count - 1 } else { current - 1 };

        // Unfocus all, then focus prev
        for (i, pane) in panes.into_iter().enumerate() {
            pane.focused = i == prev;
        }
    }

    /// Focus pane by ID
    pub fn focus_pane(&mut self, id: usize) -> bool {
        let mut found = false;
        for pane in self.root.panes_mut() {
            if pane.id == id {
                pane.focused = true;
                found = true;
            } else {
                pane.focused = false;
            }
        }
        found
    }

    /// Resize the focused split
    pub fn resize(&mut self, delta: f32) {
        fn resize_in_node(node: &mut SplitNode, delta: f32) -> bool {
            match node {
                SplitNode::Split { ratio, first, second, .. } => {
                    // Check if either child contains the focused pane
                    let first_has_focus = first.focused_pane().is_some();
                    let second_has_focus = second.focused_pane().is_some();

                    if first_has_focus || second_has_focus {
                        *ratio = (*ratio + delta).clamp(0.1, 0.9);
                        return true;
                    }

                    // Recurse into children
                    resize_in_node(first, delta) || resize_in_node(second, delta)
                }
                _ => false,
            }
        }

        resize_in_node(&mut self.root, delta);
    }

    /// Render the split panes
    fn render_node(
        &self,
        node: &SplitNode,
        area: Rect,
        buf: &mut Buffer,
        pane_renderer: &impl Fn(&Pane, Rect, &mut Buffer),
    ) {
        match node {
            SplitNode::Pane(pane) => {
                let border_style = if pane.focused {
                    self.focused_border_style
                } else {
                    self.border_style
                };

                let title = if self.show_numbers {
                    format!(" {} ", pane.id)
                } else {
                    String::new()
                };

                let block = Block::default()
                    .borders(Borders::ALL)
                    .border_style(border_style)
                    .title(title);

                let inner = block.inner(area);
                block.render(area, buf);

                pane_renderer(pane, inner, buf);
            }
            SplitNode::Split {
                direction,
                ratio,
                first,
                second,
            } => {
                let chunks = match direction {
                    SplitDirection::Horizontal => {
                        let height = (area.height as f32 * ratio) as u16;
                        Layout::default()
                            .direction(Direction::Vertical)
                            .constraints([
                                Constraint::Length(height.max(self.min_size)),
                                Constraint::Min(self.min_size),
                            ])
                            .split(area)
                    }
                    SplitDirection::Vertical => {
                        let width = (area.width as f32 * ratio) as u16;
                        Layout::default()
                            .direction(Direction::Horizontal)
                            .constraints([
                                Constraint::Length(width.max(self.min_size)),
                                Constraint::Min(self.min_size),
                            ])
                            .split(area)
                    }
                };

                self.render_node(first, chunks[0], buf, pane_renderer);
                self.render_node(second, chunks[1], buf, pane_renderer);
            }
        }
    }
}

impl Default for SplitPanes {
    fn default() -> Self {
        Self::new()
    }
}

/// State for SplitPanesWidget
pub struct SplitPanesState {
    /// The split panes layout
    pub panes: SplitPanes,
}

impl SplitPanesState {
    pub fn new() -> Self {
        let mut panes = SplitPanes::new();
        // Focus the first pane by default
        if let Some(pane) = panes.root_mut().panes_mut().first_mut() {
            pane.focused = true;
        }
        Self { panes }
    }
}

impl Default for SplitPanesState {
    fn default() -> Self {
        Self::new()
    }
}

/// Widget for rendering split panes
pub struct SplitPanesWidget<'a, F>
where
    F: Fn(&Pane, Rect, &mut Buffer),
{
    /// Pane content renderer
    pane_renderer: &'a F,
}

impl<'a, F> SplitPanesWidget<'a, F>
where
    F: Fn(&Pane, Rect, &mut Buffer),
{
    pub fn new(pane_renderer: &'a F) -> Self {
        Self { pane_renderer }
    }
}

impl<F> StatefulWidget for SplitPanesWidget<'_, F>
where
    F: Fn(&Pane, Rect, &mut Buffer),
{
    type State = SplitPanesState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        state.panes.render_node(
            state.panes.root(),
            area,
            buf,
            self.pane_renderer,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_split_panes() {
        let panes = SplitPanes::new();
        assert_eq!(panes.root().pane_count(), 1);
    }

    #[test]
    fn test_split_horizontal() {
        let mut panes = SplitPanes::new();
        // Focus the first pane
        panes.root_mut().panes_mut()[0].focused = true;

        let new_id = panes.split_horizontal();
        assert!(new_id.is_some());
        assert_eq!(panes.root().pane_count(), 2);
    }

    #[test]
    fn test_split_vertical() {
        let mut panes = SplitPanes::new();
        panes.root_mut().panes_mut()[0].focused = true;

        let new_id = panes.split_vertical();
        assert!(new_id.is_some());
        assert_eq!(panes.root().pane_count(), 2);
    }

    #[test]
    fn test_close_focused() {
        let mut panes = SplitPanes::new();
        panes.root_mut().panes_mut()[0].focused = true;
        panes.split_horizontal();

        // Now we have 2 panes
        assert_eq!(panes.root().pane_count(), 2);

        // Focus the first pane and close it
        panes.focus_pane(0);
        let closed = panes.close_focused();
        assert!(closed);
        assert_eq!(panes.root().pane_count(), 1);
    }

    #[test]
    fn test_focus_next() {
        let mut panes = SplitPanes::new();
        panes.root_mut().panes_mut()[0].focused = true;
        panes.split_horizontal();

        // Focus should be on pane 0
        assert!(panes.root().panes()[0].focused);

        panes.focus_next();
        // Focus should now be on pane 1
        assert!(panes.root().panes()[1].focused);

        panes.focus_next();
        // Focus should wrap to pane 0
        assert!(panes.root().panes()[0].focused);
    }

    #[test]
    fn test_pane_with_file() {
        let pane = Pane::new(1).with_file("test.rs");
        assert_eq!(pane.file_path, Some("test.rs".to_string()));
    }
}
