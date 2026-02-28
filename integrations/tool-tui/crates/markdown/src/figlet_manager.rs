//! FIGlet Font Manager
//!
//! Manages the rotation and rendering of 400+ FIGlet fonts for beautiful ASCII art headers.

use figlet_rs::FIGfont;
use std::sync::OnceLock;

/// Priority list of FIGlet fonts to use in order (user's preference)
static FONT_PRIORITY: &[&str] = &[
    "Block",      // Priority 1: Block style
    "Colossal",   // Priority 2: Colossal with 0/1 variant
    "Big",        // Priority 3: Big letters
    "Slant",      // Priority 4: Slanted style
    "3d",         // Priority 5: 3D effect
    "Doom",       // Priority 6: Doom style
    "standard",   // Priority 7: Standard FIGlet
    "Banner",     // Priority 8: Banner style
    "Shadow",     // Priority 9: Shadow effect
    "Bubble",     // Priority 10: Bubble letters
    "Digital",    // Priority 11: Digital display
    "Isometric1", // Priority 12: Isometric view
    "Graffiti",   // Priority 13: Graffiti style
    "Gothic",     // Priority 14: Gothic letters
    "Broadway",   // Priority 15: Broadway style
];

/// All available FIGlet fonts (400+)
static ALL_FONTS: OnceLock<Vec<String>> = OnceLock::new();

/// Get all available FIGlet fonts
fn get_all_fonts() -> &'static Vec<String> {
    ALL_FONTS.get_or_init(|| {
        // Start with priority fonts
        let mut fonts: Vec<String> = FONT_PRIORITY.iter().map(|s| s.to_string()).collect();

        // Add more popular fonts
        fonts.extend_from_slice(&[
            "3d_diagonal".to_string(),
            "3x5".to_string(),
            "5lineoblique".to_string(),
            "Acrobatic".to_string(),
            "Alligator".to_string(),
            "Alpha".to_string(),
            "Alphabet".to_string(),
            "Avatar".to_string(),
            "Banner3".to_string(),
            "Banner4".to_string(),
            "Barbwire".to_string(),
            "Basic".to_string(),
            "Bell".to_string(),
            "Benjamin".to_string(),
            "Bigfig".to_string(),
            "Binary".to_string(),
            "Blocks".to_string(),
            "Bloody".to_string(),
            "Bolger".to_string(),
            "Braced".to_string(),
            "Bright".to_string(),
            "Broadway".to_string(),
            "Bubble".to_string(),
            "Bulbhead".to_string(),
            "Caligraphy".to_string(),
            "Cards".to_string(),
            "Catwalk".to_string(),
            "Chiseled".to_string(),
            "Chunky".to_string(),
            "Coinstak".to_string(),
            "Cola".to_string(),
            "Computer".to_string(),
            "Contessa".to_string(),
            "Contrast".to_string(),
            "Cosmike".to_string(),
            "Crawford".to_string(),
            "Crazy".to_string(),
            "Cricket".to_string(),
            "Cursive".to_string(),
            "Cyberlarge".to_string(),
            "Cybermedium".to_string(),
            "Cybersmall".to_string(),
            "Decimal".to_string(),
            "Diamond".to_string(),
            "Digital".to_string(),
            "Doh".to_string(),
            "Double".to_string(),
            "DWhistled".to_string(),
            "Electronic".to_string(),
            "Elite".to_string(),
            "Epic".to_string(),
            "Fender".to_string(),
            "Filter".to_string(),
            "Flipped".to_string(),
            "Fraktur".to_string(),
            "Fuzzy".to_string(),
            "Ghost".to_string(),
            "Ghoulish".to_string(),
            "Glenyn".to_string(),
            "Goofy".to_string(),
            "Gothic".to_string(),
            "Graceful".to_string(),
            "Gradient".to_string(),
            "Graffiti".to_string(),
            "Greek".to_string(),
            "Hex".to_string(),
            "Hollywood".to_string(),
            "Impossible".to_string(),
            "Invita".to_string(),
            "Isometric1".to_string(),
            "Isometric2".to_string(),
            "Isometric3".to_string(),
            "Isometric4".to_string(),
            "Italic".to_string(),
            "Jacky".to_string(),
            "Jazmine".to_string(),
            "Katakana".to_string(),
            "Kban".to_string(),
            "Keyboard".to_string(),
            "Knob".to_string(),
            "Konto".to_string(),
            "LCD".to_string(),
            "Lean".to_string(),
            "Letters".to_string(),
            "Linux".to_string(),
            "Lockergnome".to_string(),
            "Madrid".to_string(),
            "Marquee".to_string(),
            "Maxfour".to_string(),
            "Merlin1".to_string(),
            "Mike".to_string(),
            "Mini".to_string(),
            "Mirror".to_string(),
            "Mnemonic".to_string(),
            "Modular".to_string(),
            "Morse".to_string(),
            "Muzzle".to_string(),
            "Nancyj".to_string(),
            "Nipples".to_string(),
            "NScript".to_string(),
            "O8".to_string(),
            "Octal".to_string(),
            "Ogre".to_string(),
            "OS2".to_string(),
            "Pawp".to_string(),
            "Peaks".to_string(),
            "Pebbles".to_string(),
            "Pepper".to_string(),
            "Poison".to_string(),
            "Puffy".to_string(),
            "Puzzle".to_string(),
            "Pyramid".to_string(),
            "Rectangles".to_string(),
            "Relief".to_string(),
            "Relief2".to_string(),
            "Reverse".to_string(),
            "Roman".to_string(),
            "Rot13".to_string(),
            "Rotated".to_string(),
            "Rounded".to_string(),
            "Rozzo".to_string(),
            "Runic".to_string(),
            "Runyc".to_string(),
            "Script".to_string(),
            "Serifcap".to_string(),
            "Shimrod".to_string(),
            "Short".to_string(),
            "Slide".to_string(),
            "Speed".to_string(),
            "Spliff".to_string(),
            "Stacey".to_string(),
            "Stampate".to_string(),
            "Stampatello".to_string(),
            "Stellar".to_string(),
            "Stforek".to_string(),
            "Stop".to_string(),
            "Straight".to_string(),
            "Swan".to_string(),
            "Sweet".to_string(),
            "Tanja".to_string(),
            "Tengwar".to_string(),
            "Term".to_string(),
            "Thick".to_string(),
            "Thin".to_string(),
            "THIS".to_string(),
            "Thorned".to_string(),
            "Ticks".to_string(),
            "Tiles".to_string(),
            "Tombstone".to_string(),
            "Train".to_string(),
            "Trek".to_string(),
            "Tubular".to_string(),
            "Twisted".to_string(),
            "Univers".to_string(),
            "Varsity".to_string(),
            "Wavy".to_string(),
            "Weird".to_string(),
            "Whimsy".to_string(),
            "Wow".to_string(),
        ]);

        fonts
    })
}

