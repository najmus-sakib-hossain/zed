//! Confetti Explosions & Fireworks
//!
//! Bursting particles and exploding fireworks for victory celebrations.

use crossterm::style::Color;
use rand::Rng;
use std::io;
use std::time::{Duration, Instant};

use super::{clear_screen, flush, init_animation_mode, print_at, restore_terminal, terminal_size};

const CONFETTI_CHARS: &[char] = &['*', '•', '◆', '◇', '○', '●', '★', '☆', '+', 'x'];

struct Particle {
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
    char: char,
    color: Color,
    life: u8,
}

impl Particle {
    fn new(x: f32, y: f32) -> Self {
        let mut rng = rand::thread_rng();
        let angle = rng.gen_range(0.0..std::f32::consts::TAU);
        let speed = rng.gen_range(2.0..8.0);

        let colors = [
            Color::Red,
            Color::Green,
            Color::Yellow,
            Color::Blue,
            Color::Magenta,
            Color::Cyan,
        ];

        Self {
            x,
            y,
            vx: angle.cos() * speed,
            vy: angle.sin() * speed,
            char: CONFETTI_CHARS[rng.gen_range(0..CONFETTI_CHARS.len())],
            color: colors[rng.gen_range(0..colors.len())],
            life: rng.gen_range(20..40),
        }
    }

    fn update(&mut self) {
        self.x += self.vx;
        self.y += self.vy;
        self.vy += 0.3; // Gravity
        self.vx *= 0.98; // Air resistance
        if self.life > 0 {
            self.life -= 1;
        }
    }

    fn is_alive(&self) -> bool {
        self.life > 0
    }

    fn draw(&self, width: u16, height: u16) -> io::Result<()> {
        if self.x >= 0.0 && self.x < width as f32 && self.y >= 0.0 && self.y < height as f32 {
            print_at(self.x as u16, self.y as u16, &self.char.to_string(), self.color)?;
        }
        Ok(())
    }
}

pub struct ConfettiExplosion {
    particles: Vec<Particle>,
    duration: Duration,
}

impl ConfettiExplosion {
    pub fn new() -> Self {
        Self {
            particles: Vec::new(),
            duration: Duration::from_secs(3),
        }
    }

    pub fn with_duration(mut self, duration: Duration) -> Self {
        self.duration = duration;
        self
    }

    pub fn run(&mut self) -> io::Result<()> {
        init_animation_mode()?;

        let (width, height) = terminal_size()?;
        let center_x = width as f32 / 2.0;
        let center_y = height as f32 / 2.0;

        // Create initial burst
        for _ in 0..80 {
            self.particles.push(Particle::new(center_x, center_y));
        }

        // Play success sound
        super::sounds::play_success();

        let start = Instant::now();
        let frame_time = Duration::from_millis(33); // 30 FPS for smoother performance

        while start.elapsed() < self.duration {
            let frame_start = Instant::now();

            clear_screen()?;

            // Update and draw particles
            self.particles.retain_mut(|p| {
                p.update();
                p.is_alive()
            });

            for particle in &self.particles {
                particle.draw(width, height)?;
            }

            flush()?;

            // Maintain consistent frame rate
            let elapsed = frame_start.elapsed();
            if elapsed < frame_time {
                std::thread::sleep(frame_time - elapsed);
            }
        }

        restore_terminal()?;
        Ok(())
    }
}

impl Default for ConfettiExplosion {
    fn default() -> Self {
        Self::new()
    }
}

pub fn show_confetti() -> io::Result<()> {
    ConfettiExplosion::new().run()
}

pub fn show_confetti_with_message(message: &str) -> io::Result<()> {
    init_animation_mode()?;

    let (width, height) = terminal_size()?;
    let center_x = width as f32 / 2.0;
    let center_y = height as f32 / 2.0;

    let mut particles = Vec::new();
    for _ in 0..80 {
        particles.push(Particle::new(center_x, center_y));
    }

    let start = Instant::now();
    let duration = Duration::from_secs(3);
    let frame_time = Duration::from_millis(33); // 30 FPS

    let msg_x = (width.saturating_sub(message.len() as u16)) / 2;
    let msg_y = height / 2;

    while start.elapsed() < duration {
        let frame_start = Instant::now();

        clear_screen()?;

        particles.retain_mut(|p| {
            p.update();
            p.is_alive()
        });

        for particle in &particles {
            particle.draw(width, height)?;
        }

        print_at(msg_x, msg_y, message, Color::White)?;

        flush()?;

        let elapsed = frame_start.elapsed();
        if elapsed < frame_time {
            std::thread::sleep(frame_time - elapsed);
        }
    }

    restore_terminal()?;
    Ok(())
}
