use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers, MouseButton, MouseEventKind},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};
use std::{io, time::Duration};

/// Right-click context menu demo
pub fn run_right_click_demo() -> anyhow::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, crossterm::event::EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut menu_pos: Option<(u16, u16)> = None;
    let mut selected_action = String::new();

    let result = (|| -> anyhow::Result<()> {
        loop {
            terminal.draw(|f| {
                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([Constraint::Min(0), Constraint::Length(5)])
                    .split(f.area());

                let mut lines = vec![
                    Line::from(vec![Span::styled(
                        "Right-Click Context Menu",
                        Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
                    )]),
                    Line::from(""),
                    Line::from("Right-click anywhere to open context menu."),
                    Line::from(""),
                ];

                if !selected_action.is_empty() {
                    lines.push(Line::from(vec![
                        Span::styled("Last action: ", Style::default().fg(Color::Yellow)),
                        Span::styled(
                            &selected_action,
                            Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
                        ),
                    ]));
                }

                let content = Paragraph::new(lines)
                    .block(Block::default().borders(Borders::ALL).title("Right-Click Test"));
                f.render_widget(content, chunks[0]);

                if let Some((x, y)) = menu_pos {
                    let menu_area = Rect {
                        x: x.min(f.area().width.saturating_sub(20)),
                        y: y.min(f.area().height.saturating_sub(8)),
                        width: 20,
                        height: 8,
                    };

                    let menu_items = vec![
                        Line::from("  Copy"),
                        Line::from("  Paste"),
                        Line::from("  Cut"),
                        Line::from("  ───────"),
                        Line::from("  Delete"),
                        Line::from("  Properties"),
                    ];

                    let menu = Paragraph::new(menu_items).block(
                        Block::default()
                            .borders(Borders::ALL)
                            .border_style(Style::default().fg(Color::Yellow))
                            .title("Menu"),
                    );
                    f.render_widget(menu, menu_area);
                }

                let help = Paragraph::new(vec![Line::from(vec![
                    Span::styled("Mouse: ", Style::default().fg(Color::Green)),
                    Span::raw("Right-click for menu | "),
                    Span::styled("Esc", Style::default().fg(Color::Yellow)),
                    Span::raw(": Close menu | "),
                    Span::styled("Q", Style::default().fg(Color::Red)),
                    Span::raw(": Quit"),
                ])])
                .block(Block::default().borders(Borders::ALL).title("Controls"));
                f.render_widget(help, chunks[1]);
            })?;

            if event::poll(Duration::from_millis(16))? {
                match event::read()? {
                    Event::Key(key) => {
                        if key.code == KeyCode::Char('q')
                            || (key.code == KeyCode::Char('c')
                                && key.modifiers.contains(KeyModifiers::CONTROL))
                        {
                            break;
                        }
                        if key.code == KeyCode::Esc {
                            menu_pos = None;
                        }
                    }
                    Event::Mouse(mouse) => match mouse.kind {
                        MouseEventKind::Down(MouseButton::Right) => {
                            menu_pos = Some((mouse.column, mouse.row));
                        }
                        MouseEventKind::Down(MouseButton::Left) => {
                            if menu_pos.is_some() {
                                selected_action = "Menu item clicked".to_string();
                                menu_pos = None;
                            }
                        }
                        _ => {}
                    },
                    _ => {}
                }
            }
        }
        Ok(())
    })();

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        crossterm::event::DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    result
}
