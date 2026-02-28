//! Terminal Image Display
//!
//! Display actual images in the terminal using Kitty/iTerm protocols or ASCII fallback.

use std::io;
use std::path::Path;

/// Display an image in the terminal
pub fn show_image(path: &Path) -> io::Result<()> {
    // Try to display using viuer (supports Kitty/iTerm protocols)
    let conf = viuer::Config {
        transparent: true,
        absolute_offset: false,
        width: Some(80),
        height: Some(24),
        ..Default::default()
    };

    match viuer::print_from_file(path, &conf) {
        Ok(_) => Ok(()),
        Err(e) => {
            eprintln!("Failed to display image: {}", e);
            Err(io::Error::other(e.to_string()))
        }
    }
}

/// Convert image to ASCII art and display
pub fn show_image_as_ascii(path: &Path) -> io::Result<()> {
    use std::num::NonZeroU32;

    // Load image first
    let img = image::open(path).map_err(|e| io::Error::other(e.to_string()))?;

    // Use artem to convert image to ASCII
    let config = artem::config::ConfigBuilder::new()
        .target_size(NonZeroU32::new(80).unwrap())
        .build();

    let ascii_art = artem::convert(img, &config);
    println!("{}", ascii_art);
    Ok(())
}

/// Download image from URL and display it
pub fn show_image_from_url(url: &str, ascii: bool) -> io::Result<()> {
    use std::io::Read;

    eprintln!("Downloading image from {}...", url);

    // Download using ureq (no tokio conflicts)
    let response = ureq::get(url).call().map_err(|e| io::Error::other(e.to_string()))?;

    if response.status() != 200 {
        return Err(io::Error::other(format!("Failed to download: HTTP {}", response.status())));
    }

    // Read response body
    let mut bytes = Vec::new();
    response.into_reader().read_to_end(&mut bytes)?;

    // Detect image format from content
    let ext = if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        "jpg"
    } else if bytes.starts_with(&[0x89, 0x50, 0x4E, 0x47]) {
        "png"
    } else if bytes.starts_with(b"GIF") {
        "gif"
    } else if bytes.starts_with(b"RIFF") && bytes.len() > 12 && &bytes[8..12] == b"WEBP" {
        "webp"
    } else {
        "jpg" // default
    };

    // Save to temporary file
    let temp_dir = std::env::temp_dir();
    let temp_path = temp_dir.join(format!("dx_temp_image.{}", ext));

    std::fs::write(&temp_path, &bytes)?;

    eprintln!("Downloaded {} bytes", bytes.len());

    // Display the image
    if ascii {
        show_image_as_ascii(&temp_path)?;
    } else {
        show_image(&temp_path)?;
    }

    // Clean up
    let _ = std::fs::remove_file(temp_path);

    Ok(())
}
