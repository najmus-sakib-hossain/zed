use anyhow::Result;
use colorgrad::Gradient;
use crossterm::{
    cursor,
    event::{self, Event, KeyCode},
    execute,
    style::{self, Stylize},
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use owo_colors::OwoColorize;
use std::{
    fmt::Write as FmtWrite,
    io::{self, BufWriter, Write},
    time::{Duration, Instant},
};

/// Rainbow animation showcase
pub fn show_rainbow_showcase() -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, cursor::Hide)?;

    let result = run_rainbow_loop();

    disable_raw_mode()?;
    execute!(stdout, LeaveAlternateScreen, cursor::Show)?;

    result
}

fn run_rainbow_loop() -> Result<()> {
    let start_time = Instant::now();
    let frame_duration = Duration::from_millis(16); // 60 FPS

    // Pre-create gradients once (expensive operation)
    let rainbow = colorgrad::preset::rainbow();
    let turbo = colorgrad::preset::turbo();
    let viridis = colorgrad::preset::viridis();

    loop {
        let frame_start = Instant::now();

        // Check for key press to exit (non-blocking)
        if event::poll(Duration::from_millis(0))?
            && let Event::Key(key) = event::read()?
            && matches!(key.code, KeyCode::Char('q') | KeyCode::Esc | KeyCode::Enter)
        {
            break;
        }

        let elapsed = start_time.elapsed().as_secs_f64();
        render_rainbow_frame(elapsed, &rainbow, &turbo, &viridis)?;

        // Maintain consistent frame rate
        let frame_time = frame_start.elapsed();
        if frame_time < frame_duration {
            std::thread::sleep(frame_duration - frame_time);
        }
    }

    Ok(())
}

fn render_rainbow_frame(
    time: f64,
    rainbow: &impl Gradient,
    turbo: &impl Gradient,
    viridis: &impl Gradient,
) -> Result<()> {
    // Pre-allocate buffer
    let mut buffer = String::with_capacity(4096);

    // Move cursor to home position WITHOUT clearing (prevents flicker)
    buffer.push_str("\x1b[H");
    buffer.push_str("\n\n");

    // Rainbow text
    render_rainbow_text_to_buffer(&mut buffer, "DX RAINBOW SHOWCASE", time, 0.0, rainbow);
    buffer.push_str("\n\n");

    // Animated rainbow boxes
    render_rainbow_boxes_to_buffer(&mut buffer, time, rainbow, turbo, viridis);
    buffer.push('\n');

    // Gradient bars
    render_gradient_bars_to_buffer(&mut buffer, time, rainbow);
    buffer.push('\n');

    // Rainbow wave
    render_rainbow_wave_to_buffer(&mut buffer, time, rainbow);
    buffer.push('\n');

    // Instructions
    buffer.push_str("  Press \x1b[36mEnter\x1b[0m \x1b[90m,\x1b[0m or \x1b[36mEsc\x1b[0m to exit");

    // Clear to end of screen to remove any leftover content
    buffer.push_str("\x1b[J");

    // Single write syscall
    let stdout = io::stdout();
    let mut writer = BufWriter::with_capacity(8192, stdout.lock());

    writer.write_all(buffer.as_bytes())?;
    writer.flush()?;

    Ok(())
}

fn render_rainbow_text_to_buffer(
    buffer: &mut String,
    text: &str,
    time: f64,
    offset: f64,
    gradient: &impl Gradient,
) {
    let chars: Vec<char> = text.chars().collect();
    let len = chars.len();

    buffer.push_str("  ");
    for (i, ch) in chars.iter().enumerate() {
        let t = ((i as f64 / len as f64) + time * 0.2 + offset) % 1.0;
        let color = gradient.at(t as f32);
        let rgb = color.to_rgba8();

        let _ = write!(buffer, "\x1b[1;38;2;{};{};{}m{}\x1b[0m", rgb[0], rgb[1], rgb[2], ch);
    }
    buffer.push('\n');
}

