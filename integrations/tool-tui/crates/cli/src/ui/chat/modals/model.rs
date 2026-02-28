use super::super::{modal_list::ModalList, text_input::TextInput, theme::ChatTheme};
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Widget},
};

pub struct ModelConfig<'a> {
    pub auto_mode: bool,
    pub max_mode: bool,
    pub use_multiple_models: bool,
    pub selected_model: &'a str,
    pub selected_models: &'a [String],
    pub google_models: &'a [super::super::app_state::GoogleModel],
}

pub fn get_filtered_models(search_content: &str) -> Vec<(&'static str, &'static str)> {
    let all_models = vec![
        // Antigravity models (Claude Opus 4.5, etc.)
        ("Claude-Opus-4.5 (Antigravity)", "[Antigravity]"),
        ("Claude-Sonnet-4.5 (Antigravity)", "[Antigravity]"),
        ("Claude-Haiku-4.5 (Antigravity)", "[Antigravity]"),
        ("Gemini-3.0-Pro (Antigravity)", "[Antigravity]"),
        ("Gemini-2.5-Flash (Antigravity)", "[Antigravity]"),
        ("GPT-OSS-120B (Antigravity)", "[Antigravity]"),
        // Regular models
        ("GPT-4.1", "0x"),
        ("GPT-4o", "0x"),
        ("GPT-5-mini", "0x"),
        ("Grok-Code-Fast-1", "0x"),
        ("Raptor-mini (Preview)", "0x"),
        ("Claude-Haiku-4.5", "0.33x"),
        ("Claude-Opus-4.5", "3x"),
        ("Claude-Sonnet-4", "1x"),
        ("Claude-Sonnet-4.5", "1x"),
        ("Gemini-2.5-Pro", "1x"),
        ("Gemini-3-Flash (Preview)", "0.33x"),
        ("Gemini-3-Pro (Preview)", "1x"),
        ("GPT-5", "1x"),
        ("GPT-5-Coder (Preview)", "1x"),
        ("GPT-5.1", "1x"),
        ("GPT-5.1-Codex", "1x"),
        ("GPT-5.1-Codex-Max", "1x"),
        ("GPT-5.1-Codex-Mini (Preview)", "0.33x"),
        ("GPT-5.2", "1x"),
        ("GPT-5.2-Codex", "1x"),
    ];

    if search_content.is_empty() {
        all_models
    } else {
        let search_lower = search_content.to_lowercase();
        all_models
            .into_iter()
            .filter(|(name, _)| name.to_lowercase().contains(&search_lower))
            .collect()
    }
}

