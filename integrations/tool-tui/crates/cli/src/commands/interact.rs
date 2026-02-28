use crate::ui::{
    keyboard_shortcuts::{KeyboardShortcutManager, ShortcutAction},
    terminal_interactions::{DOUBLE_CLICK_THRESHOLD, InteractiveList, Position},
};
use anyhow::Result;
use clap::Subcommand;
use crossterm::{
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers, MouseButton,
        MouseEventKind,
    },
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use owo_colors::OwoColorize;
use ratatui::{
    Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};
use std::{
    io,
    time::{Duration, Instant},
};

#[derive(Subcommand, Debug)]
pub enum InteractCommand {
    /// Test click detection
    Click,
    /// Test double-click detection
    DoubleClick,
    /// Test scroll detection
    Scroll,
    /// Test keyboard shortcuts
    Keyboard,
    /// Show all keyboard shortcuts reference
    Shortcuts,
    /// Interactive list with all features
    List {
        /// Items to display
        #[arg(default_values = &["Item 1", "Item 2", "Item 3", "Item 4", "Item 5"])]
        items: Vec<String>,
    },
    /// Test mouse hover detection
    Hover,
    /// Test right-click context menu
    RightClick,
    /// Test middle-click detection
    MiddleClick,
    /// Test text selection with mouse
    TextSelection,
    /// Test terminal resize detection
    Resize,
    /// Test terminal focus/blur detection
    Focus,
    /// Test paste detection
    Paste,
    /// Show all available interactions
    All,
}

pub fn handle_interact_command(cmd: InteractCommand) -> Result<()> {
    match cmd {
        InteractCommand::Click => run_click_demo()?,
        InteractCommand::DoubleClick => run_double_click_demo()?,
        InteractCommand::Scroll => run_scroll_demo()?,
        InteractCommand::Keyboard => run_keyboard_demo()?,
        InteractCommand::Shortcuts => show_shortcuts()?,
        InteractCommand::List { items } => {
            let mut list = InteractiveList::new(items);
            list.run()?;
        }
        InteractCommand::Hover => crate::ui::advanced_interactions::run_hover_demo()?,
        InteractCommand::RightClick => crate::ui::advanced_interactions::run_right_click_demo()?,
        InteractCommand::MiddleClick => crate::ui::advanced_interactions::run_middle_click_demo()?,
        InteractCommand::TextSelection => {
            crate::ui::advanced_interactions::run_text_selection_demo()?
        }
        InteractCommand::Resize => crate::ui::advanced_interactions::run_resize_demo()?,
        InteractCommand::Focus => crate::ui::advanced_interactions::run_focus_demo()?,
        InteractCommand::Paste => crate::ui::advanced_interactions::run_paste_demo()?,
        InteractCommand::All => show_all_interactions()?,
    }
    Ok(())
}

fn show_all_interactions() -> Result<()> {
    println!("\n{}", "DX Terminal Interactions - Complete Reference".cyan().bold());
    println!("{}", "=".repeat(60));

    println!("\n{}", "MOUSE INTERACTIONS:".yellow().bold());
    println!(
        "  {} - Left-click detection with position tracking",
        "dx interact click".green()
    );
    println!(
        "  {} - Double-click detection (300ms threshold)",
        "dx interact double-click".green()
    );
    println!("  {} - Right-click context menu", "dx interact right-click".green());
    println!("  {} - Middle mouse button detection", "dx interact middle-click".green());
    println!("  {} - Mouse hover with tooltip (1s delay)", "dx interact hover".green());
    println!("  {} - Mouse wheel scroll up/down", "dx interact scroll".green());
    println!("  {} - Click and drag text selection", "dx interact text-selection".green());

    println!("\n{}", "KEYBOARD INTERACTIONS:".yellow().bold());
    println!("  {} - Key press detection with modifiers", "dx interact keyboard".green());
    println!("  {} - Show all keyboard shortcuts", "dx interact shortcuts".green());
    println!("  {} - Paste detection (Ctrl+V)", "dx interact paste".green());

    println!("\n{}", "TERMINAL EVENTS:".yellow().bold());
    println!("  {} - Terminal window resize detection", "dx interact resize".green());
    println!("  {} - Terminal focus/blur detection", "dx interact focus".green());

    println!("\n{}", "COMBINED FEATURES:".yellow().bold());
    println!("  {} - Interactive list with all features", "dx interact list".green());

    println!("\n{}", "SUPPORTED INTERACTIONS:".cyan().bold());
    println!("  ✓ Left/Right/Middle mouse buttons");
    println!("  ✓ Single/Double/Triple click");
    println!("  ✓ Click and drag");
    println!("  ✓ Mouse hover with tooltips");
    println!("  ✓ Mouse wheel scroll");
    println!("  ✓ Text selection");
    println!("  ✓ Keyboard shortcuts (Ctrl, Alt, Shift combinations)");
    println!("  ✓ Arrow keys, Function keys, Special keys");
    println!("  ✓ Copy/Paste detection");
    println!("  ✓ Terminal resize events");
    println!("  ✓ Focus/Blur events");
    println!("  ✓ Drag and drop");
    println!("  ✓ Context menus");

    println!("\n{}", "Try any command to see it in action!".bright_black());
    println!();

    Ok(())
}

