use gpui::{div, prelude::*, px, AnyElement, IntoElement};

use crate::theme::Theme;

// ─── SplitPane ──────────────────────────────────────────────────────────────
// Resizable split pane layout common in desktop IDEs and editors.
// Provides horizontal or vertical split with a draggable divider.
//
// Usage:
//   SplitPane::horizontal()
//       .first(sidebar_content)
//       .second(main_content)
//       .initial_ratio(0.25)
//       .render(&theme)

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SplitDirection {
    Horizontal,
    Vertical,
}

pub struct SplitPane {
    direction: SplitDirection,
    first: Option<AnyElement>,
    second: Option<AnyElement>,
    ratio: f32,
    min_first: f32,
    min_second: f32,
    show_divider: bool,
}

#[allow(dead_code)]
impl SplitPane {
    pub fn horizontal() -> Self {
        Self {
            direction: SplitDirection::Horizontal,
            first: None,
            second: None,
            ratio: 0.3,
            min_first: 100.0,
            min_second: 100.0,
            show_divider: true,
        }
    }

    pub fn vertical() -> Self {
        Self {
            direction: SplitDirection::Vertical,
            first: None,
            second: None,
            ratio: 0.5,
            min_first: 80.0,
            min_second: 80.0,
            show_divider: true,
        }
    }

    pub fn first(mut self, element: impl IntoElement) -> Self {
        self.first = Some(element.into_any_element());
        self
    }

    pub fn second(mut self, element: impl IntoElement) -> Self {
        self.second = Some(element.into_any_element());
        self
    }

    pub fn initial_ratio(mut self, ratio: f32) -> Self {
        self.ratio = ratio.clamp(0.1, 0.9);
        self
    }

    pub fn min_first_size(mut self, min: f32) -> Self {
        self.min_first = min;
        self
    }

    pub fn min_second_size(mut self, min: f32) -> Self {
        self.min_second = min;
        self
    }

    pub fn show_divider(mut self, show: bool) -> Self {
        self.show_divider = show;
        self
    }

    pub fn render(self, theme: &Theme) -> impl IntoElement {
        let is_horizontal = self.direction == SplitDirection::Horizontal;
        let hover_color = theme.ring;

        let mut container = div().flex().size_full();

        if is_horizontal {
            container = container.flex_row();
        } else {
            container = container.flex_col();
        }

        // First pane
        let first_pane = if is_horizontal {
            div()
                .overflow_hidden()
                .h_full()
                .min_w(px(self.min_first))
                .flex_shrink_0()
                .w(gpui::relative(self.ratio))
        } else {
            div()
                .overflow_hidden()
                .w_full()
                .min_h(px(self.min_first))
                .flex_shrink_0()
                .h(gpui::relative(self.ratio))
        };

        let first_pane = if let Some(first) = self.first {
            first_pane.child(first)
        } else {
            first_pane
        };

        // Divider
        let divider = if self.show_divider {
            let mut d = div().flex_shrink_0().bg(theme.border);

            if is_horizontal {
                d = d
                    .w(px(1.0))
                    .h_full()
                    .cursor_col_resize()
                    .hover(move |style| style.bg(hover_color).w(px(3.0)));
            } else {
                d = d
                    .h(px(1.0))
                    .w_full()
                    .cursor_row_resize()
                    .hover(move |style| style.bg(hover_color).h(px(3.0)));
            }
            Some(d)
        } else {
            None
        };

        // Second pane
        let mut second_pane = div().overflow_hidden().flex_1();

        if is_horizontal {
            second_pane = second_pane.h_full().min_w(px(self.min_second));
        } else {
            second_pane = second_pane.w_full().min_h(px(self.min_second));
        }

        let second_pane = if let Some(second) = self.second {
            second_pane.child(second)
        } else {
            second_pane
        };

        container = container.child(first_pane);
        if let Some(d) = divider {
            container = container.child(d);
        }
        container = container.child(second_pane);

        container
    }
}

// ─── Panel ──────────────────────────────────────────────────────────────────
// A bordered panel with optional header, suitable for panel-based layouts.
//
// Usage:
//   Panel::new("Explorer")
//       .child(file_tree)
//       .collapsible(true)
//       .render(&theme)

pub struct Panel {
    title: Option<String>,
    children: Vec<AnyElement>,
    actions: Vec<AnyElement>,
    bordered: bool,
    collapsible: bool,
    collapsed: bool,
}

#[allow(dead_code)]
impl Panel {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: Some(title.into()),
            children: Vec::new(),
            actions: Vec::new(),
            bordered: true,
            collapsible: false,
            collapsed: false,
        }
    }

    pub fn untitled() -> Self {
        Self {
            title: None,
            children: Vec::new(),
            actions: Vec::new(),
            bordered: true,
            collapsible: false,
            collapsed: false,
        }
    }

    pub fn child(mut self, element: impl IntoElement) -> Self {
        self.children.push(element.into_any_element());
        self
    }

    pub fn action(mut self, element: impl IntoElement) -> Self {
        self.actions.push(element.into_any_element());
        self
    }

    pub fn bordered(mut self, bordered: bool) -> Self {
        self.bordered = bordered;
        self
    }

    pub fn collapsible(mut self, collapsible: bool) -> Self {
        self.collapsible = collapsible;
        self
    }

    pub fn collapsed(mut self, collapsed: bool) -> Self {
        self.collapsed = collapsed;
        self
    }

    pub fn render(self, theme: &Theme) -> impl IntoElement {
        let mut panel = div().flex().flex_col().size_full().bg(theme.background);

        if self.bordered {
            panel = panel.border_1().border_color(theme.border);
        }

        // Header
        if self.title.is_some() || !self.actions.is_empty() {
            let mut header = div()
                .flex()
                .items_center()
                .justify_between()
                .h(px(32.0))
                .px(px(12.0))
                .flex_shrink_0()
                .border_b_1()
                .border_color(theme.border);

            let mut left = div().flex().items_center().gap(px(8.0));

            // Collapse indicator
            if self.collapsible {
                let arrow = if self.collapsed { "▶" } else { "▼" };
                left = left.child(
                    div().text_size(px(8.0)).text_color(theme.muted_foreground).child(arrow),
                );
            }

            // Title
            if let Some(title) = self.title {
                left = left.child(
                    div()
                        .text_size(px(11.0))
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .text_color(theme.foreground)
                        .child(title),
                );
            }

            header = header.child(left);

            // Actions
            if !self.actions.is_empty() {
                let mut actions = div().flex().items_center().gap(px(4.0));
                for action in self.actions {
                    actions = actions.child(action);
                }
                header = header.child(actions);
            }

            panel = panel.child(header);
        }

        // Content
        if !self.collapsed {
            let mut content = div().flex_1().overflow_hidden();
            for child in self.children {
                content = content.child(child);
            }
            panel = panel.child(content);
        }

        panel
    }
}
