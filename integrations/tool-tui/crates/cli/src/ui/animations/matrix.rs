//! Matrix Digital Rain Animation
//!
//! The ultimate "thinking" loader - cascading green code rain with customizable
//! speed, colors, and density. Makes dx feel futuristic and proactive.

use crossterm::style::Color;
use rand::Rng;
use std::io;
use std::time::{Duration, Instant};

use super::{
    MATRIX_FRAME_DURATION, clear_screen, flush, init_animation_mode, print_at, restore_terminal,
    terminal_size,
};

/// Matrix rain character set (Katakana + ASCII)
const MATRIX_CHARS: &[char] = &[
    'ﾊ', 'ﾐ', 'ﾋ', 'ｰ', 'ｳ', 'ｼ', 'ﾅ', 'ﾓ', 'ﾆ', 'ｻ', 'ﾜ', 'ﾂ', 'ｵ', 'ﾘ', 'ｱ', 'ﾎ', 'ﾃ', 'ﾏ', 'ｹ',
    'ﾒ', 'ｴ', 'ｶ', 'ｷ', 'ﾑ', 'ﾕ', 'ﾗ', 'ｾ', 'ﾈ', 'ｽ', 'ﾀ', 'ﾇ', 'ﾍ', '0', '1', '2', '3', '4', '5',
    '6', '7', '8', '9', 'Z', ':', '.', '"', '=', '*', '+', '-', '<', '>', '¦', '|', 'ç',
];

/// A single rain drop column
struct RainDrop {
    x: u16,
    y: i32,
    speed: u8,
    length: u8,
    chars: Vec<char>,
}

impl RainDrop {
    fn new(x: u16, height: u16) -> Self {
        let mut rng = rand::thread_rng();
        let length = rng.gen_range(5..20);
        let chars: Vec<char> = (0..length)
            .map(|_| MATRIX_CHARS[rng.gen_range(0..MATRIX_CHARS.len())])
            .collect();

        Self {
            x,
            y: -(rng.gen_range(0..height as i32)),
            speed: rng.gen_range(1..4),
            length,
            chars,
        }
    }

    fn update(&mut self, height: u16) {
        self.y += self.speed as i32;
        if self.y > height as i32 + self.length as i32 {
            self.y = -(self.length as i32);
            let mut rng = rand::thread_rng();
            self.speed = rng.gen_range(1..4);
        }
    }

    fn draw(&self, height: u16) -> io::Result<()> {
        for (i, &ch) in self.chars.iter().enumerate() {
            let y = self.y + i as i32;
            if y >= 0 && y < height as i32 {
                let color = if i == self.chars.len() - 1 {
                    Color::White // Bright head
                } else if i > self.chars.len() - 4 {
                    Color::Green // Bright green
                } else {
                    Color::DarkGreen // Dim tail
                };
                print_at(self.x, y as u16, &ch.to_string(), color)?;
            }
        }
        Ok(())
    }
}

/// Matrix rain animation
pub struct MatrixRain {
    drops: Vec<RainDrop>,
    duration: Option<Duration>,
}

impl MatrixRain {
    /// Create a new matrix rain animation
    pub fn new() -> Self {
        Self {
            drops: Vec::new(),
            duration: None,
        }
    }

    /// Set the duration for the animation (None = infinite)
    pub fn with_duration(mut self, duration: Duration) -> Self {
        self.duration = Some(duration);
        self
    }

    /// Run the matrix rain animation
    pub fn run(&mut self) -> io::Result<()> {
        init_animation_mode()?;

        let (width, height) = terminal_size()?;

        // Initialize rain drops for each column
        for x in 0..width {
            if rand::thread_rng().gen_bool(0.3) {
                self.drops.push(RainDrop::new(x, height));
            }
        }

        let start = Instant::now();
        let mut frame_count = 0u64;

        loop {
            // Check duration
            if let Some(duration) = self.duration
                && start.elapsed() >= duration
            {
                break;
            }

            clear_screen()?;

            // Update and draw all drops
            for drop in &mut self.drops {
                drop.update(height);
                drop.draw(height)?;
            }

            flush()?;

            // Maintain frame rate
            let frame_time = MATRIX_FRAME_DURATION;
            std::thread::sleep(frame_time);

            frame_count += 1;

            // Add new drops occasionally
            if frame_count.is_multiple_of(10) && self.drops.len() < width as usize {
                let x = rand::thread_rng().gen_range(0..width);
                if !self.drops.iter().any(|d| d.x == x) {
                    self.drops.push(RainDrop::new(x, height));
                }
            }
        }

        restore_terminal()?;
        Ok(())
    }
}

impl Default for MatrixRain {
    fn default() -> Self {
        Self::new()
    }
}

/// Show matrix rain for a specific duration
pub fn show_matrix(duration: Duration) -> io::Result<()> {
    MatrixRain::new().with_duration(duration).run()
}

/// Show matrix rain with a message overlay
pub fn show_matrix_with_message(message: &str, duration: Duration) -> io::Result<()> {
    init_animation_mode()?;

    let (width, height) = terminal_size()?;
    let mut rain = MatrixRain::new();

    // Initialize drops
    for x in 0..width {
        if rand::thread_rng().gen_bool(0.3) {
            rain.drops.push(RainDrop::new(x, height));
        }
    }

    let start = Instant::now();
    let msg_x = (width.saturating_sub(message.len() as u16)) / 2;
    let msg_y = height / 2;

    while start.elapsed() < duration {
        clear_screen()?;

        // Draw rain
        for drop in &mut rain.drops {
            drop.update(height);
            drop.draw(height)?;
        }

        // Draw message overlay
        print_at(msg_x, msg_y, message, Color::White)?;

        flush()?;
        std::thread::sleep(MATRIX_FRAME_DURATION);
    }

    restore_terminal()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_matrix_chars_not_empty() {
        assert!(!MATRIX_CHARS.is_empty());
    }

    #[test]
    fn test_raindrop_creation() {
        let drop = RainDrop::new(10, 50);
        assert_eq!(drop.x, 10);
        assert!(drop.length >= 5 && drop.length < 20);
        assert_eq!(drop.chars.len(), drop.length as usize);
    }
}
