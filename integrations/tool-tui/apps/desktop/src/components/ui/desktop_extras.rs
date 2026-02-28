use gpui::{div, prelude::*, px, AnyElement, IntoElement};

use crate::theme::{colors::Radius, Theme};

use super::helpers::with_alpha;

// ─── KeyCombo ───────────────────────────────────────────────────────────────
// Platform-aware keyboard shortcut display using real OS modifier symbols.
//
// Usage:
//   KeyCombo::new(&["Ctrl", "Shift", "P"]).render(&theme)
//   KeyCombo::mac(&["⌘", "⇧", "P"]).render(&theme)

pub struct KeyCombo {
    keys: Vec<String>,
    separator: String,
    compact: bool,
}

#[allow(dead_code)]
impl KeyCombo {
    pub fn new(keys: &[&str]) -> Self {
        Self {
            keys: keys.iter().map(|k| (*k).to_string()).collect(),
            separator: " ".into(),
            compact: false,
        }
    }

    pub fn mac(keys: &[&str]) -> Self {
        Self::new(keys)
    }

    pub fn separator(mut self, s: impl Into<String>) -> Self {
        self.separator = s.into();
        self
    }

    pub fn compact(mut self, v: bool) -> Self {
        self.compact = v;
        self
    }

    pub fn render(self, theme: &Theme) -> impl IntoElement {
        let gap = if self.compact { px(2.0) } else { px(4.0) };

        let mut row = div().flex().items_center().gap(gap);

        for (i, key) in self.keys.iter().enumerate() {
            if i > 0 && self.separator != " " {
                row = row.child(
                    div()
                        .text_xs()
                        .text_color(theme.muted_foreground)
                        .child(self.separator.clone()),
                );
            }

            let pad = if self.compact { px(4.0) } else { px(6.0) };

            row = row.child(
                div()
                    .px(pad)
                    .py(px(2.0))
                    .rounded(Radius::SM)
                    .bg(theme.muted)
                    .border_1()
                    .border_color(theme.border)
                    .text_xs()
                    .font_weight(gpui::FontWeight::MEDIUM)
                    .text_color(theme.muted_foreground)
                    .child(key.clone()),
            );
        }

        row
    }
}

// ─── Spinner ────────────────────────────────────────────────────────────────
// Indeterminate loading spinner with arc segments.
//
// Usage:
//   Spinner::new().size(24.0).render(&theme)

pub struct Spinner {
    size: f32,
    label: Option<String>,
}

#[allow(dead_code)]
impl Spinner {
    pub fn new() -> Self {
        Self {
            size: 20.0,
            label: None,
        }
    }

    pub fn size(mut self, s: f32) -> Self {
        self.size = s;
        self
    }

    pub fn label(mut self, l: impl Into<String>) -> Self {
        self.label = Some(l.into());
        self
    }

    pub fn render(self, theme: &Theme) -> impl IntoElement {
        let mut wrapper = div().flex().items_center().gap(px(8.0));

        // Spinner "ring" – we simulate with a bordered circle + partial border
        // (GPUI doesn't have CSS border-top-color, so we render concentric
        // circles: outer = muted ring, inner = primary arc placeholder)
        let outer = div()
            .w(px(self.size))
            .h(px(self.size))
            .rounded(Radius::FULL)
            .border_2()
            .border_color(with_alpha(theme.primary, 0.3))
            .flex()
            .items_center()
            .justify_center()
            .child(
                div()
                    .w(px(self.size * 0.5))
                    .h(px(self.size * 0.5))
                    .rounded(Radius::FULL)
                    .bg(theme.primary),
            );

        wrapper = wrapper.child(outer);

        if let Some(label) = self.label {
            wrapper =
                wrapper.child(div().text_sm().text_color(theme.muted_foreground).child(label));
        }

        wrapper
    }
}

// ─── Banner ─────────────────────────────────────────────────────────────────
// Full-width informational banner for announcements or warnings.
//
// Usage:
//   Banner::info("New update available!")
//       .action_label("Update now")
//       .dismissible(true)
//       .render(&theme)

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BannerVariant {
    Info,
    Success,
    Warning,
    Error,
}

pub struct Banner {
    message: String,
    variant: BannerVariant,
    icon: Option<String>,
    action_label: Option<String>,
    dismissible: bool,
}

