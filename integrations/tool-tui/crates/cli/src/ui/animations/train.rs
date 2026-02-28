//! Steam Locomotive Train Animation
//!
//! Classic ASCII train chugging across the screen with smoke and wheels spinning.
//! Fun error/punishment animation or celebration.

use crossterm::style::Color;
use std::io;
use std::time::{Duration, Instant};

use super::{clear_screen, flush, init_animation_mode, print_at, restore_terminal, terminal_size};

/// ASCII art frames for the steam locomotive
const TRAIN_FRAMES: &[&str] = &[
    r#"
      ====        ________                ___________
  _D _|  |_______/        \__I_I_____===__|_________|
   |(_)---  |   H\________/ |   |        =|___ ___|   _________________
   /     |  |   H  |  |     |   |         ||_| |_||   _|                \_____A
  |      |  |   H  |__--------------------| [___] |   =|                        |
  | ________|___H__/__|_____/[][]~\_______|       |   -|                        |
  |/ |   |-----------I_____I [][] []  D   |=======|____|________________________|_
__/ =| o |=-~~\  /~~\  /~~\  /~~\ ____Y___________|__|__________________________|_
 |/-=|___|=O=====O=====O=====O   |_____/~\___/          |_D__D__D_|  |_D__D__D_|
  \_/      \__/  \__/  \__/  \__/      \_/               \_/   \_/    \_/   \_/
"#,
    r#"
      ====        ________                ___________
  _D _|  |_______/        \__I_I_____===__|_________|
   |(_)---  |   H\________/ |   |        =|___ ___|   _________________
   /     |  |   H  |  |     |   |         ||_| |_||   _|                \_____A
  |      |  |   H  |__--------------------| [___] |   =|                        |
  | ________|___H__/__|_____/[][]~\_______|       |   -|                        |
  |/ |   |-----------I_____I [][] []  D   |=======|____|________________________|_
__/ =| o |=-~~\  /~~\  /~~\  /~~\ ____Y___________|__|__________________________|_
 |/-=|___|=    O=====O=====O=====O|_____/~\___/          |_D__D__D_|  |_D__D__D_|
  \_/      \__/  \__/  \__/  \__/      \_/               \_/   \_/    \_/   \_/
"#,
];

/// Train animation
pub struct TrainAnimation {
    duration: Duration,
}

impl TrainAnimation {
    /// Create a new train animation
    pub fn new() -> Self {
        Self {
            duration: Duration::from_secs(3),
        }
    }

    /// Set the duration for the animation
    pub fn with_duration(mut self, duration: Duration) -> Self {
        self.duration = duration;
        self
    }

    /// Run the train animation
    pub fn run(&self) -> io::Result<()> {
        init_animation_mode()?;

        let (width, height) = terminal_size()?;
        let start = Instant::now();
        let mut frame_idx = 0;
        let mut x_pos = width as i32;
        let frame_time = Duration::from_millis(50);
        let mut last_whistle = Instant::now();

        while start.elapsed() < self.duration {
            let frame_start = Instant::now();

            clear_screen()?;

            // Draw train
            let frame = TRAIN_FRAMES[frame_idx % TRAIN_FRAMES.len()];
            let lines: Vec<&str> = frame.lines().collect();
            let train_height = lines.len() as u16;
            let y_start = height.saturating_sub(train_height) / 2;

            for (i, line) in lines.iter().enumerate() {
                if !line.is_empty() {
                    let y = y_start + i as u16;
                    if x_pos >= 0 && (x_pos as u16) < width {
                        print_at(x_pos as u16, y, line, Color::Yellow)?;
                    }
                }
            }

            flush()?;

            // Play whistle every 2 seconds
            if last_whistle.elapsed() > Duration::from_secs(2) {
                super::sounds::play_train_whistle();
                last_whistle = Instant::now();
            }

            // Move train left (slower)
            x_pos -= 1;
            if x_pos < -(80i32) {
                x_pos = width as i32;
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
}

impl Default for TrainAnimation {
    fn default() -> Self {
        Self::new()
    }
}

/// Show train animation
pub fn show_train() -> io::Result<()> {
    TrainAnimation::new().run()
}

/// Show train with custom duration
pub fn show_train_duration(duration: Duration) -> io::Result<()> {
    TrainAnimation::new().with_duration(duration).run()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_train_frames_not_empty() {
        assert!(!TRAIN_FRAMES.is_empty());
    }

    #[test]
    fn test_train_animation_creation() {
        let train = TrainAnimation::new();
        assert_eq!(train.duration, Duration::from_secs(3));
    }
}
