use crossterm::{
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
use std::io;

/// Terminal setup configuration
#[derive(Debug, Clone, Copy, Default)]
pub struct TerminalConfig {
    pub enable_mouse: bool,
    pub enable_focus: bool,
    pub enable_paste: bool,
}

impl TerminalConfig {
    pub fn with_mouse() -> Self {
        Self {
            enable_mouse: true,
            ..Default::default()
        }
    }

    pub fn with_focus() -> Self {
        Self {
            enable_focus: true,
            ..Default::default()
        }
    }

    pub fn with_paste() -> Self {
        Self {
            enable_paste: true,
            ..Default::default()
        }
    }
}

/// Initialize terminal with configuration
pub fn init_terminal(
    config: TerminalConfig,
) -> anyhow::Result<Terminal<CrosstermBackend<io::Stdout>>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();

    if config.enable_mouse {
        execute!(stdout, EnterAlternateScreen, crossterm::event::EnableMouseCapture)?;
    } else if config.enable_focus {
        execute!(stdout, EnterAlternateScreen, crossterm::event::EnableFocusChange)?;
    } else if config.enable_paste {
        execute!(stdout, EnterAlternateScreen, crossterm::event::EnableBracketedPaste)?;
    } else {
        execute!(stdout, EnterAlternateScreen)?;
    }

    let backend = CrosstermBackend::new(stdout);
    Terminal::new(backend).map_err(Into::into)
}

/// Restore terminal to normal state
pub fn restore_terminal(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    config: TerminalConfig,
) -> anyhow::Result<()> {
    disable_raw_mode()?;

    if config.enable_mouse {
        execute!(
            terminal.backend_mut(),
            LeaveAlternateScreen,
            crossterm::event::DisableMouseCapture
        )?;
    } else if config.enable_focus {
        execute!(
            terminal.backend_mut(),
            LeaveAlternateScreen,
            crossterm::event::DisableFocusChange
        )?;
    } else if config.enable_paste {
        execute!(
            terminal.backend_mut(),
            LeaveAlternateScreen,
            crossterm::event::DisableBracketedPaste
        )?;
    } else {
        execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    }

    terminal.show_cursor()?;
    Ok(())
}

/// Create standard two-panel layout (main + controls)
pub fn create_demo_layout(area: Rect) -> Vec<Rect> {
    Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(5)])
        .split(area)
        .to_vec()
}

/// Create a styled title line
pub fn create_title(text: &str) -> Line<'_> {
    Line::from(vec![Span::styled(
        text,
        Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
    )])
}

/// Create a bordered block with title
pub fn create_block(title: &str) -> Block<'static> {
    Block::default().borders(Borders::ALL).title(title.to_string())
}

/// Create a help control line
pub fn create_help_line(controls: Vec<(&str, &str)>) -> Line<'static> {
    let mut spans = Vec::new();

    for (i, (key, desc)) in controls.iter().enumerate() {
        if i > 0 {
            spans.push(Span::raw(" | "));
        }

        let color = match *key {
            "Q" => Color::Red,
            "Esc" => Color::Yellow,
            _ => Color::Green,
        };

        spans.push(Span::styled(key.to_string(), Style::default().fg(color)));
        spans.push(Span::raw(format!(": {}", desc)));
    }

    Line::from(spans)
}

/// Create a status line with label and value
pub fn create_status_line(label: &str, value: &str, value_color: Color) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!("{}: ", label), Style::default().fg(Color::Yellow)),
        Span::styled(
            value.to_string(),
            Style::default().fg(value_color).add_modifier(Modifier::BOLD),
        ),
    ])
}

/// Create a help paragraph with controls
pub fn create_help_paragraph(controls: Vec<(&str, &str)>) -> Paragraph<'static> {
    Paragraph::new(vec![create_help_line(controls)]).block(create_block("Controls"))
}
