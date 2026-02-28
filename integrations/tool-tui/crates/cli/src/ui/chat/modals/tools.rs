use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Widget},
};

use crate::ui::chat::{modal_list::ModalList, theme::ChatTheme};

#[derive(Debug, Clone)]
pub struct Tool {
    pub name: String,
    pub description: String,
    pub icon: &'static str,
    pub enabled: bool,
    pub category: String,
}

pub fn get_available_tools() -> Vec<Tool> {
    vec![
        Tool {
            name: "agent".to_string(),
            description: "Delegate tasks to other agents".to_string(),
            icon: "[A]",
            enabled: true,
            category: "Built-In".to_string(),
        },
        Tool {
            name: "edit".to_string(),
            description: "Edit files in your workspace".to_string(),
            icon: "[E]",
            enabled: true,
            category: "Built-In".to_string(),
        },
        Tool {
            name: "execute".to_string(),
            description: "Execute code and applications on your machine".to_string(),
            icon: "[X]",
            enabled: true,
            category: "Built-In".to_string(),
        },
        Tool {
            name: "read".to_string(),
            description: "Read files in your workspace".to_string(),
            icon: "[R]",
            enabled: true,
            category: "Built-In".to_string(),
        },
        Tool {
            name: "search".to_string(),
            description: "Search files in your workspace".to_string(),
            icon: "[S]",
            enabled: true,
            category: "Built-In".to_string(),
        },
        Tool {
            name: "todo".to_string(),
            description: "Manage and track todo items for task planning".to_string(),
            icon: "[T]",
            enabled: true,
            category: "Built-In".to_string(),
        },
        Tool {
            name: "vscode".to_string(),
            description: "Use VS Code features".to_string(),
            icon: "[V]",
            enabled: true,
            category: "Built-In".to_string(),
        },
        Tool {
            name: "web".to_string(),
            description: "Fetch information from the web".to_string(),
            icon: "[W]",
            enabled: true,
            category: "Built-In".to_string(),
        },
        Tool {
            name: "elevenlabs".to_string(),
            description: "Configure ElevenLabs Text-to-Speech API".to_string(),
            icon: "[🔊]",
            enabled: false,
            category: "Integrations".to_string(),
        },
    ]
}

pub fn render(
    full_area: Rect,
    buf: &mut Buffer,
    theme: &ChatTheme,
    list: &ModalList,
    tools: &[Tool],
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
        .title(" Configure Tools ")
        .title_alignment(Alignment::Center)
        .style(Style::default().bg(theme.bg));

    let inner = block.inner(area);
    block.render(area, buf);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(10),
            Constraint::Length(2),
        ])
        .split(inner);

    // Header with search and count
    let enabled_count = tools.iter().filter(|t| t.enabled).count();
    let header = Line::from(vec![
        Span::styled("Select tools that are available to chat.", Style::default().fg(theme.fg)),
        Span::raw("  "),
        Span::styled(
            format!("{} Selected", enabled_count),
            Style::default().fg(theme.accent).add_modifier(Modifier::BOLD),
        ),
    ]);

    Paragraph::new(vec![Line::from(""), header])
        .style(Style::default().bg(theme.bg))
        .render(chunks[0], buf);

    // Tools list
    let tools_area = chunks[1];
    let mut y = tools_area.y;

    // Category header
    if !tools.is_empty() {
        let category_line = Line::from(vec![
            Span::styled("[+] ", Style::default().fg(theme.border)),
            Span::styled("Built-In", Style::default().fg(theme.fg).add_modifier(Modifier::BOLD)),
        ]);
        Paragraph::new(category_line).style(Style::default().bg(theme.bg)).render(
            Rect {
                x: tools_area.x,
                y,
                width: tools_area.width,
                height: 1,
            },
            buf,
        );
        y += 1;
    }

    // Render tools
    for (i, tool) in tools.iter().enumerate() {
        if y >= tools_area.bottom() {
            break;
        }

        let is_selected = i == list.selected;
        let checkbox = if tool.enabled { "[ON]" } else { "[OFF]" };

        let tool_line = Line::from(vec![
            Span::raw("  "),
            Span::styled(checkbox, Style::default().fg(theme.accent)),
            Span::raw(" "),
            Span::styled(tool.icon, Style::default().fg(theme.accent)),
            Span::raw(" "),
            Span::styled(
                &tool.name,
                if is_selected {
                    Style::default().fg(theme.bg).bg(theme.accent).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(theme.fg)
                },
            ),
            Span::raw("  "),
            Span::styled(
                &tool.description,
                if is_selected {
                    Style::default().fg(theme.bg).bg(theme.accent)
                } else {
                    Style::default().fg(theme.border)
                },
            ),
        ]);

        Paragraph::new(tool_line)
            .style(Style::default().bg(if is_selected { theme.accent } else { theme.bg }))
            .render(
                Rect {
                    x: tools_area.x,
                    y,
                    width: tools_area.width,
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
        Span::styled("Space", Style::default().fg(theme.accent)),
        Span::raw(": Toggle | "),
        Span::styled("Enter", Style::default().fg(theme.accent)),
        Span::raw(": Save | "),
        Span::styled("Esc", Style::default().fg(theme.accent)),
        Span::raw(": Cancel"),
    ]))
    .style(Style::default().fg(theme.border));

    help.render(help_area, buf);
}
