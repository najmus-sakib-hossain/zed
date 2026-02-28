use gpui::{div, prelude::*, px, Hsla, IntoElement};

use crate::theme::{colors::Radius, Theme};

use super::helpers::with_alpha;

// ─── Minimap ────────────────────────────────────────────────────────────────
// A minimap / code overview widget for text editors.
// Shows a compressed vertical overview of content.
//
// Usage:
//   Minimap::new()
//       .lines(200)
//       .visible_range(50, 80)
//       .render(&theme)

pub struct Minimap {
    total_lines: usize,
    visible_start: usize,
    visible_end: usize,
    width: f32,
    line_colors: Vec<Option<Hsla>>,
}

#[allow(dead_code)]
impl Minimap {
    pub fn new() -> Self {
        Self {
            total_lines: 100,
            visible_start: 0,
            visible_end: 30,
            width: 60.0,
            line_colors: Vec::new(),
        }
    }

    pub fn lines(mut self, total: usize) -> Self {
        self.total_lines = total;
        self
    }

    pub fn visible_range(mut self, start: usize, end: usize) -> Self {
        self.visible_start = start;
        self.visible_end = end;
        self
    }

    pub fn width(mut self, width: f32) -> Self {
        self.width = width;
        self
    }

    pub fn line_color(mut self, line: usize, color: Hsla) -> Self {
        if self.line_colors.len() <= line {
            self.line_colors.resize(line + 1, None);
        }
        self.line_colors[line] = Some(color);
        self
    }

    pub fn render(self, theme: &Theme) -> impl IntoElement {
        let line_height = 2.0_f32;
        let _total_height = self.total_lines as f32 * line_height;
        let viewport_top = self.visible_start as f32 * line_height;
        let viewport_height = (self.visible_end - self.visible_start) as f32 * line_height;
        let default_color = with_alpha(theme.foreground, 0.15);

        let mut minimap = div()
            .relative()
            .w(px(self.width))
            .h_full()
            .bg(theme.background)
            .border_l_1()
            .border_color(theme.border)
            .overflow_hidden()
            .flex_shrink_0();

        // Line representations
        let mut lines_container = div().flex().flex_col().gap(px(1.0)).pt(px(4.0)).px(px(4.0));

        for i in 0..self.total_lines.min(500) {
            let color = self.line_colors.get(i).copied().flatten().unwrap_or(default_color);

            // Vary widths to simulate code
            let width_pct = match i % 7 {
                0 => 0.3,
                1 => 0.7,
                2 => 0.5,
                3 => 0.9,
                4 => 0.4,
                5 => 0.6,
                _ => 0.8,
            };

            lines_container = lines_container.child(
                div()
                    .h(px(line_height))
                    .w(gpui::relative(width_pct as f32))
                    .bg(color)
                    .rounded(Radius::SM),
            );
        }

        minimap = minimap.child(lines_container);

        // Viewport indicator
        minimap = minimap.child(
            div()
                .absolute()
                .top(px(viewport_top + 4.0))
                .left_0()
                .right_0()
                .h(px(viewport_height.max(10.0)))
                .bg(with_alpha(theme.foreground, 0.08))
                .border_1()
                .border_color(with_alpha(theme.foreground, 0.12)),
        );

        minimap
    }
}

// ─── Terminal ───────────────────────────────────────────────────────────────
// Terminal-like display component for logs, output, etc.
//
// Usage:
//   TerminalOutput::new()
//       .line(TermLine::command("$ cargo build"))
//       .line(TermLine::output("   Compiling dx v0.1.0"))
//       .line(TermLine::success("   Finished in 2.3s"))
//       .line(TermLine::error("error[E0382]: use of moved value"))
//       .render(&theme)

pub struct TerminalOutput {
    lines: Vec<TermLine>,
    max_height: Option<f32>,
    show_header: bool,
    title: String,
}

pub struct TermLine {
    text: String,
    kind: TermLineKind,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TermLineKind {
    Command,
    Output,
    Success,
    Error,
    Warning,
    Info,
}

#[allow(dead_code)]
impl TermLine {
    pub fn command(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            kind: TermLineKind::Command,
        }
    }

    pub fn output(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            kind: TermLineKind::Output,
        }
    }

    pub fn success(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            kind: TermLineKind::Success,
        }
    }

    pub fn error(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            kind: TermLineKind::Error,
        }
    }

    pub fn warning(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            kind: TermLineKind::Warning,
        }
    }

    pub fn info(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            kind: TermLineKind::Info,
        }
    }
}

#[allow(dead_code)]
impl TerminalOutput {
    pub fn new() -> Self {
        Self {
            lines: Vec::new(),
            max_height: None,
            show_header: true,
            title: "Terminal".to_string(),
        }
    }

    pub fn line(mut self, line: TermLine) -> Self {
        self.lines.push(line);
        self
    }

    pub fn lines(mut self, lines: Vec<TermLine>) -> Self {
        self.lines = lines;
        self
    }

    pub fn max_height(mut self, height: f32) -> Self {
        self.max_height = Some(height);
        self
    }

    pub fn show_header(mut self, show: bool) -> Self {
        self.show_header = show;
        self
    }

    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }

    pub fn render(self, theme: &Theme) -> impl IntoElement {
        let term_bg = darken(theme.background, 0.05);

        let mut terminal = div()
            .flex()
            .flex_col()
            .w_full()
            .rounded(Radius::LG)
            .bg(term_bg)
            .border_1()
            .border_color(theme.border)
            .overflow_hidden();

        if let Some(max_h) = self.max_height {
            terminal = terminal.max_h(px(max_h));
        }

        // Header
        if self.show_header {
            terminal = terminal.child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .h(px(32.0))
                    .px(px(12.0))
                    .bg(theme.muted)
                    .border_b_1()
                    .border_color(theme.border)
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(8.0))
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(6.0))
                                    .child(dot(gpui::hsla(0.0, 0.9, 0.55, 1.0)))
                                    .child(dot(gpui::hsla(0.11, 1.0, 0.59, 1.0)))
                                    .child(dot(gpui::hsla(0.39, 0.74, 0.49, 1.0))),
                            )
                            .child(
                                div()
                                    .text_size(px(11.0))
                                    .text_color(theme.muted_foreground)
                                    .child(self.title),
                            ),
                    ),
            );
        }

        // Lines
        let mut lines_container = div()
            .flex()
            .flex_col()
            .p(px(12.0))
            .gap(px(2.0))
            .font_family("monospace")
            .text_size(px(12.0))
            .overflow_y_hidden();

        for line in self.lines {
            let color = match line.kind {
                TermLineKind::Command => theme.foreground,
                TermLineKind::Output => theme.muted_foreground,
                TermLineKind::Success => theme.success,
                TermLineKind::Error => theme.destructive,
                TermLineKind::Warning => theme.warning,
                TermLineKind::Info => theme.info,
            };

            let weight = match line.kind {
                TermLineKind::Command => gpui::FontWeight::BOLD,
                _ => gpui::FontWeight::NORMAL,
            };

            lines_container = lines_container.child(
                div().text_color(color).font_weight(weight).whitespace_nowrap().child(line.text),
            );
        }

        terminal.child(lines_container)
    }
}

/// Small traffic-light style dot
fn dot(color: Hsla) -> impl IntoElement {
    div().size(px(10.0)).rounded(Radius::FULL).bg(color)
}

/// Darken a color by adjusting lightness
fn darken(color: Hsla, amount: f32) -> Hsla {
    Hsla {
        h: color.h,
        s: color.s,
        l: (color.l - amount).max(0.0),
        a: color.a,
    }
}
