use gpui::{div, prelude::*, px, Hsla, IntoElement};

use crate::theme::{colors::Radius, Theme};

// ─── ColorSwatch ────────────────────────────────────────────────────────────
// Displays a single color sample with optional label and hex value.
//
// Usage:
//   ColorSwatch::new(theme.primary)
//       .label("Primary")
//       .size(32.0)
//       .render(&theme)

pub struct ColorSwatch {
    color: Hsla,
    label: Option<String>,
    hex_label: Option<String>,
    size: f32,
    selected: bool,
    rounded: bool,
}

#[allow(dead_code)]
impl ColorSwatch {
    pub fn new(color: Hsla) -> Self {
        Self {
            color,
            label: None,
            hex_label: None,
            size: 32.0,
            selected: false,
            rounded: false,
        }
    }

    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn hex_label(mut self, hex: impl Into<String>) -> Self {
        self.hex_label = Some(hex.into());
        self
    }

    pub fn size(mut self, s: f32) -> Self {
        self.size = s;
        self
    }

    pub fn selected(mut self, v: bool) -> Self {
        self.selected = v;
        self
    }

    pub fn rounded(mut self, v: bool) -> Self {
        self.rounded = v;
        self
    }

    pub fn render(self, theme: &Theme) -> impl IntoElement {
        let mut container = div().flex().flex_col().items_center().gap(px(4.0));

        let radius = if self.rounded {
            Radius::FULL
        } else {
            Radius::MD
        };

        let mut swatch = div()
            .w(px(self.size))
            .h(px(self.size))
            .rounded(radius)
            .bg(self.color)
            .cursor_pointer();

        if self.selected {
            swatch = swatch.border_2().border_color(theme.ring);
        }

        container = container.child(swatch);

        if let Some(label) = self.label {
            container = container
                .child(div().text_size(px(10.0)).text_color(theme.foreground).child(label));
        }

        if let Some(hex) = self.hex_label {
            container = container
                .child(div().text_size(px(9.0)).text_color(theme.muted_foreground).child(hex));
        }

        container
    }
}

// ─── ColorPalette ───────────────────────────────────────────────────────────
// Grid of color swatches for theme/color selection.

pub struct ColorPalette {
    colors: Vec<(Hsla, Option<String>)>,
    selected: Option<usize>,
    swatch_size: f32,
    columns: usize,
    rounded: bool,
}

#[allow(dead_code)]
impl ColorPalette {
    pub fn new() -> Self {
        Self {
            colors: Vec::new(),
            selected: None,
            swatch_size: 28.0,
            columns: 8,
            rounded: false,
        }
    }

    pub fn color(mut self, color: Hsla, label: Option<String>) -> Self {
        self.colors.push((color, label));
        self
    }

    pub fn colors(mut self, colors: Vec<Hsla>) -> Self {
        self.colors = colors.into_iter().map(|c| (c, None)).collect();
        self
    }

    pub fn selected(mut self, idx: usize) -> Self {
        self.selected = Some(idx);
        self
    }

    pub fn swatch_size(mut self, s: f32) -> Self {
        self.swatch_size = s;
        self
    }

    pub fn columns(mut self, c: usize) -> Self {
        self.columns = c;
        self
    }

    pub fn rounded(mut self, v: bool) -> Self {
        self.rounded = v;
        self
    }

    pub fn render(self, theme: &Theme) -> impl IntoElement {
        let radius = if self.rounded {
            Radius::FULL
        } else {
            Radius::SM
        };

        let mut grid = div()
            .flex()
            .flex_wrap()
            .gap(px(4.0))
            .max_w(px((self.swatch_size + 4.0) * self.columns as f32));

        for (i, (color, _label)) in self.colors.into_iter().enumerate() {
            let is_selected = self.selected == Some(i);
            let size = self.swatch_size;

            let mut swatch =
                div().w(px(size)).h(px(size)).rounded(radius).bg(color).cursor_pointer();

            if is_selected {
                swatch = swatch.border_2().border_color(theme.ring);
            }

            grid = grid.child(swatch);
        }

        grid
    }
}

// ─── Gradient ───────────────────────────────────────────────────────────────
// Renders a simple gradient bar between two colors (uses layered divs).

pub struct GradientBar {
    from: Hsla,
    to: Hsla,
    height: f32,
    steps: usize,
}

#[allow(dead_code)]
impl GradientBar {
    pub fn new(from: Hsla, to: Hsla) -> Self {
        Self {
            from,
            to,
            height: 24.0,
            steps: 20,
        }
    }

    pub fn height(mut self, h: f32) -> Self {
        self.height = h;
        self
    }

    pub fn steps(mut self, s: usize) -> Self {
        self.steps = s.max(2);
        self
    }

    pub fn render(self) -> impl IntoElement {
        let mut bar =
            div().flex().w_full().h(px(self.height)).rounded(Radius::SM).overflow_hidden();

        for i in 0..self.steps {
            let t = i as f32 / (self.steps - 1) as f32;
            let h = self.from.h + (self.to.h - self.from.h) * t;
            let s = self.from.s + (self.to.s - self.from.s) * t;
            let l = self.from.l + (self.to.l - self.from.l) * t;
            let a = self.from.a + (self.to.a - self.from.a) * t;
            let color = gpui::hsla(h, s, l, a);
            bar = bar.child(div().flex_1().h_full().bg(color));
        }

        bar
    }
}

// ─── ThemePreview ───────────────────────────────────────────────────────────
// Shows a compact preview of a theme's key colors.

pub struct ThemePreview {
    name: String,
    background: Hsla,
    foreground: Hsla,
    primary: Hsla,
    secondary: Hsla,
    accent: Hsla,
    selected: bool,
}

#[allow(dead_code)]
impl ThemePreview {
    pub fn new(name: impl Into<String>, theme: &Theme) -> Self {
        Self {
            name: name.into(),
            background: theme.background,
            foreground: theme.foreground,
            primary: theme.primary,
            secondary: theme.secondary,
            accent: theme.accent,
            selected: false,
        }
    }

    pub fn selected(mut self, v: bool) -> Self {
        self.selected = v;
        self
    }

    pub fn render(self, theme: &Theme) -> impl IntoElement {
        let mut card = div()
            .flex()
            .flex_col()
            .gap(px(6.0))
            .p(px(8.0))
            .w(px(120.0))
            .bg(theme.card)
            .border_1()
            .border_color(if self.selected {
                theme.ring
            } else {
                theme.border
            })
            .rounded(Radius::MD)
            .cursor_pointer();

        if self.selected {
            card = card.border_2();
        }

        // Color strip
        card = card.child(
            div()
                .flex()
                .gap(px(2.0))
                .h(px(20.0))
                .rounded(Radius::SM)
                .overflow_hidden()
                .child(div().flex_1().bg(self.background))
                .child(div().flex_1().bg(self.primary))
                .child(div().flex_1().bg(self.secondary))
                .child(div().flex_1().bg(self.accent)),
        );

        // Name
        card = card.child(
            div()
                .text_size(px(11.0))
                .text_color(theme.foreground)
                .text_center()
                .child(self.name),
        );

        card
    }
}
