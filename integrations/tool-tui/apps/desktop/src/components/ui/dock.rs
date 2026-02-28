use gpui::{div, prelude::*, px, AnyElement, Hsla, IntoElement};

use crate::theme::{colors::Radius, Theme};

// ─── Dock ───────────────────────────────────────────────────────────────────
// Dockable panel container (similar to VS Code bottom/side panels).
//
// Usage:
//   Dock::new()
//       .tab(DockTab::new("Problems").badge(3))
//       .tab(DockTab::new("Output").active(true))
//       .tab(DockTab::new("Terminal"))
//       .content(terminal_element)
//       .render(&theme)

pub struct Dock {
    tabs: Vec<DockTab>,
    content: Option<AnyElement>,
    position: DockPosition,
    height: f32,
    show_close: bool,
}

#[derive(Clone, Copy)]
pub enum DockPosition {
    Bottom,
    Left,
    Right,
}

#[allow(dead_code)]
impl Dock {
    pub fn new() -> Self {
        Self {
            tabs: Vec::new(),
            content: None,
            position: DockPosition::Bottom,
            height: 200.0,
            show_close: true,
        }
    }

    pub fn tab(mut self, tab: DockTab) -> Self {
        self.tabs.push(tab);
        self
    }

    pub fn tabs(mut self, tabs: Vec<DockTab>) -> Self {
        self.tabs = tabs;
        self
    }

    pub fn content(mut self, content: impl IntoElement) -> Self {
        self.content = Some(content.into_any_element());
        self
    }

    pub fn position(mut self, pos: DockPosition) -> Self {
        self.position = pos;
        self
    }

    pub fn height(mut self, h: f32) -> Self {
        self.height = h;
        self
    }

    pub fn show_close(mut self, v: bool) -> Self {
        self.show_close = v;
        self
    }

    pub fn render(self, theme: &Theme) -> impl IntoElement {
        let is_horizontal = matches!(self.position, DockPosition::Bottom);

        let mut container = div().flex().flex_col().bg(theme.background);

        if is_horizontal {
            container = container.w_full().h(px(self.height)).border_t_1();
        } else {
            container = container.h_full().w(px(self.height)).border_l_1();
        }
        container = container.border_color(theme.border);

        // Tab bar
        let mut tab_bar = div()
            .flex()
            .items_center()
            .h(px(32.0))
            .px(px(8.0))
            .gap(px(2.0))
            .border_b_1()
            .border_color(theme.border)
            .bg(theme.background);

        for tab in self.tabs {
            tab_bar = tab_bar.child(tab.render(theme));
        }

        // Spacer
        tab_bar = tab_bar.child(div().flex_1());

        // Close button
        if self.show_close {
            let close_hover = theme.muted;
            tab_bar = tab_bar.child(
                div()
                    .flex()
                    .items_center()
                    .justify_center()
                    .w(px(20.0))
                    .h(px(20.0))
                    .rounded(Radius::SM)
                    .text_color(theme.muted_foreground)
                    .text_size(px(12.0))
                    .cursor_pointer()
                    .hover(move |s| s.bg(close_hover))
                    .child("×"),
            );
        }

        container = container.child(tab_bar);

        // Content
        if let Some(content) = self.content {
            container = container.child(div().flex_1().overflow_hidden().child(content));
        }

        container
    }
}

pub struct DockTab {
    label: String,
    active: bool,
    badge: Option<u32>,
}