#[allow(dead_code)]
impl Banner {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            variant: BannerVariant::Info,
            icon: None,
            action_label: None,
            dismissible: false,
        }
    }

    pub fn info(message: impl Into<String>) -> Self {
        Self::new(message).variant(BannerVariant::Info)
    }

    pub fn success(message: impl Into<String>) -> Self {
        Self::new(message).variant(BannerVariant::Success)
    }

    pub fn warning(message: impl Into<String>) -> Self {
        Self::new(message).variant(BannerVariant::Warning)
    }

    pub fn error(message: impl Into<String>) -> Self {
        Self::new(message).variant(BannerVariant::Error)
    }

    pub fn variant(mut self, v: BannerVariant) -> Self {
        self.variant = v;
        self
    }

    pub fn icon(mut self, icon: impl Into<String>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    pub fn action_label(mut self, label: impl Into<String>) -> Self {
        self.action_label = Some(label.into());
        self
    }

    pub fn dismissible(mut self, v: bool) -> Self {
        self.dismissible = v;
        self
    }

    fn variant_colors(&self, theme: &Theme) -> (gpui::Hsla, gpui::Hsla) {
        match self.variant {
            BannerVariant::Info => (with_alpha(theme.info, 0.15), theme.info),
            BannerVariant::Success => (with_alpha(theme.success, 0.15), theme.success),
            BannerVariant::Warning => (with_alpha(theme.warning, 0.15), theme.warning),
            BannerVariant::Error => (with_alpha(theme.destructive, 0.15), theme.destructive),
        }
    }

    fn default_icon(&self) -> &'static str {
        match self.variant {
            BannerVariant::Info => "ℹ",
            BannerVariant::Success => "✓",
            BannerVariant::Warning => "⚠",
            BannerVariant::Error => "✕",
        }
    }

    pub fn render(self, theme: &Theme) -> impl IntoElement {
        let (bg, accent) = self.variant_colors(theme);
        let icon = self.icon.clone().unwrap_or_else(|| self.default_icon().into());

        let mut row = div()
            .flex()
            .items_center()
            .gap(px(10.0))
            .w_full()
            .px(px(16.0))
            .py(px(10.0))
            .bg(bg)
            .border_l(px(3.0))
            .border_color(accent);

        row = row.child(div().text_sm().text_color(accent).child(icon));

        row = row.child(div().flex_1().text_sm().text_color(theme.foreground).child(self.message));

        if let Some(action) = self.action_label {
            row = row.child(
                div()
                    .text_sm()
                    .font_weight(gpui::FontWeight::MEDIUM)
                    .text_color(accent)
                    .cursor_pointer()
                    .child(action),
            );
        }

        if self.dismissible {
            row = row.child(
                div()
                    .text_sm()
                    .text_color(theme.muted_foreground)
                    .cursor_pointer()
                    .hover(move |s| s.text_color(theme.foreground))
                    .child("✕"),
            );
        }

        row
    }
}

// ─── Callout ────────────────────────────────────────────────────────────────
// Info/tip/warning/danger callout block (like GitHub docs callouts).
//
// Usage:
//   Callout::tip("Pro Tip")
//       .description("You can press Ctrl+K to open the command bar.")
//       .render(&theme)

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CalloutVariant {
    Note,
    Tip,
    Important,
    Warning,
    Caution,
}

pub struct Callout {
    title: String,
    description: Option<String>,
    variant: CalloutVariant,
    children: Vec<AnyElement>,
}

#[allow(dead_code)]
impl Callout {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            description: None,
            variant: CalloutVariant::Note,
            children: Vec::new(),
        }
    }

    pub fn note(title: impl Into<String>) -> Self {
        Self::new(title).variant(CalloutVariant::Note)
    }

    pub fn tip(title: impl Into<String>) -> Self {
        Self::new(title).variant(CalloutVariant::Tip)
    }

    pub fn important(title: impl Into<String>) -> Self {
        Self::new(title).variant(CalloutVariant::Important)
    }

    pub fn warning(title: impl Into<String>) -> Self {
        Self::new(title).variant(CalloutVariant::Warning)
    }

    pub fn caution(title: impl Into<String>) -> Self {
        Self::new(title).variant(CalloutVariant::Caution)
    }

    pub fn variant(mut self, v: CalloutVariant) -> Self {
        self.variant = v;
        self
    }

    pub fn description(mut self, d: impl Into<String>) -> Self {
        self.description = Some(d.into());
        self
    }

    pub fn child(mut self, child: impl IntoElement + 'static) -> Self {
        self.children.push(child.into_any_element());
        self
    }

    fn variant_colors(&self, theme: &Theme) -> (gpui::Hsla, gpui::Hsla) {
        match self.variant {
            CalloutVariant::Note => (with_alpha(theme.info, 0.10), theme.info),
            CalloutVariant::Tip => (with_alpha(theme.success, 0.10), theme.success),
            CalloutVariant::Important => (with_alpha(theme.primary, 0.10), theme.primary),
            CalloutVariant::Warning => (with_alpha(theme.warning, 0.10), theme.warning),
            CalloutVariant::Caution => (with_alpha(theme.destructive, 0.10), theme.destructive),
        }
    }

    fn variant_icon(&self) -> &'static str {
        match self.variant {
            CalloutVariant::Note => "ℹ",
            CalloutVariant::Tip => "💡",
            CalloutVariant::Important => "❗",
            CalloutVariant::Warning => "⚠",
            CalloutVariant::Caution => "🔴",
        }
    }

    pub fn render(self, theme: &Theme) -> impl IntoElement {
        let (bg, accent) = self.variant_colors(theme);

        let mut block = div()
            .flex()
            .flex_col()
            .gap(px(6.0))
            .w_full()
            .p(px(12.0))
            .rounded(Radius::MD)
            .bg(bg)
            .border_l(px(3.0))
            .border_color(accent);

        // Header
        block = block.child(
            div()
                .flex()
                .items_center()
                .gap(px(6.0))
                .child(div().text_sm().text_color(accent).child(self.variant_icon()))
                .child(
                    div()
                        .text_sm()
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .text_color(accent)
                        .child(self.title),
                ),
        );

        if let Some(desc) = self.description {
            block =
                block.child(div().text_sm().text_color(theme.foreground).pl(px(22.0)).child(desc));
        }

        for child in self.children {
            block = block.child(div().pl(px(22.0)).child(child));
        }

        block
    }
}

