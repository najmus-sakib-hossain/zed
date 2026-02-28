//! Conway's Game of Life Animation
//!
//! Classic cellular automaton - mesmerizing patterns evolving in real-time.

use crossterm::style::Color;
use rand::Rng;
use std::io;
use std::time::{Duration, Instant};

use super::{clear_screen, flush, init_animation_mode, print_at, restore_terminal, terminal_size};

struct GameOfLife {
    grid: Vec<Vec<bool>>,
    width: usize,
    height: usize,
}

impl GameOfLife {
    fn new(width: usize, height: usize, density: f32) -> Self {
        let mut rng = rand::thread_rng();
        let grid = (0..height)
            .map(|_| (0..width).map(|_| rng.gen_bool(density as f64)).collect())
            .collect();

        Self {
            grid,
            width,
            height,
        }
    }

    fn count_neighbors(&self, x: usize, y: usize) -> usize {
        let mut count = 0;
        for dy in -1..=1 {
            for dx in -1..=1 {
                if dx == 0 && dy == 0 {
                    continue;
                }
                let nx = (x as i32 + dx + self.width as i32) % self.width as i32;
                let ny = (y as i32 + dy + self.height as i32) % self.height as i32;
                if self.grid[ny as usize][nx as usize] {
                    count += 1;
                }
            }
        }
        count
    }

    fn step(&mut self) {
        let mut new_grid = self.grid.clone();

        #[allow(clippy::needless_range_loop)]
        for y in 0..self.height {
            for x in 0..self.width {
                let neighbors = self.count_neighbors(x, y);
                let alive = self.grid[y][x];

                new_grid[y][x] = matches!((alive, neighbors), (true, 2) | (true, 3) | (false, 3));
            }
        }

        self.grid = new_grid;
    }

    fn draw(&self) -> io::Result<()> {
        for (y, row) in self.grid.iter().enumerate() {
            for (x, &cell) in row.iter().enumerate() {
                if cell {
                    // Use simple dots for cleaner look
                    let color = Color::Green;
                    print_at(x as u16, y as u16, "●", color)?;
                }
            }
        }
        Ok(())
    }
}

pub fn show_game_of_life(duration: Duration) -> io::Result<()> {
    init_animation_mode()?;

    let (width, height) = terminal_size()?;
    // Lower density for cleaner look
    let mut game = GameOfLife::new(width as usize, height as usize, 0.15);

    let start = Instant::now();
    let frame_time = Duration::from_millis(150); // Slower for better visibility

    while start.elapsed() < duration {
        let frame_start = Instant::now();

        clear_screen()?;
        game.draw()?;
        game.step();
        flush()?;

        let elapsed = frame_start.elapsed();
        if elapsed < frame_time {
            std::thread::sleep(frame_time - elapsed);
        }
    }

    restore_terminal()?;
    Ok(())
}