#[allow(dead_code)]
impl DockTab {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            active: false,
            badge: None,
        }
    }

    pub fn active(mut self, v: bool) -> Self {
        self.active = v;
        self
    }

    pub fn badge(mut self, count: u32) -> Self {
        self.badge = Some(count);
        self
    }

    pub fn render(self, theme: &Theme) -> impl IntoElement {
        let fg = if self.active {
            theme.foreground
        } else {
            theme.muted_foreground
        };
        let hover_bg = theme.muted;

        let mut tab = div()
            .flex()
            .items_center()
            .gap(px(4.0))
            .px(px(8.0))
            .py(px(4.0))
            .rounded_t(Radius::SM)
            .text_color(fg)
            .text_size(px(12.0))
            .cursor_pointer()
            .hover(move |s| s.bg(hover_bg));

        if self.active {
            tab = tab.border_b_2().border_color(theme.primary);
        }

        tab = tab.child(self.label);

        if let Some(count) = self.badge {
            tab = tab.child(
                div()
                    .flex()
                    .items_center()
                    .justify_center()
                    .min_w(px(16.0))
                    .h(px(16.0))
                    .px(px(4.0))
                    .rounded(Radius::FULL)
                    .bg(theme.primary)
                    .text_color(theme.primary_foreground)
                    .text_size(px(9.0))
                    .font_weight(gpui::FontWeight::BOLD)
                    .child(count.to_string()),
            );
        }

        tab
    }
}

// ─── Resizable ──────────────────────────────────────────────────────────────
// A resizable container with a visual drag handle.

#[derive(Clone, Copy)]
pub enum ResizeDirection {
    Horizontal,
    Vertical,
}

pub struct ResizeHandle {
    direction: ResizeDirection,
    size: f32,
}

#[allow(dead_code)]
impl ResizeHandle {
    pub fn new(direction: ResizeDirection) -> Self {
        Self {
            direction,
            size: 4.0,
        }
    }

    pub fn size(mut self, s: f32) -> Self {
        self.size = s;
        self
    }

    pub fn render(self, theme: &Theme) -> impl IntoElement {
        let hover_color = theme.primary;
        let bg = theme.border;

        match self.direction {
            ResizeDirection::Horizontal => div()
                .w(px(self.size))
                .h_full()
                .flex_shrink_0()
                .bg(bg)
                .cursor_col_resize()
                .hover(move |s| s.bg(hover_color)),
            ResizeDirection::Vertical => div()
                .w_full()
                .h(px(self.size))
                .flex_shrink_0()
                .bg(bg)
                .cursor_row_resize()
                .hover(move |s| s.bg(hover_color)),
        }
    }
}

// ─── WindowControls ─────────────────────────────────────────────────────────
// Minimal window control buttons (minimize, maximize, close) for custom
// title bars.
//
// Usage:
//   WindowControls::new().platform_style(true).render(&theme)

pub struct WindowControls {
    show_minimize: bool,
    show_maximize: bool,
    show_close: bool,
    button_size: f32,
}

#[allow(dead_code)]
impl WindowControls {
    pub fn new() -> Self {
        Self {
            show_minimize: true,
            show_maximize: true,
            show_close: true,
            button_size: 14.0,
        }
    }

    pub fn show_minimize(mut self, v: bool) -> Self {
        self.show_minimize = v;
        self
    }

    pub fn show_maximize(mut self, v: bool) -> Self {
        self.show_maximize = v;
        self
    }

    pub fn show_close(mut self, v: bool) -> Self {
        self.show_close = v;
        self
    }

    pub fn button_size(mut self, s: f32) -> Self {
        self.button_size = s;
        self
    }

    pub fn render(self, _theme: &Theme) -> impl IntoElement {
        let mut row = div().flex().items_center().gap(px(8.0)).px(px(8.0));

        let btn_style = move |size: f32, color: Hsla, hover_color: Hsla| {
            div()
                .flex()
                .items_center()
                .justify_center()
                .w(px(size))
                .h(px(size))
                .rounded(Radius::FULL)
                .bg(color)
                .cursor_pointer()
                .hover(move |s| s.bg(hover_color))
        };

        if self.show_minimize {
            // Yellow-ish minimize
            let min_color = gpui::hsla(0.14, 0.9, 0.58, 1.0);
            let min_hover = gpui::hsla(0.14, 0.9, 0.48, 1.0);
            row = row.child(btn_style(self.button_size, min_color, min_hover));
        }

        if self.show_maximize {
            // Green maximize
            let max_color = gpui::hsla(0.33, 0.9, 0.50, 1.0);
            let max_hover = gpui::hsla(0.33, 0.9, 0.40, 1.0);
            row = row.child(btn_style(self.button_size, max_color, max_hover));
        }

        if self.show_close {
            // Red close
            let close_color = gpui::hsla(0.0, 0.9, 0.55, 1.0);
            let close_hover = gpui::hsla(0.0, 0.9, 0.45, 1.0);
            row = row.child(btn_style(self.button_size, close_color, close_hover));
        }

        row
    }
}

