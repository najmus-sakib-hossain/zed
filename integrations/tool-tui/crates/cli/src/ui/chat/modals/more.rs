use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Widget},
};

use crate::ui::chat::{modal_list::ModalList, theme::ChatTheme};

#[derive(Debug, Clone)]
pub struct MoreOption {
    pub name: String,
    pub description: String,
    pub icon: &'static str,
    pub shortcut: &'static str,
}

pub fn get_more_options() -> Vec<MoreOption> {
    vec![
        MoreOption {
            name: "Settings".to_string(),
            description: "Configure chat preferences and behavior".to_string(),
            icon: "S",
            shortcut: "S",
        },
        MoreOption {
            name: "History".to_string(),
            description: "View and search conversation history".to_string(),
            icon: "H",
            shortcut: "H",
        },
        MoreOption {
            name: "Export".to_string(),
            description: "Export conversation to file".to_string(),
            icon: "E",
            shortcut: "E",
        },
        MoreOption {
            name: "Clear".to_string(),
            description: "Clear current conversation".to_string(),
            icon: "C",
            shortcut: "C",
        },
        MoreOption {
            name: "Help".to_string(),
            description: "View keyboard shortcuts and help".to_string(),
            icon: "?",
            shortcut: "?",
        },
        MoreOption {
            name: "About".to_string(),
            description: "About DX Chat".to_string(),
            icon: "A",
            shortcut: "A",
        },
    ]
}

pub fn render(
    full_area: Rect,
    buf: &mut Buffer,
    theme: &ChatTheme,
    list: &ModalList,
    options: &[MoreOption],
) {
    let modal_width = full_area.width.saturating_sub(20).min(70);
    let modal_height = full_area.height.saturating_sub(10).min(20);
    let modal_x = (full_area.width.saturating_sub(modal_width)) / 2;
    let modal_y = (full_area.height.saturating_sub(modal_height)) / 2;

    let area = Rect {
        x: modal_x,
        y: modal_y,
        width: modal_width,
        height: modal_height,
    };

    // Clear background
    Clear.render(area, buf);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.border))
        .border_type(ratatui::widgets::BorderType::Rounded)
        .title(Span::styled(
            " More Options ",
            Style::default().fg(theme.accent).add_modifier(Modifier::BOLD),
        ))
        .title_alignment(Alignment::Center)
        .style(Style::default().bg(theme.bg));

    let inner = block.inner(area);
    block.render(area, buf);

    let mut y = inner.y + 1;

    for (i, option) in options.iter().enumerate() {
        if y >= inner.bottom() - 2 {
            break;
        }

        let is_selected = i == list.selected;

        let option_line = Line::from(vec![
            Span::raw("  "),
            Span::styled(
                &option.name,
                if is_selected {
                    Style::default().fg(theme.bg).bg(theme.accent).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(theme.fg).add_modifier(Modifier::BOLD)
                },
            ),
            Span::raw("  "),
            Span::styled(
                &option.description,
                if is_selected {
                    Style::default().fg(theme.bg).bg(theme.accent)
                } else {
                    Style::default().fg(theme.border)
                },
            ),
            Span::raw("  "),
            Span::styled(
                option.shortcut,
                if is_selected {
                    Style::default().fg(theme.bg).bg(theme.accent)
                } else {
                    Style::default().fg(theme.accent)
                },
            ),
        ]);

        Paragraph::new(option_line)
            .style(Style::default().bg(if is_selected { theme.accent } else { theme.bg }))
            .render(
                Rect {
                    x: inner.x,
                    y,
                    width: inner.width,
                    height: 1,
                },
                buf,
            );
        y += 1;
    }

    // Help text at bottom
    let help_area = Rect {
        x: area.x + 2,
        y: area.y + area.height - 2,
        width: area.width.saturating_sub(4),
        height: 1,
    };

    let help = Paragraph::new(Line::from(vec![
        Span::styled("↑↓", Style::default().fg(theme.accent)),
        Span::raw(": Navigate | "),
        Span::styled("Enter", Style::default().fg(theme.accent)),
        Span::raw(": Select | "),
        Span::styled("Esc", Style::default().fg(theme.accent)),
        Span::raw(": Close"),
    ]))
    .style(Style::default().fg(theme.border));

    help.render(help_area, buf);
}
