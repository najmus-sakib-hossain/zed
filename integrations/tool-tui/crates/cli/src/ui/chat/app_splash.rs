//! Splash screen rendering with figlet fonts

use super::theme::ChatTheme;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Paragraph, Widget},
};

pub fn render(area: Rect, buf: &mut Buffer, theme: &ChatTheme, font_index: usize) {
    let all_fonts = get_valid_fonts();
    let current_font = all_fonts[font_index % all_fonts.len()];

    let figlet_lines = if let Ok(font_data) = dx_font::figlet::read_font(current_font) {
        let font_str = String::from_utf8(font_data.clone())
            .unwrap_or_else(|_| String::from_utf8_lossy(&font_data).to_string());
        render_figlet(&font_str, "DX")
    } else {
        vec![]
    };

    let mut splash_lines = vec![Line::from("")];

    if !figlet_lines.is_empty() {
        for line in figlet_lines {
            splash_lines.push(Line::from(Span::styled(
                line,
                Style::default().fg(theme.accent).add_modifier(Modifier::BOLD),
            )));
        }
    } else {
        splash_lines.push(Line::from(vec![
            Span::styled("▸ ", Style::default().fg(theme.accent)),
            Span::styled("DX", Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
        ]));
    }

    splash_lines.push(Line::from(""));
    splash_lines.push(Line::from(Span::styled(
        "Enhanced Development Experience",
        Style::default().fg(theme.border),
    )));
    splash_lines.push(Line::from(""));
    splash_lines.push(Line::from(Span::styled(
        format!("Font: {}", current_font),
        Style::default().fg(theme.border).add_modifier(Modifier::DIM),
    )));

    Paragraph::new(splash_lines)
        .alignment(ratatui::layout::Alignment::Center)
        .block(Block::default())
        .render(area, buf);
}

fn render_figlet(font_data: &str, text: &str) -> Vec<String> {
    let lines: Vec<&str> = font_data.lines().collect();
    if lines.is_empty() {
        return vec![];
    }

    let header_line = lines.iter().find(|line| {
        line.starts_with("flf2") || line.starts_with("tlf2") || line.starts_with("flc2")
    });

    let header = match header_line {
        Some(h) => h,
        None => return vec![],
    };

    let header_idx = lines.iter().position(|line| line == header).unwrap_or(0);

    let hardblank = if header.len() > 5 {
        header.chars().nth(5).unwrap_or('$')
    } else {
        '$'
    };

    let parts: Vec<&str> = header.split_whitespace().collect();
    if parts.len() < 2 {
        return vec![];
    }

    let height = parts[1].parse::<usize>().unwrap_or(0);
    if height == 0 || height > 50 {
        return vec![];
    }

    let comment_lines = if parts.len() > 5 {
        parts[5].parse::<usize>().unwrap_or(0)
    } else {
        0
    };

    let start_line = header_idx + 1 + comment_lines;
    if start_line >= lines.len() {
        return vec![];
    }

    let mut result = vec![String::new(); height];

    for ch in text.chars() {
        let ascii = ch as u32;
        if !(32..=126).contains(&ascii) {
            continue;
        }

        let char_index = (ascii - 32) as usize;
        let char_start = start_line + (char_index * height);

        if char_start + height > lines.len() {
            break;
        }

        // Extract character lines and clean them
        let mut char_lines = Vec::new();
        for i in 0..height {
            if char_start + i >= lines.len() {
                break;
            }

            let mut line = lines[char_start + i].to_string();

            // Remove end markers (@@, @, $$, $, ##, #)
            for marker in &["@@", "@", "$$", "$", "##", "#"] {
                if line.ends_with(marker) {
                    line = line[..line.len() - marker.len()].to_string();
                    break;
                }
            }

            // Replace hardblank with space
            if hardblank != ' ' {
                line = line.replace(hardblank, " ");
            }

            // Remove control characters
            line = line.chars().map(|c| if c.is_control() { ' ' } else { c }).collect();

            char_lines.push(line);
        }

        // Find the actual width of this character (rightmost non-space position)
        let char_width = char_lines.iter().map(|line| line.trim_end().len()).max().unwrap_or(0);

        // Append each line, trimmed to actual character width
        for (i, line) in char_lines.iter().enumerate() {
            if i < result.len() {
                let trimmed = if line.len() > char_width {
                    &line[..char_width]
                } else {
                    line.as_str()
                };
                result[i].push_str(trimmed);
            }
        }
    }

    // Clean up result
    let trimmed: Vec<String> = result
        .into_iter()
        .map(|line| line.trim_end().to_string())
        .filter(|line| !line.is_empty())
        .collect();

    if trimmed.iter().all(|line| line.trim().is_empty()) {
        return vec![];
    }

    // Find minimum left padding and remove it for alignment
    let min_leading_spaces = trimmed
        .iter()
        .filter(|line| !line.trim().is_empty())
        .map(|line| line.chars().take_while(|c| c.is_whitespace()).count())
        .min()
        .unwrap_or(0);

    let aligned: Vec<String> = trimmed
        .into_iter()
        .map(|line| {
            if line.len() > min_leading_spaces {
                line[min_leading_spaces..].to_string()
            } else {
                line
            }
        })
        .collect();

    aligned
}

fn get_valid_fonts() -> Vec<&'static str> {
    vec![
        "1Row",
        "3_d",
        "3d",
        "3d-diagonal",
        "3d_ascii",
        "3d_diagonal",
        "3x5",
        "4Max",
        "5_line_oblique",
        "5lineoblique",
        "Acrobatic",
        "Alligator",
        "Alligator2",
        "Alpha",
        "Alphabet",
        "Arrows",
        "Avatar",
        "Banner",
        "Banner3",
        "Banner4",
        "Barbwire",
        "Basic",
        "Bear",
        "Bell",
        "Benjamin",
        "Big",
        "Bigfig",
        "Binary",
        "Block",
        "Blocks",
        "Bloody",
        "Bolger",
        "Braced",
        "Bright",
        "Broadway",
        "Bubble",
        "Bulbhead",
        "Caligraphy",
        "Caligraphy2",
        "Cards",
        "Catwalk",
        "Chiseled",
        "Chunky",
        "Coinstak",
        "Cola",
        "Colossal",
        "Computer",
        "Contessa",
        "Contrast",
        "Cosmike",
        "Crawford",
        "Crawford2",
        "Crazy",
        "Cricket",
        "Cursive",
        "Cyberlarge",
        "Cybermedium",
        "Cybersmall",
        "Cygnet",
        "DANC4",
        "DWhistled",
        "Decimal",
        "Diamond",
        "Digital",
        "Doh",
        "Doom",
        "Double",
        "Electronic",
        "Elite",
        "Epic",
        "Fender",
        "Filter",
        "Flipped",
        "Fraktur",
        "Fuzzy",
        "Georgi16",
        "Georgia11",
        "Ghost",
        "Ghoulish",
        "Glenyn",
        "Goofy",
        "Gothic",
        "Graceful",
        "Gradient",
        "Graffiti",
        "Greek",
        "Hex",
        "Hieroglyphs",
        "Hollywood",
        "Impossible",
        "Invita",
        "Isometric1",
        "Isometric2",
        "Isometric3",
        "Isometric4",
        "Italic",
        "Ivrit",
        "Jacky",
        "Jazmine",
        "Jerusalem",
        "Katakana",
        "Kban",
        "Keyboard",
        "Knob",
        "Konto",
        "LCD",
        "Lean",
        "Letters",
        "Linux",
        "Lockergnome",
        "Madrid",
        "Marquee",
        "Maxfour",
        "Merlin1",
        "Merlin2",
        "Mike",
        "Mini",
        "Mirror",
        "Mnemonic",
        "Modular",
        "Morse",
        "Morse2",
        "Moscow1",
        "Muzzle",
        "NScript",
        "Nancyj",
        "Nipples",
        "O8",
        "OS2",
        "Octal",
        "Ogre",
        "Pawp",
        "Peaks",
        "Pebbles",
        "Pepper",
        "Poison",
        "Puffy",
        "Puzzle",
        "Pyramid",
        "Rammstein",
        "Rectangles",
        "Relief",
        "Relief2",
        "Reverse",
        "Roman",
        "Rotated",
        "Rounded",
        "Rozzo",
        "Runic",
        "Runyc",
        "Script",
        "Serifcap",
        "Shadow",
        "Shimrod",
        "Short",
        "Slant",
        "Slide",
        "Small",
        "Soft",
        "Speed",
        "Spliff",
        "Stacey",
        "Stampate",
        "Stampatello",
        "Standard@",
        "Stellar",
        "Stforek",
        "Stop",
        "Straight",
        "Swan",
        "Sweet",
        "THIS",
        "Tanja",
        "Tengwar",
        "Term",
        "Test1",
        "Thick",
        "Thin",
        "Thorned",
        "Ticks",
        "Tiles",
        "Tombstone",
        "Train",
        "Trek",
        "Tsalagi",
        "Tubular",
        "Twisted",
        "Univers",
        "Varsity",
        "Wavy",
        "Weird",
        "Whimsy",
        "Wow",
    ]
}