// ─── Splitter ───────────────────────────────────────────────────────────────
// A container that holds two children with a draggable splitter between them.
//
// Usage:
//   Splitter::horizontal()
//       .first(sidebar_element, 250.0)
//       .second(main_content)
//       .render(&theme)

pub struct Splitter {
    direction: SplitterDirection,
    first: Option<AnyElement>,
    first_size: f32,
    second: Option<AnyElement>,
    min_first: f32,
    handle_size: f32,
}

#[derive(Clone, Copy)]
pub enum SplitterDirection {
    Horizontal,
    Vertical,
}

#[allow(dead_code)]
impl Splitter {
    pub fn horizontal() -> Self {
        Self {
            direction: SplitterDirection::Horizontal,
            first: None,
            first_size: 250.0,
            second: None,
            min_first: 100.0,
            handle_size: 4.0,
        }
    }

    pub fn vertical() -> Self {
        Self {
            direction: SplitterDirection::Vertical,
            first: None,
            first_size: 200.0,
            second: None,
            min_first: 80.0,
            handle_size: 4.0,
        }
    }

    pub fn first(mut self, element: impl IntoElement, size: f32) -> Self {
        self.first = Some(element.into_any_element());
        self.first_size = size;
        self
    }

    pub fn second(mut self, element: impl IntoElement) -> Self {
        self.second = Some(element.into_any_element());
        self
    }

    pub fn min_first(mut self, v: f32) -> Self {
        self.min_first = v;
        self
    }

    pub fn handle_size(mut self, s: f32) -> Self {
        self.handle_size = s;
        self
    }

    pub fn render(self, theme: &Theme) -> impl IntoElement {
        let handle_bg = theme.border;
        let handle_hover = theme.primary;
        let handle_size = self.handle_size;

        match self.direction {
            SplitterDirection::Horizontal => {
                let mut container = div().flex().w_full().h_full();

                // First panel
                if let Some(first) = self.first {
                    container = container.child(
                        div()
                            .w(px(self.first_size))
                            .h_full()
                            .flex_shrink_0()
                            .overflow_hidden()
                            .child(first),
                    );
                }

                // Handle
                container = container.child(
                    div()
                        .w(px(handle_size))
                        .h_full()
                        .flex_shrink_0()
                        .bg(handle_bg)
                        .cursor_col_resize()
                        .hover(move |s| s.bg(handle_hover)),
                );

                // Second panel
                if let Some(second) = self.second {
                    container =
                        container.child(div().flex_1().h_full().overflow_hidden().child(second));
                }

                container
            }
            SplitterDirection::Vertical => {
                let mut container = div().flex().flex_col().w_full().h_full();

                if let Some(first) = self.first {
                    container = container.child(
                        div()
                            .h(px(self.first_size))
                            .w_full()
                            .flex_shrink_0()
                            .overflow_hidden()
                            .child(first),
                    );
                }

                container = container.child(
                    div()
                        .h(px(handle_size))
                        .w_full()
                        .flex_shrink_0()
                        .bg(handle_bg)
                        .cursor_row_resize()
                        .hover(move |s| s.bg(handle_hover)),
                );

                if let Some(second) = self.second {
                    container =
                        container.child(div().flex_1().w_full().overflow_hidden().child(second));
                }

                container
            }
        }
    }
}
