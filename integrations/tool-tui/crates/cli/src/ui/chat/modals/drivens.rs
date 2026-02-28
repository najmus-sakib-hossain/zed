use super::super::{
    app_data::{Task, TaskPriority, TaskStatus},
    modal_list::ModalList,
    theme::ChatTheme,
};
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
    tasks: &[Task],
    tasks_count: usize,
) {
    let modal_width = area.width.saturating_sub(10).min(120);
    let modal_height = area.height.saturating_sub(6).min(40);
    let modal_area = Rect {
        x: (area.width.saturating_sub(modal_width)) / 2,
        y: (area.height.saturating_sub(modal_height)) / 2,
        width: modal_width,
        height: modal_height,
    };

    Clear.render(modal_area, buf);

    let title = if tasks_count > 0 {
        format!(" Drivens ({}) ", tasks_count)
    } else {
        " Drivens (No drivens) ".to_string()
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.border))
        .border_type(ratatui::widgets::BorderType::Rounded)
        .title(Span::styled(
            title,
            Style::default().fg(theme.accent).add_modifier(Modifier::BOLD),
        ))
        .style(Style::default().bg(theme.bg));

    let inner = block.inner(modal_area);
    block.render(modal_area, buf);

    if tasks.is_empty() {
        let msg = Line::from(Span::styled(
            "No drivens found",
            Style::default().fg(theme.border).add_modifier(Modifier::ITALIC),
        ));

        Paragraph::new(msg).alignment(ratatui::layout::Alignment::Center).render(
            Rect {
                x: inner.x,
                y: inner.y + inner.height / 2,
                width: inner.width,
                height: 1,
            },
            buf,
        );
    } else {
        let visible_items = (inner.height.saturating_sub(2)) as usize;
        let (start_idx, end_idx) = list.get_visible_range(visible_items);

        let mut y = inner.y + 1;

        for (idx, task) in tasks[start_idx..end_idx].iter().enumerate() {
            let actual_idx = start_idx + idx;
            let is_selected = actual_idx == list.selected;

            let priority_color = match task.priority {
                TaskPriority::High => theme.accent,
                TaskPriority::Medium => theme.accent,
                TaskPriority::Low => theme.accent,
            };

            let (status_icon, status_color) = match task.status {
                TaskStatus::Todo => ("TODO", theme.accent),
                TaskStatus::InProgress => ("PROGRESS", theme.accent),
                TaskStatus::Done => ("DONE", theme.accent),
            };

            let bg_style = if is_selected {
                Style::default().bg(theme.accent).fg(theme.bg)
            } else {
                Style::default().bg(theme.bg).fg(theme.fg)
            };

            if is_selected {
                for x in inner.x + 1..inner.x + inner.width - 1 {
                    if y < inner.bottom() - 1 {
                        let cell = &mut buf[(x, y)];
                        cell.set_style(bg_style);
                    }
                }
            }

            let spans = vec![
                Span::styled(" ", bg_style),
                Span::styled(
                    status_icon,
                    Style::default()
                        .fg(if is_selected { theme.bg } else { status_color })
                        .bg(if is_selected { theme.accent } else { theme.bg })
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled("  ", bg_style),
                Span::styled(
                    &task.title,
                    Style::default()
                        .fg(if is_selected {
                            theme.bg
                        } else {
                            priority_color
                        })
                        .bg(if is_selected { theme.accent } else { theme.bg })
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(": ", bg_style),
                Span::styled(&task.description, bg_style),
            ];

            let line = Line::from(spans);

            if y < inner.bottom() - 1 {
                Paragraph::new(line).render(
                    Rect {
                        x: inner.x + 1,
                        y,
                        width: inner.width.saturating_sub(2),
                        height: 1,
                    },
                    buf,
                );
            }

            y += 1;

            if is_selected
                && let (Some(file_path), Some(line_num)) = (&task.file_path, task.line_number)
            {
                let location = format!("    -> {}:{}", file_path, line_num);
                let location_line = Line::from(Span::styled(
                    location,
                    Style::default()
                        .fg(if is_selected { theme.bg } else { theme.border })
                        .bg(if is_selected { theme.accent } else { theme.bg })
                        .add_modifier(Modifier::ITALIC),
                ));

                if y < inner.bottom() - 1 {
                    Paragraph::new(location_line).render(
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
