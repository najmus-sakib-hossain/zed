use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers, MouseButton, MouseEventKind},
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

/// Middle-click detection demo
pub fn run_middle_click_demo() -> anyhow::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, crossterm::event::EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut middle_clicks = 0;
    let mut last_pos = None;

    let result = (|| -> anyhow::Result<()> {
        loop {
            terminal.draw(|f| {
                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([Constraint::Min(0), Constraint::Length(5)])
                    .split(f.area());

                let lines = vec![
                    Line::from(vec![Span::styled(
                        "Middle-Click Detection",
                        Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
                    )]),
                    Line::from(""),
                    Line::from("Click your mouse wheel (middle button) to test."),
                    Line::from(""),
                    Line::from(vec![
                        Span::styled("Middle clicks: ", Style::default().fg(Color::Yellow)),
                        Span::styled(
                            format!("{}", middle_clicks),
                            Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
                        ),
                    ]),
                    Line::from(""),
                    Line::from(if let Some((x, y)) = last_pos {
                        format!("Last position: x={}, y={}", x, y)
                    } else {
                        "No clicks yet".to_string()
                    }),
                ];

                let content = Paragraph::new(lines)
                    .block(Block::default().borders(Borders::ALL).title("Middle-Click Test"));
                f.render_widget(content, chunks[0]);

                let help = Paragraph::new(vec![Line::from(vec![
                    Span::styled("Mouse: ", Style::default().fg(Color::Green)),
                    Span::raw("Middle-click to test | "),
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
                    Event::Mouse(mouse) => {
                        if let MouseEventKind::Down(MouseButton::Middle) = mouse.kind {
                            middle_clicks += 1;
                            last_pos = Some((mouse.column, mouse.row));
                        }
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
        crossterm::event::DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    result
}
