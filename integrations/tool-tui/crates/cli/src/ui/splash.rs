use anyhow::Result;
use crossterm::{
    cursor,
    event::{self, Event, KeyCode},
    execute,
    terminal::{
        Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode,
        enable_raw_mode,
    },
};
use owo_colors::OwoColorize;
use rand::seq::SliceRandom;
use std::{
    io::{self, Write},
    time::{Duration, Instant},
};

const SPLASH_TEXT: &str = "Dx";
const SUBTITLE: &str = "Enhanced Development Experience";
const FONT_CHANGE_INTERVAL: Duration = Duration::from_secs(5);

/// Simple figlet font parser
struct FigletFont {
    height: usize,
    chars: std::collections::HashMap<char, Vec<String>>,
}

impl FigletFont {
    fn parse(data: &str) -> Option<Self> {
        let lines: Vec<&str> = data.lines().collect();
        if lines.is_empty() {
            return None;
        }

        // Parse header: flf2a$ height baseline maxlen oldlayout commentlines
        let header = lines[0];
        let parts: Vec<&str> = header.split_whitespace().collect();
        if parts.len() < 2 {
            return None;
        }

        let height = parts[1].parse::<usize>().ok()?;
        let comment_lines = if parts.len() > 5 {
            parts[5].parse::<usize>().unwrap_or(0)
        } else {
            0
        };

        let mut chars = std::collections::HashMap::new();
        let start_line = 1 + comment_lines;

        // Parse ASCII characters (32-126)
        let mut current_line = start_line;
        for ascii in 32..=126 {
            if current_line + height > lines.len() {
                break;
            }

            let mut char_lines = Vec::new();
            for i in 0..height {
                let line = lines[current_line + i];
                // Remove end markers (@ or @@)
                let cleaned = line.trim_end_matches("@@").trim_end_matches('@').replace('$', "");
                char_lines.push(cleaned);
            }

            chars.insert(ascii as u8 as char, char_lines);
            current_line += height;
        }

        Some(FigletFont { height, chars })
    }

    fn render(&self, text: &str) -> Vec<String> {
        let mut result = vec![String::new(); self.height];

        for ch in text.chars() {
            if let Some(char_lines) = self.chars.get(&ch) {
                for (i, line) in char_lines.iter().enumerate() {
                    if i < result.len() {
                        result[i].push_str(line);
                    }
                }
            } else {
                // Fallback for unknown characters
                for row in result.iter_mut().take(self.height) {
                    row.push(' ');
                }
            }
        }

        result
    }
}

/// Renders an animated splash screen with cycling figlet fonts
pub fn show_splash() -> Result<()> {
    // Get all available fonts
    let fonts = dx_font::figlet::list_fonts()?;

    // Filter to block/bold fonts for better visual impact
    let preferred_fonts = vec![
        "Block",
        "Colossal",
        "Big",
        "Banner",
        "Banner3",
        "Doom",
        "Epic",
        "Graffiti",
        "Isometric1",
        "Isometric2",
        "Larry3d",
        "Ogre",
        "Slant",
        "Standard",
        "Starwars",
        "3d",
        "3d_diagonal",
        "Blocks",
        "Broadway",
        "Chunky",
        "Cyberlarge",
        "Doh",
        "Ghost",
        "Gothic",
        "Graceful",
        "Lean",
        "Mini",
        "Modular",
        "Rounded",
        "Shadow",
        "Small",
        "Speed",
        "Stampatello",
        "Stellar",
        "Thick",
        "Thin",
    ];

    // Prioritize preferred fonts, then shuffle the rest
    let mut priority_fonts = Vec::new();
    let mut other_fonts = Vec::new();

    for font in fonts {
        if preferred_fonts.iter().any(|p| font.contains(p)) {
            priority_fonts.push(font);
        } else {
            other_fonts.push(font);
        }
    }

    let mut rng = rand::thread_rng();
    other_fonts.shuffle(&mut rng);

    priority_fonts.extend(other_fonts);
    let fonts = priority_fonts;

    if fonts.is_empty() {
        eprintln!("No figlet fonts found!");
        return Ok(());
    }

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, cursor::Hide)?;

    let result = run_splash_loop(&fonts);

    disable_raw_mode()?;
    execute!(stdout, LeaveAlternateScreen, cursor::Show)?;

    result
}

