use super::super::{
    app_data::{ChangeType, GitChange},
    modal_list::ModalList,
    theme::ChatTheme,
};
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Widget},
};

pub fn render(
    area: Rect,
    buf: &mut Buffer,
    theme: &ChatTheme,
    list: &ModalList,
    git_changes: &[GitChange],
    changes_count: usize,
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

    let title = if changes_count > 0 {
        format!(" Changes ({}) ", changes_count)
    } else {
        " Changes (No changes) ".to_string()
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

    if git_changes.is_empty() {
        let msg = Line::from(Span::styled(
            "No changes detected in the repository",
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

        for (idx, change) in git_changes[start_idx..end_idx].iter().enumerate() {
            let actual_idx = start_idx + idx;
            let is_selected = actual_idx == list.selected;

            let change_icon = match change.change_type {
                ChangeType::Modified => ("M", theme.accent),
                ChangeType::Added => ("+", theme.accent),
                ChangeType::Deleted => ("-", theme.accent),
                ChangeType::Renamed => ("R", theme.accent),
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
                    change_icon.0,
                    Style::default()
                        .fg(change_icon.1)
                        .bg(if is_selected { theme.accent } else { theme.bg })
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(" ", bg_style),
                Span::styled(
                    &change.file_path,
                    if is_selected {
                        bg_style.add_modifier(Modifier::BOLD)
                    } else {
                        bg_style
                    },
                ),
            ];

            if change.additions > 0 || change.deletions > 0 {
                let stats = format!(" +{} -{}", change.additions, change.deletions);
                let mut line_spans = spans.clone();
                line_spans.push(Span::styled(
                    stats,
                    Style::default()
                        .fg(if is_selected { theme.bg } else { theme.border })
                        .bg(if is_selected { theme.accent } else { theme.bg }),
                ));
                let line = Line::from(line_spans);

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
            } else {
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
            }

            y += 1;

            if is_selected && !change.diff.is_empty() && y < inner.bottom() - 2 {
                let diff_lines: Vec<Line> = change
                    .diff
                    .lines()
                    .take(5)
                    .map(|line| {
                        let (text, color) = if line.starts_with('+') && !line.starts_with("+++") {
                            (line, Color::Green)
                        } else if line.starts_with('-') && !line.starts_with("---") {
                            (line, Color::Red)
                        } else if line.starts_with("@@") {
                            (line, Color::Cyan)
                        } else {
                            (line, theme.border)
                        };

                        Line::from(Span::styled(
                            format!("  {}", text),
                            Style::default().fg(color).bg(theme.bg),
                        ))
                    })
                    .collect();

                for diff_line in diff_lines {
                    if y < inner.bottom() - 1 {
                        Paragraph::new(diff_line).render(
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
