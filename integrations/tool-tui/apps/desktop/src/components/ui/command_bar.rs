use gpui::{div, prelude::*, px, IntoElement};

use crate::theme::{colors::Radius, Theme};

// ─── CommandBar ─────────────────────────────────────────────────────────────
// A Spotlight / Raycast / VS-Code-style command bar overlay.
// Shows a search input at top and a scrollable list of grouped results.
//
// Usage:
//   CommandBar::new()
//       .placeholder("Type a command...")
//       .search_value("open")
//       .group(CommandBarGroup::new("Files")
//           .item(CommandBarAction::new("Open File").shortcut("Ctrl+O").icon("📁"))
//       )
//       .render(&theme)

pub struct CommandBar {
    placeholder: String,
    search_value: String,
    groups: Vec<CommandBarGroup>,
    width: f32,
    max_height: f32,
    show_overlay: bool,
}

#[allow(dead_code)]
impl CommandBar {
    pub fn new() -> Self {
        Self {
            placeholder: "Type a command…".into(),
            search_value: String::new(),
            groups: Vec::new(),
            width: 560.0,
            max_height: 420.0,
            show_overlay: true,
        }
    }

    pub fn placeholder(mut self, p: impl Into<String>) -> Self {
        self.placeholder = p.into();
        self
    }

    pub fn search_value(mut self, v: impl Into<String>) -> Self {
        self.search_value = v.into();
        self
    }

    pub fn group(mut self, g: CommandBarGroup) -> Self {
        self.groups.push(g);
        self
    }

    pub fn width(mut self, w: f32) -> Self {
        self.width = w;
        self
    }

    pub fn max_height(mut self, h: f32) -> Self {
        self.max_height = h;
        self
    }

    pub fn show_overlay(mut self, v: bool) -> Self {
        self.show_overlay = v;
        self
    }

    // ── Render ──

    pub fn render(self, theme: &Theme) -> impl IntoElement {
        let overlay_bg = if self.show_overlay {
            theme.overlay
        } else {
            gpui::transparent_black()
        };

        div()
            .absolute()
            .inset_0()
            .flex()
            .justify_center()
            .pt(px(80.0))
            .bg(overlay_bg)
            .child(
                div()
                    .w(px(self.width))
                    .max_h(px(self.max_height))
                    .flex()
                    .flex_col()
                    .rounded(Radius::LG)
                    .bg(theme.popover)
                    .border_1()
                    .border_color(theme.border)
                    .shadow_xl()
                    .overflow_hidden()
                    // Search input
                    .child(self.render_input(theme))
                    // Results
                    .child(self.render_results(theme)),
            )
    }

    fn render_input(&self, theme: &Theme) -> impl IntoElement {
        div()
            .flex()
            .items_center()
            .gap(px(8.0))
            .px(px(16.0))
            .py(px(12.0))
            .border_b_1()
            .border_color(theme.border)
            .child(div().text_sm().text_color(theme.muted_foreground).child("🔍"))
            .child(
                div()
                    .flex_1()
                    .text_sm()
                    .text_color(if self.search_value.is_empty() {
                        theme.muted_foreground
                    } else {
                        theme.foreground
                    })
                    .child(if self.search_value.is_empty() {
                        self.placeholder.clone()
                    } else {
                        self.search_value.clone()
                    }),
            )
    }

    fn render_results(self, theme: &Theme) -> impl IntoElement {
        let mut list = div().flex().flex_col().py(px(4.0)).overflow_hidden();

        for group in self.groups {
            list = list.child(group.render(theme));
        }

        list
    }
}

// ── Group ──

pub struct CommandBarGroup {
    label: String,
    items: Vec<CommandBarAction>,
}

#[allow(dead_code)]
impl CommandBarGroup {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            items: Vec::new(),
        }
    }

    pub fn item(mut self, item: CommandBarAction) -> Self {
        self.items.push(item);
        self
    }

    pub fn items(mut self, items: Vec<CommandBarAction>) -> Self {
        self.items = items;
        self
    }

    fn render(self, theme: &Theme) -> impl IntoElement {
        let mut col = div().flex().flex_col();

        // Section header
        col = col.child(
            div()
                .px(px(12.0))
                .py(px(6.0))
                .text_xs()
                .font_weight(gpui::FontWeight::MEDIUM)
                .text_color(theme.muted_foreground)
                .child(self.label),
        );

        for item in self.items {
            col = col.child(item.render(theme));
        }

        col
    }
}

// ── Action ──

pub struct CommandBarAction {
    label: String,
    icon: Option<String>,
    shortcut: Option<String>,
    description: Option<String>,
    selected: bool,
    disabled: bool,
}

