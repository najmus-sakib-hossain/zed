use crate::theme::{colors::Radius, Theme};
use gpui::{div, prelude::*, px, AnyElement, IntoElement};

// ─── Tooltip ────────────────────────────────────────────────────────────────
// A shadcn-ui style Tooltip (static display version).
// In GPUI, true hover-triggered tooltips require view-level state management.
// This provides the visual presentation.
//
// Usage:
//   Tooltip::new("This is a tooltip").render(&theme)

pub struct Tooltip {
    content: String,
    side: TooltipSide,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TooltipSide {
    Top,
    Bottom,
    Left,
    Right,
}

impl Tooltip {
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            side: TooltipSide::Top,
        }
    }

    #[allow(dead_code)]
    pub fn side(mut self, side: TooltipSide) -> Self {
        self.side = side;
        self
    }

    pub fn render(self, theme: &Theme) -> impl IntoElement {
        div()
            .px(px(12.0))
            .py(px(6.0))
            .rounded(Radius::DEFAULT)
            .bg(theme.popover)
            .border_1()
            .border_color(theme.border)
            .text_size(px(12.0))
            .text_color(theme.popover_foreground)
            .child(self.content)
    }
}

// ─── Popover ────────────────────────────────────────────────────────────────
// A shadcn-ui style Popover container.
//
// Usage:
//   Popover::new()
//       .child(div().child("Popover content"))
//       .render(&theme)

pub struct Popover {
    children: Vec<AnyElement>,
    width: Option<gpui::Pixels>,
}

impl Popover {
    pub fn new() -> Self {
        Self {
            children: Vec::new(),
            width: None,
        }
    }

    pub fn child(mut self, child: impl IntoElement) -> Self {
        self.children.push(child.into_any_element());
        self
    }

    #[allow(dead_code)]
    pub fn width(mut self, width: gpui::Pixels) -> Self {
        self.width = Some(width);
        self
    }

    pub fn render(self, theme: &Theme) -> impl IntoElement {
        let mut popover = div()
            .flex()
            .flex_col()
            .p(px(16.0))
            .rounded(Radius::LG)
            .bg(theme.popover)
            .border_1()
            .border_color(theme.border);

        if let Some(w) = self.width {
            popover = popover.w(w);
        }

        for child in self.children {
            popover = popover.child(child);
        }

        popover
    }
}

// ─── DropdownMenu ───────────────────────────────────────────────────────────
// A shadcn-ui style DropdownMenu.
//
// Usage:
//   DropdownMenu::new()
//       .item(DropdownMenuItem::new("Profile").icon("👤"))
//       .separator()
//       .item(DropdownMenuItem::new("Settings").shortcut("⌘,"))
//       .separator()
//       .item(DropdownMenuItem::new("Log out").destructive(true))
//       .render(&theme)

pub struct DropdownMenu {
    items: Vec<DropdownMenuEntry>,
    width: gpui::Pixels,
    label: Option<String>,
}

enum DropdownMenuEntry {
    Item(DropdownMenuItem),
    Separator,
    Label(String),
}

pub struct DropdownMenuItem {
    label: String,
    icon: Option<String>,
    shortcut: Option<String>,
    disabled: bool,
    destructive: bool,
}

impl DropdownMenuItem {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            icon: None,
            shortcut: None,
            disabled: false,
            destructive: false,
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

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn destructive(mut self, destructive: bool) -> Self {
        self.destructive = destructive;
        self
    }
}

impl DropdownMenu {
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            width: px(200.0),
            label: None,
        }
    }

    pub fn item(mut self, item: DropdownMenuItem) -> Self {
        self.items.push(DropdownMenuEntry::Item(item));
        self
    }

    pub fn separator(mut self) -> Self {
        self.items.push(DropdownMenuEntry::Separator);
        self
    }

    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.items.push(DropdownMenuEntry::Label(label.into()));
        self
    }

    #[allow(dead_code)]
    pub fn width(mut self, width: gpui::Pixels) -> Self {
        self.width = width;
        self
    }

    pub fn render(self, theme: &Theme) -> impl IntoElement {
        let mut menu = div()
            .flex()
            .flex_col()
            .w(self.width)
            .py(px(4.0))
            .rounded(Radius::LG)
            .bg(theme.popover)
            .border_1()
            .border_color(theme.border);

        for entry in self.items {
            match entry {
                DropdownMenuEntry::Item(item) => {
                    let text_color = if item.destructive {
                        theme.destructive
                    } else if item.disabled {
                        theme.muted_foreground
                    } else {
                        theme.popover_foreground
                    };
                    let hover_bg = theme.accent;

                    let mut el = div()
                        .flex()
                        .items_center()
                        .gap(px(8.0))
                        .px(px(8.0))
                        .py(px(6.0))
                        .mx(px(4.0))
                        .rounded(Radius::SM)
                        .text_size(px(14.0))
                        .text_color(text_color);

                    if !item.disabled {
                        el = el.cursor_pointer().hover(move |style| style.bg(hover_bg));
                    } else {
                        el = el.opacity(0.5);
                    }

                    if let Some(icon) = item.icon {
                        el = el.child(div().w(px(16.0)).text_color(text_color).child(icon));
                    }

                    el = el.child(div().flex_1().child(item.label));

                    if let Some(shortcut) = item.shortcut {
                        el = el.child(
                            div()
                                .text_size(px(12.0))
                                .text_color(theme.muted_foreground)
                                .child(shortcut),
                        );
                    }

                    menu = menu.child(el);
                }
                DropdownMenuEntry::Separator => {
                    menu = menu.child(div().my(px(4.0)).mx(px(-4.0)).h(px(1.0)).bg(theme.border));
                }
                DropdownMenuEntry::Label(label) => {
                    menu = menu.child(
                        div()
                            .px(px(8.0))
                            .py(px(6.0))
                            .text_size(px(12.0))
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .text_color(theme.foreground)
                            .child(label),
                    );
                }
            }
        }

        menu
    }
}

