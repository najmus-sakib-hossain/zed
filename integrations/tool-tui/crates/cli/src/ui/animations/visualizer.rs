//! Real-time Audio Visualizer for Terminal
//!
//! Displays animated waveforms, spectrum analyzers, and VU meters
//! while audio is playing.

use std::io::{self, Write};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crossterm::{
    cursor, execute,
    style::{Color, Print, SetForegroundColor},
    terminal::{self, Clear, ClearType},
};

/// Visualizer styles
#[derive(Debug, Clone, Copy)]
pub enum VisualizerStyle {
    /// Classic waveform bars
    Bars,
    /// Spectrum analyzer
    Spectrum,
    /// VU meter
    VuMeter,
    /// Oscilloscope
    Oscilloscope,
    /// Circular/radial
    Radial,
}

/// Audio visualizer state
pub struct Visualizer {
    style: VisualizerStyle,
    width: u16,
    height: u16,
    samples: Arc<Mutex<Vec<f32>>>,
    running: Arc<Mutex<bool>>,
}

impl Visualizer {
    /// Create a new visualizer
    pub fn new(style: VisualizerStyle) -> io::Result<Self> {
        let (width, height) = terminal::size()?;

        Ok(Self {
            style,
            width,
            height: height.saturating_sub(4), // Leave room for info
            samples: Arc::new(Mutex::new(vec![0.0; width as usize])),
            running: Arc::new(Mutex::new(false)),
        })
    }

    /// Start the visualizer
    pub fn start(&mut self) -> io::Result<()> {
        *self.running.lock().unwrap() = true;

        // Enter alternate screen
        execute!(io::stdout(), terminal::EnterAlternateScreen, cursor::Hide)?;

        Ok(())
    }

    /// Stop the visualizer
    pub fn stop(&mut self) -> io::Result<()> {
        *self.running.lock().unwrap() = false;

        // Restore terminal
        execute!(io::stdout(), terminal::LeaveAlternateScreen, cursor::Show)?;

        Ok(())
    }

    /// Update samples (called from audio thread)
    pub fn update_samples(&self, new_samples: &[f32]) {
        if let Ok(mut samples) = self.samples.lock() {
            samples.clear();
            samples.extend_from_slice(new_samples);
        }
    }

    /// Render a single frame
    pub fn render_frame(&self) -> io::Result<()> {
        if !*self.running.lock().unwrap() {
            return Ok(());
        }

        let samples = self.samples.lock().unwrap().clone();

        execute!(io::stdout(), Clear(ClearType::All), cursor::MoveTo(0, 0))?;

        match self.style {
            VisualizerStyle::Bars => self.render_bars(&samples)?,
            VisualizerStyle::Spectrum => self.render_spectrum(&samples)?,
            VisualizerStyle::VuMeter => self.render_vu_meter(&samples)?,
            VisualizerStyle::Oscilloscope => self.render_oscilloscope(&samples)?,
            VisualizerStyle::Radial => self.render_radial(&samples)?,
        }

        io::stdout().flush()?;

        Ok(())
    }

    /// Render bar-style visualizer
    fn render_bars(&self, samples: &[f32]) -> io::Result<()> {
        let bar_chars = ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];

        // Title
        execute!(
            io::stdout(),
            cursor::MoveTo(0, 0),
            SetForegroundColor(Color::Cyan),
            Print("🎵 Audio Visualizer - Bars Mode")
        )?;

        // Render bars
        for y in 0..self.height {
            execute!(io::stdout(), cursor::MoveTo(0, y + 2))?;

            for (i, &sample) in samples.iter().enumerate() {
                if i >= self.width as usize {
                    break;
                }

                let normalized = (sample.abs() * self.height as f32) as u16;
                let bar_height = self.height.saturating_sub(y);

                if normalized >= bar_height {
                    let intensity = (sample.abs() * 255.0) as u8;
                    let color = if intensity > 200 {
                        Color::Red
                    } else if intensity > 150 {
                        Color::Yellow
                    } else if intensity > 100 {
                        Color::Green
                    } else {
                        Color::Blue
                    };

                    execute!(io::stdout(), SetForegroundColor(color), Print(bar_chars[7]))?;
                } else {
                    execute!(io::stdout(), Print(" "))?;
                }
            }
        }

