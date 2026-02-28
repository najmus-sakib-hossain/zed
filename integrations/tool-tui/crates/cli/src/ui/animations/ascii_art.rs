//! ASCII Art Animations
//!
//! Convert images to animated ASCII art with color.

use std::io;
use std::time::{Duration, Instant};

use super::{clear_screen, flush, init_animation_mode, restore_terminal, terminal_size};

/// Nyan Cat ASCII animation frames
const NYAN_CAT_FRAMES: &[&str] = &[
    r#"
     _____________________
    /                     \
   |  ♪ ♫ ♪ ♫ ♪ ♫ ♪ ♫ ♪  |
    \_____________________/
         /    \
    +---+      +---+
    |   |      |   |
    +---+      +---+
      ,------,
     /        \
    |  ^    ^  |
    |  (oo)    |
     \  ~~   /
      '----'
    "#,
    r#"
     _____________________
    /                     \
   |  ♫ ♪ ♫ ♪ ♫ ♪ ♫ ♪ ♫  |
    \_____________________/
         /    \
    +---+      +---+
    |   |      |   |
    +---+      +---+
      ,------,
     /        \
    |  ^    ^  |
    |  (oo)    |
     \  ~~   /
      '----'
    "#,
];

pub fn show_nyan_cat(duration: Duration) -> io::Result<()> {
    init_animation_mode()?;

    let (width, height) = terminal_size()?;
    let start = Instant::now();
    let mut frame_idx = 0;
    let mut x_pos = 0i32;
    let frame_time = Duration::from_millis(200);

    while start.elapsed() < duration {
        let frame_start = Instant::now();

        clear_screen()?;

        // Draw Nyan Cat
        let frame = NYAN_CAT_FRAMES[frame_idx % NYAN_CAT_FRAMES.len()];
        let lines: Vec<&str> = frame.lines().collect();
        let y_start = (height / 2).saturating_sub(lines.len() as u16 / 2);

        for (i, line) in lines.iter().enumerate() {
            if !line.is_empty() {
                let y = y_start + i as u16;
                if x_pos >= 0 && (x_pos as u16) < width {
                    use crossterm::style::Color;
                    super::print_at(x_pos as u16, y, line, Color::Magenta)?;
                }
            }
        }

        flush()?;

        // Move right
        x_pos += 2;
        if x_pos > width as i32 {
            x_pos = -(30i32);
        }

        frame_idx += 1;

        let elapsed = frame_start.elapsed();
        if elapsed < frame_time {
            std::thread::sleep(frame_time - elapsed);
        }
    }

    restore_terminal()?;
    Ok(())
}

/// Bouncing DVD logo
pub fn show_dvd_logo(duration: Duration) -> io::Result<()> {
    init_animation_mode()?;

    let (width, height) = terminal_size()?;
    let logo = [
        "██████╗ ██╗   ██╗",
        "██╔══██╗╚██╗ ██╔╝",
        "██║  ██║ ╚████╔╝ ",
        "██║  ██║  ╚██╔╝  ",
        "██████╔╝   ██║   ",
        "╚═════╝    ╚═╝   ",
    ];

    let mut x = (width / 2) as f32;
    let mut y = (height / 2) as f32;
    let mut dx = 1.0;
    let mut dy = 0.5;

    let start = Instant::now();
    let frame_time = Duration::from_millis(50);

    let colors = [
        crossterm::style::Color::Red,
        crossterm::style::Color::Green,
        crossterm::style::Color::Blue,
        crossterm::style::Color::Yellow,
        crossterm::style::Color::Magenta,
        crossterm::style::Color::Cyan,
    ];
    let mut color_idx = 0;

    while start.elapsed() < duration {
        let frame_start = Instant::now();

        clear_screen()?;

        // Draw logo
        for (i, line) in logo.iter().enumerate() {
            let draw_y = (y as u16 + i as u16).min(height - 1);
            super::print_at(x as u16, draw_y, line, colors[color_idx])?;
        }

        // Update position
        x += dx;
        y += dy;

        // Bounce off edges
        if x <= 0.0 || x >= (width as f32 - 20.0) {
            dx = -dx;
            color_idx = (color_idx + 1) % colors.len();
        }
        if y <= 0.0 || y >= (height as f32 - logo.len() as f32) {
            dy = -dy;
            color_idx = (color_idx + 1) % colors.len();
        }

        flush()?;

        let elapsed = frame_start.elapsed();
        if elapsed < frame_time {
            std::thread::sleep(frame_time - elapsed);
        }
    }

    restore_terminal()?;
    Ok(())
}
