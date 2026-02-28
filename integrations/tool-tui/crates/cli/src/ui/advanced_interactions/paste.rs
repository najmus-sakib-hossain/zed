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
    widgets::{Block, Borders, Paragraph, Wrap},
};
use std::{io, time::Duration};

/// Paste detection demo
pub fn run_paste_demo() -> anyhow::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, crossterm::event::EnableBracketedPaste)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut pasted_text = String::new();
    let mut paste_count = 0;

    let result = (|| -> anyhow::Result<()> {
        loop {
            terminal.draw(|f| {
                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([Constraint::Min(0), Constraint::Length(5)])
                    .split(f.area());

                let mut lines = vec![
                    Line::from(vec![Span::styled(
                        "Paste Detection",
                        Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
                    )]),
                    Line::from(""),
                    Line::from("Try pasting text (Ctrl+V or right-click paste)."),
                    Line::from(""),
                    Line::from(vec![
                        Span::styled("Paste count: ", Style::default().fg(Color::Yellow)),
                        Span::styled(
                            format!("{}", paste_count),
                            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
                        ),
                    ]),
                ];

                if !pasted_text.is_empty() {
                    lines.push(Line::from(""));
                    lines.push(Line::from(vec![Span::styled(
                        "Last paste: ",
                        Style::default().fg(Color::Yellow),
                    )]));
                    lines.push(Line::from(Span::styled(
                        &pasted_text,
                        Style::default().fg(Color::Green),
                    )));
                }

                let content = Paragraph::new(lines)
                    .block(Block::default().borders(Borders::ALL).title("Paste Test"))
                    .wrap(Wrap { trim: true });
                f.render_widget(content, chunks[0]);

                let help = Paragraph::new(vec![Line::from(vec![
                    Span::styled("Keyboard: ", Style::default().fg(Color::Green)),
                    Span::raw("Paste text | "),
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
                    Event::Paste(text) => {
                        pasted_text = text;
                        paste_count += 1;
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
        crossterm::event::DisableBracketedPaste
    )?;
    terminal.show_cursor()?;

    result
}
