//! Rendering logic for CodeViewer
//!
//! Handles syntax highlighting, line number formatting, and visual display.

use owo_colors::OwoColorize;
use syntect::easy::HighlightLines;
use syntect::highlighting::{Theme, ThemeSet};
use syntect::parsing::{SyntaxReference, SyntaxSet};
use syntect::util::as_24_bit_terminal_escaped;

use crate::ui::components::traits::RenderContext;
use crate::ui::editor::search::SearchEngine;
use crate::ui::editor::viewer::{CodeViewer, LineNumberMode};

impl CodeViewer {
    /// Get line number width (digits needed)
    pub(crate) fn line_number_width(&self) -> usize {
        if self.config.line_numbers == LineNumberMode::None {
            0
        } else {
            let max_line = self.lines.len().max(1);
            ((max_line as f64).log10().floor() as usize) + 1 + 2 // +2 for padding
        }
    }

    /// Format line number based on mode
    pub(crate) fn format_line_number(&self, line_idx: usize) -> String {
        let width = self.line_number_width().saturating_sub(2);
        match self.config.line_numbers {
            LineNumberMode::None => String::new(),
            LineNumberMode::Absolute => format!("{:>width$} │ ", line_idx + 1),
            LineNumberMode::Relative => {
                let rel = if line_idx == self.cursor_line {
                    0
                } else {
                    (line_idx as isize - self.cursor_line as isize).unsigned_abs()
                };
                format!("{:>width$} │ ", rel)
            }
            LineNumberMode::Hybrid => {
                if line_idx == self.cursor_line {
                    format!("{:>width$} │ ", line_idx + 1)
                } else {
                    let rel = (line_idx as isize - self.cursor_line as isize).unsigned_abs();
                    format!("{:>width$} │ ", rel)
                }
            }
        }
    }

    /// Get the current theme
    pub(crate) fn get_theme(&self) -> &Theme {
        self.theme_set
            .themes
            .get(&self.config.theme_name)
            .unwrap_or_else(|| &self.theme_set.themes["base16-ocean.dark"])
    }

    /// Get the current syntax reference
    pub(crate) fn get_syntax(&self) -> &SyntaxReference {
        self.current_syntax
            .and_then(|i| self.syntax_set.syntaxes().get(i))
            .unwrap_or_else(|| self.syntax_set.find_syntax_plain_text())
    }

    /// Render the viewer
    pub(crate) fn render_internal(&self, ctx: &RenderContext<'_>) -> Vec<String> {
        let mut output = Vec::with_capacity(ctx.bounds.height as usize);
        let theme = self.get_theme();
        let syntax = self.get_syntax();
        let line_num_width = self.line_number_width();

        let mut highlighter = HighlightLines::new(syntax, theme);

        let start = self.scroll_offset;
        let end = (start + ctx.bounds.height as usize).min(self.lines.len());

        for line_idx in start..end {
            let line = &self.lines[line_idx];

            // Line number
            let line_num = if self.config.line_numbers != LineNumberMode::None {
                let num_str = self.format_line_number(line_idx);
                if line_idx == self.cursor_line && ctx.focused {
                    format!("{}", num_str.yellow().bold())
                } else {
                    format!("{}", num_str.bright_black())
                }
            } else {
                String::new()
            };

            // Syntax highlighted content
            let highlighted = if let Ok(ranges) = highlighter.highlight_line(line, &self.syntax_set)
            {
                as_24_bit_terminal_escaped(&ranges[..], false)
            } else {
                line.to_string()
            };

            // Handle horizontal scroll
            let content_width = ctx.bounds.width as usize - line_num_width;
            let visible_content: String = highlighted
                .chars()
                .skip(self.h_scroll_offset)
                .take(content_width)
                .collect();

            // Build final line
            let mut output_line = format!("{}{}", line_num, visible_content);

            // Highlight current line
            if line_idx == self.cursor_line && self.config.highlight_line && ctx.focused {
                output_line = format!("{}", output_line.on_truecolor(40, 40, 50));
            }

            // Search match highlighting
            if self.search.matches().iter().any(|m| m.line == line_idx) {
                output_line = format!("{}", output_line.on_truecolor(60, 50, 0));
            }

            output.push(output_line);
        }

        // Pad remaining lines
        while output.len() < ctx.bounds.height as usize {
            let tilde = if self.config.line_numbers != LineNumberMode::None {
                format!(
                    "{:>width$} │ ~",
                    "",
                    width = line_num_width.saturating_sub(4)
                )
            } else {
                "~".to_string()
            };
            output.push(format!("{}", tilde.bright_black()));
        }

        output
    }
}
