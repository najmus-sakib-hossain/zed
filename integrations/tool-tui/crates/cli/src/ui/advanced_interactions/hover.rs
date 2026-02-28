use crossterm::event::{self, Event, KeyCode, KeyModifiers, MouseEventKind};
use ratatui::text::Line;
use std::time::{Duration, Instant};

use super::common::{
    TerminalConfig, create_block, create_demo_layout, create_help_paragraph, create_status_line,
    create_title, init_terminal, restore_terminal,
};

/// Mouse hover detection demo
pub fn run_hover_demo() -> anyhow::Result<()> {
    let config = TerminalConfig::with_mouse();
    let mut terminal = init_terminal(config)?;

    let mut hover_pos: Option<(u16, u16)> = None;
    let mut hover_time = Instant::now();
    let mut tooltip_visible = false;

    let result = (|| -> anyhow::Result<()> {
        loop {
            terminal.draw(|f| {
                let chunks = create_demo_layout(f.area());

                let mut lines: Vec<Line> = vec![
                    create_title("Mouse Hover Detection"),
                    Line::from(""),
                    Line::from("Move your mouse over this area to detect hover."),
                    Line::from("Hover for 1 second to show tooltip."),
                    Line::from(""),
                ];

                if let Some(pos) = hover_pos {
                    lines.push(create_status_line(
                        "Hovering at",
                        &format!("x={}, y={}", pos.0, pos.1),
                        ratatui::style::Color::Green,
                    ));

                    if tooltip_visible {
                        lines.push(Line::from(""));
                        lines.push(Line::from("💡 Tooltip: You've been hovering for 1+ second!"));
                    }
                }

                let content =
                    ratatui::widgets::Paragraph::new(lines).block(create_block("Hover Test"));
                f.render_widget(content, chunks[0]);

                let help = create_help_paragraph(vec![("Mouse", "Move to hover"), ("Q", "Quit")]);
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
                        if mouse.kind == MouseEventKind::Moved {
                            let new_pos = (mouse.column, mouse.row);
                            if hover_pos != Some(new_pos) {
                                hover_pos = Some(new_pos);
                                hover_time = Instant::now();
                                tooltip_visible = false;
                            }
                        }
                    }
                    _ => {}
                }
            }

            if hover_pos.is_some() && hover_time.elapsed() > Duration::from_secs(1) {
                tooltip_visible = true;
            }
        }
        Ok(())
    })();

    restore_terminal(&mut terminal, config)?;
    result
}
