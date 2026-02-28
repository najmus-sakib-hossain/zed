use super::super::{modal_list::ModalList, modes::ChatMode, theme::ChatTheme};
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Widget},
};

pub fn render(
    area: Rect,
    buf: &mut Buffer,
    theme: &ChatTheme,
    list: &ModalList,
    current_mode: ChatMode,
) {
    let modal_width = area.width.saturating_sub(20).min(60);
    let modal_height = area.height.saturating_sub(10).min(15);
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
            " Select Mode ",
            Style::default().fg(theme.accent).add_modifier(Modifier::BOLD),
        ))
        .style(Style::default().bg(theme.bg));

    let inner = block.inner(modal_area);
    block.render(modal_area, buf);

    let mut y = inner.y + 1;

    let options = [
        ("Agent", "Ctrl+I", 0),
        ("Plan", "", 1),
        ("Debug", "", 2),
        ("Ask", "", 3),
    ];

    for (title, shortcut, idx) in options.iter() {
        if y >= inner.bottom() - 1 {
            break;
        }

        let is_selected = *idx == list.selected;

        let bg_style = if is_selected {
            Style::default().bg(theme.accent).fg(theme.bg)
        } else {
            Style::default().bg(theme.bg).fg(theme.fg)
        };

        let title_style = if is_selected {
            bg_style.add_modifier(Modifier::BOLD)
        } else {
            bg_style
        };

        if is_selected {
            for x in inner.x + 1..inner.x + inner.width - 1 {
                let cell = &mut buf[(x, y)];
                cell.set_style(bg_style);
            }
        }

        let is_current = match *idx {
            0 => current_mode == ChatMode::Agent,
            1 => current_mode == ChatMode::Plan,
            3 => current_mode == ChatMode::Ask,
            _ => false,
        };

        let mut spans = vec![
            Span::styled("  ", bg_style),
            Span::styled(*title, title_style),
        ];

        if !shortcut.is_empty() {
            spans.push(Span::styled("  ", bg_style));
            spans.push(Span::styled(
                *shortcut,
                if is_selected {
                    bg_style
                } else {
                    Style::default().fg(theme.border).bg(theme.bg)
                },
            ));
        }

        if is_current {
            let remaining_width = inner.width.saturating_sub(2) as usize;
            let content_width = title.len() + shortcut.len() + 4;
            let padding = remaining_width.saturating_sub(content_width);
            spans.push(Span::styled(" ".repeat(padding.saturating_sub(2)), bg_style));
            spans.push(Span::styled("[ACTIVE]", title_style));
        }

        let line = Line::from(spans);

        Paragraph::new(line).render(
            Rect {
                x: inner.x + 1,
                y,
                width: inner.width.saturating_sub(2),
                height: 1,
            },
            buf,
        );

        y += 1;
    }

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