#[allow(dead_code)]
impl CommandBarAction {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            icon: None,
            shortcut: None,
            description: None,
            selected: false,
            disabled: false,
        }
    }

    pub fn icon(mut self, icon: impl Into<String>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    pub fn shortcut(mut self, shortcut: impl Into<String>) -> Self {
        self.shortcut = Some(shortcut.into());
        self
    }

    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    fn render(self, theme: &Theme) -> impl IntoElement {
        let bg = if self.selected {
            theme.accent
        } else {
            gpui::transparent_black()
        };
        let fg = if self.selected {
            theme.accent_foreground
        } else {
            theme.popover_foreground
        };

        let hover_bg = theme.accent;

        let mut row = div()
            .flex()
            .items_center()
            .gap(px(10.0))
            .px(px(12.0))
            .py(px(6.0))
            .mx(px(4.0))
            .rounded(Radius::SM)
            .bg(bg)
            .text_color(fg)
            .cursor_pointer()
            .hover(move |s| s.bg(hover_bg));

        if let Some(icon) = self.icon {
            row = row.child(div().w(px(20.0)).text_center().text_sm().child(icon));
        }

        // Label + description
        let mut label_col = div().flex().flex_col().flex_1();
        label_col = label_col.child(div().text_sm().child(self.label));
        if let Some(desc) = self.description {
            label_col =
                label_col.child(div().text_xs().text_color(theme.muted_foreground).child(desc));
        }
        row = row.child(label_col);

        if let Some(shortcut) = self.shortcut {
            row = row.child(
                div()
                    .text_xs()
                    .text_color(theme.muted_foreground)
                    .px(px(6.0))
                    .py(px(2.0))
                    .rounded(Radius::SM)
                    .bg(theme.muted)
                    .child(shortcut),
            );
        }

        if self.disabled {
            row = row.opacity(0.5).cursor_default();
        }

        row
    }
}

// ─── Spotlight ──────────────────────────────────────────────────────────────
// A minimal spotlight variant (just search + flat results, no groups).
//
// Usage:
//   Spotlight::new()
//       .item(SpotlightItem::new("Open Settings").icon("⚙"))
//       .render(&theme)

pub struct Spotlight {
    placeholder: String,
    search_value: String,
    items: Vec<SpotlightItem>,
    width: f32,
}

#[allow(dead_code)]
impl Spotlight {
    pub fn new() -> Self {
        Self {
            placeholder: "Search…".into(),
            search_value: String::new(),
            items: Vec::new(),
            width: 480.0,
        }
    }

    pub fn placeholder(mut self, p: impl Into<String>) -> Self {
        self.placeholder = p.into();
        self
    }

    pub fn search_value(mut self, v: impl Into<String>) -> Self {
        self.search_value = v.into();
        self
    }

    pub fn item(mut self, item: SpotlightItem) -> Self {
        self.items.push(item);
        self
    }

    pub fn width(mut self, w: f32) -> Self {
        self.width = w;
        self
    }

    pub fn render(self, theme: &Theme) -> impl IntoElement {
        let mut wrapper = div()
            .w(px(self.width))
            .flex()
            .flex_col()
            .rounded(Radius::LG)
            .bg(theme.popover)
            .border_1()
            .border_color(theme.border)
            .shadow_xl()
            .overflow_hidden();

        // Input
        wrapper = wrapper.child(
            div()
                .flex()
                .items_center()
                .gap(px(8.0))
                .px(px(16.0))
                .py(px(12.0))
                .border_b_1()
                .border_color(theme.border)
                .child(div().text_sm().text_color(theme.muted_foreground).child("🔍"))
                .child(
                    div()
                        .flex_1()
                        .text_sm()
                        .text_color(if self.search_value.is_empty() {
                            theme.muted_foreground
                        } else {
                            theme.foreground
                        })
                        .child(if self.search_value.is_empty() {
                            self.placeholder.clone()
                        } else {
                            self.search_value.clone()
                        }),
                ),
        );

        // Items
        let mut list = div().flex().flex_col().py(px(4.0));
        for item in self.items {
            list = list.child(item.render(theme));
        }
        wrapper = wrapper.child(list);

        wrapper
    }
}

pub struct SpotlightItem {
    label: String,
    icon: Option<String>,
    trailing: Option<String>,
    selected: bool,
}

#[allow(dead_code)]
impl SpotlightItem {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            icon: None,
            trailing: None,
            selected: false,
        }
    }

    pub fn icon(mut self, icon: impl Into<String>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    pub fn trailing(mut self, t: impl Into<String>) -> Self {
        self.trailing = Some(t.into());
        self
    }

    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    fn render(self, theme: &Theme) -> impl IntoElement {
        let bg = if self.selected {
            theme.accent
        } else {
            gpui::transparent_black()
        };

        let mut row = div()
            .flex()
            .items_center()
            .gap(px(10.0))
            .px(px(12.0))
            .py(px(8.0))
            .mx(px(4.0))
            .rounded(Radius::SM)
            .bg(bg)
            .cursor_pointer()
            .hover(move |s| s.bg(theme.accent));

        if let Some(icon) = self.icon {
            row = row.child(div().w(px(20.0)).text_center().text_sm().child(icon));
        }

        row = row.child(div().flex_1().text_sm().text_color(theme.foreground).child(self.label));

        if let Some(trailing) = self.trailing {
            row = row.child(div().text_xs().text_color(theme.muted_foreground).child(trailing));
        }

        row
    }
}