// ─── ContextMenu ────────────────────────────────────────────────────────────
// An alias for DropdownMenu styled as a context menu.
pub type ContextMenu = DropdownMenu;
pub type ContextMenuItem = DropdownMenuItem;

// ─── Command (Command Palette Item) ─────────────────────────────────────────
// A command palette item (like shadcn Command).

pub struct CommandItem {
    icon: Option<String>,
    label: String,
    shortcut: Option<String>,
    group: Option<String>,
}

impl CommandItem {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            icon: None,
            label: label.into(),
            shortcut: None,
            group: None,
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

    #[allow(dead_code)]
    pub fn group(mut self, group: impl Into<String>) -> Self {
        self.group = Some(group.into());
        self
    }

    pub fn render(self, theme: &Theme) -> impl IntoElement {
        let hover_bg = theme.accent;

        let mut el = div()
            .flex()
            .items_center()
            .gap(px(8.0))
            .px(px(8.0))
            .py(px(8.0))
            .mx(px(4.0))
            .rounded(Radius::SM)
            .cursor_pointer()
            .hover(move |style| style.bg(hover_bg));

        if let Some(icon) = self.icon {
            el = el.child(
                div()
                    .w(px(16.0))
                    .text_size(px(14.0))
                    .text_color(theme.muted_foreground)
                    .child(icon),
            );
        }

        el = el.child(
            div()
                .flex_1()
                .text_size(px(14.0))
                .text_color(theme.foreground)
                .child(self.label),
        );

        if let Some(shortcut) = self.shortcut {
            el = el.child(
                div().text_size(px(12.0)).text_color(theme.muted_foreground).child(shortcut),
            );
        }

        el
    }
}

// ─── CommandPalette ─────────────────────────────────────────────────────────
// The full command palette with search.

pub struct CommandPalette {
    placeholder: String,
    items: Vec<CommandPaletteGroup>,
    search_value: String,
}

struct CommandPaletteGroup {
    label: String,
    items: Vec<CommandItem>,
}

impl CommandPalette {
    pub fn new() -> Self {
        Self {
            placeholder: "Type a command or search...".to_string(),
            items: Vec::new(),
            search_value: String::new(),
        }
    }

    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = placeholder.into();
        self
    }

    pub fn group(mut self, label: impl Into<String>, items: Vec<CommandItem>) -> Self {
        self.items.push(CommandPaletteGroup {
            label: label.into(),
            items,
        });
        self
    }

    #[allow(dead_code)]
    pub fn search_value(mut self, value: impl Into<String>) -> Self {
        self.search_value = value.into();
        self
    }

    pub fn render(self, theme: &Theme) -> impl IntoElement {
        let mut palette = div()
            .flex()
            .flex_col()
            .w(px(480.0))
            .max_h(px(400.0))
            .rounded(Radius::LG)
            .bg(theme.popover)
            .border_1()
            .border_color(theme.border)
            .overflow_hidden();

        // Search input
        palette = palette.child(
            div()
                .flex()
                .items_center()
                .gap(px(8.0))
                .px(px(12.0))
                .py(px(12.0))
                .border_b_1()
                .border_color(theme.border)
                .child(div().text_size(px(14.0)).text_color(theme.muted_foreground).child("🔍"))
                .child(
                    div()
                        .flex_1()
                        .text_size(px(14.0))
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

        // Groups
        let mut content = div().flex().flex_col().overflow_y_hidden().py(px(4.0));

        for group in self.items {
            // Group label
            content = content.child(
                div()
                    .px(px(8.0))
                    .py(px(6.0))
                    .text_size(px(12.0))
                    .font_weight(gpui::FontWeight::MEDIUM)
                    .text_color(theme.muted_foreground)
                    .child(group.label),
            );

            for item in group.items {
                content = content.child(item.render(theme));
            }
        }

        palette = palette.child(content);
        palette
    }
}