fn run_click_demo() -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut clicks: Vec<Position> = Vec::new();
    let mut last_pos: Option<Position> = None;

    let result = (|| -> Result<()> {
        loop {
            terminal.draw(|f| {
                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([Constraint::Min(0), Constraint::Length(5)])
                    .split(f.area());

                // Main area
                let mut lines = vec![
                    Line::from(vec![Span::styled(
                        "Click Detection Test",
                        Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
                    )]),
                    Line::from(""),
                    Line::from("Click anywhere in this area to register a click."),
                    Line::from(""),
                ];

                if let Some(pos) = last_pos {
                    lines.push(Line::from(vec![
                        Span::styled("Last click: ", Style::default().fg(Color::Yellow)),
                        Span::styled(
                            format!("x={}, y={}", pos.x, pos.y),
                            Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
                        ),
                    ]));
                }

                lines.push(Line::from(""));
                lines.push(Line::from(vec![
                    Span::styled("Total clicks: ", Style::default().fg(Color::Yellow)),
                    Span::styled(
                        format!("{}", clicks.len()),
                        Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
                    ),
                ]));

                if !clicks.is_empty() {
                    lines.push(Line::from(""));
                    lines.push(Line::from(Span::styled(
                        "Recent clicks:",
                        Style::default().fg(Color::Yellow),
                    )));
                    for (i, pos) in clicks.iter().rev().take(5).enumerate() {
                        lines.push(Line::from(format!("  {}. x={}, y={}", i + 1, pos.x, pos.y)));
                    }
                }

                let content = Paragraph::new(lines)
                    .block(Block::default().borders(Borders::ALL).title("Click Test"));
                f.render_widget(content, chunks[0]);

                // Help
                let help = Paragraph::new(vec![Line::from(vec![
                    Span::styled("Mouse: ", Style::default().fg(Color::Green)),
                    Span::raw("Click anywhere to test | "),
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
                        if let MouseEventKind::Down(MouseButton::Left) = mouse.kind {
                            let pos = Position {
                                x: mouse.column,
                                y: mouse.row,
                            };
                            clicks.push(pos);
                            last_pos = Some(pos);
                        }
                    }
                    _ => {}
                }
            }
        }
        Ok(())
    })();

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;
    terminal.show_cursor()?;

    result
}

fn run_double_click_demo() -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut single_clicks = 0;
    let mut double_clicks = 0;
    let mut last_click: Option<(Position, Instant)> = None;
    let mut last_event = "None".to_string();

    let result = (|| -> Result<()> {
        loop {
            terminal.draw(|f| {
                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([Constraint::Min(0), Constraint::Length(5)])
                    .split(f.area());

                let lines = vec![
                    Line::from(vec![Span::styled(
                        "Double-Click Detection Test",
                        Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
                    )]),
                    Line::from(""),
                    Line::from(
                        "Click once for single-click, click twice quickly for double-click.",
                    ),
                    Line::from(""),
                    Line::from(vec![
                        Span::styled("Single clicks: ", Style::default().fg(Color::Yellow)),
                        Span::styled(
                            format!("{}", single_clicks),
                            Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
                        ),
                    ]),
                    Line::from(vec![
                        Span::styled("Double clicks: ", Style::default().fg(Color::Yellow)),
                        Span::styled(
                            format!("{}", double_clicks),
                            Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD),
                        ),
                    ]),
                    Line::from(""),
                    Line::from(vec![
                        Span::styled("Last event: ", Style::default().fg(Color::Yellow)),
                        Span::styled(
                            &last_event,
                            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
                        ),
                    ]),
                    Line::from(""),
                    Line::from(Span::styled(
                        "Tip: Double-click threshold is 300ms",
                        Style::default().fg(Color::Gray),
                    )),
                ];

                let content = Paragraph::new(lines)
                    .block(Block::default().borders(Borders::ALL).title("Double-Click Test"));
                f.render_widget(content, chunks[0]);

                let help = Paragraph::new(vec![Line::from(vec![
                    Span::styled("Mouse: ", Style::default().fg(Color::Green)),
                    Span::raw("Click once or twice | "),
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
                        if let MouseEventKind::Down(MouseButton::Left) = mouse.kind {
                            let pos = Position {
                                x: mouse.column,
                                y: mouse.row,
                            };
                            let now = Instant::now();

                            if let Some((last_pos, last_time)) = last_click
                                && last_time.elapsed() < DOUBLE_CLICK_THRESHOLD
                                && last_pos.x == pos.x
                                && last_pos.y == pos.y
                            {
                                double_clicks += 1;
                                last_event = format!("Double-click at x={}, y={}", pos.x, pos.y);
                                last_click = None;
                                continue;
                            }

                            single_clicks += 1;
                            last_event = format!("Single click at x={}, y={}", pos.x, pos.y);
                            last_click = Some((pos, now));
                        }
                    }
                    _ => {}
                }
            }
        }
        Ok(())
    })();

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;
    terminal.show_cursor()?;

    result
}

fn run_scroll_demo() -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut scroll_position = 0i32;
    let mut scroll_events: Vec<String> = Vec::new();

    // Generate long content to scroll through
    let content_lines: Vec<String> =
        (1..=50).map(|i| format!("Line {} - This is scrollable content", i)).collect();

    let result = (|| -> Result<()> {
        loop {
            terminal.draw(|f| {
                let chunks = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([Constraint::Min(0), Constraint::Length(3)])
                    .split(f.area());

                let main_chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([Constraint::Min(0), Constraint::Length(5)])
                    .split(chunks[0]);

                // Calculate visible range
                let visible_height = (main_chunks[0].height as usize).saturating_sub(4);
                let start_line = scroll_position.max(0) as usize;
                let end_line = (start_line + visible_height).min(content_lines.len());

                let mut lines = vec![
                    Line::from(vec![Span::styled(
                        "Scroll Detection with Browser-Style Scrollbar",
                        Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
                    )]),
                    Line::from(""),
                ];

                // Add visible content
                for line in &content_lines[start_line..end_line] {
                    lines.push(Line::from(line.as_str()));
                }

                let content = Paragraph::new(lines)
                    .block(Block::default().borders(Borders::ALL).title("Scrollable Content"));
                f.render_widget(content, main_chunks[0]);

                // Browser-style scrollbar on the right
                let scrollbar_height = chunks[1].height.saturating_sub(2);
                let total_content = content_lines.len() as f32;
                let visible_ratio = visible_height as f32 / total_content;
                let thumb_height = (scrollbar_height as f32 * visible_ratio).max(1.0) as u16;
                let scroll_ratio =
                    scroll_position as f32 / (total_content - visible_height as f32).max(1.0);
                let thumb_position =
                    (scroll_ratio * (scrollbar_height - thumb_height) as f32) as u16;

                let mut scrollbar_lines = Vec::new();
                for i in 0..scrollbar_height {
                    if i >= thumb_position && i < thumb_position + thumb_height {
                        scrollbar_lines
                            .push(Line::from(Span::styled("█", Style::default().fg(Color::Cyan))));
                    } else {
                        scrollbar_lines.push(Line::from(Span::styled(
                            "│",
                            Style::default().fg(Color::DarkGray),
                        )));
                    }
                }

                let scrollbar =
                    Paragraph::new(scrollbar_lines).block(Block::default().borders(Borders::ALL));
                f.render_widget(scrollbar, chunks[1]);

                // Help
                let help_text = vec![Line::from(vec![
                    Span::styled("Position: ", Style::default().fg(Color::Yellow)),
                    Span::styled(
                        format!("{}/{}", scroll_position, content_lines.len()),
                        Style::default().fg(Color::Green),
                    ),
                    Span::raw(" | "),
                    Span::styled("Mouse: ", Style::default().fg(Color::Green)),
                    Span::raw("Scroll wheel | "),
                    Span::styled("Q", Style::default().fg(Color::Red)),
                    Span::raw(": Quit"),
                ])];

                let help = Paragraph::new(help_text)
                    .block(Block::default().borders(Borders::ALL).title("Controls"));
                f.render_widget(help, main_chunks[1]);
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
                    Event::Mouse(mouse) => match mouse.kind {
                        MouseEventKind::ScrollDown => {
                            scroll_position =
                                (scroll_position + 1).min(content_lines.len() as i32 - 10);
                            scroll_events.push("Scroll DOWN".to_string());
                        }
                        MouseEventKind::ScrollUp => {
                            scroll_position = (scroll_position - 1).max(0);
                            scroll_events.push("Scroll UP".to_string());
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
    execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;
    terminal.show_cursor()?;

    result
}

fn run_keyboard_demo() -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let shortcut_manager = KeyboardShortcutManager::new();
    let mut key_events: Vec<(String, Option<ShortcutAction>)> = Vec::new();

    let result = (|| -> Result<()> {
        loop {
            terminal.draw(|f| {
                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([Constraint::Min(0), Constraint::Length(5)])
                    .split(f.area());

                let mut lines = vec![
                    Line::from(vec![Span::styled(
                        "Keyboard Shortcut Detection Test",
                        Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
                    )]),
                    Line::from(""),
                    Line::from("Press any key or key combination to test detection."),
                    Line::from(""),
                ];

                if !key_events.is_empty() {
                    lines.push(Line::from(Span::styled(
                        "Recent key presses:",
                        Style::default().fg(Color::Yellow),
                    )));
                    for (i, (key_str, action)) in key_events.iter().rev().take(10).enumerate() {
                        if let Some(action) = action {
                            lines.push(Line::from(vec![
                                Span::raw(format!("  {}. ", i + 1)),
                                Span::styled(key_str, Style::default().fg(Color::Green)),
                                Span::raw(" → "),
                                Span::styled(&action.description, Style::default().fg(Color::Cyan)),
                            ]));
                        } else {
                            lines.push(Line::from(vec![
                                Span::raw(format!("  {}. ", i + 1)),
                                Span::styled(key_str, Style::default().fg(Color::Gray)),
                                Span::raw(" (no action)"),
                            ]));
                        }
                    }
                }

                let content = Paragraph::new(lines)
                    .block(Block::default().borders(Borders::ALL).title("Keyboard Test"));
                f.render_widget(content, chunks[0]);

                let help = Paragraph::new(vec![Line::from(vec![
                    Span::styled("Keyboard: ", Style::default().fg(Color::Green)),
                    Span::raw("Press any key | "),
                    Span::styled("Q", Style::default().fg(Color::Red)),
                    Span::raw(": Quit | "),
                    Span::styled("F1", Style::default().fg(Color::Yellow)),
                    Span::raw(": Show all shortcuts"),
                ])])
                .block(Block::default().borders(Borders::ALL).title("Controls"));
                f.render_widget(help, chunks[1]);
            })?;

            if event::poll(Duration::from_millis(16))?
                && let Event::Key(key) = event::read()?
            {
                if key.code == KeyCode::Char('q')
                    || (key.code == KeyCode::Char('c')
                        && key.modifiers.contains(KeyModifiers::CONTROL))
                {
                    break;
                }

                if key.code == KeyCode::F(1) {
                    // Show shortcuts in a separate screen
                    disable_raw_mode()?;
                    execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;
                    terminal.show_cursor()?;

                    println!("\n{}\n", shortcut_manager.format_help());
                    println!("Press Enter to continue...");
                    let mut input = String::new();
                    io::stdin().read_line(&mut input)?;

                    enable_raw_mode()?;
                    execute!(terminal.backend_mut(), EnterAlternateScreen, EnableMouseCapture)?;
                    continue;
                }

                let binding =
                    crate::ui::keyboard_shortcuts::KeyBinding::new(key.code, key.modifiers);
                let key_str = binding.display();
                let action = shortcut_manager.get_action(key.code, key.modifiers).cloned();

                key_events.push((key_str, action));
            }
        }
        Ok(())
    })();

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;
    terminal.show_cursor()?;

    result
}

fn show_shortcuts() -> Result<()> {
    let manager = KeyboardShortcutManager::new();
    println!("{}", manager.format_help());
    Ok(())
}