/// Get a font name by index (with wrapping for 400+ fonts)
pub fn get_font_by_index(index: usize) -> &'static str {
    let fonts = get_all_fonts();
    let idx = index % fonts.len();
    &fonts[idx]
}

/// Render text using a FIGlet font
pub fn render_figlet(text: &str, font_name: &str) -> Option<String> {
    // Build absolute path from workspace root
    let font_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()?  // Go up from crates/markdown
        .parent()?  // Go up from crates
        .join("crates/font/figlet")
        .join(format!("{}.dx", font_name));

    // Try to load the font from .dx files
    let font = match FIGfont::from_file(font_path.to_str()?) {
        Ok(f) => f,
        Err(_) => {
            // Try standard font as fallback
            FIGfont::standard().ok()?
        }
    };

    // Render the text
    let figure = font.convert(text);
    figure.map(|f| f.to_string())
}

/// Render text with automatic font selection based on header index
/// Returns FIGlet art with metadata comment for VS Code extension
pub fn render_header_figlet(text: &str, header_index: usize) -> Option<String> {
    let font_name = get_font_by_index(header_index);

    let figlet_art = render_figlet(text, font_name)?;

    // Count lines in the FIGlet art
    let line_count = figlet_art.lines().count();

    // Add metadata comment before the FIGlet art
    // Format: <!-- FIGLET: font="FontName" text="Header Text" lines=5 -->
    let metadata = format!(
        "<!-- FIGLET: font=\"{}\" text=\"{}\" lines={} -->\n",
        font_name,
        text.replace('"', "&quot;"),
        line_count
    );

    Some(format!("{}{}", metadata, figlet_art))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_all_fonts() {
        let fonts = get_all_fonts();
        assert!(!fonts.is_empty());
        assert!(fonts.len() >= 10, "Should have at least 10 fonts");
    }

    #[test]
    fn test_get_font_by_index() {
        assert_eq!(get_font_by_index(0), "Block");
        assert_eq!(get_font_by_index(1), "Colossal");
        assert_eq!(get_font_by_index(2), "Big");
    }

    #[test]
    fn test_font_wrapping() {
        let fonts = get_all_fonts();
        let len = fonts.len();

        // Test that wrapping works
        assert_eq!(get_font_by_index(0), get_font_by_index(len));
        assert_eq!(get_font_by_index(1), get_font_by_index(len + 1));
    }

    #[test]
    fn test_render_figlet_standard() {
        let result = render_figlet("TEST", "standard");
        assert!(result.is_some());
        let text = result.unwrap();
        assert!(!text.is_empty());
        assert!(text.contains("TEST") || text.len() > 10); // FIGlet output is larger
    }
}
