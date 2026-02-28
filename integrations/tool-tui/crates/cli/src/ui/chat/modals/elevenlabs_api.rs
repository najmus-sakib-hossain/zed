//! ElevenLabs API key configuration modal

use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Widget},
};

use crate::ui::chat::{text_input::TextInput, theme::ChatTheme};

pub fn render(
    area: Rect,
    buf: &mut Buffer,
    theme: &ChatTheme,
    api_key_input: &TextInput,
    cursor_visible: bool,
) {
    let modal_width = 60.min(area.width.saturating_sub(4));
    let modal_height = 12;

    let modal_area = Rect {
        x: (area.width.saturating_sub(modal_width)) / 2,
        y: (area.height.saturating_sub(modal_height)) / 2,
        width: modal_width,
        height: modal_height,
    };

    // Dim background
    for y in area.top()..area.bottom() {
        for x in area.left()..area.right() {
            let cell = &mut buf[(x, y)];
            cell.set_style(Style::default().bg(theme.bg).fg(theme.border));
        }
    }

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.accent))
        .border_type(ratatui::widgets::BorderType::Rounded)
        .style(Style::default().bg(theme.bg));

    let inner = block.inner(modal_area);
    block.render(modal_area, buf);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Length(1),
            Constraint::Length(3),
            Constraint::Length(1),
            Constraint::Length(3),
        ])
        .split(inner);

    // Title
    let title = Line::from(vec![Span::styled(
        "ElevenLabs API Key",
        Style::default().fg(theme.accent).add_modifier(Modifier::BOLD),
    )]);
    Paragraph::new(title).alignment(Alignment::Center).render(chunks[0], buf);

    // Instructions
    let instructions = Line::from(vec![Span::styled(
        "Get your API key from: https://elevenlabs.io/app/settings/api-keys",
        Style::default().fg(theme.fg),
    )]);
    Paragraph::new(instructions).alignment(Alignment::Center).render(chunks[1], buf);

    // Input field
    let input_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.border))
        .border_type(ratatui::widgets::BorderType::Rounded);

    let input_inner = input_block.inner(chunks[2]);
    input_block.render(chunks[2], buf);

    // Render input text (masked)
    let masked_text = if api_key_input.content.is_empty() {
        "Enter your ElevenLabs API key...".to_string()
    } else {
        "•".repeat(api_key_input.content.len())
    };

    let input_style = if api_key_input.content.is_empty() {
        Style::default().fg(theme.border)
    } else {
        Style::default().fg(theme.fg)
    };

    let masked_len = masked_text.len();
    Paragraph::new(masked_text).style(input_style).render(input_inner, buf);

    // Render cursor
    if cursor_visible && !api_key_input.content.is_empty() {
        let cursor_x = input_inner.x + (api_key_input.cursor_position.min(masked_len) as u16);
        let cursor_y = input_inner.y;
        if cursor_x < input_inner.right() {
            let cell = &mut buf[(cursor_x, cursor_y)];
            cell.set_style(Style::default().bg(theme.accent).fg(theme.bg));
        }
    }

    // Help text
    let help = Line::from(vec![
        Span::styled("Enter", Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
        Span::styled(" to save  ", Style::default().fg(theme.fg)),
        Span::styled("Esc", Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
        Span::styled(" to cancel", Style::default().fg(theme.fg)),
    ]);
    Paragraph::new(help).alignment(Alignment::Center).render(chunks[4], buf);
}
