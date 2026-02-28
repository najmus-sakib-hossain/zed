//! Particle Systems & Custom Effects
//!
//! Rain, snow, starfields, and other particle-based animations.

use crossterm::style::Color;
use rand::Rng;
use std::io;
use std::time::{Duration, Instant};

use super::{clear_screen, flush, init_animation_mode, print_at, restore_terminal, terminal_size};

struct Star {
    x: u16,
    y: u16,
    brightness: u8,
    twinkle_speed: u8,
}

impl Star {
    fn new(width: u16, height: u16) -> Self {
        let mut rng = rand::thread_rng();
        Self {
            x: rng.gen_range(0..width),
            y: rng.gen_range(0..height),
            brightness: rng.gen_range(0..3),
            twinkle_speed: rng.gen_range(1..5),
        }
    }

    fn update(&mut self, frame: u64) {
        if frame.is_multiple_of(self.twinkle_speed as u64) {
            self.brightness = (self.brightness + 1) % 3;
        }
    }

    fn draw(&self) -> io::Result<()> {
        let (char, color) = match self.brightness {
            0 => ('.', Color::DarkGrey),
            1 => ('*', Color::White),
            _ => ('✦', Color::Yellow),
        };
        print_at(self.x, self.y, &char.to_string(), color)?;
        Ok(())
    }
}

pub struct Starfield {
    stars: Vec<Star>,
    duration: Option<Duration>,
}

impl Starfield {
    pub fn new(density: usize) -> Self {
        let (width, height) = terminal_size().unwrap_or((80, 24));
        let mut stars = Vec::new();

        for _ in 0..density {
            stars.push(Star::new(width, height));
        }

        Self {
            stars,
            duration: None,
        }
    }

    pub fn with_duration(mut self, duration: Duration) -> Self {
        self.duration = Some(duration);
        self
    }

    pub fn run(&mut self) -> io::Result<()> {
        init_animation_mode()?;

        let start = Instant::now();
        let mut frame = 0u64;
        let frame_time = Duration::from_millis(100); // 10 FPS for gentle twinkling

        loop {
            if let Some(duration) = self.duration
                && start.elapsed() >= duration
            {
                break;
            }

            let frame_start = Instant::now();

            clear_screen()?;

            for star in &mut self.stars {
                star.update(frame);
                star.draw()?;
            }

            flush()?;

            frame += 1;

            let elapsed = frame_start.elapsed();
            if elapsed < frame_time {
                std::thread::sleep(frame_time - elapsed);
            }
        }

        restore_terminal()?;
        Ok(())
    }
}

struct Raindrop {
    x: u16,
    y: u16,
    speed: u8,
    char: char,
    color: Color,
}

impl Raindrop {
    fn new(width: u16) -> Self {
        let chars = ['│', '┃', '║', '|', '¦'];
        let colors = [Color::Cyan, Color::Blue, Color::DarkCyan, Color::White];

        let mut rng = rand::thread_rng();

        Self {
            x: rng.gen_range(0..width),
            y: 0,
            speed: rng.gen_range(1..4),
            char: chars[rng.gen_range(0..chars.len())],
            color: colors[rng.gen_range(0..colors.len())],
        }
    }

    fn update(&mut self, height: u16) {
        self.y += self.speed as u16;
        if self.y >= height {
            self.y = 0;
            let mut rng = rand::thread_rng();
            self.x = rng.gen_range(0..100).min(self.x);
        }
    }

    fn draw(&self) -> io::Result<()> {
        print_at(self.x, self.y, &self.char.to_string(), self.color)?;
        Ok(())
    }
}

pub struct Rain {
    drops: Vec<Raindrop>,
    duration: Duration,
}

impl Rain {
    pub fn new(intensity: usize) -> Self {
        let (width, _) = terminal_size().unwrap_or((80, 24));
        let mut drops = Vec::new();

        for _ in 0..intensity {
            drops.push(Raindrop::new(width));
        }

        Self {
            drops,
            duration: Duration::from_secs(5),
        }
    }

    pub fn with_duration(mut self, duration: Duration) -> Self {
        self.duration = duration;
        self
    }

    pub fn run(&mut self) -> io::Result<()> {
        init_animation_mode()?;

        let (_, height) = terminal_size()?;
        let start = Instant::now();
        let frame_time = Duration::from_millis(50); // 20 FPS for smoother rain

        while start.elapsed() < self.duration {
            let frame_start = Instant::now();

            clear_screen()?;

            for drop in &mut self.drops {
                drop.update(height);
                drop.draw()?;
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
}

pub fn show_starfield(duration: Duration) -> io::Result<()> {
    Starfield::new(100).with_duration(duration).run()
}

pub fn show_rain(duration: Duration) -> io::Result<()> {
    Rain::new(50).with_duration(duration).run()
}
