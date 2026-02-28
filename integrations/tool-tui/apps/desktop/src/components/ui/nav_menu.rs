use gpui::{div, prelude::*, px, IntoElement};

use crate::theme::{colors::Radius, Theme};

// ─── NavMenu ────────────────────────────────────────────────────────────────
// Desktop-native navigation menu with sections, items, and nesting support.
// Used for application menus, settings panels, and navigation sidebars.
//
// Usage:
//   NavMenu::new()
//       .section(NavSection::new("General")
//           .item(NavItem::new("dashboard", "Dashboard").icon("📊").active(true))
//           .item(NavItem::new("settings", "Settings").icon("⚙"))
//       )
//       .render(&theme)

pub struct NavMenu {
    sections: Vec<NavSection>,
    collapsed: bool,
}

pub struct NavSection {
    label: Option<String>,
    items: Vec<NavItem>,
}

pub struct NavItem {
    id: String,
    label: String,
    icon: Option<String>,
    active: bool,
    disabled: bool,
    badge: Option<NavBadge>,
    children: Vec<NavItem>,
    expanded: bool,
}

#[derive(Clone)]
pub struct NavBadge {
    text: String,
    variant: NavBadgeVariant,
}

#[derive(Clone, Copy)]
pub enum NavBadgeVariant {
    Default,
    #[allow(dead_code)]
    Success,
    #[allow(dead_code)]
    Warning,
    #[allow(dead_code)]
    Destructive,
}

#[allow(dead_code)]
impl NavBadge {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            variant: NavBadgeVariant::Default,
        }
    }

    pub fn variant(mut self, variant: NavBadgeVariant) -> Self {
        self.variant = variant;
        self
    }
}

#[allow(dead_code)]
impl NavItem {
    pub fn new(id: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            icon: None,
            active: false,
            disabled: false,
            badge: None,
            children: Vec::new(),
            expanded: false,
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

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn badge(mut self, badge: NavBadge) -> Self {
        self.badge = Some(badge);
        self
    }

    pub fn child(mut self, item: NavItem) -> Self {
        self.children.push(item);
        self
    }

    pub fn expanded(mut self, expanded: bool) -> Self {
        self.expanded = expanded;
        self
    }
}

#[allow(dead_code)]
impl NavSection {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: Some(label.into()),
            items: Vec::new(),
        }
    }

    pub fn unlabeled() -> Self {
        Self {
            label: None,
            items: Vec::new(),
        }
    }

    pub fn item(mut self, item: NavItem) -> Self {
        self.items.push(item);
        self
    }
}

#[allow(dead_code)]
impl NavMenu {
    pub fn new() -> Self {
        Self {
            sections: Vec::new(),
            collapsed: false,
        }
    }

    pub fn section(mut self, section: NavSection) -> Self {
        self.sections.push(section);
        self
    }

    pub fn collapsed(mut self, collapsed: bool) -> Self {
        self.collapsed = collapsed;
        self
    }

    pub fn render(self, theme: &Theme) -> impl IntoElement {
        let mut menu = div().flex().flex_col().gap(px(16.0)).py(px(8.0));

        for section in self.sections {
            menu = menu.child(Self::render_section(&section, self.collapsed, theme));
        }

        menu
    }

    fn render_section(section: &NavSection, collapsed: bool, theme: &Theme) -> impl IntoElement {
        let mut container = div().flex().flex_col().gap(px(2.0));

        // Section label
        if !collapsed {
            if let Some(ref label) = section.label {
                container = container.child(
                    div()
                        .px(px(12.0))
                        .py(px(4.0))
                        .text_size(px(11.0))
                        .font_weight(gpui::FontWeight::MEDIUM)
                        .text_color(theme.muted_foreground)
                        .child(label.to_uppercase()),
                );
            }
        }

        // Items
        for item in &section.items {
            container = Self::render_item_into(container, item, collapsed, 0, theme);
        }

        container
    }

    fn render_item_into(
        mut parent: gpui::Div,
        item: &NavItem,
        collapsed: bool,
        depth: u32,
        theme: &Theme,
    ) -> gpui::Div {
        let indent = depth as f32 * 16.0;
        let bg = if item.active {
            theme.accent
        } else {
            gpui::transparent_black()
        };
        let fg = if item.active {
            theme.accent_foreground
        } else if item.disabled {
            theme.muted_foreground
        } else {
            theme.foreground
        };
        let hover_bg = theme.accent;
        let has_children = !item.children.is_empty();

        let mut row = div()
            .flex()
            .items_center()
            .gap(px(8.0))
            .w_full()
            .h(px(32.0))
            .px(px(12.0))
            .pl(px(12.0 + indent))
            .rounded(Radius::MD)
            .bg(bg)
            .text_color(fg)
            .text_size(px(13.0))
            .mx(px(4.0));

        if !item.disabled {
            row = row.cursor_pointer().hover(move |style| style.bg(hover_bg));
        } else {
            row = row.opacity(0.5);
        }

        // Icon
        if let Some(ref icon) = item.icon {
            row = row.child(div().flex_shrink_0().text_size(px(16.0)).child(icon.clone()));
        }

        // Label (hidden when collapsed)
        if !collapsed {
            row = row.child(
                div()
                    .flex_1()
                    .overflow_hidden()
                    .text_ellipsis()
                    .whitespace_nowrap()
                    .child(item.label.clone()),
            );

            // Expand arrow for parent items
            if has_children {
                let arrow = if item.expanded { "▼" } else { "▶" };
                row = row.child(
                    div().text_size(px(8.0)).text_color(theme.muted_foreground).child(arrow),
                );
            }

            // Badge
            if let Some(ref badge) = item.badge {
                let badge_bg = match badge.variant {
                    NavBadgeVariant::Default => theme.muted,
                    NavBadgeVariant::Success => theme.success,
                    NavBadgeVariant::Warning => theme.warning,
                    NavBadgeVariant::Destructive => theme.destructive,
                };
                let badge_fg = match badge.variant {
                    NavBadgeVariant::Default => theme.muted_foreground,
                    NavBadgeVariant::Success => theme.success_foreground,
                    NavBadgeVariant::Warning => theme.warning_foreground,
                    NavBadgeVariant::Destructive => theme.destructive_foreground,
                };

                row = row.child(
                    div()
                        .flex_shrink_0()
                        .px(px(6.0))
                        .py(px(1.0))
                        .rounded(Radius::FULL)
                        .bg(badge_bg)
                        .text_color(badge_fg)
                        .text_size(px(10.0))
                        .font_weight(gpui::FontWeight::MEDIUM)
                        .child(badge.text.clone()),
                );
            }
        }

        parent = parent.child(row);

        // Render children if expanded
        if has_children && item.expanded && !collapsed {
            for child in &item.children {
                parent = Self::render_item_into(parent, child, collapsed, depth + 1, theme);
            }
        }

        parent
    }
}
