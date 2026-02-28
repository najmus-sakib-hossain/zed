//! Animation module exports

pub mod ascii_art;
pub mod confetti;
pub mod gameoflife;
pub mod images;
pub mod matrix;
pub mod particles;
pub mod sounds;
pub mod train;
pub mod transitions;
pub mod video;
pub mod visualizer;

use std::io::{self, Write};
use std::time::Duration;

use crossterm::{
    cursor, execute,
    style::{Color, Print, SetForegroundColor},
    terminal::{self, Clear, ClearType},
};

/// Animation frame rate (60 FPS for smooth animations)
pub const FRAME_DURATION: Duration = Duration::from_millis(16);

/// Matrix rain frame rate (80ms for classic feel)
pub const MATRIX_FRAME_DURATION: Duration = Duration::from_millis(80);

/// Initialize terminal for animations
pub fn init_animation_mode() -> io::Result<()> {
    terminal::enable_raw_mode()?;
    execute!(io::stdout(), terminal::EnterAlternateScreen, cursor::Hide)?;
    Ok(())
}

/// Restore terminal from animation mode
pub fn restore_terminal() -> io::Result<()> {
    execute!(io::stdout(), terminal::LeaveAlternateScreen, cursor::Show)?;
    terminal::disable_raw_mode()?;
    Ok(())
}

/// Clear the screen
pub fn clear_screen() -> io::Result<()> {
    execute!(io::stdout(), Clear(ClearType::All), cursor::MoveTo(0, 0))?;
    Ok(())
}

/// Print colored text at position
pub fn print_at(x: u16, y: u16, text: &str, color: Color) -> io::Result<()> {
    execute!(io::stdout(), cursor::MoveTo(x, y), SetForegroundColor(color), Print(text))?;
    Ok(())
}

/// Flush output
pub fn flush() -> io::Result<()> {
    io::stdout().flush()
}

/// Get terminal size
pub fn terminal_size() -> io::Result<(u16, u16)> {
    terminal::size()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_frame_durations() {
        assert_eq!(FRAME_DURATION.as_millis(), 16);
        assert_eq!(MATRIX_FRAME_DURATION.as_millis(), 80);
    }
}