// ─── Code Block ─────────────────────────────────────────────────────────────
// Display code snippets with a header, copy button, and line numbers.
//
// Usage:
//   CodeBlock::new("fn main() {\n    println!(\"hello\");\n}")
//       .language("rust")
//       .show_line_numbers(true)
//       .render(&theme)

pub struct CodeBlock {
    code: String,
    language: Option<String>,
    title: Option<String>,
    show_line_numbers: bool,
    max_height: Option<f32>,
}

#[allow(dead_code)]
impl CodeBlock {
    pub fn new(code: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            language: None,
            title: None,
            show_line_numbers: false,
            max_height: None,
        }
    }

    pub fn language(mut self, lang: impl Into<String>) -> Self {
        self.language = Some(lang.into());
        self
    }

    pub fn title(mut self, t: impl Into<String>) -> Self {
        self.title = Some(t.into());
        self
    }

    pub fn show_line_numbers(mut self, v: bool) -> Self {
        self.show_line_numbers = v;
        self
    }

    pub fn max_height(mut self, h: f32) -> Self {
        self.max_height = Some(h);
        self
    }

    pub fn render(self, theme: &Theme) -> impl IntoElement {
        let mut wrapper = div()
            .flex()
            .flex_col()
            .rounded(Radius::MD)
            .bg(theme.card)
            .border_1()
            .border_color(theme.border)
            .overflow_hidden();

        // Header
        let has_header = self.title.is_some() || self.language.is_some();
        if has_header {
            let mut header = div()
                .flex()
                .items_center()
                .justify_between()
                .px(px(12.0))
                .py(px(6.0))
                .border_b_1()
                .border_color(theme.border)
                .bg(theme.muted);

            let header_text =
                self.title.clone().or_else(|| self.language.clone()).unwrap_or_default();

            header = header.child(
                div()
                    .text_xs()
                    .font_weight(gpui::FontWeight::MEDIUM)
                    .text_color(theme.muted_foreground)
                    .child(header_text),
            );

            // Copy button placeholder
            header = header.child(
                div()
                    .text_xs()
                    .text_color(theme.muted_foreground)
                    .cursor_pointer()
                    .hover(move |s| s.text_color(theme.foreground))
                    .child("📋"),
            );

            wrapper = wrapper.child(header);
        }

        // Code body
        let lines: Vec<&str> = self.code.lines().collect();

        let mut body = div().flex().p(px(12.0)).overflow_hidden();

        if let Some(mh) = self.max_height {
            body = body.max_h(px(mh));
        }

        if self.show_line_numbers {
            let gutter = div()
                .flex()
                .flex_col()
                .pr(px(12.0))
                .mr(px(12.0))
                .border_r_1()
                .border_color(theme.border)
                .children((1..=lines.len()).map(|n| {
                    div()
                        .text_xs()
                        .text_color(theme.muted_foreground)
                        .line_height(px(20.0))
                        .text_right()
                        .w(px(24.0))
                        .child(format!("{n}"))
                }));
            body = body.child(gutter);
        }

        let code_col = div().flex().flex_col().flex_1().children(lines.into_iter().map(|line| {
            div()
                .text_xs()
                .text_color(theme.foreground)
                .line_height(px(20.0))
                .whitespace_nowrap()
                .child(line.to_string())
        }));

        body = body.child(code_col);
        wrapper = wrapper.child(body);

        wrapper
    }
}
