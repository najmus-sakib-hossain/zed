use gpui::{div, prelude::*, px, Hsla, IntoElement};

use crate::theme::{colors::Radius, Theme};

use super::helpers::with_alpha;

// ─── KeyboardShortcut ───────────────────────────────────────────────────────
// Display keyboard shortcuts in a readable format.
// Common in desktop apps for menus, tooltips, and command palettes.
//
// Usage:
//   KeyboardShortcut::new(&["Ctrl", "Shift", "P"]).render(&theme)
//   KeyboardShortcut::mac(&["⌘", "⇧", "P"]).render(&theme)

pub struct KeyboardShortcut {
    keys: Vec<String>,
    separator: String,
    compact: bool,
}

#[allow(dead_code)]
impl KeyboardShortcut {
    pub fn new(keys: &[&str]) -> Self {
        Self {
            keys: keys.iter().map(|k| (*k).to_string()).collect(),
            separator: "+".to_string(),
            compact: false,
        }
    }

    pub fn mac(keys: &[&str]) -> Self {
        Self {
            keys: keys.iter().map(|k| (*k).to_string()).collect(),
            separator: String::new(),
            compact: true,
        }
    }

    pub fn separator(mut self, sep: impl Into<String>) -> Self {
        self.separator = sep.into();
        self
    }

    pub fn compact(mut self, compact: bool) -> Self {
        self.compact = compact;
        self
    }

    pub fn render(self, theme: &Theme) -> impl IntoElement {
        let mut container = div().flex().items_center().gap(px(2.0));

        for (i, key) in self.keys.iter().enumerate() {
            // Separator between keys
            if i > 0 && !self.separator.is_empty() {
                container = container.child(
                    div()
                        .text_size(px(10.0))
                        .text_color(theme.muted_foreground)
                        .child(self.separator.clone()),
                );
            }

            let padding = if self.compact { px(3.0) } else { px(6.0) };

            container = container.child(
                div()
                    .flex()
                    .items_center()
                    .justify_center()
                    .min_w(px(20.0))
                    .h(px(20.0))
                    .px(padding)
                    .rounded(Radius::SM)
                    .bg(theme.muted)
                    .border_1()
                    .border_color(theme.border)
                    .border_b_2()
                    .text_size(px(11.0))
                    .font_weight(gpui::FontWeight::MEDIUM)
                    .text_color(theme.muted_foreground)
                    .child(key.clone()),
            );
        }

        container
    }
}

// ─── Chip ───────────────────────────────────────────────────────────────────
// Small interactive tag/chip for filters, selections, etc.
//
// Usage:
//   Chip::new("Rust").removable(true).render(&theme)

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ChipVariant {
    Default,
    Primary,
    Secondary,
    Outline,
}

pub struct Chip {
    label: String,
    variant: ChipVariant,
    icon: Option<String>,
    removable: bool,
    selected: bool,
}

#[allow(dead_code)]
impl Chip {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            variant: ChipVariant::Default,
            icon: None,
            removable: false,
            selected: false,
        }
    }

    pub fn variant(mut self, variant: ChipVariant) -> Self {
        self.variant = variant;
        self
    }

    pub fn icon(mut self, icon: impl Into<String>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    pub fn removable(mut self, removable: bool) -> Self {
        self.removable = removable;
        self
    }

    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    pub fn render(self, theme: &Theme) -> impl IntoElement {
        let (bg, fg, border) = match self.variant {
            ChipVariant::Default => (theme.muted, theme.foreground, theme.border),
            ChipVariant::Primary => (theme.primary, theme.primary_foreground, theme.primary),
            ChipVariant::Secondary => {
                (theme.secondary, theme.secondary_foreground, theme.secondary)
            }
            ChipVariant::Outline => (gpui::transparent_black(), theme.foreground, theme.border),
        };

        let bg = if self.selected { theme.primary } else { bg };
        let fg = if self.selected {
            theme.primary_foreground
        } else {
            fg
        };

        let hover_bg = with_alpha(bg, 0.8);

        let mut chip = div()
            .flex()
            .items_center()
            .gap(px(4.0))
            .h(px(24.0))
            .px(px(8.0))
            .rounded(Radius::FULL)
            .bg(bg)
            .text_color(fg)
            .border_1()
            .border_color(border)
            .text_size(px(12.0))
            .cursor_pointer()
            .hover(move |style| style.bg(hover_bg));

        if let Some(icon) = self.icon {
            chip = chip.child(div().text_size(px(12.0)).child(icon));
        }

        chip = chip.child(self.label);

        if self.removable {
            chip = chip.child(
                div().text_size(px(10.0)).text_color(fg).ml(px(2.0)).cursor_pointer().child("✕"),
            );
        }

        chip
    }
}

// ─── Tag ────────────────────────────────────────────────────────────────────
// Colored tag for categorization (similar to GitHub labels).

pub struct Tag {
    label: String,
    color: Option<Hsla>,
}

#[allow(dead_code)]
impl Tag {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            color: None,
        }
    }

    pub fn color(mut self, color: Hsla) -> Self {
        self.color = Some(color);
        self
    }

    pub fn render(self, theme: &Theme) -> impl IntoElement {
        let color = self.color.unwrap_or(theme.primary);
        let bg = with_alpha(color, 0.15);

        div()
            .flex()
            .items_center()
            .h(px(20.0))
            .px(px(8.0))
            .rounded(Radius::FULL)
            .bg(bg)
            .text_color(color)
            .text_size(px(11.0))
            .font_weight(gpui::FontWeight::MEDIUM)
            .child(self.label)
    }
}

// ─── DotIndicator ───────────────────────────────────────────────────────────
// Small colored dot indicator (online status, notifications, etc.)

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DotStatus {
    Online,
    Offline,
    Busy,
    Away,
    Custom,
}

pub struct DotIndicator {
    status: DotStatus,
    color: Option<Hsla>,
    size: f32,
    pulse: bool,
}

#[allow(dead_code)]
impl DotIndicator {
    pub fn new(status: DotStatus) -> Self {
        Self {
            status,
            color: None,
            size: 8.0,
            pulse: false,
        }
    }

    pub fn online() -> Self {
        Self::new(DotStatus::Online)
    }

    pub fn offline() -> Self {
        Self::new(DotStatus::Offline)
    }

    pub fn busy() -> Self {
        Self::new(DotStatus::Busy)
    }

    pub fn away() -> Self {
        Self::new(DotStatus::Away)
    }

    pub fn color(mut self, color: Hsla) -> Self {
        self.color = Some(color);
        self.status = DotStatus::Custom;
        self
    }

    pub fn size(mut self, size: f32) -> Self {
        self.size = size;
        self
    }

    pub fn pulse(mut self, pulse: bool) -> Self {
        self.pulse = pulse;
        self
    }

    pub fn render(self, theme: &Theme) -> impl IntoElement {
        let color = self.color.unwrap_or(match self.status {
            DotStatus::Online => theme.success,
            DotStatus::Offline => theme.muted_foreground,
            DotStatus::Busy => theme.destructive,
            DotStatus::Away => theme.warning,
            DotStatus::Custom => theme.primary,
        });

        div().size(px(self.size)).rounded(Radius::FULL).bg(color).flex_shrink_0()
    }
}
