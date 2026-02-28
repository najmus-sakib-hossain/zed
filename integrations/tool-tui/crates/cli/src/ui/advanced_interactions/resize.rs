use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Size},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};
use std::{io, time::Duration};

/// Resize detection demo
pub fn run_resize_demo() -> anyhow::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut resize_count = 0;
    let mut last_size = terminal.size()?;

    let result = (|| -> anyhow::Result<()> {
        loop {
            terminal.draw(|f| {
                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([Constraint::Min(0), Constraint::Length(5)])
                    .split(f.area());

                let lines = vec![
                    Line::from(vec![Span::styled(
                        "Terminal Resize Detection",
                        Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
                    )]),
                    Line::from(""),
                    Line::from("Try resizing your terminal window."),
                    Line::from(""),
                    Line::from(vec![
                        Span::styled("Current size: ", Style::default().fg(Color::Yellow)),
                        Span::styled(
                            format!("{}x{}", last_size.width, last_size.height),
                            Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
                        ),
                    ]),
                    Line::from(vec![
                        Span::styled("Resize count: ", Style::default().fg(Color::Yellow)),
                        Span::styled(
                            format!("{}", resize_count),
                            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
                        ),
                    ]),
                ];

                let content = Paragraph::new(lines)
                    .block(Block::default().borders(Borders::ALL).title("Resize Test"));
                f.render_widget(content, chunks[0]);

                let help = Paragraph::new(vec![Line::from(vec![
                    Span::styled("Terminal: ", Style::default().fg(Color::Green)),
                    Span::raw("Resize window | "),
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
                    Event::Resize(width, height) => {
                        resize_count += 1;
                        last_size = Size { width, height };
                    }
                    _ => {}
                }
            }
        }
        Ok(())
    })();

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}