fn render_rainbow_boxes_to_buffer(
    buffer: &mut String,
    time: f64,
    rainbow: &impl Gradient,
    turbo: &impl Gradient,
    viridis: &impl Gradient,
) {
    // Only 3 gradients
    let gradients: Vec<(&str, &dyn Gradient)> =
        vec![("Rainbow", rainbow), ("Turbo", turbo), ("Viridis", viridis)];

    for (name, gradient) in gradients {
        let _ = write!(buffer, "  \x1b[1;37m{}\x1b[0m ", name);

        let width = 35;
        for i in 0..width {
            let t = ((i as f64 / width as f64) + time * 0.1) % 1.0;
            let color = gradient.at(t as f32);
            let rgb = color.to_rgba8();

            let _ = write!(buffer, "\x1b[38;2;{};{};{}m█\x1b[0m", rgb[0], rgb[1], rgb[2]);
        }
        buffer.push('\n');
    }
}

fn render_gradient_bars_to_buffer(buffer: &mut String, time: f64, gradient: &impl Gradient) {
    buffer.push_str("  \x1b[1;36m▶\x1b[0m Animated Gradient Bars\n\n");

    let height = 3;
    let width = 40;

    for row in 0..height {
        buffer.push_str("  ");
        for col in 0..width {
            let x = col as f64 / width as f64;
            let y = row as f64 / height as f64;

            let t = ((x + y * 0.5 + time * 0.15) % 1.0).abs();
            let color = gradient.at(t as f32);
            let rgb = color.to_rgba8();

            let _ = write!(buffer, "\x1b[38;2;{};{};{}m▓\x1b[0m", rgb[0], rgb[1], rgb[2]);
        }
        buffer.push('\n');
    }
}

fn render_rainbow_wave_to_buffer(buffer: &mut String, time: f64, gradient: &impl Gradient) {
    buffer.push_str("\n  \x1b[1;36m▶\x1b[0m Rainbow Wave\n\n");

    let width = 40;
    let height = 5;

    for row in 0..height {
        buffer.push_str("  ");
        for col in 0..width {
            let x = col as f64 / width as f64;
            let wave = ((x * 6.0 + time * 2.0).sin() * 0.5 + 0.5) * height as f64;

            if (wave as usize) == row {
                let t = (x + time * 0.2) % 1.0;
                let color = gradient.at(t as f32);
                let rgb = color.to_rgba8();

                let _ = write!(buffer, "\x1b[1;38;2;{};{};{}m●\x1b[0m", rgb[0], rgb[1], rgb[2]);
            } else {
                buffer.push(' ');
            }
        }
        buffer.push('\n');
    }
}

/// Quick rainbow text demo
pub fn rainbow_text(text: &str) {
    let gradient = colorgrad::preset::rainbow();
    let chars: Vec<char> = text.chars().collect();
    let len = chars.len();

    for (i, ch) in chars.iter().enumerate() {
        let t = i as f64 / len as f64;
        let color = gradient.at(t as f32);
        let rgb = color.to_rgba8();

        print!(
            "{}",
            ch.to_string()
                .with(style::Color::Rgb {
                    r: rgb[0],
                    g: rgb[1],
                    b: rgb[2],
                })
                .bold()
        );
    }
    println!();
}

/// Animated rainbow box
pub fn rainbow_box(title: &str, content: &[&str], time: f64) -> Result<()> {
    let width = content.iter().map(|s| s.len()).max().unwrap_or(40).max(title.len()) + 4;
    let gradient = colorgrad::preset::rainbow();

    // Top border
    print!("  ");
    for i in 0..width {
        let t = ((i as f64 / width as f64) + time * 0.2) % 1.0;
        let color = gradient.at(t as f32);
        let rgb = color.to_rgba8();
        print!(
            "{}",
            "─".with(style::Color::Rgb {
                r: rgb[0],
                g: rgb[1],
                b: rgb[2],
            })
        );
    }
    println!();

    // Title
    println!("  │ {} │", title.bright_white().bold());

    // Content
    for line in content {
        println!("  │ {:<width$} │", line, width = width - 4);
    }

    // Bottom border
    print!("  ");
    for i in 0..width {
        let t = ((i as f64 / width as f64) + time * 0.2 + 0.5) % 1.0;
        let color = gradient.at(t as f32);
        let rgb = color.to_rgba8();
        print!(
            "{}",
            "─".with(style::Color::Rgb {
                r: rgb[0],
                g: rgb[1],
                b: rgb[2],
            })
        );
    }
    println!();

    Ok(())
}
