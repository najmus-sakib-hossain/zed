use figlet_rs::FIGfont;

fn main() {
    println!("Testing FIGlet font loading...\n");

    // Debug path construction
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    println!("CARGO_MANIFEST_DIR: {}", manifest_dir);

    let font_path = std::path::PathBuf::from(manifest_dir)
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("crates/font/figlet")
        .join("Block.dx");

    println!("Font path: {:?}", font_path);
    println!("Font exists: {}", font_path.exists());

    println!("\n---\n");

    // Test loading Block.dx
    if let Some(path_str) = font_path.to_str() {
        match FIGfont::from_file(path_str) {
            Ok(font) => {
                println!("✓ Successfully loaded Block.dx");
                if let Some(fig) = font.convert("TEST") {
                    println!("{}", fig);
                }
            }
            Err(e) => {
                println!("✗ Failed to load Block.dx: {:?}", e);
            }
        }
    }

    println!("\n---\n");

    // Test standard font
    match FIGfont::standard() {
        Ok(font) => {
            println!("✓ Successfully loaded standard font");
            if let Some(fig) = font.convert("TEST") {
                println!("{}", fig);
            }
        }
        Err(e) => {
            println!("✗ Failed to load standard font: {:?}", e);
        }
    }
}