fn run_splash_loop(fonts: &[String]) -> Result<()> {
    let mut current_font_index = 0;
    let mut last_change = Instant::now();
    let mut current_rendered: Option<Vec<String>> = None;
    let mut last_displayed_font = String::new();

    loop {
        // Check for key press to exit (non-blocking)
        if event::poll(Duration::from_millis(16))?
            && let Event::Key(key) = event::read()?
            && matches!(key.code, KeyCode::Char('q') | KeyCode::Esc | KeyCode::Enter)
        {
            break;
        }

        // Change font every 5 seconds
        let should_change = last_change.elapsed() >= FONT_CHANGE_INTERVAL;
        if should_change {
            current_font_index = (current_font_index + 1) % fonts.len();
            last_change = Instant::now();
            current_rendered = None; // Force re-render
        }

        // Only render if font changed
        let current_font = &fonts[current_font_index];
        if current_font != &last_displayed_font {
            // Load and render new font
            if current_rendered.is_none() {
                current_rendered = load_and_render_font(current_font);
            }

            // Display (only when changed)
            render_splash(current_font, current_font_index, fonts.len(), &current_rendered)?;

            last_displayed_font = current_font.clone();
        }

        // Sleep to reduce CPU usage
        std::thread::sleep(Duration::from_millis(100));
    }

    Ok(())
}

fn load_and_render_font(font_name: &str) -> Option<Vec<String>> {
    if let Ok(font_data) = dx_font::figlet::read_font(font_name)
        && let Ok(font_str) = String::from_utf8(font_data)
        && let Some(font) = FigletFont::parse(&font_str)
    {
        return Some(font.render(SPLASH_TEXT));
    }
    None
}

fn render_splash(
    font_name: &str,
    index: usize,
    total: usize,
    rendered: &Option<Vec<String>>,
) -> Result<()> {
    let mut stdout = io::stdout();

    // Clear and move to top
    execute!(stdout, Clear(ClearType::All), cursor::MoveTo(0, 0))?;

    println!();
    println!();

    // Display rendered figlet text or fallback
    if let Some(lines) = rendered {
        for line in lines {
            println!("  {}", line.bright_cyan().bold());
        }
    } else {
        // Fallback block text
        display_block_text(SPLASH_TEXT);
    }

    println!();
    println!();
    println!("  {}", SUBTITLE.bright_cyan());
    println!();
    println!();

    // Show current font info
    println!("  {} {}", "Font:".bright_black(), font_name.bright_white().bold());
    println!(
        "  {} {}/{}",
        "Progress:".bright_black(),
        (index + 1).to_string().bright_green(),
        total.to_string().bright_black()
    );
    println!();
    println!(
        "  {} Press {} {} or {} to continue",
        "→".bright_cyan(),
        "Enter".bright_yellow(),
        ",".bright_black(),
        "Esc".bright_yellow()
    );

    stdout.flush()?;
    Ok(())
}

fn display_block_text(text: &str) {
    // Simple block letter rendering as fallback
    let lines = match text {
        "Dx" => vec![
            "  ██████╗ ██╗  ██╗",
            "  ██╔══██╗╚██╗██╔╝",
            "  ██║  ██║ ╚███╔╝ ",
            "  ██║  ██║ ██╔██╗ ",
            "  ██████╔╝██╔╝ ██╗",
            "  ╚═════╝ ╚═╝  ╚═╝",
        ],
        _ => vec![text],
    };

    for line in lines {
        println!("{}", line.bright_cyan().bold());
    }
}

/// Quick splash that shows for a brief moment on startup
pub fn show_quick_splash() {
    println!();
    display_block_text("Dx");
    println!();
    println!("  {}", SUBTITLE.bright_cyan());
    println!();
    std::thread::sleep(Duration::from_millis(800));
}
