use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Widget},
};

use crate::ui::chat::{
    app_data::Agent, modal_list::ModalList, text_input::TextInput, theme::ChatTheme,
};

/// Render the workspaces modal
pub fn render(
    area: Rect,
    buf: &mut Buffer,
    agents: &[Agent],
    list: &ModalList,
    theme: &ChatTheme,
    create_mode: bool,
    create_input: &TextInput,
) {
    // Center the modal
    let modal_width = area.width.saturating_sub(10).min(120);
    let modal_height = area.height.saturating_sub(6).min(35);
    let modal_x = (area.width.saturating_sub(modal_width)) / 2;
    let modal_y = (area.height.saturating_sub(modal_height)) / 2;

    let modal_area = Rect {
        x: modal_x,
        y: modal_y,
        width: modal_width,
        height: modal_height,
    };

    // Clear background
    Clear.render(modal_area, buf);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.border))
        .border_type(ratatui::widgets::BorderType::Rounded)
        .title(Span::styled(
            " Active Workspaces ",
            Style::default().fg(theme.accent).add_modifier(Modifier::BOLD),
        ))
        .style(Style::default().bg(theme.bg));

    let inner = block.inner(modal_area);
    block.render(modal_area, buf);

    if agents.is_empty() {
        let empty_y = inner.y + inner.height / 2;
        Paragraph::new(Line::from(Span::styled(
            "No active workspaces",
            Style::default().fg(theme.border).add_modifier(Modifier::ITALIC),
        )))
        .alignment(Alignment::Center)
        .render(
            Rect {
                x: inner.x,
                y: empty_y,
                width: inner.width,
                height: 1,
            },
            buf,
        );
        return;
    }

    let mut y = inner.y + 1;

    // Render header row
    if y < inner.bottom() {
        let header_spans = vec![
            Span::styled("Sta ", Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
            Span::styled(
                format!("{:<15}", "Name"),
                Style::default().fg(theme.accent).add_modifier(Modifier::BOLD),
            ),
            Span::raw(" "),
            Span::styled(
                format!("{:<15}", "Model"),
                Style::default().fg(theme.accent).add_modifier(Modifier::BOLD),
            ),
            Span::raw(" "),
            Span::styled(
                format!("{:<35}", "Task"),
                Style::default().fg(theme.accent).add_modifier(Modifier::BOLD),
            ),
            Span::raw(" "),
            Span::styled(
                format!("{:>4}", "Prog"),
                Style::default().fg(theme.accent).add_modifier(Modifier::BOLD),
            ),
            Span::raw("  "),
            Span::styled(
                format!("{:>6}", "Tokens"),
                Style::default().fg(theme.accent).add_modifier(Modifier::BOLD),
            ),
            Span::raw("  "),
            Span::styled(
                format!("{:>5}", "Time"),
                Style::default().fg(theme.accent).add_modifier(Modifier::BOLD),
            ),
        ];

        Paragraph::new(Line::from(header_spans)).render(
            Rect {
                x: inner.x + 2,
                y,
                width: inner.width.saturating_sub(4),
                height: 1,
            },
            buf,
        );
        y += 2;
    }

    // Calculate visible range
    let visible_items = inner.bottom().saturating_sub(y).saturating_sub(2);
    let (start_idx, end_idx) = list.get_visible_range(visible_items as usize);

    // Render agent rows
    for (idx, agent) in agents[start_idx..end_idx.min(agents.len())].iter().enumerate() {
        if y >= inner.bottom().saturating_sub(1) {
            break;
        }

        let actual_idx = start_idx + idx;
        let is_selected = actual_idx == list.selected;

        let bg_style = if is_selected {
            Style::default().bg(theme.accent).fg(theme.bg)
        } else {
            Style::default().bg(theme.bg).fg(theme.fg)
        };

        // Highlight entire row if selected
        if is_selected {
            for x in inner.x + 2..inner.x + inner.width - 2 {
                if x < buf.area().right() && y < buf.area().bottom() {
                    let cell = &mut buf[(x, y)];
                    cell.set_style(bg_style);
                }
            }
        }

        // Format the row data with proper column widths
        let status_icon = agent.status.icon();
        let status_color = agent.status.color();
        let name = pad_or_truncate(&agent.name, 15);
        let model = pad_or_truncate(&agent.model, 15);
        let task = pad_or_truncate(&agent.task, 35);
        let progress = format!("{:>3}%", (agent.progress * 100.0) as u32);
        let tokens = format!("{:>6}", format_tokens(agent.tokens_used));
        let time = format!("{:>5}", format_duration(agent.duration));

        let spans = vec![
            Span::styled(
                format!("[{}] ", status_icon),
                if is_selected {
                    bg_style
                } else {
                    Style::default().fg(status_color)
                },
            ),
            Span::styled(name, bg_style),
            Span::styled(" ", bg_style),
            Span::styled(model, bg_style),
            Span::styled(" ", bg_style),
            Span::styled(task, bg_style),
            Span::styled(" ", bg_style),
            Span::styled(progress, bg_style),
            Span::styled("  ", bg_style),
            Span::styled(tokens, bg_style),
            Span::styled("  ", bg_style),
            Span::styled(time, bg_style),
        ];

        Paragraph::new(Line::from(spans)).render(
            Rect {
                x: inner.x + 2,
                y,
                width: inner.width.saturating_sub(4),
                height: 1,
            },
            buf,
        );

        y += 1;
    }

    // Show help text at bottom
    if inner.height > 2 {
        let help_text = if create_mode {
            Line::from(vec![
                Span::styled("Enter", Style::default().fg(theme.accent)),
                Span::styled(": Create | ", Style::default().fg(theme.border)),
                Span::styled("Esc", Style::default().fg(theme.accent)),
                Span::styled(": Cancel", Style::default().fg(theme.border)),
            ])
        } else {
            Line::from(vec![
                Span::styled("↑↓", Style::default().fg(theme.accent)),
                Span::styled(": Navigate | ", Style::default().fg(theme.border)),
                Span::styled("Enter", Style::default().fg(theme.accent)),
                Span::styled(": Switch | ", Style::default().fg(theme.border)),
                Span::styled("n", Style::default().fg(theme.accent)),
                Span::styled(": New Workspace | ", Style::default().fg(theme.border)),
                Span::styled("Esc", Style::default().fg(theme.accent)),
                Span::styled(": Close", Style::default().fg(theme.border)),
            ])
        };

        Paragraph::new(help_text).alignment(Alignment::Center).render(
            Rect {
                x: inner.x,
                y: inner.y + inner.height.saturating_sub(1),
                width: inner.width,
                height: 1,
            },
            buf,
        );
    }

    // Render create input if in create mode
    if create_mode {
        let input_y = inner.y + inner.height.saturating_sub(3);

        // Label
        Paragraph::new(Line::from(Span::styled(
            "New Workspace Name:",
            Style::default().fg(theme.accent).add_modifier(Modifier::BOLD),
        )))
        .render(
            Rect {
                x: inner.x + 2,
                y: input_y,
                width: inner.width.saturating_sub(4),
                height: 1,
            },
            buf,
        );

        // Input box
        let input_content = &create_input.content;
        Paragraph::new(Line::from(Span::styled(
            input_content,
            Style::default().fg(theme.fg).bg(theme.bg),
        )))
        .render(
            Rect {
                x: inner.x + 2,
                y: input_y + 1,
                width: inner.width.saturating_sub(4),
                height: 1,
            },
            buf,
        );
    }
}

fn pad_or_truncate(s: &str, max_len: usize) -> String {
    if s.len() > max_len {
        format!("{}...", &s[..max_len.saturating_sub(3)])
    } else {
        format!("{:<width$}", s, width = max_len)
    }
}

fn format_tokens(tokens: usize) -> String {
    if tokens >= 1_000_000 {
        format!("{:.1}M", tokens as f64 / 1_000_000.0)
    } else if tokens >= 1_000 {
        format!("{:.1}k", tokens as f64 / 1_000.0)
    } else {
        tokens.to_string()
    }
}

fn format_duration(duration: std::time::Duration) -> String {
    let secs = duration.as_secs();
    if secs >= 3600 {
        format!("{}h{}m", secs / 3600, (secs % 3600) / 60)
    } else if secs >= 60 {
        format!("{}m{}s", secs / 60, secs % 60)
    } else {
        format!("{}s", secs)
    }
}