pub fn render(
    area: Rect,
    buf: &mut Buffer,
    theme: &ChatTheme,
    search: &TextInput,
    list: &ModalList,
    config: &ModelConfig,
) {
    let modal_width = 70.min(area.width.saturating_sub(4));
    let modal_height = 30.min(area.height.saturating_sub(4));
    let modal_area = Rect {
        x: (area.width.saturating_sub(modal_width)) / 2,
        y: (area.height.saturating_sub(modal_height)) / 2,
        width: modal_width,
        height: modal_height,
    };

    Clear.render(modal_area, buf);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.border))
        .border_type(ratatui::widgets::BorderType::Rounded)
        .title(Span::styled(
            " Select Model ",
            Style::default().fg(theme.accent).add_modifier(Modifier::BOLD),
        ))
        .style(Style::default().bg(theme.bg));

    let inner = block.inner(modal_area);
    block.render(modal_area, buf);

    let mut y = inner.y + 1;

    if inner.height < 10 {
        return;
    }

    // Search box with accent border for auto-focus
    let search_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.accent))
        .border_type(ratatui::widgets::BorderType::Rounded);

    let search_area = Rect {
        x: inner.x + 2,
        y,
        width: inner.width.saturating_sub(4),
        height: 3,
    };

    if search_area.width < 10 || y + 3 > inner.bottom() {
        return;
    }

    let search_inner = search_block.inner(search_area);
    search_block.render(search_area, buf);

    // Render search text with selection
    if search.content.is_empty() {
        let search_text = Span::styled("Search models...", Style::default().fg(theme.border));
        Paragraph::new(Line::from(search_text)).render(search_inner, buf);
    } else {
        // Render with selection highlighting
        if search.has_selection() {
            let (sel_start, sel_end) =
                if let (Some(start), Some(end)) = (search.selection_start, search.selection_end) {
                    if start < end {
                        (start, end)
                    } else {
                        (end, start)
                    }
                } else {
                    (0, 0)
                };

            let mut x = search_inner.x;
            for (i, ch) in search.content.chars().enumerate() {
                if x >= search_inner.right() {
                    break;
                }
                let is_selected = i >= sel_start && i < sel_end;
                let style = if is_selected {
                    Style::default().bg(theme.fg).fg(theme.bg)
                } else {
                    Style::default().bg(theme.bg).fg(theme.fg)
                };
                let cell = &mut buf[(x, search_inner.y)];
                cell.set_char(ch);
                cell.set_style(style);
                x += 1;
            }
        } else {
            Paragraph::new(Line::from(Span::styled(
                &search.content,
                Style::default().fg(theme.fg),
            )))
            .render(search_inner, buf);
        }
    }

    // Render cursor for auto-focus
    let cursor_x = search_inner.x + search.cursor_position as u16;
    if cursor_x < search_inner.right() {
        let cell = &mut buf[(cursor_x, search_inner.y)];
        cell.set_char('▎');
        cell.set_style(Style::default().fg(theme.accent));
    }

    y += 4;

    if y >= inner.bottom() {
        return;
    }

    let models = get_filtered_models(&search.content);
    let total_items = 1 + 1 + 3 + config.google_models.len() + models.len(); // Configure API Key + Sign in + 3 config options + models
    let available_height = inner.bottom().saturating_sub(y).saturating_sub(2);
    let visible_count = available_height.min(15) as usize;

    let (start_idx, end_idx) = list.get_visible_range(visible_count);

    for i in start_idx..end_idx.min(total_items) {
        if y >= inner.bottom().saturating_sub(1) {
            break;
        }

        let is_selected = i == list.selected;

        let bg_style = if is_selected {
            Style::default().bg(theme.accent).fg(theme.bg)
        } else {
            Style::default().bg(theme.bg).fg(theme.fg)
        };

        if is_selected {
            let start_x = inner.x + 2;
            let end_x = (inner.x + inner.width).saturating_sub(2);
            for x in start_x..end_x {
                if x < buf.area().right() && y < buf.area().bottom() {
                    let cell = &mut buf[(x, y)];
                    cell.set_style(bg_style);
                }
            }
        }

        if i == 0 {
            // First item is "Configure Google API Key" at the top
            let label = "Configure Google API Key";
            let toggle_char = "[Configure]";

            let mut spans = vec![Span::styled("  ", bg_style), Span::styled(label, bg_style)];

            let remaining_width = inner.width.saturating_sub(4) as usize;
            let content_width = label.len() + toggle_char.len() + 4;
            if remaining_width > content_width {
                let padding = remaining_width.saturating_sub(content_width);
                spans.push(Span::styled(" ".repeat(padding), bg_style));
            } else {
                spans.push(Span::styled(" ", bg_style));
            }
            spans.push(Span::styled(toggle_char, bg_style));

            Paragraph::new(Line::from(spans)).render(
                Rect {
                    x: inner.x + 2,
                    y,
                    width: inner.width.saturating_sub(4),
                    height: 1,
                },
                buf,
            );
        } else if i == 1 {
            // Second item is "Sign in with Google"
            let label = "Sign in with Google";
            let toggle_char = "[OAuth]";

            let mut spans = vec![Span::styled("  ", bg_style), Span::styled(label, bg_style)];

            let remaining_width = inner.width.saturating_sub(4) as usize;
            let content_width = label.len() + toggle_char.len() + 4;
            if remaining_width > content_width {
                let padding = remaining_width.saturating_sub(content_width);
                spans.push(Span::styled(" ".repeat(padding), bg_style));
            } else {
                spans.push(Span::styled(" ", bg_style));
            }
            spans.push(Span::styled(toggle_char, bg_style));

            Paragraph::new(Line::from(spans)).render(
                Rect {
                    x: inner.x + 2,
                    y,
                    width: inner.width.saturating_sub(4),
                    height: 1,
                },
                buf,
            );
        } else if i >= 2 && i < 5 {
            // Items 2-4 are config options (Auto, MAX Mode, Use Multiple Models)
            let (label, enabled) = match i {
                2 => ("Auto", config.auto_mode),
                3 => ("MAX Mode", config.max_mode),
                4 => ("Use Multiple Models", config.use_multiple_models),
                _ => ("", false),
            };

            let toggle_char = if enabled { "[ON]" } else { "[OFF]" };

            let mut spans = vec![Span::styled("  ", bg_style), Span::styled(label, bg_style)];

            let remaining_width = inner.width.saturating_sub(4) as usize;
            let content_width = label.len() + toggle_char.len() + 4;
            if remaining_width > content_width {
                let padding = remaining_width.saturating_sub(content_width);
                spans.push(Span::styled(" ".repeat(padding), bg_style));
            } else {
                spans.push(Span::styled(" ", bg_style));
            }
            spans.push(Span::styled(toggle_char, bg_style));

            Paragraph::new(Line::from(spans)).render(
                Rect {
                    x: inner.x + 2,
                    y,
                    width: inner.width.saturating_sub(4),
                    height: 1,
                },
                buf,
            );
        } else {
            // After first 5 items (Configure API Key + Sign in + 3 config options), show Google models first, then regular models
            let model_index = i - 5;
            if model_index < config.google_models.len() {
                // Google model (shown first)
                if let Some(google_model) = config.google_models.get(model_index) {
                    let is_current = google_model.display_name == config.selected_model;

                    let mut spans = vec![
                        Span::styled("  ", bg_style),
                        Span::styled(google_model.display_name.as_str(), bg_style),
                    ];

                    let remaining_width = inner.width.saturating_sub(4) as usize;
                    let content_width = google_model.display_name.len() + 10;
                    if remaining_width > content_width {
                        let padding = remaining_width.saturating_sub(content_width);
                        spans.push(Span::styled(" ".repeat(padding), bg_style));
                    } else {
                        spans.push(Span::styled(" ", bg_style));
                    }
                    spans.push(Span::styled("[Google]", bg_style));

                    if is_current {
                        spans.push(Span::styled(" [ACTIVE]", bg_style));
                    }

                    Paragraph::new(Line::from(spans)).render(
                        Rect {
                            x: inner.x + 2,
                            y,
                            width: inner.width.saturating_sub(4),
                            height: 1,
                        },
                        buf,
                    );
                }
            } else {
                // Regular model (shown after Google models)
                let regular_model_index = model_index - config.google_models.len();
                if let Some((name, cost)) = models.get(regular_model_index) {
                    let is_current = if config.use_multiple_models {
                        config.selected_models.contains(&name.to_string())
                    } else {
                        *name == config.selected_model
                    };

                    let mut spans =
                        vec![Span::styled("  ", bg_style), Span::styled(*name, bg_style)];

                    let remaining_width = inner.width.saturating_sub(4) as usize;
                    let content_width = name.len() + cost.len() + 4;
                    if remaining_width > content_width {
                        let padding = remaining_width.saturating_sub(content_width);
                        spans.push(Span::styled(" ".repeat(padding), bg_style));
                    } else {
                        spans.push(Span::styled(" ", bg_style));
                    }
                    spans.push(Span::styled(*cost, bg_style));

                    if is_current {
                        spans.push(Span::styled(" [ACTIVE]", bg_style));
                    }

                    Paragraph::new(Line::from(spans)).render(
                        Rect {
                            x: inner.x + 2,
                            y,
                            width: inner.width.saturating_sub(4),
                            height: 1,
                        },
                        buf,
                    );
                }
            }
        }

        y += 1;
    }

    if inner.height > 2 {
        let help_text = Line::from(vec![
            Span::styled("↑↓", Style::default().fg(theme.accent)),
            Span::styled(" Navigate  ", Style::default().fg(theme.border)),
            Span::styled("Enter", Style::default().fg(theme.accent)),
            Span::styled(" Select  ", Style::default().fg(theme.border)),
            Span::styled("Esc", Style::default().fg(theme.accent)),
            Span::styled(" Close", Style::default().fg(theme.border)),
        ]);

        Paragraph::new(help_text).alignment(ratatui::layout::Alignment::Center).render(
            Rect {
                x: inner.x,
                y: inner.y + inner.height.saturating_sub(1),
                width: inner.width,
                height: 1,
            },
            buf,
        );
    }
}
