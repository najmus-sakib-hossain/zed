use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Widget},
};

use crate::ui::chat::{modal_list::ModalList, theme::ChatTheme};

#[derive(Debug, Clone)]
pub struct Checkpoint {
    pub id: String,
    pub timestamp: String,
    pub message: String,
    pub files_changed: usize,
}

pub fn get_checkpoints() -> Vec<Checkpoint> {
    vec![
        Checkpoint {
            id: "cp-001".to_string(),
            timestamp: "2 minutes ago".to_string(),
            message: "Added authentication module".to_string(),
            files_changed: 5,
        },
        Checkpoint {
            id: "cp-002".to_string(),
            timestamp: "15 minutes ago".to_string(),
            message: "Fixed database connection issue".to_string(),
            files_changed: 3,
        },
        Checkpoint {
            id: "cp-003".to_string(),
            timestamp: "1 hour ago".to_string(),
            message: "Implemented user profile page".to_string(),
            files_changed: 8,
        },
        Checkpoint {
            id: "cp-004".to_string(),
            timestamp: "3 hours ago".to_string(),
            message: "Updated API endpoints".to_string(),
            files_changed: 12,
        },
    ]
}

pub fn render(
    full_area: Rect,
    buf: &mut Buffer,
    theme: &ChatTheme,
    list: &ModalList,
    _selected_mode: &str,
) {
    let modal_width = full_area.width.saturating_sub(20).min(80);
    let modal_height = full_area.height.saturating_sub(10).min(30);
    let modal_x = (full_area.width.saturating_sub(modal_width)) / 2;
    let modal_y = (full_area.height.saturating_sub(modal_height)) / 2;

    let area = Rect {
        x: modal_x,
        y: modal_y,
        width: modal_width,
        height: modal_height,
    };

    Clear.render(area, buf);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.border))
        .border_type(ratatui::widgets::BorderType::Rounded)
        .title(" Checkpoints ")
        .title_alignment(Alignment::Center)
        .style(Style::default().bg(theme.bg));

    let inner = block.inner(area);
    block.render(area, buf);

    let checkpoints = get_checkpoints();

    if checkpoints.is_empty() {
        let msg = Line::from(Span::styled(
            "No checkpoints found",
            Style::default().fg(theme.border).add_modifier(Modifier::ITALIC),
        ));

        Paragraph::new(msg).alignment(Alignment::Center).render(
            Rect {
                x: inner.x,
                y: inner.y + inner.height / 2,
                width: inner.width,
                height: 1,
            },
            buf,
        );
    } else {
        let mut y = inner.y + 1;

        for (i, checkpoint) in checkpoints.iter().enumerate() {
            if y >= inner.bottom() - 2 {
                break;
            }

            let is_selected = i == list.selected;

            let bg_style = if is_selected {
                Style::default().bg(theme.accent).fg(theme.bg)
            } else {
                Style::default().bg(theme.bg).fg(theme.fg)
            };

            if is_selected {
                for x in inner.x + 1..inner.x + inner.width - 1 {
                    let cell = &mut buf[(x, y)];
                    cell.set_style(bg_style);
                }
            }

            let line = Line::from(vec![
                Span::styled("  ", bg_style),
                Span::styled(
                    &checkpoint.id,
                    if is_selected {
                        bg_style.add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(theme.accent)
                    },
                ),
                Span::styled("  ", bg_style),
                Span::styled(&checkpoint.message, bg_style),
                Span::styled("  ", bg_style),
                Span::styled(
                    format!("({} files)", checkpoint.files_changed),
                    if is_selected {
                        bg_style
                    } else {
                        Style::default().fg(theme.border)
                    },
                ),
            ]);

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

            if is_selected {
                let timestamp_line = Line::from(Span::styled(
                    format!("    {}", checkpoint.timestamp),
                    if is_selected {
                        bg_style.add_modifier(Modifier::ITALIC)
                    } else {
                        Style::default().fg(theme.border).add_modifier(Modifier::ITALIC)
                    },
                ));

                if y < inner.bottom() - 2 {
                    Paragraph::new(timestamp_line).render(
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
            }
        }
    }

    let help_text = Line::from(vec![
        Span::styled("↑↓", Style::default().fg(theme.accent)),
        Span::styled(" Navigate  ", Style::default().fg(theme.border)),
        Span::styled("Enter", Style::default().fg(theme.accent)),
        Span::styled(" Restore  ", Style::default().fg(theme.border)),
        Span::styled("Esc", Style::default().fg(theme.accent)),
        Span::styled(" Close", Style::default().fg(theme.border)),
    ]);

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
