use crate::theme::{colors::Radius, Theme};
use gpui::{div, prelude::*, px, AnyElement, ClickEvent, Hsla, IntoElement};

/// Type alias for click handler to reduce complexity.
type ClickHandler = Box<dyn Fn(&ClickEvent, &mut gpui::Window, &mut gpui::App) + 'static>;

// ─── Button ─────────────────────────────────────────────────────────────────
// A shadcn-ui style Button with variants, sizes, and full interactivity.
//
// Usage:
//   Button::new("Click me")
//       .variant(ButtonVariant::Primary)
//       .size(ButtonSize::Default)
//       .on_click(|cx| { /* handler */ })
//       .render(&theme)

/// Button variant matching shadcn-ui button variants
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ButtonVariant {
    Default,
    Secondary,
    Destructive,
    Outline,
    Ghost,
    Link,
}

/// Button size matching shadcn-ui button sizes
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ButtonSize {
    Sm,
    Default,
    Lg,
    Icon,
}

pub struct Button {
    label: String,
    variant: ButtonVariant,
    size: ButtonSize,
    disabled: bool,
    full_width: bool,
    icon_left: Option<String>,
    icon_right: Option<String>,
    on_click: Option<ClickHandler>,
}

impl Button {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            variant: ButtonVariant::Default,
            size: ButtonSize::Default,
            disabled: false,
            full_width: false,
            icon_left: None,
            icon_right: None,
            on_click: None,
        }
    }

    /// Shorthand constructors for variants
    pub fn primary(label: impl Into<String>) -> Self {
        Self::new(label).variant(ButtonVariant::Default)
    }

    pub fn secondary(label: impl Into<String>) -> Self {
        Self::new(label).variant(ButtonVariant::Secondary)
    }

    pub fn destructive(label: impl Into<String>) -> Self {
        Self::new(label).variant(ButtonVariant::Destructive)
    }

    pub fn outline(label: impl Into<String>) -> Self {
        Self::new(label).variant(ButtonVariant::Outline)
    }

    pub fn ghost(label: impl Into<String>) -> Self {
        Self::new(label).variant(ButtonVariant::Ghost)
    }

    pub fn link(label: impl Into<String>) -> Self {
        Self::new(label).variant(ButtonVariant::Link)
    }

    /// Icon-only button
    pub fn icon(icon: impl Into<String>) -> Self {
        Self::new("")
            .variant(ButtonVariant::Outline)
            .size(ButtonSize::Icon)
            .with_icon_left(icon)
    }

    // ── Builder methods ──

    pub fn variant(mut self, variant: ButtonVariant) -> Self {
        self.variant = variant;
        self
    }

    pub fn size(mut self, size: ButtonSize) -> Self {
        self.size = size;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn full_width(mut self, full: bool) -> Self {
        self.full_width = full;
        self
    }

    pub fn with_icon_left(mut self, icon: impl Into<String>) -> Self {
        self.icon_left = Some(icon.into());
        self
    }

    pub fn with_icon_right(mut self, icon: impl Into<String>) -> Self {
        self.icon_right = Some(icon.into());
        self
    }

    pub fn on_click(
        mut self,
        handler: impl Fn(&ClickEvent, &mut gpui::Window, &mut gpui::App) + 'static,
    ) -> Self {
        self.on_click = Some(Box::new(handler));
        self
    }

    // ── Render ──

    pub fn render(self, theme: &Theme) -> impl IntoElement {
        let (bg, fg, hover_bg, border_color) = self.variant_colors(theme);

        let (h_pad, v_pad, font_size, min_h, gap) = match self.size {
            ButtonSize::Sm => (px(12.0), px(4.0), px(12.0), px(32.0), px(4.0)),
            ButtonSize::Default => (px(16.0), px(8.0), px(14.0), px(36.0), px(8.0)),
            ButtonSize::Lg => (px(24.0), px(8.0), px(14.0), px(40.0), px(8.0)),
            ButtonSize::Icon => (px(0.0), px(0.0), px(14.0), px(36.0), px(0.0)),
        };

        let is_icon = self.size == ButtonSize::Icon;
        let radius = if is_icon { Radius::MD } else { Radius::DEFAULT };
        let disabled = self.disabled;
        let is_link = self.variant == ButtonVariant::Link;
        let has_border = self.variant == ButtonVariant::Outline;

        let mut el = div()
            .id("btn")
            .flex()
            .items_center()
            .justify_center()
            .gap(gap)
            .rounded(radius)
            .bg(bg)
            .text_color(fg)
            .min_h(min_h)
            .text_size(font_size);

        // Size-specific styling
        if is_icon {
            el = el.w(px(36.0)).h(px(36.0));
        } else {
            el = el.px(h_pad).py(v_pad);
        }

        // Full width
        if self.full_width {
            el = el.w_full();
        }

        // Border for outline variant
        if has_border {
            el = el.border_1().border_color(border_color);
        }

        // Link underline
        if is_link {
            // Links rendered with underline text styling
            el = el.bg(gpui::transparent_black());
        }

        // Disabled styling
        if disabled {
            el = el.opacity(0.5);
        } else {
            el = el
                .cursor_pointer()
                .hover(move |style| style.bg(hover_bg))
                .active(move |style| style.opacity(0.9));
        }

        // Icon left
        if let Some(icon) = self.icon_left {
            el = el.child(div().text_color(fg).child(icon));
        }

        // Label
        if !self.label.is_empty() {
            el = el.child(div().font_weight(gpui::FontWeight::MEDIUM).child(self.label));
        }

        // Icon right
        if let Some(icon) = self.icon_right {
            el = el.child(div().text_color(fg).child(icon));
        }

        // On click handler
        if let Some(handler) = self.on_click {
            el = el.on_click(handler);
        }

        el
    }

    fn variant_colors(&self, theme: &Theme) -> (Hsla, Hsla, Hsla, Hsla) {
        match self.variant {
            ButtonVariant::Default => (
                theme.primary,
                theme.primary_foreground,
                with_alpha(theme.primary, 0.9),
                theme.primary,
            ),
            ButtonVariant::Secondary => (
                theme.secondary,
                theme.secondary_foreground,
                with_alpha(theme.secondary, 0.8),
                theme.secondary,
            ),
            ButtonVariant::Destructive => (
                theme.destructive,
                theme.destructive_foreground,
                with_alpha(theme.destructive, 0.9),
                theme.destructive,
            ),
            ButtonVariant::Outline => {
                (gpui::transparent_black(), theme.foreground, theme.accent, theme.border)
            }
            ButtonVariant::Ghost => (
                gpui::transparent_black(),
                theme.foreground,
                theme.accent,
                gpui::transparent_black(),
            ),
            ButtonVariant::Link => (
                gpui::transparent_black(),
                theme.primary,
                gpui::transparent_black(),
                gpui::transparent_black(),
            ),
        }
    }
}

// ─── IconButton (shorthand for common icon buttons) ─────────────────────────

pub struct IconButton;

#[allow(dead_code)]
impl IconButton {
    pub fn create(icon: impl Into<String>) -> Button {
        Button::icon(icon)
    }

    pub fn ghost(icon: impl Into<String>) -> Button {
        Button::new("")
            .variant(ButtonVariant::Ghost)
            .size(ButtonSize::Icon)
            .with_icon_left(icon)
    }
}

// ─── ButtonGroup ────────────────────────────────────────────────────────────

pub struct ButtonGroup {
    children: Vec<AnyElement>,
}

#[allow(dead_code)]
impl ButtonGroup {
    pub fn new() -> Self {
        Self {
            children: Vec::new(),
        }
    }

    pub fn child(mut self, element: impl IntoElement) -> Self {
        self.children.push(element.into_any_element());
        self
    }

    pub fn render(self) -> impl IntoElement {
        let mut group = div().flex().items_center();
        for child in self.children {
            group = group.child(child);
        }
        group
    }
}

/// Helper to create an HSLA color with modified alpha
fn with_alpha(color: Hsla, alpha: f32) -> Hsla {
    Hsla {
        h: color.h,
        s: color.s,
        l: color.l,
        a: alpha,
    }
}
