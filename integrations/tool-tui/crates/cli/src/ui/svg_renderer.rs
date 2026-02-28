use anyhow::{Context, Result};
use image::{DynamicImage, ImageBuffer, Rgba};
use std::path::Path;

/// Render SVG to terminal using viuer
pub fn render_svg_file(path: &Path, width: Option<u32>) -> Result<()> {
    // Read SVG file
    let svg_data = std::fs::read(path)
        .with_context(|| format!("Failed to read SVG file: {}", path.display()))?;

    render_svg_data(&svg_data, width)
}

/// Render SVG data to terminal
pub fn render_svg_data(svg_data: &[u8], width: Option<u32>) -> Result<()> {
    // Parse SVG
    let opt = usvg::Options::default();
    let tree = usvg::Tree::from_data(svg_data, &opt).context("Failed to parse SVG")?;

    // Get SVG dimensions
    let size = tree.size();
    let svg_width = size.width() as u32;
    let svg_height = size.height() as u32;

    // Calculate render size
    let render_width = width.unwrap_or(svg_width.min(80));
    let render_height = (render_width as f32 * svg_height as f32 / svg_width as f32) as u32;

    // Create pixmap for rendering
    let mut pixmap =
        tiny_skia::Pixmap::new(render_width, render_height).context("Failed to create pixmap")?;

    // Render SVG to pixmap
    let transform = tiny_skia::Transform::from_scale(
        render_width as f32 / svg_width as f32,
        render_height as f32 / svg_height as f32,
    );

    resvg::render(&tree, transform, &mut pixmap.as_mut());

    // Convert pixmap to image
    let img_buffer: ImageBuffer<Rgba<u8>, Vec<u8>> =
        ImageBuffer::from_raw(render_width, render_height, pixmap.data().to_vec())
            .context("Failed to create image buffer")?;

    let img = DynamicImage::ImageRgba8(img_buffer);

    // Display in terminal using viuer
    let conf = viuer::Config {
        transparent: true,
        absolute_offset: false,
        width: Some(render_width),
        ..Default::default()
    };

    viuer::print(&img, &conf).context("Failed to display image in terminal")?;

    Ok(())
}

/// Create and render an SVG icon from code
pub fn render_svg_icon(svg_code: &str, width: Option<u32>) -> Result<()> {
    render_svg_data(svg_code.as_bytes(), width)
}

/// Collection of SVG icons as strings
pub struct SvgIcons;

