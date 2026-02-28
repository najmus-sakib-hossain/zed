use gpui::{div, prelude::*, px, IntoElement};

use crate::theme::{colors::Radius, Theme};

// ─── WorkspaceTabs ──────────────────────────────────────────────────────────
// Browser / IDE-style draggable tab bar for multi-document workspaces.
//
// Usage:
//   WorkspaceTabs::new()
//       .tab(WorkspaceTab::new("main.rs").icon("📄").active(true).modified(true))
//       .tab(WorkspaceTab::new("lib.rs").icon("📄"))
//       .show_add(true)
//       .render(&theme)

pub struct WorkspaceTabs {
    tabs: Vec<WorkspaceTab>,
    show_add_button: bool,
    height: f32,
}

#[allow(dead_code)]
impl WorkspaceTabs {
    pub fn new() -> Self {
        Self {
            tabs: Vec::new(),
            show_add_button: true,
            height: 36.0,
        }
    }

    pub fn tab(mut self, tab: WorkspaceTab) -> Self {
        self.tabs.push(tab);
        self
    }

    pub fn tabs(mut self, tabs: Vec<WorkspaceTab>) -> Self {
        self.tabs = tabs;
        self
    }

    pub fn show_add(mut self, show: bool) -> Self {
        self.show_add_button = show;
        self
    }

    pub fn height(mut self, h: f32) -> Self {
        self.height = h;
        self
    }

    pub fn render(self, theme: &Theme) -> impl IntoElement {
        let mut bar = div()
            .flex()
            .items_end()
            .h(px(self.height))
            .bg(theme.card)
            .border_b_1()
            .border_color(theme.border)
            .overflow_hidden();

        for tab in self.tabs {
            bar = bar.child(tab.render(theme));
        }

        if self.show_add_button {
            bar = bar.child(
                div()
                    .flex()
                    .items_center()
                    .justify_center()
                    .w(px(32.0))
                    .h(px(self.height))
                    .text_sm()
                    .text_color(theme.muted_foreground)
                    .cursor_pointer()
                    .hover(move |s| s.text_color(theme.foreground))
                    .child("+"),
            );
        }

        bar
    }
}

// ── Single Tab ──

pub struct WorkspaceTab {
    label: String,
    icon: Option<String>,
    active: bool,
    modified: bool,
    pinned: bool,
    closable: bool,
}

#[allow(dead_code)]
impl WorkspaceTab {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            icon: None,
            active: false,
            modified: false,
            pinned: false,
            closable: true,
        }
    }

    pub fn icon(mut self, icon: impl Into<String>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    pub fn active(mut self, active: bool) -> Self {
        self.active = active;
        self
    }

    pub fn modified(mut self, modified: bool) -> Self {
        self.modified = modified;
        self
    }

    pub fn pinned(mut self, pinned: bool) -> Self {
        self.pinned = pinned;
        self
    }

    pub fn closable(mut self, closable: bool) -> Self {
        self.closable = closable;
        self
    }

    pub fn render(self, theme: &Theme) -> impl IntoElement {
        let bg = if self.active {
            theme.background
        } else {
            gpui::transparent_black()
        };
        let fg = if self.active {
            theme.foreground
        } else {
            theme.muted_foreground
        };
        let border_bottom = if self.active {
            gpui::transparent_black()
        } else {
            theme.border
        };
        let hover_bg = if self.active {
            theme.background
        } else {
            theme.ghost_hover
        };

        let mut tab = div()
            .flex()
            .items_center()
            .gap(px(6.0))
            .h_full()
            .px(px(12.0))
            .bg(bg)
            .border_b_1()
            .border_color(border_bottom)
            .border_r_1()
            .border_color(theme.border)
            .text_color(fg)
            .text_sm()
            .cursor_pointer()
            .hover(move |s| s.bg(hover_bg));

        if let Some(icon) = self.icon {
            tab = tab.child(div().text_xs().child(icon));
        }

        tab = tab.child(self.label);

        // Modified dot
        if self.modified {
            tab = tab.child(
                div().w(px(6.0)).h(px(6.0)).rounded(px(3.0)).bg(theme.primary).flex_shrink_0(),
            );
        }

        // Close button (unless pinned)
        if self.closable && !self.pinned {
            tab = tab.child(
                div()
                    .flex()
                    .items_center()
                    .justify_center()
                    .w(px(16.0))
                    .h(px(16.0))
                    .rounded(Radius::SM)
                    .text_xs()
                    .text_color(theme.muted_foreground)
                    .cursor_pointer()
                    .hover(move |s| s.bg(theme.muted).text_color(theme.foreground))
                    .child("✕"),
            );
        }

        if self.pinned {
            tab = tab.child(div().text_xs().text_color(theme.muted_foreground).child("📌"));
        }

        tab
    }
}

// ─── TabGroup ───────────────────────────────────────────────────────────────
// Groups multiple tab rows or allows nesting of tab bars.
//
// Usage:
//   TabGroup::new()
//       .row(WorkspaceTabs::new().tab(...))
//       .render(&theme)

pub struct TabGroup {
    rows: Vec<WorkspaceTabs>,
}

#[allow(dead_code)]
impl TabGroup {
    pub fn new() -> Self {
        Self { rows: Vec::new() }
    }

    pub fn row(mut self, tabs: WorkspaceTabs) -> Self {
        self.rows.push(tabs);
        self
    }

    pub fn render(self, theme: &Theme) -> impl IntoElement {
        let mut col = div().flex().flex_col();
        for row in self.rows {
            col = col.child(row.render(theme));
        }
        col
    }
}
