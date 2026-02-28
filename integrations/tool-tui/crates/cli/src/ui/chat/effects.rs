use ratatui::style::Color;
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub struct ShimmerEffect {
    colors: Vec<Color>,
    start_time: Instant,
    duration: Duration,
}

impl ShimmerEffect {
    pub fn new(colors: Vec<Color>) -> Self {
        Self {
            colors,
            start_time: Instant::now(),
            duration: Duration::from_millis(1500),
        }
    }

    pub fn current_color(&self) -> Color {
        let elapsed = self.start_time.elapsed().as_millis() as f32;
        let cycle = (elapsed % self.duration.as_millis() as f32) / self.duration.as_millis() as f32;

        let index = (cycle * (self.colors.len() - 1) as f32) as usize;
        let next_index = (index + 1).min(self.colors.len() - 1);

        let t = (cycle * (self.colors.len() - 1) as f32) - index as f32;

        self.interpolate_color(self.colors[index], self.colors[next_index], t)
    }

    fn interpolate_color(&self, c1: Color, c2: Color, t: f32) -> Color {
        match (c1, c2) {
            (Color::Rgb(r1, g1, b1), Color::Rgb(r2, g2, b2)) => {
                let r = (r1 as f32 + (r2 as f32 - r1 as f32) * t) as u8;
                let g = (g1 as f32 + (g2 as f32 - g1 as f32) * t) as u8;
                let b = (b1 as f32 + (b2 as f32 - b1 as f32) * t) as u8;
                Color::Rgb(r, g, b)
            }
            _ => c1,
        }
    }

    pub fn reset(&mut self) {
        self.start_time = Instant::now();
    }
}

#[derive(Debug, Clone)]
pub struct TypingIndicator {
    dots: usize,
    last_update: Instant,
    interval: Duration,
}

impl TypingIndicator {
    pub fn new() -> Self {
        Self {
            dots: 0,
            last_update: Instant::now(),
            interval: Duration::from_millis(500),
        }
    }

    pub fn update(&mut self) {
        if self.last_update.elapsed() >= self.interval {
            self.dots = (self.dots + 1) % 4;
            self.last_update = Instant::now();
        }
    }

    pub fn text(&self, is_visible: bool) -> String {
        if is_visible {
            match self.dots {
                0 => "".to_string(),
                1 => ".".to_string(),
                2 => "..".to_string(),
                _ => "...".to_string(),
            }
        } else {
            String::new()
        }
    }

    pub fn is_visible(&self) -> bool {
        // Blink every 500ms
        (self.last_update.elapsed().as_millis() / 500) % 2 == 0
    }
}

impl Default for TypingIndicator {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct PulseEffect {
    start_time: Instant,
    duration: Duration,
}

impl PulseEffect {
    pub fn new() -> Self {
        Self {
            start_time: Instant::now(),
            duration: Duration::from_millis(1000),
        }
    }

    pub fn opacity(&self) -> f32 {
        let elapsed = self.start_time.elapsed().as_millis() as f32;
        let cycle = (elapsed % self.duration.as_millis() as f32) / self.duration.as_millis() as f32;

        // Sine wave for smooth pulsing
        0.5 + 0.5 * (cycle * std::f32::consts::PI * 2.0).sin()
    }

    pub fn reset(&mut self) {
        self.start_time = Instant::now();
    }
}

impl Default for PulseEffect {
    fn default() -> Self {
        Self::new()
    }
}
