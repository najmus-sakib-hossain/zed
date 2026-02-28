use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};
use std::{io, time::Duration};

/// Focus/Blur detection demo
pub fn run_focus_demo() -> anyhow::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, crossterm::event::EnableFocusChange)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut is_focused = true;
    let mut focus_changes = 0;

    let result = (|| -> anyhow::Result<()> {
        loop {
            terminal.draw(|f| {
                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([Constraint::Min(0), Constraint::Length(5)])
                    .split(f.area());

                let status_color = if is_focused {
                    Color::Green
                } else {
                    Color::Gray
                };
                let status_text = if is_focused { "FOCUSED" } else { "UNFOCUSED" };

                let lines = vec![
                    Line::from(vec![Span::styled(
                        "Terminal Focus Detection",
                        Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
                    )]),
                    Line::from(""),
                    Line::from("Click outside the terminal to lose focus, click back to regain."),
                    Line::from(""),
                    Line::from(vec![
                        Span::styled("Status: ", Style::default().fg(Color::Yellow)),
                        Span::styled(
                            status_text,
                            Style::default().fg(status_color).add_modifier(Modifier::BOLD),
                        ),
                    ]),
                    Line::from(vec![
                        Span::styled("Focus changes: ", Style::default().fg(Color::Yellow)),
                        Span::styled(
                            format!("{}", focus_changes),
                            Style::default().fg(Color::Cyan),
                        ),
                    ]),
                ];

                let content = Paragraph::new(lines)
                    .block(Block::default().borders(Borders::ALL).title("Focus Test"));
                f.render_widget(content, chunks[0]);

                let help = Paragraph::new(vec![Line::from(vec![
                    Span::styled("Terminal: ", Style::default().fg(Color::Green)),
                    Span::raw("Click in/out | "),
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
                    }
                    Event::FocusGained => {
                        is_focused = true;
                        focus_changes += 1;
                    }
                    Event::FocusLost => {
                        is_focused = false;
                        focus_changes += 1;
                    }
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
        crossterm::event::DisableFocusChange
    )?;
    terminal.show_cursor()?;

    result
}
