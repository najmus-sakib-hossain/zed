use super::super::{modal_list::ModalList, text_input::TextInput, theme::ChatTheme};
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Widget},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AddModalFocus {
    Search,
    Options,
}

pub fn render(
    area: Rect,
    buf: &mut Buffer,
    theme: &ChatTheme,
    search: &TextInput,
    list: &ModalList,
    focus: AddModalFocus,
) {
    let modal_width = area.width.saturating_sub(20).min(80);
    let modal_height = area.height.saturating_sub(10).min(30);
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
            " Add to Request ",
            Style::default().fg(theme.accent).add_modifier(Modifier::BOLD),
        ))
        .style(Style::default().bg(theme.bg));

    let inner = block.inner(modal_area);
    block.render(modal_area, buf);

    let mut y = inner.y;

    let search_focused = focus == AddModalFocus::Search;
    let search_border_style = if search_focused {
        Style::default().fg(theme.accent)
    } else {
        Style::default().fg(theme.border)
    };

    let search_block = Block::default()
        .borders(Borders::ALL)
        .border_style(search_border_style)
        .border_type(ratatui::widgets::BorderType::Rounded);

    let search_area = Rect {
        x: inner.x + 1,
        y,
        width: inner.width.saturating_sub(2),
        height: 3,
    };

    let search_inner = search_block.inner(search_area);
    search_block.render(search_area, buf);

    // Render search text with selection
    if search.content.is_empty() {
        let search_text = Span::styled(
            "Search for files and content to add to your request...",
            Style::default().fg(theme.border),
        );
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

    if search_focused {
        let cursor_x = search_inner.x + search.cursor_position as u16;
        if cursor_x < search_inner.right() {
            let cell = &mut buf[(cursor_x, search_inner.y)];
            cell.set_char('▎');
            cell.set_style(Style::default().fg(theme.accent));
        }
    }

    y += 3; // No gap - directly after search box

    let options = vec![
        ("Files & Folders", "Browse and select files or folders"),
        ("Instructions", "Add custom instructions or context"),
        ("Screenshot Window", "Capture a screenshot"),
        ("Source Control", "Add git changes or diffs"),
        ("Problems", "Include diagnostics and errors"),
        ("Symbols", "Add code symbols or definitions"),
        ("Tools", "Select available tools"),
        ("", ""),
        ("README.md", "Add project README"),
        ("recently opened", "Select from recent files"),
    ];

    for (i, (title, _desc)) in options.iter().enumerate() {
        if y >= inner.bottom() - 1 {
            break;
        }

        if title.is_empty() {
            y += 1;
            continue;
        }

        let is_selected = i == list.selected && focus == AddModalFocus::Options;

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

        let line = Line::from(vec![
            Span::styled("  ", bg_style),
            Span::styled(*title, title_style),
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
    }

    let help_text = Line::from(vec![
        Span::styled("Tab", Style::default().fg(theme.accent)),
        Span::styled(" Switch Focus  ", Style::default().fg(theme.border)),
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