        // Footer
        execute!(
            io::stdout(),
            cursor::MoveTo(0, self.height + 2),
            SetForegroundColor(Color::DarkGrey),
            Print("Press Ctrl+C to stop")
        )?;

        Ok(())
    }

    /// Render spectrum analyzer
    fn render_spectrum(&self, samples: &[f32]) -> io::Result<()> {
        // Title
        execute!(
            io::stdout(),
            cursor::MoveTo(0, 0),
            SetForegroundColor(Color::Magenta),
            Print("🎵 Audio Visualizer - Spectrum Analyzer")
        )?;

        // Render frequency bands
        let bands = 32;
        let band_width = self.width / bands;

        for band in 0..bands {
            let start_idx = (band as usize * samples.len()) / bands as usize;
            let end_idx = ((band + 1) as usize * samples.len()) / bands as usize;

            let avg = if end_idx > start_idx {
                samples[start_idx..end_idx].iter().map(|s| s.abs()).sum::<f32>()
                    / (end_idx - start_idx) as f32
            } else {
                0.0
            };

            let bar_height = (avg * self.height as f32) as u16;

            for y in 0..self.height {
                let x = band * band_width;
                execute!(io::stdout(), cursor::MoveTo(x, self.height - y + 1))?;

                if y < bar_height {
                    let color = match y * 100 / self.height {
                        0..=33 => Color::Green,
                        34..=66 => Color::Yellow,
                        _ => Color::Red,
                    };

                    execute!(
                        io::stdout(),
                        SetForegroundColor(color),
                        Print("█".repeat(band_width as usize))
                    )?;
                }
            }
        }

        // Footer
        execute!(
            io::stdout(),
            cursor::MoveTo(0, self.height + 2),
            SetForegroundColor(Color::DarkGrey),
            Print("Press Ctrl+C to stop")
        )?;

        Ok(())
    }

    /// Render VU meter
    fn render_vu_meter(&self, samples: &[f32]) -> io::Result<()> {
        // Title
        execute!(
            io::stdout(),
            cursor::MoveTo(0, 0),
            SetForegroundColor(Color::Yellow),
            Print("🎵 Audio Visualizer - VU Meter")
        )?;

        // Calculate peak and RMS
        let peak = samples.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
        let rms = (samples.iter().map(|s| s * s).sum::<f32>() / samples.len() as f32).sqrt();

        let center_y = self.height / 2;

        // Left channel (peak)
        execute!(
            io::stdout(),
            cursor::MoveTo(2, center_y - 2),
            SetForegroundColor(Color::White),
            Print("L PEAK │")
        )?;

        let peak_width = (peak * (self.width - 15) as f32) as u16;
        for x in 0..peak_width {
            let color = match x * 100 / (self.width - 15) {
                0..=70 => Color::Green,
                71..=85 => Color::Yellow,
                _ => Color::Red,
            };
            execute!(
                io::stdout(),
                cursor::MoveTo(11 + x, center_y - 2),
                SetForegroundColor(color),
                Print("█")
            )?;
        }

        // Right channel (RMS)
        execute!(
            io::stdout(),
            cursor::MoveTo(2, center_y + 2),
            SetForegroundColor(Color::White),
            Print("R RMS  │")
        )?;

        let rms_width = (rms * (self.width - 15) as f32) as u16;
        for x in 0..rms_width {
            let color = match x * 100 / (self.width - 15) {
                0..=70 => Color::Green,
                71..=85 => Color::Yellow,
                _ => Color::Red,
            };
            execute!(
                io::stdout(),
                cursor::MoveTo(11 + x, center_y + 2),
                SetForegroundColor(color),
                Print("█")
            )?;
        }

        // Values
        execute!(
            io::stdout(),
            cursor::MoveTo(2, center_y),
            SetForegroundColor(Color::Cyan),
            Print(format!("Peak: {:.2}  RMS: {:.2}", peak, rms))
        )?;

        // Footer
        execute!(
            io::stdout(),
            cursor::MoveTo(0, self.height + 2),
            SetForegroundColor(Color::DarkGrey),
            Print("Press Ctrl+C to stop")
        )?;

        Ok(())
    }

    /// Render oscilloscope
    fn render_oscilloscope(&self, samples: &[f32]) -> io::Result<()> {
        // Title
        execute!(
            io::stdout(),
            cursor::MoveTo(0, 0),
            SetForegroundColor(Color::Green),
            Print("🎵 Audio Visualizer - Oscilloscope")
        )?;

        let center_y = self.height / 2 + 1;

        // Draw center line
        execute!(io::stdout(), cursor::MoveTo(0, center_y), SetForegroundColor(Color::DarkGrey))?;
        for _ in 0..self.width {
            execute!(io::stdout(), Print("─"))?;
        }

        // Draw waveform
        for (i, &sample) in samples.iter().enumerate() {
            if i >= self.width as usize {
                break;
            }

            let y_offset = (sample * (self.height as f32 / 2.0)) as i16;
            let y = (center_y as i16 - y_offset).max(1).min(self.height as i16 + 1) as u16;

            let color = if sample.abs() > 0.8 {
                Color::Red
            } else if sample.abs() > 0.5 {
                Color::Yellow
            } else {
                Color::Green
            };

            execute!(
                io::stdout(),
                cursor::MoveTo(i as u16, y),
                SetForegroundColor(color),
                Print("●")
            )?;
        }

        // Footer
        execute!(
            io::stdout(),
            cursor::MoveTo(0, self.height + 2),
            SetForegroundColor(Color::DarkGrey),
            Print("Press Ctrl+C to stop")
        )?;

        Ok(())
    }

    /// Render radial visualizer
    fn render_radial(&self, samples: &[f32]) -> io::Result<()> {
        // Title
        execute!(
            io::stdout(),
            cursor::MoveTo(0, 0),
            SetForegroundColor(Color::Magenta),
            Print("🎵 Audio Visualizer - Radial")
        )?;

        let center_x = self.width / 2;
        let center_y = self.height / 2 + 1;
        let max_radius = center_y.min(center_x / 2);

        // Draw radial bars
        let num_bars = 24;
        for i in 0..num_bars {
            let angle = (i as f32 / num_bars as f32) * 2.0 * std::f32::consts::PI;
            let sample_idx = (i * samples.len()) / num_bars;
            let amplitude = samples.get(sample_idx).unwrap_or(&0.0).abs();

            let radius = (amplitude * max_radius as f32) as u16;

            for r in 0..radius {
                let x = center_x as i16 + (r as f32 * angle.cos()) as i16;
                let y = center_y as i16 + (r as f32 * angle.sin() * 0.5) as i16;

                if x >= 0 && x < self.width as i16 && y >= 1 && y <= self.height as i16 + 1 {
                    let color = match r * 100 / max_radius {
                        0..=33 => Color::Blue,
                        34..=66 => Color::Cyan,
                        _ => Color::Magenta,
                    };

                    execute!(
                        io::stdout(),
                        cursor::MoveTo(x as u16, y as u16),
                        SetForegroundColor(color),
                        Print("●")
                    )?;
                }
            }
        }

        // Footer
        execute!(
            io::stdout(),
            cursor::MoveTo(0, self.height + 2),
            SetForegroundColor(Color::DarkGrey),
            Print("Press Ctrl+C to stop")
        )?;

        Ok(())
    }
}

/// Simple visualizer that generates fake data for demo
pub fn demo_visualizer(style: VisualizerStyle, duration_secs: u64) -> io::Result<()> {
    let mut viz = Visualizer::new(style)?;
    viz.start()?;

    let start = Instant::now();
    let mut frame = 0;

    while start.elapsed().as_secs() < duration_secs {
        // Generate fake audio samples
        let mut samples = Vec::new();
        for i in 0..viz.width {
            let t = frame as f32 * 0.1 + i as f32 * 0.1;
            let sample = (t.sin() * 0.5 + (t * 2.0).sin() * 0.3 + (t * 3.0).sin() * 0.2) * 0.8;
            samples.push(sample);
        }

        viz.update_samples(&samples);
        viz.render_frame()?;

        std::thread::sleep(Duration::from_millis(33)); // ~30 FPS
        frame += 1;
    }

    viz.stop()?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_visualizer_creation() {
        let viz = Visualizer::new(VisualizerStyle::Bars);
        assert!(viz.is_ok());
    }
}
