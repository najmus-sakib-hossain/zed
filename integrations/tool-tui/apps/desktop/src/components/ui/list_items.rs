use gpui::{div, prelude::*, px, AnyElement, IntoElement};

use crate::theme::{colors::Radius, Theme};

// ─── FileItem ───────────────────────────────────────────────────────────────
// File/folder row for file browsers, search results, recent files, etc.
//
// Usage:
//   FileItem::new("main.rs")
//       .icon("📄")
//       .path("src/main.rs")
//       .size("2.4 KB")
//       .modified("2 hours ago")
//       .render(&theme)

pub struct FileItem {
    name: String,
    icon: Option<String>,
    path: Option<String>,
    size: Option<String>,
    modified: Option<String>,
    selected: bool,
    focused: bool,
}

#[allow(dead_code)]
impl FileItem {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            icon: None,
            path: None,
            size: None,
            modified: None,
            selected: false,
            focused: false,
        }
    }

    pub fn icon(mut self, icon: impl Into<String>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    pub fn path(mut self, path: impl Into<String>) -> Self {
        self.path = Some(path.into());
        self
    }

    pub fn size(mut self, size: impl Into<String>) -> Self {
        self.size = Some(size.into());
        self
    }

    pub fn modified(mut self, modified: impl Into<String>) -> Self {
        self.modified = Some(modified.into());
        self
    }

    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    pub fn focused(mut self, focused: bool) -> Self {
        self.focused = focused;
        self
    }

    pub fn render(self, theme: &Theme) -> impl IntoElement {
        let bg = if self.selected {
            theme.accent
        } else {
            gpui::transparent_black()
        };
        let fg = if self.selected {
            theme.accent_foreground
        } else {
            theme.foreground
        };
        let hover_bg = theme.accent;
        let focus_border = theme.ring;

        let mut row = div()
            .flex()
            .items_center()
            .gap(px(8.0))
            .w_full()
            .h(px(28.0))
            .px(px(8.0))
            .bg(bg)
            .text_color(fg)
            .text_size(px(13.0))
            .cursor_pointer()
            .hover(move |style| style.bg(hover_bg));

        if self.focused {
            row = row.border_1().border_color(focus_border);
        }

        // Icon
        if let Some(icon) = self.icon {
            row = row.child(div().flex_shrink_0().text_size(px(14.0)).child(icon));
        }

        // Name
        row = row.child(
            div()
                .flex_1()
                .overflow_hidden()
                .text_ellipsis()
                .whitespace_nowrap()
                .child(self.name),
        );

        // Path (subdued)
        if let Some(path) = self.path {
            row = row.child(
                div()
                    .flex_shrink_0()
                    .text_size(px(11.0))
                    .text_color(theme.muted_foreground)
                    .overflow_hidden()
                    .text_ellipsis()
                    .whitespace_nowrap()
                    .max_w(px(200.0))
                    .child(path),
            );
        }

        // Size
        if let Some(size) = self.size {
            row = row.child(
                div()
                    .flex_shrink_0()
                    .w(px(60.0))
                    .text_right()
                    .text_size(px(11.0))
                    .text_color(theme.muted_foreground)
                    .child(size),
            );
        }

        // Modified
        if let Some(modified) = self.modified {
            row = row.child(
                div()
                    .flex_shrink_0()
                    .w(px(80.0))
                    .text_right()
                    .text_size(px(11.0))
                    .text_color(theme.muted_foreground)
                    .child(modified),
            );
        }

        row
    }
}

// ─── ListItem ───────────────────────────────────────────────────────────────
// Generic interactive list item with leading, content, and trailing areas.
//
// Usage:
//   ListItem::new("item-1")
//       .line_height(avatar)
//       .title("John Doe")
//       .subtitle("john@example.com")
//       .trailing(badge)
//       .render(&theme)

pub struct ListItem {
    id: String,
    leading: Option<AnyElement>,
    title: Option<String>,
    subtitle: Option<String>,
    trailing: Option<AnyElement>,
    selected: bool,
    disabled: bool,
    compact: bool,
}

