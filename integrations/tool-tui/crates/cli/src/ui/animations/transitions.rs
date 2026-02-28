//! Text & UI Transitions
//!
//! Smooth fades, slides, typing effects, and polished transitions.

use crossterm::style::Color;
use std::io;
use std::time::Duration;

use super::{flush, print_at};

/// Typing effect - simulates text being typed character by character
pub fn typing_effect(text: &str, delay: Duration) -> io::Result<()> {
    for ch in text.chars() {
        print!("{}", ch);
        flush()?;
        std::thread::sleep(delay);
    }
    println!();
    Ok(())
}

/// Typing effect with color
pub fn typing_effect_colored(text: &str, color: Color, delay: Duration) -> io::Result<()> {
    use crossterm::execute;
    use crossterm::style::{Print, SetForegroundColor};

    execute!(io::stdout(), SetForegroundColor(color))?;
    for ch in text.chars() {
        execute!(io::stdout(), Print(ch))?;
        flush()?;
        std::thread::sleep(delay);
    }
    println!();
    Ok(())
}

/// Fade in text by gradually increasing brightness
pub fn fade_in_text(x: u16, y: u16, text: &str, duration: Duration) -> io::Result<()> {
    let steps = 10u32;
    let step_duration = duration / steps;

    let colors = [Color::DarkGrey, Color::Grey, Color::White];

    for i in 0..steps {
        let color_idx = ((i as usize * colors.len()) / steps as usize).min(colors.len() - 1);
        print_at(x, y, text, colors[color_idx])?;
        flush()?;
        std::thread::sleep(step_duration);
    }

    Ok(())
}

/// Slide text from left to right
pub fn slide_text(y: u16, text: &str, duration: Duration) -> io::Result<()> {
    let (width, _) = super::terminal_size()?;
    let steps = width.min(50) as usize;
    let step_duration = duration / steps as u32;

    for i in 0..steps {
        let x = ((i * width as usize) / steps) as u16;
        print_at(x, y, text, Color::White)?;
        flush()?;
        std::thread::sleep(step_duration);
    }

    Ok(())
}

/// Pulse effect - text grows and shrinks
pub fn pulse_text(x: u16, y: u16, text: &str, cycles: usize) -> io::Result<()> {
    let colors = [
        Color::DarkGrey,
        Color::Grey,
        Color::White,
        Color::Yellow,
        Color::White,
        Color::Grey,
    ];

    for _ in 0..cycles {
        for color in &colors {
            print_at(x, y, text, *color)?;
            flush()?;
            std::thread::sleep(Duration::from_millis(100));
        }
    }

    Ok(())
}

/// Shimmer effect - text sparkles
pub fn shimmer_text(x: u16, y: u16, text: &str, duration: Duration) -> io::Result<()> {
    use rand::Rng;

    let start = std::time::Instant::now();
    let mut rng = rand::thread_rng();

    while start.elapsed() < duration {
        let color = if rng.gen_bool(0.3) {
            Color::Yellow
        } else {
            Color::White
        };

        print_at(x, y, text, color)?;
        flush()?;
        std::thread::sleep(Duration::from_millis(50));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_typing_effect_duration() {
        let delay = Duration::from_millis(10);
        let text = "test";
        // Just verify it doesn't panic
        let _ = typing_effect(text, delay);
    }
}
