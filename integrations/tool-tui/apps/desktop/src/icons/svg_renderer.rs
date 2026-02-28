#![allow(dead_code)]

use anyhow::Result;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tiny_skia::Pixmap;
use usvg::{Options, Tree};

/// Cache for rendered SVG images to avoid re-rendering
///
/// Performance optimizations:
/// 1. **Caching**: Rendered SVGs are cached by ID and size to avoid redundant rendering
/// 2. **Lazy rendering**: Only visible icons (current page) are rendered, not all icons
/// 3. **Efficient SVG parsing**: Uses resvg + tiny-skia for fast rendering to PNG
/// 4. **Memory management**: Cache can be cleared when needed to free memory
///
/// This approach eliminates lag by:
/// - Not rendering thousands of icons at once
/// - Reusing previously rendered icons when navigating
/// - Converting SVG to PNG once, then using fast image rendering
pub struct SvgCache {
    cache: Arc<Mutex<HashMap<String, PathBuf>>>,
    temp_dir: PathBuf,
}

impl SvgCache {
    pub fn new() -> Self {
        // Create temp directory for cached PNGs
        let temp_dir = std::env::temp_dir().join("dx-icon-cache");
        let _ = std::fs::create_dir_all(&temp_dir);

        Self {
            cache: Arc::new(Mutex::new(HashMap::new())),
            temp_dir,
        }
    }

    /// Render an SVG string to a PNG file with caching, returns the file path
    pub fn render_svg(&self, svg_id: &str, svg_string: &str, target_size: u32) -> Result<PathBuf> {
        let cache_key = format!("{}:{}", svg_id, target_size);

        // Check cache first
        {
            let cache = self.cache.lock().unwrap();
            if let Some(cached_path) = cache.get(&cache_key) {
                if cached_path.exists() {
                    return Ok(cached_path.clone());
                }
            }
        }

        // Render the SVG to PNG (with error handling to prevent crashes)
        let png_path = self.render_svg_to_png(svg_id, svg_string, target_size)?;

        // Store in cache
        {
            let mut cache = self.cache.lock().unwrap();
            cache.insert(cache_key, png_path.clone());
        }

        Ok(png_path)
    }

    /// Render SVG string to PNG file using resvg
    fn render_svg_to_png(
        &self,
        svg_id: &str,
        svg_string: &str,
        target_size: u32,
    ) -> Result<PathBuf> {
        // Parse SVG with error handling
        let opt = Options::default();
        let tree = Tree::from_str(svg_string, &opt)
            .map_err(|e| anyhow::anyhow!("Failed to parse SVG: {}", e))?;

        // Calculate scaling to fit target size
        let svg_size = tree.size();
        if svg_size.width() == 0.0 || svg_size.height() == 0.0 {
            return Err(anyhow::anyhow!(
                "Invalid SVG size: {}x{}",
                svg_size.width(),
                svg_size.height()
            ));
        }

        let scale = (target_size as f32 / svg_size.width().max(svg_size.height())).min(4.0);
        let width = (svg_size.width() * scale).ceil() as u32;
        let height = (svg_size.height() * scale).ceil() as u32;

        if width == 0 || height == 0 || width > 4096 || height > 4096 {
            return Err(anyhow::anyhow!("Invalid dimensions: {}x{}", width, height));
        }

        // Create pixmap and render
        let mut pixmap =
            Pixmap::new(width, height).ok_or_else(|| anyhow::anyhow!("Failed to create pixmap"))?;

        let transform = tiny_skia::Transform::from_scale(scale, scale);
        resvg::render(&tree, transform, &mut pixmap.as_mut());

        // Save to PNG file
        let safe_id = svg_id.replace([':', '/', '\\', '<', '>', '|', '?', '*'], "_");
        let png_path = self.temp_dir.join(format!("{}_{}.png", safe_id, target_size));

        pixmap.save_png(&png_path)?;

        Ok(png_path)
    }

    /// Clear the cache to free memory
    #[allow(dead_code)]
    pub fn clear(&self) {
        let mut cache = self.cache.lock().unwrap();
        cache.clear();

        // Optionally delete temp files
        let _ = std::fs::remove_dir_all(&self.temp_dir);
        let _ = std::fs::create_dir_all(&self.temp_dir);
    }

    /// Get cache size
    #[allow(dead_code)]
    pub fn cache_size(&self) -> usize {
        let cache = self.cache.lock().unwrap();
        cache.len()
    }
}

impl Default for SvgCache {
    fn default() -> Self {
        Self::new()
    }
}
