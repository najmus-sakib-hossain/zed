use std::fs;
use std::path::PathBuf;

fn fonts_dir() -> PathBuf {
    PathBuf::from("crates/font/figlet")
}

fn read_font(name: &str) -> std::io::Result<Vec<u8>> {
    let path = fonts_dir().join(format!("{}.dx", name));
    fs::read(path)
}

fn render_figlet(font_data: &str, text: &str) -> Vec<String> {
    let lines: Vec<&str> = font_data.lines().collect();
    if lines.is_empty() {
        return vec![];
    }

    // Find the header line (may not be first line due to PHP headers, comments, etc.)
    let header_line = lines.iter().find(|line| {
        line.starts_with("flf2") || line.starts_with("tlf2") || line.starts_with("flc2")
    });

    let header = match header_line {
        Some(h) => h,
        None => return vec![], // Not a valid font file
    };

    // Find the index of the header line
    let header_idx = lines.iter().position(|line| line == header).unwrap_or(0);

    // Extract hardblank character (5th character in header, e.g., "flf2a$" -> '$')
    let hardblank = if header.len() > 5 {
        header.chars().nth(5).unwrap_or('$')
    } else {
        '$'
    };

    let parts: Vec<&str> = header.split_whitespace().collect();
    if parts.len() < 2 {
        return vec![];
    }

    let height_str = parts[1];
    let height = height_str.parse::<usize>().unwrap_or(0);
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

        for i in 0..height {
            if char_start + i >= lines.len() {
                break;
            }

            let mut line = lines[char_start + i].to_string();

            // Remove end markers - try multiple common markers
            let end_markers = vec!["@@", "@", "$$", "$", "##", "#"];
            for marker in end_markers {
                if line.ends_with(marker) {
                    line = line[..line.len() - marker.len()].to_string();
                    break;
                }
            }

            // Replace hardblank with space
            if hardblank != ' ' {
                line = line.replace(hardblank, " ");
            }

            // Clean up control characters and non-printable chars
            line = line.chars().map(|c| if c.is_control() { ' ' } else { c }).collect();

            result[i].push_str(&line);
        }
    }

    // Trim trailing spaces from each line and filter out completely empty lines
    let trimmed: Vec<String> = result
        .into_iter()
        .map(|line| line.trim_end().to_string())
        .filter(|line| !line.is_empty())
        .collect();

    // If all lines are empty or whitespace-only, return empty vec
    if trimmed.iter().all(|line| line.trim().is_empty()) {
        return vec![];
    }

    trimmed
}

fn main() {
    let fonts_path = fonts_dir();
    let entries = fs::read_dir(&fonts_path).expect("Failed to read fonts directory");

    let mut font_names: Vec<String> = entries
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let path = entry.path();
            if path.extension()? == "dx" {
                path.file_stem()?.to_str().map(|s| s.to_string())
            } else {
                None
            }
        })
        .collect();

    font_names.sort();

    let mut working = 0;
    let mut fallback = 0;
    let mut failed_fonts = Vec::new();

    println!("Testing {} fonts...\n", font_names.len());

    for font_name in &font_names {
        match read_font(font_name) {
            Ok(font_data) => {
                // Try UTF-8 first, then lossy conversion for binary fonts
                let font_str = String::from_utf8(font_data.clone())
                    .unwrap_or_else(|_| String::from_utf8_lossy(&font_data).to_string());

                let result = render_figlet(&font_str, "DX");
                if result.is_empty() {
                    fallback += 1;
                    failed_fonts.push(font_name.clone());
                    println!("❌ FALLBACK: {}", font_name);
                } else {
                    working += 1;
                    println!("✓ OK: {}", font_name);
                }
            }
            Err(e) => {
                fallback += 1;
                failed_fonts.push(font_name.clone());
                println!("❌ READ ERROR: {} - {}", font_name, e);
            }
        }
    }

    println!("\n=== SUMMARY ===");
    println!("Total fonts: {}", font_names.len());
    println!(
        "Working: {} ({:.1}%)",
        working,
        (working as f64 / font_names.len() as f64) * 100.0
    );
    println!(
        "Fallback: {} ({:.1}%)",
        fallback,
        (fallback as f64 / font_names.len() as f64) * 100.0
    );

    if !failed_fonts.is_empty() {
        println!("\n=== FAILED FONTS ===");
        for font in &failed_fonts {
            println!("  - {}", font);
        }
    }
}
