use gpui::{div, prelude::*, px, IntoElement};

use crate::theme::Theme;

// ─── TreeView ───────────────────────────────────────────────────────────────
// Desktop-native tree view for file explorers, navigation trees, etc.
//
// Usage:
//   TreeView::new()
//       .node(TreeNode::branch("src", vec![
//           TreeNode::leaf("main.rs").icon("📄"),
//           TreeNode::branch("components", vec![
//               TreeNode::leaf("button.rs"),
//           ]),
//       ]))
//       .render(&theme)

pub struct TreeView {
    nodes: Vec<TreeNode>,
    indent_size: f32,
    selected_id: Option<String>,
    show_lines: bool,
}

pub struct TreeNode {
    id: String,
    label: String,
    icon: Option<String>,
    children: Vec<TreeNode>,
    expanded: bool,
    depth: u32,
}

#[allow(dead_code)]
impl TreeNode {
    pub fn leaf(label: impl Into<String>) -> Self {
        let label_str: String = label.into();
        Self {
            id: label_str.clone(),
            label: label_str,
            icon: None,
            children: Vec::new(),
            expanded: false,
            depth: 0,
        }
    }

    pub fn branch(label: impl Into<String>, children: Vec<TreeNode>) -> Self {
        let label_str: String = label.into();
        Self {
            id: label_str.clone(),
            label: label_str,
            icon: None,
            children,
            expanded: true,
            depth: 0,
        }
    }

    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = id.into();
        self
    }

    pub fn icon(mut self, icon: impl Into<String>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    pub fn expanded(mut self, expanded: bool) -> Self {
        self.expanded = expanded;
        self
    }

    fn is_branch(&self) -> bool {
        !self.children.is_empty()
    }

    fn with_depth(mut self, depth: u32) -> Self {
        self.depth = depth;
        self.children = self.children.into_iter().map(|c| c.with_depth(depth + 1)).collect();
        self
    }
}

#[allow(dead_code)]
impl TreeView {
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            indent_size: 16.0,
            selected_id: None,
            show_lines: false,
        }
    }

    pub fn node(mut self, node: TreeNode) -> Self {
        self.nodes.push(node.with_depth(0));
        self
    }

    pub fn nodes(mut self, nodes: Vec<TreeNode>) -> Self {
        self.nodes = nodes.into_iter().map(|n| n.with_depth(0)).collect();
        self
    }

    pub fn indent_size(mut self, size: f32) -> Self {
        self.indent_size = size;
        self
    }

    pub fn selected(mut self, id: impl Into<String>) -> Self {
        self.selected_id = Some(id.into());
        self
    }

    pub fn show_lines(mut self, show: bool) -> Self {
        self.show_lines = show;
        self
    }

    pub fn render(self, theme: &Theme) -> impl IntoElement {
        let mut tree = div().flex().flex_col().w_full();

        for node in self.nodes {
            tree = Self::render_node_into(tree, &node, theme, self.indent_size, &self.selected_id);
        }

        tree
    }

    fn render_node_into(
        mut parent: gpui::Div,
        node: &TreeNode,
        theme: &Theme,
        indent_size: f32,
        selected_id: &Option<String>,
    ) -> gpui::Div {
        let indent = indent_size * node.depth as f32;
        let is_selected = selected_id.as_ref().is_some_and(|id| id == &node.id);
        let is_branch = node.is_branch();

        let bg = if is_selected {
            theme.accent
        } else {
            gpui::transparent_black()
        };
        let fg = if is_selected {
            theme.accent_foreground
        } else {
            theme.foreground
        };
        let hover_bg = theme.accent;

        let mut row = div()
            .flex()
            .items_center()
            .gap(px(4.0))
            .w_full()
            .h(px(24.0))
            .pl(px(indent + 8.0))
            .pr(px(8.0))
            .bg(bg)
            .text_color(fg)
            .text_size(px(13.0))
            .cursor_pointer()
            .hover(move |style| style.bg(hover_bg));

        // Expand/collapse indicator for branches
        if is_branch {
            let arrow = if node.expanded { "▼" } else { "▶" };
            row = row.child(
                div()
                    .text_size(px(8.0))
                    .text_color(theme.muted_foreground)
                    .w(px(12.0))
                    .flex_shrink_0()
                    .child(arrow),
            );
        } else {
            // Spacer for alignment with branches
            row = row.child(div().w(px(12.0)).flex_shrink_0());
        }

        // Icon
        if let Some(ref icon) = node.icon {
            row = row.child(div().text_size(px(14.0)).flex_shrink_0().child(icon.clone()));
        }

        // Label
        row = row.child(
            div()
                .overflow_hidden()
                .text_ellipsis()
                .whitespace_nowrap()
                .flex_1()
                .child(node.label.clone()),
        );

        parent = parent.child(row);

        // Render children if expanded
        if is_branch && node.expanded {
            for child in &node.children {
                parent = Self::render_node_into(parent, child, theme, indent_size, selected_id);
            }
        }

        parent
    }
}