#[allow(dead_code)]
impl ListItem {
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            leading: None,
            title: None,
            subtitle: None,
            trailing: None,
            selected: false,
            disabled: false,
            compact: false,
        }
    }

    pub fn leading(mut self, element: impl IntoElement) -> Self {
        self.leading = Some(element.into_any_element());
        self
    }

    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    pub fn subtitle(mut self, subtitle: impl Into<String>) -> Self {
        self.subtitle = Some(subtitle.into());
        self
    }

    pub fn trailing(mut self, element: impl IntoElement) -> Self {
        self.trailing = Some(element.into_any_element());
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

    pub fn compact(mut self, compact: bool) -> Self {
        self.compact = compact;
        self
    }

    pub fn render(self, theme: &Theme) -> impl IntoElement {
        let height = if self.compact { 36.0 } else { 48.0 };
        let bg = if self.selected {
            theme.accent
        } else {
            gpui::transparent_black()
        };
        let hover_bg = theme.accent;

        let mut row = div()
            .flex()
            .items_center()
            .gap(px(12.0))
            .w_full()
            .h(px(height))
            .px(px(12.0))
            .bg(bg)
            .rounded(Radius::MD);

        if self.disabled {
            row = row.opacity(0.5);
        } else {
            row = row.cursor_pointer().hover(move |style| style.bg(hover_bg));
        }

        // Leading
        if let Some(leading) = self.leading {
            row = row.child(div().flex_shrink_0().child(leading));
        }

        // Content
        let mut content = div().flex().flex_col().flex_1().overflow_hidden();

        if let Some(title) = self.title {
            let fg = if self.selected {
                theme.accent_foreground
            } else {
                theme.foreground
            };
            content = content.child(
                div()
                    .text_size(px(13.0))
                    .font_weight(gpui::FontWeight::MEDIUM)
                    .text_color(fg)
                    .overflow_hidden()
                    .text_ellipsis()
                    .whitespace_nowrap()
                    .child(title),
            );
        }

        if let Some(subtitle) = self.subtitle {
            content = content.child(
                div()
                    .text_size(px(11.0))
                    .text_color(theme.muted_foreground)
                    .overflow_hidden()
                    .text_ellipsis()
                    .whitespace_nowrap()
                    .child(subtitle),
            );
        }

        row = row.child(content);

        // Trailing
        if let Some(trailing) = self.trailing {
            row = row.child(div().flex_shrink_0().child(trailing));
        }

        row
    }
}

// ─── EmptyList ──────────────────────────────────────────────────────────────
// Empty state placeholder for lists with no content.

pub struct EmptyList {
    icon: Option<String>,
    title: String,
    description: Option<String>,
    action: Option<AnyElement>,
}

#[allow(dead_code)]
impl EmptyList {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            icon: None,
            title: title.into(),
            description: None,
            action: None,
        }
    }

    pub fn icon(mut self, icon: impl Into<String>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    pub fn action(mut self, action: impl IntoElement) -> Self {
        self.action = Some(action.into_any_element());
        self
    }

    pub fn render(self, theme: &Theme) -> impl IntoElement {
        let mut container = div()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap(px(12.0))
            .py(px(40.0))
            .w_full();

        if let Some(icon) = self.icon {
            container = container
                .child(div().text_size(px(32.0)).text_color(theme.muted_foreground).child(icon));
        }

        container = container.child(
            div()
                .text_size(px(14.0))
                .font_weight(gpui::FontWeight::MEDIUM)
                .text_color(theme.foreground)
                .child(self.title),
        );

        if let Some(desc) = self.description {
            container = container.child(
                div()
                    .text_size(px(12.0))
                    .text_color(theme.muted_foreground)
                    .text_center()
                    .max_w(px(300.0))
                    .child(desc),
            );
        }

        if let Some(action) = self.action {
            container = container.child(div().mt(px(4.0)).child(action));
        }

        container
    }
}
