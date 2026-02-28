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

/// Text selection with mouse demo
pub fn run_text_selection_demo() -> anyhow::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, crossterm::event::EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let lines_text = [
        "Line 1: Click and drag to select text",
        "Line 2: Selected text will be highlighted",
        "Line 3: Try selecting across multiple lines",
        "Line 4: The background will change color",
        "Line 5: This demonstrates text selection",
        "Line 6: Just like in a text editor",
        "Line 7: You can select any part",
        "Line 8: Release mouse to finish selection",
    ];

    let mut selection_start: Option<(u16, u16)> = None;
    let mut selection_end: Option<(u16, u16)> = None;
    let mut is_selecting = false;

    let result = (|| -> anyhow::Result<()> {
        loop {
            terminal.draw(|f| {
                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([Constraint::Min(0), Constraint::Length(5)])
                    .split(f.area());

                let mut display_lines = vec![
                    Line::from(vec![Span::styled(
                        "Text Selection with Highlighting",
                        Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
                    )]),
                    Line::from(""),
                ];

                for (line_idx, text) in lines_text.iter().enumerate() {
                    let line_y = (line_idx + 3) as u16;
                    let mut spans = Vec::new();

                    if let (Some(start), Some(end)) = (selection_start, selection_end) {
                        let (sel_start, sel_end) =
                            if start.1 < end.1 || (start.1 == end.1 && start.0 <= end.0) {
                                (start, end)
                            } else {
                                (end, start)
                            };

                        if line_y >= sel_start.1 && line_y <= sel_end.1 {
                            let chars: Vec<char> = text.chars().collect();
                            let text_len = chars.len();

                            let start_col = if line_y == sel_start.1 {
                                (sel_start.0.saturating_sub(2) as usize).min(text_len)
                            } else {
                                0
                            };
                            let end_col = if line_y == sel_end.1 {
                                (sel_end.0.saturating_sub(2) as usize).min(text_len)
                            } else {
                                text_len
                            };

                            let start_col = start_col.min(text_len);
                            let end_col = end_col.min(text_len).max(start_col);

                            if start_col > 0 {
                                let before: String = chars[..start_col].iter().collect();
                                spans.push(Span::raw(before));
                            }

                            if start_col < end_col {
                                let selected: String = chars[start_col..end_col].iter().collect();
                                spans.push(Span::styled(
                                    selected,
                                    Style::default()
                                        .bg(Color::Blue)
                                        .fg(Color::White)
                                        .add_modifier(Modifier::BOLD),
                                ));
                            }

                            if end_col < text_len {
                                let after: String = chars[end_col..].iter().collect();
                                spans.push(Span::raw(after));
                            }
                        } else {
                            spans.push(Span::raw(*text));
                        }
                    } else {
                        spans.push(Span::raw(*text));
                    }

                    display_lines.push(Line::from(spans));
                }

                display_lines.push(Line::from(""));

                if let (Some(start), Some(end)) = (selection_start, selection_end) {
                    display_lines.push(Line::from(vec![
                        Span::styled("Selection: ", Style::default().fg(Color::Yellow)),
                        Span::styled(
                            format!("({},{}) to ({},{})", start.0, start.1, end.0, end.1),
                            Style::default().fg(Color::Green),
                        ),
                    ]));
                }

                let content = Paragraph::new(display_lines)
                    .block(Block::default().borders(Borders::ALL).title("Selection Test"));
                f.render_widget(content, chunks[0]);

                let help = Paragraph::new(vec![Line::from(vec![
                    Span::styled("Mouse: ", Style::default().fg(Color::Green)),
                    Span::raw("Click and drag to select | "),
                    Span::styled("Esc", Style::default().fg(Color::Yellow)),
                    Span::raw(": Clear | "),
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
                            selection_start = None;
                            selection_end = None;
                        }
                    }
                    Event::Mouse(mouse) => match mouse.kind {
                        MouseEventKind::Down(MouseButton::Left) => {
                            selection_start = Some((mouse.column, mouse.row));
                            selection_end = Some((mouse.column, mouse.row));
                            is_selecting = true;
                        }
                        MouseEventKind::Drag(MouseButton::Left) => {
                            if is_selecting {
                                selection_end = Some((mouse.column, mouse.row));
                            }
                        }
                        MouseEventKind::Up(MouseButton::Left) => {
                            is_selecting = false;
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