impl SvgIcons {
    pub fn robot() -> &'static str {
        r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100">
            <rect x="20" y="30" width="60" height="50" rx="5" fill="#4A90E2" stroke="#2E5C8A" stroke-width="2"/>
            <circle cx="35" cy="45" r="8" fill="#FFF"/>
            <circle cx="65" cy="45" r="8" fill="#FFF"/>
            <circle cx="35" cy="45" r="4" fill="#000"/>
            <circle cx="65" cy="45" r="4" fill="#000"/>
            <rect x="40" y="60" width="20" height="3" rx="1.5" fill="#2E5C8A"/>
            <rect x="15" y="40" width="10" height="20" rx="3" fill="#4A90E2" stroke="#2E5C8A" stroke-width="2"/>
            <rect x="75" y="40" width="10" height="20" rx="3" fill="#4A90E2" stroke="#2E5C8A" stroke-width="2"/>
            <rect x="45" y="15" width="10" height="15" fill="#4A90E2" stroke="#2E5C8A" stroke-width="2"/>
            <circle cx="50" cy="15" r="5" fill="#FFD700" stroke="#FFA500" stroke-width="2"/>
        </svg>"##
    }

    pub fn user() -> &'static str {
        r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100">
            <circle cx="50" cy="35" r="20" fill="#6C63FF" stroke="#4A47A3" stroke-width="2"/>
            <path d="M 20 85 Q 20 60 50 60 Q 80 60 80 85 Z" fill="#6C63FF" stroke="#4A47A3" stroke-width="2"/>
        </svg>"##
    }

    pub fn lightning() -> &'static str {
        r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100">
            <path d="M 55 10 L 30 50 L 45 50 L 40 90 L 70 45 L 55 45 Z" fill="#FFD700" stroke="#FFA500" stroke-width="2"/>
        </svg>"##
    }

    pub fn chat() -> &'static str {
        r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100">
            <rect x="15" y="25" width="70" height="50" rx="10" fill="#4ECDC4" stroke="#2C9A93" stroke-width="2"/>
            <path d="M 35 75 L 30 85 L 40 75 Z" fill="#4ECDC4" stroke="#2C9A93" stroke-width="2"/>
            <circle cx="35" cy="50" r="4" fill="#FFF"/>
            <circle cx="50" cy="50" r="4" fill="#FFF"/>
            <circle cx="65" cy="50" r="4" fill="#FFF"/>
        </svg>"##
    }

    pub fn clipboard() -> &'static str {
        r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100">
            <rect x="25" y="20" width="50" height="65" rx="5" fill="#95E1D3" stroke="#5FB8A8" stroke-width="2"/>
            <rect x="40" y="15" width="20" height="10" rx="3" fill="#5FB8A8"/>
            <line x1="35" y1="40" x2="65" y2="40" stroke="#5FB8A8" stroke-width="2"/>
            <line x1="35" y1="50" x2="65" y2="50" stroke="#5FB8A8" stroke-width="2"/>
            <line x1="35" y1="60" x2="55" y2="60" stroke="#5FB8A8" stroke-width="2"/>
        </svg>"##
    }

    pub fn rocket() -> &'static str {
        r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100">
            <path d="M 50 10 L 60 40 L 70 70 L 50 65 L 30 70 L 40 40 Z" fill="#FF6B6B" stroke="#C92A2A" stroke-width="2"/>
            <circle cx="50" cy="35" r="6" fill="#FFF" stroke="#C92A2A" stroke-width="1"/>
            <path d="M 30 70 L 25 85 L 30 75 Z" fill="#FFA500" stroke="#FF6B00" stroke-width="1"/>
            <path d="M 70 70 L 75 85 L 70 75 Z" fill="#FFA500" stroke="#FF6B00" stroke-width="1"/>
            <path d="M 50 65 L 48 80 L 52 80 Z" fill="#FFA500" stroke="#FF6B00" stroke-width="1"/>
        </svg>"##
    }

    pub fn check() -> &'static str {
        r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100">
            <circle cx="50" cy="50" r="40" fill="#51CF66" stroke="#2F9E44" stroke-width="3"/>
            <path d="M 30 50 L 45 65 L 70 35" stroke="#FFF" stroke-width="6" fill="none" stroke-linecap="round" stroke-linejoin="round"/>
        </svg>"##
    }

    pub fn error() -> &'static str {
        r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100">
            <circle cx="50" cy="50" r="40" fill="#FF6B6B" stroke="#C92A2A" stroke-width="3"/>
            <line x1="35" y1="35" x2="65" y2="65" stroke="#FFF" stroke-width="6" stroke-linecap="round"/>
            <line x1="65" y1="35" x2="35" y2="65" stroke="#FFF" stroke-width="6" stroke-linecap="round"/>
        </svg>"##
    }

    pub fn warning() -> &'static str {
        r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100">
            <path d="M 50 10 L 90 85 L 10 85 Z" fill="#FFD43B" stroke="#F59F00" stroke-width="3"/>
            <line x1="50" y1="35" x2="50" y2="60" stroke="#000" stroke-width="4" stroke-linecap="round"/>
            <circle cx="50" cy="72" r="4" fill="#000"/>
        </svg>"##
    }

    pub fn gear() -> &'static str {
        r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100">
            <path d="M 50 10 L 55 25 L 70 20 L 75 35 L 90 40 L 85 55 L 90 60 L 75 65 L 70 80 L 55 75 L 50 90 L 45 75 L 30 80 L 25 65 L 10 60 L 15 45 L 10 40 L 25 35 L 30 20 L 45 25 Z" fill="#868E96" stroke="#495057" stroke-width="2"/>
            <circle cx="50" cy="50" r="15" fill="#495057" stroke="#212529" stroke-width="2"/>
        </svg>"##
    }

    pub fn database() -> &'static str {
        r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100">
            <ellipse cx="50" cy="25" rx="35" ry="12" fill="#4DABF7" stroke="#1971C2" stroke-width="2"/>
            <rect x="15" y="25" width="70" height="50" fill="#4DABF7"/>
            <line x1="15" y1="45" x2="85" y2="45" stroke="#1971C2" stroke-width="2"/>
            <line x1="15" y1="60" x2="85" y2="60" stroke="#1971C2" stroke-width="2"/>
            <ellipse cx="50" cy="75" rx="35" ry="12" fill="#4DABF7" stroke="#1971C2" stroke-width="2"/>
            <line x1="15" y1="25" x2="15" y2="75" stroke="#1971C2" stroke-width="2"/>
            <line x1="85" y1="25" x2="85" y2="75" stroke="#1971C2" stroke-width="2"/>
        </svg>"##
    }
}

/// Demo function to show all SVG icons
pub fn show_svg_gallery() -> Result<()> {
    use owo_colors::OwoColorize;

    println!("\n{}", "═".repeat(70).bright_cyan());
    println!("{}", "  DX CLI Real SVG Icon Gallery".bright_white().bold());
    println!("{}", "  Rendered using resvg + viuer".bright_black());
    println!("{}\n", "═".repeat(70).bright_cyan());

    let icons = vec![
        ("Robot/AI", SvgIcons::robot()),
        ("User", SvgIcons::user()),
        ("Lightning", SvgIcons::lightning()),
        ("Chat", SvgIcons::chat()),
        ("Clipboard", SvgIcons::clipboard()),
        ("Rocket", SvgIcons::rocket()),
        ("Check", SvgIcons::check()),
        ("Error", SvgIcons::error()),
        ("Warning", SvgIcons::warning()),
        ("Gear", SvgIcons::gear()),
        ("Database", SvgIcons::database()),
    ];

    for (name, svg) in icons {
        println!("  {}", name.bright_yellow().bold());
        println!("  {}", "─".repeat(40).bright_black());

        if let Err(e) = render_svg_icon(svg, Some(20)) {
            eprintln!("  {} Failed to render: {}", "✗".red(), e);
        }

        println!();
    }

    println!("{}", "═".repeat(70).bright_cyan());
    println!("  {} Real SVG rendering in terminal!", "✓".bright_green());
    println!("{}\n", "═".repeat(70).bright_cyan());

    Ok(())
}
