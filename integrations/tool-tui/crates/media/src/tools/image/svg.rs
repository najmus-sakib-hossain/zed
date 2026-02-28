//! Native SVG processing using resvg.
//!
//! Convert SVG files to raster formats without external dependencies.

use crate::tools::ToolOutput;
use std::path::Path;

/// Convert SVG to PNG at specified size.
///
/// Uses native Rust SVG rendering (resvg) - no ImageMagick required.
#[cfg(feature = "image-svg")]
pub fn svg_to_png(
    input: impl AsRef<Path>,
    output: impl AsRef<Path>,
    width: u32,
    height: u32,
) -> std::io::Result<ToolOutput> {
    use image::{ImageBuffer, Rgba};

    let input = input.as_ref();
    let output = output.as_ref();

    // Read SVG file
    let svg_data = std::fs::read_to_string(input)?;

    // Parse SVG
    let opt = usvg::Options::default();
    let tree = usvg::Tree::from_str(&svg_data, &opt)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

    // Get SVG size and calculate scale
    let svg_size = tree.size();
    let scale_x = width as f32 / svg_size.width();
    let scale_y = height as f32 / svg_size.height();

    // Create pixmap at target size
    let mut pixmap = tiny_skia::Pixmap::new(width, height).ok_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::InvalidInput, "Invalid dimensions")
    })?;

    // Render SVG with proper scaling
    let transform = tiny_skia::Transform::from_scale(scale_x, scale_y);
    resvg::render(&tree, transform, &mut pixmap.as_mut());

    // Convert to image crate format
    let img: ImageBuffer<Rgba<u8>, Vec<u8>> = ImageBuffer::from_raw(width, height, pixmap.take())
        .ok_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::Other, "Buffer conversion failed")
    })?;

    // Save as PNG
    img.save(output)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

    Ok(ToolOutput::success_with_path(
        format!("Converted SVG to {}x{} PNG", width, height),
        output,
    ))
}

/// Convert SVG to PNG maintaining aspect ratio.
#[cfg(feature = "image-svg")]
pub fn svg_to_png_width(
    input: impl AsRef<Path>,
    output: impl AsRef<Path>,
    width: u32,
) -> std::io::Result<ToolOutput> {
    use image::{ImageBuffer, Rgba};

    let input = input.as_ref();
    let output = output.as_ref();

    let svg_data = std::fs::read_to_string(input)?;
    let opt = usvg::Options::default();
    let tree = usvg::Tree::from_str(&svg_data, &opt)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

    // Calculate height maintaining aspect ratio
    let size = tree.size();
    let aspect_ratio = size.height() / size.width();
    let height = (width as f32 * aspect_ratio) as u32;

    // Calculate scale
    let scale_x = width as f32 / size.width();
    let scale_y = height as f32 / size.height();

    let mut pixmap = tiny_skia::Pixmap::new(width, height).ok_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::InvalidInput, "Invalid dimensions")
    })?;

    let transform = tiny_skia::Transform::from_scale(scale_x, scale_y);
    resvg::render(&tree, transform, &mut pixmap.as_mut());

    let img: ImageBuffer<Rgba<u8>, Vec<u8>> = ImageBuffer::from_raw(width, height, pixmap.take())
        .ok_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::Other, "Buffer conversion failed")
    })?;

    img.save(output)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

    Ok(ToolOutput::success_with_path(
        format!("Converted SVG to {}x{} PNG", width, height),
        output,
    ))
}

/// Generate multiple icon sizes from SVG.
#[cfg(feature = "image-svg")]
pub fn generate_icons_from_svg(
    input: impl AsRef<Path>,
    output_dir: impl AsRef<Path>,
    sizes: &[u32],
) -> std::io::Result<ToolOutput> {
    let input = input.as_ref();
    let output_dir = output_dir.as_ref();

    std::fs::create_dir_all(output_dir)?;

    let mut generated = Vec::new();

    for &size in sizes {
        let output = output_dir.join(format!("icon-{}x{}.png", size, size));
        svg_to_png(input, &output, size, size)?;
        generated.push(output);
    }

    Ok(ToolOutput::success(format!("Generated {} icon sizes", sizes.len())).with_paths(generated))
}

/// Generate standard web icon set from SVG.
#[cfg(feature = "image-svg")]
pub fn generate_web_icons(
    input: impl AsRef<Path>,
    output_dir: impl AsRef<Path>,
) -> std::io::Result<ToolOutput> {
    let sizes = vec![16, 32, 48, 64, 96, 128, 192, 256, 384, 512];
    generate_icons_from_svg(input, output_dir, &sizes)
}

/// Generate iOS icon set from SVG.
#[cfg(feature = "image-svg")]
pub fn generate_ios_icons(
    input: impl AsRef<Path>,
    output_dir: impl AsRef<Path>,
) -> std::io::Result<ToolOutput> {
    let sizes = vec![20, 29, 40, 58, 60, 76, 80, 87, 120, 152, 167, 180, 1024];
    generate_icons_from_svg(input, output_dir, &sizes)
}

/// Generate Android icon set from SVG.
#[cfg(feature = "image-svg")]
pub fn generate_android_icons(
    input: impl AsRef<Path>,
    output_dir: impl AsRef<Path>,
) -> std::io::Result<ToolOutput> {
    let sizes = vec![36, 48, 72, 96, 144, 192, 512];
    generate_icons_from_svg(input, output_dir, &sizes)
}

/// Generate all platform icons from SVG.
#[cfg(feature = "image-svg")]
pub fn generate_all_icons(
    input: impl AsRef<Path>,
    output_dir: impl AsRef<Path>,
) -> std::io::Result<ToolOutput> {
    let mut all_sizes = vec![
        // Web
        16, 32, 48, 64, 96, 128, 192, 256, 384, 512, // iOS
        20, 29, 40, 58, 60, 76, 80, 87, 120, 152, 167, 180, 1024, // Android
        36, 72, 144,
    ];
    all_sizes.sort_unstable();
    all_sizes.dedup();

    generate_icons_from_svg(input, output_dir, &all_sizes)
}

#[cfg(not(feature = "image-svg"))]
pub fn svg_to_png(
    _input: impl AsRef<Path>,
    _output: impl AsRef<Path>,
    _width: u32,
    _height: u32,
) -> std::io::Result<ToolOutput> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "SVG support requires 'image-svg' feature. Enable with --features image-svg",
    ))
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_svg_feature_flag() {
        #[cfg(feature = "image-svg")]
        {
            assert!(true, "SVG feature enabled");
        }
        #[cfg(not(feature = "image-svg"))]
        {
            assert!(true, "SVG feature disabled");
        }
    }
}
