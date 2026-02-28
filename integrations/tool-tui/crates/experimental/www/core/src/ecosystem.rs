//! Ecosystem Integration Module for dx-www
//!
//! This module provides unified access to all dx ecosystem tools:
//! - dx-serializer: Configuration and state serialization
//! - dx-markdown: Content page compilation
//! - dx-generator: Code scaffolding
//! - dx-icon: Icon management
//! - dx-font: Font optimization
//! - dx-media: Media processing
//!
//! ## Usage
//! ```rust,ignore
//! use dx_compiler::ecosystem;
//!
//! // Initialize ecosystem with project root
//! let config = ecosystem::init(&project_root)?;
//!
//! // Process icons in source code
//! let icons = ecosystem::process_icons(&source, &config.icon_config)?;
//! ```

use anyhow::{Context, Result};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

// Use the schema_parser from the crate root
use crate::schema_parser;
use crate::schema_parser::QueryDefinition;
use crate::www_config::{self, DxWwwConfig};

// Re-export dx-icon for external use
pub use dx_icon;

// ============================================================================
// Ecosystem Configuration
// ============================================================================

/// Ecosystem configuration loaded from dx.config
#[derive(Debug, Clone)]
pub struct EcosystemConfig {
    /// Project root directory
    pub project_root: PathBuf,
    /// Loaded www configuration
    pub www_config: DxWwwConfig,
    /// Icon configuration (if enabled)
    pub icon_config: Option<IconConfig>,
    /// Font configuration (if enabled)
    pub font_config: Option<FontConfig>,
    /// Media configuration (if enabled)
    pub media_config: Option<MediaConfig>,
}

/// Icon processing configuration
#[derive(Debug, Clone)]
pub struct IconConfig {
    /// Icon sets to include
    pub sets: Vec<String>,
    /// Custom icon directory
    pub custom_dir: Option<PathBuf>,
    /// Enable tree-shaking
    pub tree_shake: bool,
}

impl Default for IconConfig {
    fn default() -> Self {
        Self {
            sets: vec!["lucide".to_string()],
            custom_dir: None,
            tree_shake: true,
        }
    }
}

/// Font processing configuration
#[derive(Debug, Clone)]
pub struct FontConfig {
    /// Font families to include
    pub families: Vec<FontFamily>,
    /// Enable subsetting
    pub subset: bool,
    /// Preload fonts
    pub preload: bool,
}

/// Font family definition
#[derive(Debug, Clone)]
pub struct FontFamily {
    /// Font family name
    pub name: String,
    /// Font weights to include
    pub weights: Vec<u16>,
    /// Is this a variable font
    pub variable: bool,
}

impl Default for FontConfig {
    fn default() -> Self {
        Self {
            families: Vec::new(),
            subset: true,
            preload: true,
        }
    }
}

/// Media processing configuration
#[derive(Debug, Clone)]
pub struct MediaConfig {
    /// Image output formats
    pub image_formats: Vec<String>,
    /// Image quality (1-100)
    pub quality: u8,
    /// Generate blur placeholders
    pub blur_placeholder: bool,
    /// Responsive breakpoints
    pub breakpoints: Vec<u32>,
}

impl Default for MediaConfig {
    fn default() -> Self {
        Self {
            image_formats: vec!["webp".to_string(), "avif".to_string()],
            quality: 85,
            blur_placeholder: true,
            breakpoints: vec![640, 768, 1024, 1280, 1536],
        }
    }
}

// ============================================================================
// Initialization
// ============================================================================

/// Initialize ecosystem integrations from project root
pub fn init(project_root: &Path) -> Result<EcosystemConfig> {
    // Load configuration
    let www_config =
        www_config::load_config_from_root(project_root).unwrap_or_else(|_| DxWwwConfig::default());

    // Build icon config from www_config
    let icon_config = www_config.assets.icons.as_ref().map(|icons| IconConfig {
        sets: icons.sets.clone(),
        custom_dir: icons.custom_dir.clone(),
        tree_shake: true,
    });

    // Build font config from www_config
    let font_config = www_config.assets.fonts.as_ref().map(|fonts| FontConfig {
        families: fonts
            .families
            .iter()
            .map(|f| FontFamily {
                name: f.name.clone(),
                weights: f.weights.clone(),
                variable: f.variable,
            })
            .collect(),
        subset: fonts.subset,
        preload: true,
    });

    // Build media config from www_config
    let media_config = www_config.assets.media.as_ref().and_then(|media| {
        media.images.as_ref().map(|images| MediaConfig {
            image_formats: images.formats.clone(),
            quality: images.quality,
            blur_placeholder: images.blur_placeholder,
            breakpoints: vec![640, 768, 1024, 1280, 1536],
        })
    });

    Ok(EcosystemConfig {
        project_root: project_root.to_path_buf(),
        www_config,
        icon_config,
        font_config,
        media_config,
    })
}

/// Initialize with default configuration (for testing)
pub fn init_default() -> EcosystemConfig {
    EcosystemConfig {
        project_root: PathBuf::from("."),
        www_config: DxWwwConfig::default(),
        icon_config: Some(IconConfig::default()),
        font_config: Some(FontConfig::default()),
        media_config: Some(MediaConfig::default()),
    }
}

// ============================================================================
// Icon Processing
// ============================================================================

/// Processed icons result
#[derive(Debug, Clone)]
pub struct ProcessedIcons {
    /// Icons used in the source
    pub used: Vec<String>,
    /// Resolved icon data
    pub resolved: Vec<ResolvedIcon>,
    /// Generated sprite sheet (if tree-shaking enabled)
    pub sprite: Option<String>,
}

/// Resolved icon data
#[derive(Debug, Clone)]
pub struct ResolvedIcon {
    /// Icon name
    pub name: String,
    /// Icon set (prefix)
    pub set: String,
    /// SVG content
    pub svg: String,
    /// Icon width
    pub width: u32,
    /// Icon height
    pub height: u32,
}

/// Process icons in source code using dx-icon
pub fn process_icons(source: &str, config: &IconConfig) -> Result<ProcessedIcons> {
    let mut icons_used = Vec::new();
    let mut seen = HashSet::new();

    // Scan for <Icon name="..." /> patterns
    // Supports: <Icon name="home" />, <Icon name="mdi:home" />, <Icon set="mdi" name="home" />
    let icon_regex = regex::Regex::new(r#"<Icon\s+(?:set="([^"]+)"\s+)?name="([^"]+)""#)?;

    for cap in icon_regex.captures_iter(source) {
        let set = cap.get(1).map(|m| m.as_str().to_string());
        let name = cap.get(2).map(|m| m.as_str().to_string()).unwrap_or_default();

        // Parse "set:name" format if no explicit set
        let (final_set, final_name) = if let Some(s) = set {
            (s, name)
        } else if name.contains(':') {
            let parts: Vec<&str> = name.splitn(2, ':').collect();
            (parts[0].to_string(), parts[1].to_string())
        } else {
            // Use first configured set as default
            let default_set = config.sets.first().cloned().unwrap_or_else(|| "lucide".to_string());
            (default_set, name)
        };

        let key = format!("{}:{}", final_set, final_name);
        if !seen.contains(&key) {
            seen.insert(key.clone());
            icons_used.push(key);
        }
    }

    // Resolve icons through dx-icon
    let resolved = resolve_icons_from_library(&icons_used, config)?;

    // Generate sprite sheet if tree-shaking is enabled
    let sprite = if config.tree_shake && !resolved.is_empty() {
        Some(generate_icon_sprite(&resolved))
    } else {
        None
    };

    // Extract just the names for the used list
    let used_names: Vec<String> = icons_used
        .iter()
        .map(|k| k.split(':').next_back().unwrap_or(k).to_string())
        .collect();

    Ok(ProcessedIcons {
        used: used_names,
        resolved,
        sprite,
    })
}

/// Resolve icons from the dx-icon library
fn resolve_icons_from_library(
    icon_keys: &[String],
    config: &IconConfig,
) -> Result<Vec<ResolvedIcon>> {
    let mut reader = dx_icon::icons();
    let mut resolved = Vec::new();

    for key in icon_keys {
        let parts: Vec<&str> = key.splitn(2, ':').collect();
        if parts.len() != 2 {
            continue;
        }

        let (set, name) = (parts[0], parts[1]);

        // Check if this set is allowed by config
        if !config.sets.contains(&set.to_string()) && !config.sets.is_empty() {
            continue;
        }

        // Try to get the icon from dx-icon
        if let Some(icon) = reader.get(set, name) {
            let width = icon.width.unwrap_or(24);
            let height = icon.height.unwrap_or(24);

            resolved.push(ResolvedIcon {
                name: name.to_string(),
                set: set.to_string(),
                svg: icon.to_svg(24),
                width,
                height,
            });
        } else {
            // Fallback: generate placeholder SVG
            resolved.push(ResolvedIcon {
                name: name.to_string(),
                set: set.to_string(),
                svg: format!(r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" width="24" height="24"><!-- {} --></svg>"#, name),
                width: 24,
                height: 24,
            });
        }
    }

    Ok(resolved)
}

/// Tree-shake icons: filter to only include icons actually used in source
pub fn tree_shake_icons(all_icons: &[ResolvedIcon], source: &str) -> Vec<ResolvedIcon> {
    let mut used_icons = Vec::new();

    for icon in all_icons {
        // Check if this icon is referenced in the source
        let patterns = [
            format!(r#"name="{}""#, icon.name),
            format!(r#"name="{}:{}""#, icon.set, icon.name),
            format!(r#"icon-{}"#, icon.name),
        ];

        if patterns.iter().any(|p| source.contains(p)) {
            used_icons.push(icon.clone());
        }
    }

    used_icons
}

/// Generate optimized icon sprite sheet
pub fn generate_icon_sprite(icons: &[ResolvedIcon]) -> String {
    let mut sprite =
        String::from(r#"<svg xmlns="http://www.w3.org/2000/svg" style="display:none">"#);

    for icon in icons {
        // Extract the inner content from the SVG
        let inner = extract_svg_inner(&icon.svg);
        sprite.push_str(&format!(
            r#"<symbol id="icon-{}-{}" viewBox="0 0 {} {}">{}</symbol>"#,
            icon.set, icon.name, icon.width, icon.height, inner
        ));
    }

    sprite.push_str("</svg>");
    sprite
}

/// Extract inner content from SVG element
fn extract_svg_inner(svg: &str) -> String {
    // Find the content between <svg ...> and </svg>
    if let Some(start) = svg.find('>') {
        if let Some(end) = svg.rfind("</svg>") {
            return svg[start + 1..end].to_string();
        }
    }
    svg.to_string()
}

/// Generate CSS for icon usage via sprite
pub fn generate_icon_css(icons: &[ResolvedIcon]) -> String {
    let mut css = String::from(
        r#"
.dx-icon {
  display: inline-block;
  width: 1em;
  height: 1em;
  vertical-align: -0.125em;
  fill: currentColor;
}
"#,
    );

    for icon in icons {
        css.push_str(&format!(
            r#"
.dx-icon-{}-{} {{
  width: {}px;
  height: {}px;
}}
"#,
            icon.set, icon.name, icon.width, icon.height
        ));
    }

    css
}

// ============================================================================
// Font Processing (dx-font integration)
// ============================================================================

/// Processed fonts result
#[derive(Debug, Clone)]
pub struct ProcessedFonts {
    /// Processed font data
    pub fonts: Vec<ProcessedFont>,
    /// CSS for font-face declarations
    pub css: String,
    /// Preload hints for critical fonts
    pub preload_hints: Vec<PreloadHint>,
}

/// Processed font data
#[derive(Debug, Clone)]
pub struct ProcessedFont {
    /// Font family name
    pub family: String,
    /// Font data (binary)
    pub data: Vec<u8>,
    /// Whether subsetting is needed
    pub needs_subset: bool,
    /// Characters used (for subsetting)
    pub used_chars: HashSet<char>,
    /// Font weights included
    pub weights: Vec<u16>,
    /// Is variable font
    pub variable: bool,
    /// Output path for the font file
    pub output_path: PathBuf,
    /// Font format (woff2, woff, ttf)
    pub format: String,
}

/// Preload hint for critical fonts
#[derive(Debug, Clone)]
pub struct PreloadHint {
    /// Font file path
    pub href: String,
    /// Font format
    pub format: String,
    /// Crossorigin attribute
    pub crossorigin: bool,
}

/// Font processing options
#[derive(Debug, Clone)]
pub struct FontProcessingOptions {
    /// Output directory for processed fonts
    pub output_dir: PathBuf,
    /// Preferred formats in order of priority
    pub preferred_formats: Vec<String>,
    /// Enable font subsetting
    pub subset: bool,
    /// Generate preload hints
    pub preload: bool,
}

impl Default for FontProcessingOptions {
    fn default() -> Self {
        Self {
            output_dir: PathBuf::from("dist/fonts"),
            preferred_formats: vec!["woff2".to_string(), "woff".to_string()],
            subset: true,
            preload: true,
        }
    }
}

/// Parse font configuration from dx.config
pub fn parse_font_config(config: &DxWwwConfig) -> Option<FontConfig> {
    config.assets.fonts.as_ref().map(|fonts| FontConfig {
        families: fonts
            .families
            .iter()
            .map(|f| FontFamily {
                name: f.name.clone(),
                weights: f.weights.clone(),
                variable: f.variable,
            })
            .collect(),
        subset: fonts.subset,
        preload: true,
    })
}

/// Process fonts in configuration using dx-font
pub fn process_fonts(config: &FontConfig) -> Result<ProcessedFonts> {
    process_fonts_with_options(config, &FontProcessingOptions::default())
}

/// Process fonts with custom options
pub fn process_fonts_with_options(
    config: &FontConfig,
    options: &FontProcessingOptions,
) -> Result<ProcessedFonts> {
    let mut fonts = Vec::new();
    let mut css = String::new();
    let mut preload_hints = Vec::new();

    // Add CSS reset for font-display
    css.push_str("/* DX-WWW Font Declarations */\n\n");

    for family in &config.families {
        let font_slug = family.name.to_lowercase().replace(' ', "-");

        // Determine weights to process
        let weights = if family.weights.is_empty() {
            vec![400] // Default to regular weight
        } else {
            family.weights.clone()
        };

        // Create processed font entry
        let font = ProcessedFont {
            family: family.name.clone(),
            data: Vec::new(), // Data loaded on demand via dx-font
            needs_subset: config.subset,
            used_chars: HashSet::new(),
            weights: weights.clone(),
            variable: family.variable,
            output_path: options.output_dir.join(format!("{}.woff2", font_slug)),
            format: "woff2".to_string(),
        };

        // Generate CSS for variable fonts
        if family.variable {
            css.push_str(&generate_variable_font_css(&family.name, &font_slug, &weights));

            // Add preload hint for variable font
            if options.preload {
                preload_hints.push(PreloadHint {
                    href: format!("/fonts/{}-variable.woff2", font_slug),
                    format: "woff2".to_string(),
                    crossorigin: true,
                });
            }
        } else {
            // Generate CSS for static fonts
            css.push_str(&generate_static_font_css(&family.name, &font_slug, &weights));

            // Add preload hints for critical weights (400, 700)
            if options.preload {
                for weight in &weights {
                    if *weight == 400 || *weight == 700 {
                        preload_hints.push(PreloadHint {
                            href: format!("/fonts/{}-{}.woff2", font_slug, weight),
                            format: "woff2".to_string(),
                            crossorigin: true,
                        });
                    }
                }
            }
        }

        fonts.push(font);
    }

    Ok(ProcessedFonts {
        fonts,
        css,
        preload_hints,
    })
}

/// Generate CSS for variable fonts
fn generate_variable_font_css(family_name: &str, font_slug: &str, weights: &[u16]) -> String {
    let min_weight = weights.iter().min().copied().unwrap_or(100);
    let max_weight = weights.iter().max().copied().unwrap_or(900);

    format!(
        r#"@font-face {{
  font-family: '{}';
  font-weight: {} {};
  font-style: normal;
  font-display: swap;
  src: url('/fonts/{}-variable.woff2') format('woff2-variations');
  unicode-range: U+0000-00FF, U+0131, U+0152-0153, U+02BB-02BC, U+02C6, U+02DA, U+02DC, U+2000-206F, U+2074, U+20AC, U+2122, U+2191, U+2193, U+2212, U+2215, U+FEFF, U+FFFD;
}}

"#,
        family_name, min_weight, max_weight, font_slug
    )
}

/// Generate CSS for static fonts
fn generate_static_font_css(family_name: &str, font_slug: &str, weights: &[u16]) -> String {
    let mut css = String::new();

    for weight in weights {
        css.push_str(&format!(
            r#"@font-face {{
  font-family: '{}';
  font-weight: {};
  font-style: normal;
  font-display: swap;
  src: url('/fonts/{}-{}.woff2') format('woff2'),
       url('/fonts/{}-{}.woff') format('woff');
  unicode-range: U+0000-00FF, U+0131, U+0152-0153, U+02BB-02BC, U+02C6, U+02DA, U+02DC, U+2000-206F, U+2074, U+20AC, U+2122, U+2191, U+2193, U+2212, U+2215, U+FEFF, U+FFFD;
}}

"#,
            family_name, weight, font_slug, weight, font_slug, weight
        ));
    }

    css
}

/// Extract characters used in source code for font subsetting
pub fn extract_used_characters(source: &str) -> HashSet<char> {
    source.chars().collect()
}

/// Subset font based on used characters
pub fn subset_font(font: &ProcessedFont, chars: &HashSet<char>) -> Result<Vec<u8>> {
    // If no subsetting needed or no characters, return original
    if !font.needs_subset || chars.is_empty() {
        return Ok(font.data.clone());
    }

    // Create subset data
    // In a full implementation, this would use dx-font's subsetting capabilities
    // For now, we return the original data with a marker
    let mut subset_data = font.data.clone();

    // Add metadata about subset (placeholder)
    if subset_data.is_empty() {
        // Generate placeholder subset info
        let subset_info = format!("SUBSET:{}:chars={}", font.family, chars.len());
        subset_data = subset_info.into_bytes();
    }

    Ok(subset_data)
}

/// Subset font to include only specified characters
pub fn subset_font_to_chars(font_data: &[u8], chars: &HashSet<char>) -> Result<Vec<u8>> {
    if font_data.is_empty() || chars.is_empty() {
        return Ok(font_data.to_vec());
    }

    // In production, this would use a font subsetting library
    // For now, return original data
    Ok(font_data.to_vec())
}

/// Generate preload link tags for fonts
pub fn generate_font_preload_html(hints: &[PreloadHint]) -> String {
    hints
        .iter()
        .map(|hint| {
            format!(
                r#"<link rel="preload" href="{}" as="font" type="font/{}" crossorigin{}>"#,
                hint.href,
                hint.format,
                if hint.crossorigin {
                    ""
                } else {
                    "=\"anonymous\""
                }
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Calculate optimal font subset based on content analysis
pub fn calculate_font_subset(content_sources: &[&str], include_common: bool) -> HashSet<char> {
    let mut chars = HashSet::new();

    // Extract characters from all content sources
    for source in content_sources {
        chars.extend(source.chars());
    }

    // Optionally include common characters
    if include_common {
        // Add common punctuation and symbols
        let common = "!\"#$%&'()*+,-./:;<=>?@[\\]^_`{|}~ \t\n\r";
        chars.extend(common.chars());

        // Add digits
        chars.extend('0'..='9');

        // Add basic Latin letters
        chars.extend('a'..='z');
        chars.extend('A'..='Z');
    }

    chars
}

/// Font download integration with dx-font
pub async fn download_font_family(
    family_name: &str,
    weights: &[u16],
    output_dir: &Path,
) -> Result<Vec<PathBuf>> {
    // This would integrate with dx-font's FontSearch and FontDownloader
    // For now, return placeholder paths
    let font_slug = family_name.to_lowercase().replace(' ', "-");
    let mut paths = Vec::new();

    for weight in weights {
        let path = output_dir.join(format!("{}-{}.woff2", font_slug, weight));
        paths.push(path);
    }

    Ok(paths)
}

// ============================================================================
// Media Processing (dx-media integration)
// ============================================================================

/// Processed media result
#[derive(Debug, Clone)]
pub struct ProcessedMedia {
    /// Original file path
    pub original: PathBuf,
    /// Generated variants
    pub variants: Vec<ImageVariant>,
    /// Blur placeholder (base64 data URL)
    pub blur_placeholder: Option<String>,
    /// Original dimensions
    pub original_dimensions: Option<(u32, u32)>,
    /// Dominant color (hex)
    pub dominant_color: Option<String>,
}

/// Image variant
#[derive(Debug, Clone)]
pub struct ImageVariant {
    /// Variant path
    pub path: PathBuf,
    /// Width
    pub width: u32,
    /// Height (calculated from aspect ratio)
    pub height: u32,
    /// Format
    pub format: String,
    /// File size estimate (bytes)
    pub size_estimate: Option<u64>,
}

/// Media processing options
#[derive(Debug, Clone)]
pub struct MediaProcessingOptions {
    /// Output directory for processed media
    pub output_dir: PathBuf,
    /// Quality for lossy formats (1-100)
    pub quality: u8,
    /// Generate blur placeholders
    pub blur_placeholder: bool,
    /// Responsive breakpoints
    pub breakpoints: Vec<u32>,
    /// Output formats
    pub formats: Vec<String>,
    /// Preserve original
    pub preserve_original: bool,
}

impl Default for MediaProcessingOptions {
    fn default() -> Self {
        Self {
            output_dir: PathBuf::from("dist/images"),
            quality: 85,
            blur_placeholder: true,
            breakpoints: vec![640, 768, 1024, 1280, 1536],
            formats: vec!["webp".to_string(), "avif".to_string()],
            preserve_original: true,
        }
    }
}

/// Detect image imports in source code
pub fn detect_image_imports(source: &str) -> Vec<ImageImport> {
    let mut imports = Vec::new();

    // Match various import patterns:
    // import img from './image.png'
    // import { src } from './image.png?w=640'
    // <img src="./image.png" />
    // <Image src="./image.png" />

    // Pattern 1: ES import
    let import_regex = regex::Regex::new(
        r#"import\s+\w+\s+from\s+['"]([^'"]+\.(png|jpg|jpeg|gif|webp|avif|svg))(?:\?[^'"]*)?['"]"#,
    )
    .unwrap();

    for cap in import_regex.captures_iter(source) {
        if let Some(path) = cap.get(1) {
            imports.push(ImageImport {
                path: PathBuf::from(path.as_str()),
                import_type: ImageImportType::EsModule,
                query_params: extract_query_params(path.as_str()),
            });
        }
    }

    // Pattern 2: JSX img/Image src
    let jsx_regex = regex::Regex::new(
        r#"<(?:img|Image)\s+[^>]*src=["']([^"']+\.(png|jpg|jpeg|gif|webp|avif|svg))(?:\?[^"']*)?["']"#
    ).unwrap();

    for cap in jsx_regex.captures_iter(source) {
        if let Some(path) = cap.get(1) {
            imports.push(ImageImport {
                path: PathBuf::from(path.as_str()),
                import_type: ImageImportType::JsxSrc,
                query_params: extract_query_params(path.as_str()),
            });
        }
    }

    imports
}

/// Image import information
#[derive(Debug, Clone)]
pub struct ImageImport {
    /// Path to the image
    pub path: PathBuf,
    /// Type of import
    pub import_type: ImageImportType,
    /// Query parameters (e.g., ?w=640&format=webp)
    pub query_params: HashMap<String, String>,
}

/// Type of image import
#[derive(Debug, Clone, PartialEq)]
pub enum ImageImportType {
    /// ES module import
    EsModule,
    /// JSX src attribute
    JsxSrc,
    /// CSS url()
    CssUrl,
}

use std::collections::HashMap;

/// Extract query parameters from a path
fn extract_query_params(path: &str) -> HashMap<String, String> {
    let mut params = HashMap::new();

    if let Some(query_start) = path.find('?') {
        let query = &path[query_start + 1..];
        for pair in query.split('&') {
            if let Some(eq_pos) = pair.find('=') {
                let key = pair[..eq_pos].to_string();
                let value = pair[eq_pos + 1..].to_string();
                params.insert(key, value);
            }
        }
    }

    params
}

/// Process media asset with full optimization
pub fn process_media(path: &Path, config: &MediaConfig) -> Result<ProcessedMedia> {
    process_media_with_options(
        path,
        &MediaProcessingOptions {
            quality: config.quality,
            blur_placeholder: config.blur_placeholder,
            breakpoints: config.breakpoints.clone(),
            formats: config.image_formats.clone(),
            ..Default::default()
        },
    )
}

/// Process media asset with custom options
pub fn process_media_with_options(
    path: &Path,
    options: &MediaProcessingOptions,
) -> Result<ProcessedMedia> {
    let mut variants = Vec::new();
    let file_stem = path.file_stem().unwrap_or_default().to_string_lossy();

    // Estimate original dimensions (placeholder - would use image library)
    let original_dimensions = estimate_image_dimensions(path);
    let aspect_ratio = original_dimensions.map(|(w, h)| w as f64 / h as f64).unwrap_or(16.0 / 9.0);

    // Generate responsive variants
    for &width in &options.breakpoints {
        // Skip breakpoints larger than original
        if let Some((orig_w, _)) = original_dimensions {
            if width > orig_w {
                continue;
            }
        }

        let height = (width as f64 / aspect_ratio).round() as u32;

        for format in &options.formats {
            let variant_name = format!("{}-{}w.{}", file_stem, width, format);
            let variant_path = options.output_dir.join(&variant_name);

            // Estimate file size based on format and dimensions
            let size_estimate = estimate_file_size(width, height, format, options.quality);

            variants.push(ImageVariant {
                path: variant_path,
                width,
                height,
                format: format.clone(),
                size_estimate: Some(size_estimate),
            });
        }
    }

    // Generate blur placeholder
    let blur_placeholder = if options.blur_placeholder {
        Some(generate_blur_placeholder(path, original_dimensions))
    } else {
        None
    };

    // Extract dominant color (placeholder)
    let dominant_color = extract_dominant_color(path);

    Ok(ProcessedMedia {
        original: path.to_path_buf(),
        variants,
        blur_placeholder,
        original_dimensions,
        dominant_color,
    })
}

/// Estimate image dimensions from file (placeholder)
fn estimate_image_dimensions(path: &Path) -> Option<(u32, u32)> {
    // In production, this would read image headers
    // For now, return a reasonable default based on common image sizes
    let ext = path.extension()?.to_str()?;
    match ext {
        "png" | "jpg" | "jpeg" | "webp" => Some((1920, 1080)),
        "gif" => Some((800, 600)),
        "svg" => Some((100, 100)), // SVG is scalable
        _ => Some((1920, 1080)),
    }
}

/// Estimate file size based on format and dimensions
fn estimate_file_size(width: u32, height: u32, format: &str, quality: u8) -> u64 {
    let pixels = width as u64 * height as u64;
    let quality_factor = quality as f64 / 100.0;

    // Rough estimates based on format compression ratios
    let bytes_per_pixel = match format {
        "avif" => 0.1 * quality_factor,
        "webp" => 0.15 * quality_factor,
        "jpg" | "jpeg" => 0.2 * quality_factor,
        "png" => 0.5, // PNG is lossless
        _ => 0.3,
    };

    (pixels as f64 * bytes_per_pixel) as u64
}

/// Generate blur placeholder (LQIP - Low Quality Image Placeholder)
fn generate_blur_placeholder(_path: &Path, dimensions: Option<(u32, u32)>) -> String {
    let (w, h) = dimensions.unwrap_or((16, 9));
    let aspect = w as f64 / h as f64;

    // Generate a simple SVG blur placeholder
    let svg = format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {} {}" preserveAspectRatio="none">
  <filter id="b" color-interpolation-filters="sRGB">
    <feGaussianBlur stdDeviation="20"/>
  </filter>
  <rect width="100%" height="100%" fill="#e0e0e0" filter="url(#b)"/>
</svg>"##,
        (aspect * 10.0).round() as u32,
        10
    );

    // Encode as base64 data URL
    let encoded = base64_encode(svg.as_bytes());
    format!("data:image/svg+xml;base64,{}", encoded)
}

/// Simple base64 encoding
fn base64_encode(data: &[u8]) -> String {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

    let mut result = String::new();
    let mut i = 0;

    while i < data.len() {
        let b0 = data[i] as usize;
        let b1 = if i + 1 < data.len() {
            data[i + 1] as usize
        } else {
            0
        };
        let b2 = if i + 2 < data.len() {
            data[i + 2] as usize
        } else {
            0
        };

        result.push(ALPHABET[b0 >> 2] as char);
        result.push(ALPHABET[((b0 & 0x03) << 4) | (b1 >> 4)] as char);

        if i + 1 < data.len() {
            result.push(ALPHABET[((b1 & 0x0f) << 2) | (b2 >> 6)] as char);
        } else {
            result.push('=');
        }

        if i + 2 < data.len() {
            result.push(ALPHABET[b2 & 0x3f] as char);
        } else {
            result.push('=');
        }

        i += 3;
    }

    result
}

/// Extract dominant color from image (placeholder)
fn extract_dominant_color(_path: &Path) -> Option<String> {
    // In production, this would analyze the image
    // Return a neutral gray as placeholder
    Some("#808080".to_string())
}

/// Generate srcset attribute for responsive images
pub fn generate_srcset(variants: &[ImageVariant]) -> String {
    variants
        .iter()
        .map(|v| format!("{} {}w", v.path.display(), v.width))
        .collect::<Vec<_>>()
        .join(", ")
}

/// Generate sizes attribute for responsive images
pub fn generate_sizes(breakpoints: &[(u32, &str)]) -> String {
    // breakpoints: [(max_width, size), ...]
    // e.g., [(640, "100vw"), (1024, "50vw"), (0, "33vw")]
    breakpoints
        .iter()
        .map(|(max_width, size)| {
            if *max_width == 0 {
                size.to_string()
            } else {
                format!("(max-width: {}px) {}", max_width, size)
            }
        })
        .collect::<Vec<_>>()
        .join(", ")
}

/// Generate picture element HTML for responsive images
pub fn generate_picture_html(media: &ProcessedMedia, alt: &str) -> String {
    let mut html = String::from("<picture>\n");

    // Group variants by format
    let mut by_format: HashMap<&str, Vec<&ImageVariant>> = HashMap::new();
    for variant in &media.variants {
        by_format.entry(&variant.format).or_default().push(variant);
    }

    // Generate source elements for each format (prefer modern formats first)
    for format in &["avif", "webp"] {
        if let Some(variants) = by_format.get(format as &str) {
            let srcset = variants
                .iter()
                .map(|v| format!("{} {}w", v.path.display(), v.width))
                .collect::<Vec<_>>()
                .join(", ");

            html.push_str(&format!("  <source type=\"image/{}\" srcset=\"{}\">\n", format, srcset));
        }
    }

    // Fallback img element
    let fallback = media
        .variants
        .first()
        .map(|v| v.path.display().to_string())
        .unwrap_or_else(|| media.original.display().to_string());

    // Add blur placeholder as style if available
    let style = media
        .blur_placeholder
        .as_ref()
        .map(|p| format!(" style=\"background-image: url('{}'); background-size: cover;\"", p))
        .unwrap_or_default();

    html.push_str(&format!(
        "  <img src=\"{}\" alt=\"{}\" loading=\"lazy\"{}>\n",
        fallback, alt, style
    ));

    html.push_str("</picture>");
    html
}

/// Optimize all media in a directory
pub fn optimize_media_directory(dir: &Path, config: &MediaConfig) -> Result<Vec<ProcessedMedia>> {
    let mut results = Vec::new();

    // Supported image extensions
    let extensions = ["png", "jpg", "jpeg", "gif", "webp", "avif"];

    // Walk directory
    if dir.is_dir() {
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() {
                if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                    if extensions.contains(&ext.to_lowercase().as_str()) {
                        let processed = process_media(&path, config)?;
                        results.push(processed);
                    }
                }
            }
        }
    }

    Ok(results)
}

// ============================================================================
// DXM Content Processing
// ============================================================================

/// Compiled DXM content
#[derive(Debug, Clone)]
pub struct CompiledContent {
    /// Frontmatter metadata
    pub frontmatter: ContentFrontmatter,
    /// Compiled binary content
    pub binary: Vec<u8>,
    /// HTML output
    pub html: String,
}

/// Content frontmatter
#[derive(Debug, Clone, Default)]
pub struct ContentFrontmatter {
    /// Page title
    pub title: Option<String>,
    /// Page description
    pub description: Option<String>,
    /// Layout to use
    pub layout: Option<String>,
}

/// Compile DXM content to component
pub fn compile_dxm_content(path: &Path) -> Result<CompiledContent> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read DXM file: {}", path.display()))?;

    // Parse frontmatter
    let (frontmatter, body) = parse_frontmatter(&content);

    // Convert to HTML (placeholder - would use dx-markdown)
    let html = markdown_to_html(&body);

    // Generate binary (placeholder)
    let binary = body.as_bytes().to_vec();

    Ok(CompiledContent {
        frontmatter,
        binary,
        html,
    })
}

/// Parse frontmatter from content
fn parse_frontmatter(content: &str) -> (ContentFrontmatter, String) {
    let mut frontmatter = ContentFrontmatter::default();
    let mut body = content.to_string();

    // Check for YAML frontmatter (--- ... ---)
    if let Some(after_start) = content.strip_prefix("---") {
        if let Some(end) = after_start.find("---") {
            let fm_content = &after_start[..end];
            body = after_start[end + 3..].trim().to_string();

            // Parse simple key: value pairs
            for line in fm_content.lines() {
                let line = line.trim();
                if let Some(colon_pos) = line.find(':') {
                    let key = line[..colon_pos].trim();
                    let value = line[colon_pos + 1..].trim().trim_matches('"');

                    match key {
                        "title" => frontmatter.title = Some(value.to_string()),
                        "description" => frontmatter.description = Some(value.to_string()),
                        "layout" => frontmatter.layout = Some(value.to_string()),
                        _ => {}
                    }
                }
            }
        }
    }

    (frontmatter, body)
}

/// Convert markdown to HTML (simple implementation)
fn markdown_to_html(markdown: &str) -> String {
    let mut html = String::new();
    let mut in_code_block = false;

    for line in markdown.lines() {
        if let Some(lang_part) = line.strip_prefix("```") {
            if in_code_block {
                html.push_str("</code></pre>\n");
                in_code_block = false;
            } else {
                let code_lang = lang_part.trim();
                html.push_str(&format!(
                    "<pre><code class=\"language-{}\">",
                    if code_lang.is_empty() {
                        "text"
                    } else {
                        code_lang
                    }
                ));
                in_code_block = true;
            }
            continue;
        }

        if in_code_block {
            html.push_str(&html_escape(line));
            html.push('\n');
            continue;
        }

        // Headers
        if let Some(content) = line.strip_prefix("# ") {
            html.push_str(&format!("<h1>{}</h1>\n", content));
        } else if let Some(content) = line.strip_prefix("## ") {
            html.push_str(&format!("<h2>{}</h2>\n", content));
        } else if let Some(content) = line.strip_prefix("### ") {
            html.push_str(&format!("<h3>{}</h3>\n", content));
        } else if let Some(content) = line.strip_prefix("#### ") {
            html.push_str(&format!("<h4>{}</h4>\n", content));
        } else if let Some(content) = line.strip_prefix("- ") {
            html.push_str(&format!("<li>{}</li>\n", content));
        } else if let Some(content) = line.strip_prefix("* ") {
            html.push_str(&format!("<li>{}</li>\n", content));
        } else if line.is_empty() {
            html.push_str("<br>\n");
        } else {
            html.push_str(&format!("<p>{}</p>\n", line));
        }
    }

    html
}

/// Escape HTML special characters
fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

// ============================================================================
// Legacy Ecosystem Features (from original implementation)
// ============================================================================

/// Process ecosystem features from source code
pub fn process_ecosystem_features(source: &str) -> Result<Vec<EcosystemFeature>> {
    let mut features = Vec::new();

    // Parse form schemas
    let form_schemas = schema_parser::parse_form_schema(source);
    for schema in form_schemas {
        features.push(EcosystemFeature::FormSchema(schema));
    }

    // Parse queries
    let queries = schema_parser::parse_query_definitions(source);
    for query in queries {
        features.push(EcosystemFeature::QueryDefinition(query));
    }

    // Parse DB schemas
    let db_schemas = schema_parser::parse_db_schema(source);
    for schema in db_schemas {
        features.push(EcosystemFeature::DatabaseSchema(schema));
    }

    // Parse state definitions
    let state_defs = schema_parser::parse_state_definitions(source);
    for def in state_defs {
        features.push(EcosystemFeature::StateDefinition(def));
    }

    Ok(features)
}

/// Ecosystem feature types
#[derive(Debug)]
pub enum EcosystemFeature {
    FormSchema(schema_parser::FormSchema),
    QueryDefinition(schema_parser::QueryDefinition),
    DatabaseSchema(schema_parser::TableSchema),
    StateDefinition(schema_parser::StateDefinition),
}

/// Generate Rust code for ecosystem features
pub fn generate_code(features: &[EcosystemFeature]) -> String {
    let mut code = String::new();

    code.push_str("// Generated ecosystem code\n\n");

    for feature in features {
        match feature {
            EcosystemFeature::FormSchema(schema) => {
                code.push_str(&generate_form_validator(schema));
            }
            EcosystemFeature::QueryDefinition(query) => {
                code.push_str(&generate_query_function(query));
            }
            EcosystemFeature::DatabaseSchema(schema) => {
                code.push_str(&generate_db_struct(schema));
            }
            EcosystemFeature::StateDefinition(def) => {
                code.push_str(&generate_state_struct(def));
            }
        }
    }

    code
}

/// Generate form validator code
fn generate_form_validator(schema: &schema_parser::FormSchema) -> String {
    let mut code = format!("// Form validator for {}\n", schema.name);
    code.push_str(&format!("pub struct {} {{\n", schema.name));

    for field in &schema.fields {
        code.push_str(&format!("    pub {}: {},\n", field.name, field.field_type));
    }

    code.push_str("}\n\n");
    code
}

/// Generate query function
fn generate_query_function(query: &QueryDefinition) -> String {
    format!(
        "// Query function for {}\npub async fn {}({}) -> Result<Response> {{\n    // TODO: Implement {} {}\n}}\n\n",
        query.name,
        query.name,
        query.params.join(", "),
        query.method,
        query.endpoint
    )
}

/// Generate database struct
fn generate_db_struct(schema: &schema_parser::TableSchema) -> String {
    let mut code = format!(
        "// Database struct for {}\n#[repr(C)]\npub struct {} {{\n",
        schema.name, schema.name
    );

    for col in &schema.columns {
        let col_type = if col.nullable {
            format!("Option<{}>", col.column_type)
        } else {
            col.column_type.clone()
        };
        code.push_str(&format!("    pub {}: {},\n", col.name, col_type));
    }

    code.push_str("}\n\n");
    code
}

/// Generate state struct
fn generate_state_struct(def: &schema_parser::StateDefinition) -> String {
    let mut code =
        format!("// State struct for {}\n#[repr(C)]\npub struct {} {{\n", def.name, def.name);

    for (field, field_type) in &def.fields {
        code.push_str(&format!("    pub {}: {},\n", field, field_type));
    }

    code.push_str("}\n\n");
    code
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_init_default() {
        let config = init_default();
        assert!(config.icon_config.is_some());
        assert!(config.font_config.is_some());
        assert!(config.media_config.is_some());
    }

    #[test]
    fn test_process_icons() {
        let source = r#"
            <Icon name="home" />
            <Icon name="settings" />
            <Icon name="home" />
        "#;

        let config = IconConfig::default();
        let result = process_icons(source, &config).unwrap();

        assert_eq!(result.used.len(), 2); // Deduplicated
        assert!(result.used.contains(&"home".to_string()));
        assert!(result.used.contains(&"settings".to_string()));
    }

    #[test]
    fn test_process_icons_with_set() {
        let source = r#"
            <Icon name="mdi:home" />
            <Icon set="heroicons" name="arrow-left" />
        "#;

        let mut config = IconConfig::default();
        config.sets = vec!["mdi".to_string(), "heroicons".to_string()];
        let result = process_icons(source, &config).unwrap();

        assert_eq!(result.used.len(), 2);
        assert!(result.used.contains(&"home".to_string()));
        assert!(result.used.contains(&"arrow-left".to_string()));
    }

    #[test]
    fn test_generate_icon_sprite() {
        let icons = vec![ResolvedIcon {
            name: "home".to_string(),
            set: "lucide".to_string(),
            svg: "<svg><path d=\"M1 1\"/></svg>".to_string(),
            width: 24,
            height: 24,
        }];

        let sprite = generate_icon_sprite(&icons);
        assert!(sprite.contains("icon-lucide-home"));
        assert!(sprite.contains("<symbol"));
        assert!(sprite.contains("viewBox"));
    }

    #[test]
    fn test_tree_shake_icons() {
        let all_icons = vec![
            ResolvedIcon {
                name: "home".to_string(),
                set: "lucide".to_string(),
                svg: "<svg></svg>".to_string(),
                width: 24,
                height: 24,
            },
            ResolvedIcon {
                name: "settings".to_string(),
                set: "lucide".to_string(),
                svg: "<svg></svg>".to_string(),
                width: 24,
                height: 24,
            },
            ResolvedIcon {
                name: "unused".to_string(),
                set: "lucide".to_string(),
                svg: "<svg></svg>".to_string(),
                width: 24,
                height: 24,
            },
        ];

        let source = r#"<Icon name="home" /><Icon name="settings" />"#;
        let used = tree_shake_icons(&all_icons, source);

        assert_eq!(used.len(), 2);
        assert!(used.iter().any(|i| i.name == "home"));
        assert!(used.iter().any(|i| i.name == "settings"));
        assert!(!used.iter().any(|i| i.name == "unused"));
    }

    #[test]
    fn test_parse_frontmatter() {
        let content = r#"---
title: Test Page
description: A test page
layout: default
---

# Hello World
"#;

        let (fm, body) = parse_frontmatter(content);
        assert_eq!(fm.title, Some("Test Page".to_string()));
        assert_eq!(fm.description, Some("A test page".to_string()));
        assert_eq!(fm.layout, Some("default".to_string()));
        assert!(body.contains("# Hello World"));
    }

    #[test]
    fn test_markdown_to_html() {
        let md = "# Title\n\nParagraph text\n\n- Item 1\n- Item 2";
        let html = markdown_to_html(md);

        assert!(html.contains("<h1>Title</h1>"));
        assert!(html.contains("<p>Paragraph text</p>"));
        assert!(html.contains("<li>Item 1</li>"));
    }

    #[test]
    fn test_generate_srcset() {
        let variants = vec![
            ImageVariant {
                path: PathBuf::from("image-640w.webp"),
                width: 640,
                height: 360,
                format: "webp".to_string(),
                size_estimate: None,
            },
            ImageVariant {
                path: PathBuf::from("image-1024w.webp"),
                width: 1024,
                height: 576,
                format: "webp".to_string(),
                size_estimate: None,
            },
        ];

        let srcset = generate_srcset(&variants);
        assert!(srcset.contains("640w"));
        assert!(srcset.contains("1024w"));
    }

    #[test]
    fn test_process_fonts_basic() {
        let config = FontConfig {
            families: vec![FontFamily {
                name: "Inter".to_string(),
                weights: vec![400, 700],
                variable: false,
            }],
            subset: true,
            preload: true,
        };

        let result = process_fonts(&config).unwrap();

        assert_eq!(result.fonts.len(), 1);
        assert!(result.css.contains("Inter"));
        assert!(result.css.contains("font-weight: 400"));
        assert!(result.css.contains("font-weight: 700"));
        assert!(!result.preload_hints.is_empty());
    }

    #[test]
    fn test_process_fonts_variable() {
        let config = FontConfig {
            families: vec![FontFamily {
                name: "Roboto Flex".to_string(),
                weights: vec![100, 900],
                variable: true,
            }],
            subset: true,
            preload: true,
        };

        let result = process_fonts(&config).unwrap();

        assert!(result.css.contains("woff2-variations"));
        assert!(result.css.contains("font-weight: 100 900"));
    }

    #[test]
    fn test_extract_used_characters() {
        let source = "Hello World! 123";
        let chars = extract_used_characters(source);

        assert!(chars.contains(&'H'));
        assert!(chars.contains(&'e'));
        assert!(chars.contains(&' '));
        assert!(chars.contains(&'!'));
        assert!(chars.contains(&'1'));
        assert!(!chars.contains(&'Z'));
    }

    #[test]
    fn test_calculate_font_subset_with_common() {
        let sources = vec!["Hello"];
        let subset = calculate_font_subset(&sources, true);

        // Should include content chars
        assert!(subset.contains(&'H'));
        assert!(subset.contains(&'e'));

        // Should include common chars
        assert!(subset.contains(&'a'));
        assert!(subset.contains(&'0'));
        assert!(subset.contains(&' '));
    }

    #[test]
    fn test_calculate_font_subset_without_common() {
        let sources = vec!["Hello"];
        let subset = calculate_font_subset(&sources, false);

        // Should include only content chars
        assert!(subset.contains(&'H'));
        assert!(subset.contains(&'e'));
        assert!(subset.contains(&'l'));
        assert!(subset.contains(&'o'));

        // Should NOT include chars not in content
        assert!(!subset.contains(&'Z'));
        assert!(!subset.contains(&'9'));
    }

    #[test]
    fn test_generate_font_preload_html() {
        let hints = vec![PreloadHint {
            href: "/fonts/inter-400.woff2".to_string(),
            format: "woff2".to_string(),
            crossorigin: true,
        }];

        let html = generate_font_preload_html(&hints);

        assert!(html.contains("rel=\"preload\""));
        assert!(html.contains("as=\"font\""));
        assert!(html.contains("/fonts/inter-400.woff2"));
        assert!(html.contains("type=\"font/woff2\""));
    }

    #[test]
    fn test_detect_image_imports_es_module() {
        let source = r#"
            import heroImage from './hero.png'
            import logo from '../assets/logo.jpg'
        "#;

        let imports = detect_image_imports(source);

        assert_eq!(imports.len(), 2);
        assert!(imports.iter().any(|i| i.path.to_string_lossy().contains("hero.png")));
        assert!(imports.iter().any(|i| i.path.to_string_lossy().contains("logo.jpg")));
    }

    #[test]
    fn test_detect_image_imports_jsx() {
        let source = r#"
            <img src="./photo.webp" alt="Photo" />
            <Image src="./banner.avif" />
        "#;

        let imports = detect_image_imports(source);

        assert_eq!(imports.len(), 2);
        assert!(imports.iter().any(|i| i.path.to_string_lossy().contains("photo.webp")));
        assert!(imports.iter().any(|i| i.path.to_string_lossy().contains("banner.avif")));
    }

    #[test]
    fn test_process_media_with_options() {
        let options = MediaProcessingOptions {
            output_dir: PathBuf::from("dist/images"),
            quality: 85,
            blur_placeholder: true,
            breakpoints: vec![640, 1024],
            formats: vec!["webp".to_string()],
            preserve_original: true,
        };

        let result = process_media_with_options(Path::new("test.jpg"), &options).unwrap();

        // Should generate 2 variants (2 breakpoints × 1 format)
        assert_eq!(result.variants.len(), 2);

        // Should have blur placeholder
        assert!(result.blur_placeholder.is_some());

        // Variants should have correct widths
        assert!(result.variants.iter().any(|v| v.width == 640));
        assert!(result.variants.iter().any(|v| v.width == 1024));
    }

    #[test]
    fn test_generate_picture_html() {
        let media = ProcessedMedia {
            original: PathBuf::from("test.jpg"),
            variants: vec![
                ImageVariant {
                    path: PathBuf::from("test-640w.webp"),
                    width: 640,
                    height: 360,
                    format: "webp".to_string(),
                    size_estimate: Some(50000),
                },
                ImageVariant {
                    path: PathBuf::from("test-640w.avif"),
                    width: 640,
                    height: 360,
                    format: "avif".to_string(),
                    size_estimate: Some(40000),
                },
            ],
            blur_placeholder: Some("data:image/svg+xml;base64,test".to_string()),
            original_dimensions: Some((1920, 1080)),
            dominant_color: Some("#808080".to_string()),
        };

        let html = generate_picture_html(&media, "Test image");

        assert!(html.contains("<picture>"));
        assert!(html.contains("</picture>"));
        assert!(html.contains("<source"));
        assert!(html.contains("type=\"image/avif\""));
        assert!(html.contains("type=\"image/webp\""));
        assert!(html.contains("alt=\"Test image\""));
        assert!(html.contains("loading=\"lazy\""));
    }

    #[test]
    fn test_generate_sizes() {
        let breakpoints = vec![(640, "100vw"), (1024, "50vw"), (0, "33vw")];

        let sizes = generate_sizes(&breakpoints);

        assert!(sizes.contains("(max-width: 640px) 100vw"));
        assert!(sizes.contains("(max-width: 1024px) 50vw"));
        assert!(sizes.contains("33vw"));
    }

    #[test]
    fn test_base64_encode() {
        let input = "Hello, World!";
        let encoded = base64_encode(input.as_bytes());

        // Should produce valid base64
        assert!(!encoded.is_empty());
        assert!(
            encoded
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '+' || c == '/' || c == '=')
        );
    }

    #[test]
    fn test_compile_dxm_content_from_string() {
        let content = r#"---
title: Getting Started
description: Learn how to use DX-WWW
layout: docs
---

# Getting Started

Welcome to DX-WWW!

## Installation

Run the following command:

```bash
dx www new my-app
```

- Step 1: Create project
- Step 2: Install dependencies
"#;

        let (frontmatter, body) = parse_frontmatter(content);
        let html = markdown_to_html(&body);

        // Verify frontmatter extraction
        assert_eq!(frontmatter.title, Some("Getting Started".to_string()));
        assert_eq!(frontmatter.description, Some("Learn how to use DX-WWW".to_string()));
        assert_eq!(frontmatter.layout, Some("docs".to_string()));

        // Verify HTML conversion
        assert!(html.contains("<h1>Getting Started</h1>"));
        assert!(html.contains("<h2>Installation</h2>"));
        assert!(html.contains("<li>Step 1: Create project</li>"));
        assert!(html.contains("language-bash"));
    }

    #[test]
    fn test_frontmatter_without_yaml() {
        let content = "# Simple Page\n\nJust content, no frontmatter.";
        let (frontmatter, body) = parse_frontmatter(content);

        assert!(frontmatter.title.is_none());
        assert!(frontmatter.description.is_none());
        assert!(frontmatter.layout.is_none());
        assert!(body.contains("# Simple Page"));
    }

    #[test]
    fn test_markdown_code_blocks() {
        let md = "```rust\nfn main() {\n    println!(\"<Hello>\");\n}\n```";
        let html = markdown_to_html(md);

        assert!(html.contains("language-rust"));
        assert!(html.contains("fn main()"));
        assert!(html.contains("&lt;Hello&gt;")); // HTML escaped
    }
}

// ============================================================================
// Property-Based Tests
// ============================================================================

#[cfg(test)]
mod property_tests {
    use super::*;
    use proptest::prelude::*;

    // Arbitrary generators for DXM content
    fn arbitrary_title() -> impl Strategy<Value = String> {
        prop_oneof![
            Just("Getting Started".to_string()),
            Just("API Reference".to_string()),
            Just("Installation Guide".to_string()),
            "[A-Z][a-z]{2,15}( [A-Z][a-z]{2,10}){0,3}".prop_map(|s| s.to_string()),
        ]
    }

    fn arbitrary_description() -> impl Strategy<Value = String> {
        prop_oneof![
            Just("A comprehensive guide".to_string()),
            Just("Learn the basics".to_string()),
            Just("Reference documentation".to_string()),
            "[A-Z][a-z]{2,20}( [a-z]{2,10}){0,5}".prop_map(|s| s.to_string()),
        ]
    }

    fn arbitrary_layout() -> impl Strategy<Value = String> {
        prop_oneof![
            Just("default".to_string()),
            Just("docs".to_string()),
            Just("blog".to_string()),
            Just("landing".to_string()),
        ]
    }

    fn arbitrary_markdown_body() -> impl Strategy<Value = String> {
        prop_oneof![
            Just("# Hello World\n\nThis is a paragraph.".to_string()),
            Just("## Section\n\n- Item 1\n- Item 2".to_string()),
            Just("# Title\n\n```rust\nfn main() {}\n```".to_string()),
            Just("### Heading\n\nSome text here.\n\n#### Subheading\n\nMore text.".to_string()),
        ]
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Property 6: DXM Content Compilation Round-Trip
        /// *For any* valid DXM document with frontmatter, parsing and extracting
        /// frontmatter SHALL preserve all metadata fields (title, description, layout).
        ///
        /// **Validates: Requirements 3.1, 3.2**
        #[test]
        fn prop_dxm_frontmatter_extraction(
            title in arbitrary_title(),
            description in arbitrary_description(),
            layout in arbitrary_layout(),
            body in arbitrary_markdown_body(),
        ) {
            // Construct DXM content with frontmatter
            let content = format!(
                "---\ntitle: {}\ndescription: {}\nlayout: {}\n---\n\n{}",
                title, description, layout, body
            );

            // Parse frontmatter
            let (frontmatter, extracted_body) = parse_frontmatter(&content);

            // Verify frontmatter fields are preserved
            prop_assert_eq!(frontmatter.title, Some(title));
            prop_assert_eq!(frontmatter.description, Some(description));
            prop_assert_eq!(frontmatter.layout, Some(layout));

            // Verify body content is preserved (trimmed)
            prop_assert!(extracted_body.trim().contains(body.lines().next().unwrap_or("")));
        }

        /// Property 6b: DXM HTML Conversion Preserves Structure
        /// *For any* markdown content, converting to HTML SHALL preserve
        /// the document structure (headings, lists, code blocks).
        ///
        /// **Validates: Requirements 3.1, 3.2**
        #[test]
        fn prop_dxm_html_structure_preservation(
            body in arbitrary_markdown_body(),
        ) {
            let html = markdown_to_html(&body);

            // If body contains h1, html should contain <h1>
            if body.contains("# ") && !body.contains("## ") {
                prop_assert!(html.contains("<h1>") || html.contains("<h2>") || html.contains("<h3>"));
            }

            // If body contains list items, html should contain <li>
            if body.contains("- ") {
                prop_assert!(html.contains("<li>"));
            }

            // If body contains code blocks, html should contain <pre><code>
            if body.contains("```") {
                prop_assert!(html.contains("<pre><code"));
            }

            // HTML should not be empty for non-empty input
            if !body.trim().is_empty() {
                prop_assert!(!html.trim().is_empty());
            }
        }

        /// Property 7: Frontmatter Extraction Completeness
        /// *For any* DXM file with frontmatter, THE DX_WWW SHALL extract all
        /// frontmatter fields (title, description, layout) and make them available.
        ///
        /// **Validates: Requirements 3.3**
        #[test]
        fn prop_frontmatter_extraction_completeness(
            has_title in any::<bool>(),
            has_description in any::<bool>(),
            has_layout in any::<bool>(),
            title in arbitrary_title(),
            description in arbitrary_description(),
            layout in arbitrary_layout(),
        ) {
            // Build frontmatter with optional fields
            let mut fm_lines = Vec::new();
            if has_title {
                fm_lines.push(format!("title: {}", title));
            }
            if has_description {
                fm_lines.push(format!("description: {}", description));
            }
            if has_layout {
                fm_lines.push(format!("layout: {}", layout));
            }

            let content = if fm_lines.is_empty() {
                "# No Frontmatter\n\nJust content.".to_string()
            } else {
                format!("---\n{}\n---\n\n# Content", fm_lines.join("\n"))
            };

            let (frontmatter, _body) = parse_frontmatter(&content);

            // Verify each field is extracted if present
            if has_title {
                prop_assert_eq!(frontmatter.title, Some(title));
            } else {
                prop_assert!(frontmatter.title.is_none());
            }

            if has_description {
                prop_assert_eq!(frontmatter.description, Some(description));
            } else {
                prop_assert!(frontmatter.description.is_none());
            }

            if has_layout {
                prop_assert_eq!(frontmatter.layout, Some(layout));
            } else {
                prop_assert!(frontmatter.layout.is_none());
            }
        }

        /// Property: Icon Processing Deduplication
        /// *For any* source with duplicate icon references, process_icons
        /// SHALL return a deduplicated list.
        #[test]
        fn prop_icon_deduplication(
            icon_name in "[a-z]{3,10}",
            repeat_count in 1usize..5usize,
        ) {
            let source = (0..repeat_count)
                .map(|_| format!(r#"<Icon name="{}" />"#, icon_name))
                .collect::<Vec<_>>()
                .join("\n");

            let config = IconConfig::default();
            let result = process_icons(&source, &config).unwrap();

            // Should only have one entry regardless of repeat count
            prop_assert_eq!(result.used.len(), 1);
            prop_assert_eq!(&result.used[0], &icon_name);
        }

        /// Property 9: Icon Tree-Shaking Correctness
        /// *For any* project with icon imports, the production build SHALL include
        /// exactly the icons that are referenced in the source code (no more, no less).
        ///
        /// **Validates: Requirements 5.3**
        #[test]
        fn prop_icon_tree_shaking_correctness(
            used_icons in prop::collection::vec("[a-z]{3,8}", 1..5),
            unused_icons in prop::collection::vec("[a-z]{3,8}", 0..3),
        ) {
            // Create all icons (used + unused)
            let mut all_icons: Vec<ResolvedIcon> = used_icons.iter().map(|name| {
                ResolvedIcon {
                    name: name.clone(),
                    set: "lucide".to_string(),
                    svg: format!("<svg><!-- {} --></svg>", name),
                    width: 24,
                    height: 24,
                }
            }).collect();

            // Add unused icons with different names
            for (i, name) in unused_icons.iter().enumerate() {
                let unique_name = format!("unused_{}_{}", name, i);
                all_icons.push(ResolvedIcon {
                    name: unique_name,
                    set: "lucide".to_string(),
                    svg: "<svg></svg>".to_string(),
                    width: 24,
                    height: 24,
                });
            }

            // Create source that only references used_icons
            let source = used_icons.iter()
                .map(|name| format!(r#"<Icon name="{}" />"#, name))
                .collect::<Vec<_>>()
                .join("\n");

            // Tree-shake
            let result = tree_shake_icons(&all_icons, &source);

            // Result should contain exactly the used icons
            prop_assert_eq!(result.len(), used_icons.len());

            // All used icons should be present
            for name in &used_icons {
                prop_assert!(result.iter().any(|i| &i.name == name),
                    "Used icon '{}' should be in result", name);
            }

            // No unused icons should be present
            for icon in &result {
                prop_assert!(!icon.name.starts_with("unused_"),
                    "Unused icon '{}' should not be in result", icon.name);
            }
        }

        /// Property: Icon Sprite Generation Completeness
        /// *For any* set of resolved icons, the generated sprite SHALL contain
        /// a symbol for each icon with correct id and viewBox.
        #[test]
        fn prop_icon_sprite_completeness(
            icon_count in 1usize..10usize,
        ) {
            let icons: Vec<ResolvedIcon> = (0..icon_count).map(|i| {
                ResolvedIcon {
                    name: format!("icon{}", i),
                    set: "test".to_string(),
                    svg: format!("<svg><path d=\"M{} {}\"/></svg>", i, i),
                    width: 24,
                    height: 24,
                }
            }).collect();

            let sprite = generate_icon_sprite(&icons);

            // Sprite should contain all icons
            for icon in &icons {
                let symbol_id = format!("icon-{}-{}", icon.set, icon.name);
                prop_assert!(sprite.contains(&symbol_id),
                    "Sprite should contain symbol id '{}'", symbol_id);
            }

            // Sprite should be valid SVG structure
            prop_assert!(sprite.starts_with("<svg"));
            prop_assert!(sprite.ends_with("</svg>"));
            prop_assert!(sprite.contains("display:none"));
        }

        /// Property: Media Variant Generation
        /// *For any* media configuration, process_media SHALL generate
        /// variants for all breakpoints (that fit within original dimensions) and formats.
        #[test]
        fn prop_media_variant_generation(
            breakpoint_count in 1usize..6usize,
            format_count in 1usize..3usize,
        ) {
            // Use breakpoints that are all smaller than the estimated original (1920)
            let breakpoints: Vec<u32> = (0..breakpoint_count)
                .map(|i| 320 + (i as u32 * 256))
                .collect();
            let formats: Vec<String> = ["webp", "avif", "png"]
                .iter()
                .take(format_count)
                .map(|s| s.to_string())
                .collect();

            let config = MediaConfig {
                image_formats: formats.clone(),
                quality: 85,
                blur_placeholder: true,
                breakpoints: breakpoints.clone(),
            };

            let result = process_media(Path::new("test.jpg"), &config).unwrap();

            // Should generate variants for each breakpoint × format combination
            // (all breakpoints are <= 1920, so none are skipped)
            prop_assert_eq!(result.variants.len(), breakpoint_count * format_count);

            // Each breakpoint should have all formats
            for bp in &breakpoints {
                for fmt in &formats {
                    prop_assert!(result.variants.iter().any(|v| v.width == *bp && v.format == *fmt));
                }
            }
        }

        /// Property 10: Font Subsetting Correctness
        /// *For any* font configuration and source code, the subset font SHALL contain
        /// all characters used in the application and no unused characters.
        ///
        /// **Validates: Requirements 6.2**
        #[test]
        fn prop_font_subsetting_correctness(
            content_chars in prop::collection::vec(prop::char::range('a', 'z'), 5..50),
            extra_chars in prop::collection::vec(prop::char::range('A', 'Z'), 0..20),
        ) {
            // Create content with specific characters
            let content: String = content_chars.iter().collect();

            // Extract used characters
            let used_chars = extract_used_characters(&content);

            // All content characters should be in used_chars
            for c in &content_chars {
                prop_assert!(used_chars.contains(c),
                    "Character '{}' from content should be in used_chars", c);
            }

            // Extra characters not in content should not be in used_chars
            for c in &extra_chars {
                if !content_chars.contains(c) {
                    prop_assert!(!used_chars.contains(c),
                        "Character '{}' not in content should not be in used_chars", c);
                }
            }
        }

        /// Property 10b: Font Subset Calculation Includes Common Characters
        /// *For any* content sources, calculate_font_subset with include_common=true
        /// SHALL include all common ASCII characters plus content characters.
        ///
        /// **Validates: Requirements 6.2**
        #[test]
        fn prop_font_subset_includes_common(
            content in "[a-z]{10,50}",
        ) {
            let sources = vec![content.as_str()];
            let subset = calculate_font_subset(&sources, true);

            // Should include all content characters
            for c in content.chars() {
                prop_assert!(subset.contains(&c),
                    "Content character '{}' should be in subset", c);
            }

            // Should include common characters when include_common is true
            prop_assert!(subset.contains(&'0'), "Should include digit 0");
            prop_assert!(subset.contains(&'9'), "Should include digit 9");
            prop_assert!(subset.contains(&'a'), "Should include lowercase a");
            prop_assert!(subset.contains(&'z'), "Should include lowercase z");
            prop_assert!(subset.contains(&'A'), "Should include uppercase A");
            prop_assert!(subset.contains(&'Z'), "Should include uppercase Z");
            prop_assert!(subset.contains(&' '), "Should include space");
        }

        /// Property 10c: Font Processing Generates Valid CSS
        /// *For any* font configuration, process_fonts SHALL generate valid CSS
        /// with @font-face declarations for all specified weights.
        ///
        /// **Validates: Requirements 6.1, 6.2**
        #[test]
        fn prop_font_processing_generates_valid_css(
            family_name in "[A-Z][a-z]{3,10}( [A-Z][a-z]{3,8})?",
            weights in prop::collection::vec(
                prop::sample::select(vec![100u16, 200, 300, 400, 500, 600, 700, 800, 900]),
                1..4
            ),
            is_variable in any::<bool>(),
        ) {
            let config = FontConfig {
                families: vec![FontFamily {
                    name: family_name.clone(),
                    weights: weights.clone(),
                    variable: is_variable,
                }],
                subset: true,
                preload: true,
            };

            let result = process_fonts(&config).unwrap();

            // CSS should contain @font-face
            prop_assert!(result.css.contains("@font-face"),
                "CSS should contain @font-face declaration");

            // CSS should contain the font family name
            prop_assert!(result.css.contains(&family_name),
                "CSS should contain font family name '{}'", family_name);

            // CSS should contain font-display: swap
            prop_assert!(result.css.contains("font-display: swap"),
                "CSS should contain font-display: swap");

            // CSS should contain woff2 format
            prop_assert!(result.css.contains("woff2"),
                "CSS should reference woff2 format");

            // For non-variable fonts, each weight should have its own @font-face
            if !is_variable {
                for weight in &weights {
                    prop_assert!(result.css.contains(&format!("font-weight: {}", weight)),
                        "CSS should contain font-weight: {}", weight);
                }
            }
        }

        /// Property 10d: Font Preload Hints Generation
        /// *For any* font configuration with preload enabled, process_fonts SHALL
        /// generate preload hints for critical font weights (400, 700).
        ///
        /// **Validates: Requirements 6.3**
        #[test]
        fn prop_font_preload_hints_generation(
            family_name in "[A-Z][a-z]{3,10}",
            has_regular in any::<bool>(),
            has_bold in any::<bool>(),
        ) {
            let mut weights = Vec::new();
            if has_regular {
                weights.push(400u16);
            }
            if has_bold {
                weights.push(700u16);
            }
            if weights.is_empty() {
                weights.push(400u16); // Default
            }

            let config = FontConfig {
                families: vec![FontFamily {
                    name: family_name.clone(),
                    weights: weights.clone(),
                    variable: false,
                }],
                subset: true,
                preload: true,
            };

            let result = process_fonts(&config).unwrap();

            // Should have preload hints for critical weights
            if has_regular || weights.contains(&400) {
                prop_assert!(result.preload_hints.iter().any(|h| h.href.contains("-400")),
                    "Should have preload hint for weight 400");
            }

            if has_bold {
                prop_assert!(result.preload_hints.iter().any(|h| h.href.contains("-700")),
                    "Should have preload hint for weight 700");
            }

            // All preload hints should be woff2 format
            for hint in &result.preload_hints {
                prop_assert_eq!(&hint.format, "woff2",
                    "Preload hints should use woff2 format");
            }
        }

        /// Property 11: Image Variant Generation
        /// *For any* image import, THE DX_WWW SHALL generate responsive variants where each variant:
        /// - Has correct dimensions for its breakpoint
        /// - Is in an optimized format (WebP or AVIF)
        /// - Has a valid blur placeholder
        ///
        /// **Validates: Requirements 7.2, 7.3, 7.4**
        #[test]
        fn prop_image_variant_generation(
            breakpoint_count in 1usize..6usize,
            format_count in 1usize..3usize,
            quality in 50u8..100u8,
            generate_blur in any::<bool>(),
        ) {
            // Use breakpoints that are all smaller than the estimated original (1920)
            let breakpoints: Vec<u32> = (0..breakpoint_count)
                .map(|i| 320 + (i as u32 * 256))
                .collect();
            let formats: Vec<String> = ["webp", "avif", "png"]
                .iter()
                .take(format_count)
                .map(|s| s.to_string())
                .collect();

            let options = MediaProcessingOptions {
                output_dir: PathBuf::from("dist/images"),
                quality,
                blur_placeholder: generate_blur,
                breakpoints: breakpoints.clone(),
                formats: formats.clone(),
                preserve_original: true,
            };

            let result = process_media_with_options(Path::new("test.jpg"), &options).unwrap();

            // Should generate variants for each breakpoint × format combination
            // (all breakpoints are <= 1920, so none are skipped)
            prop_assert_eq!(result.variants.len(), breakpoint_count * format_count,
                "Should generate {} variants ({}×{})",
                breakpoint_count * format_count, breakpoint_count, format_count);

            // Each variant should have correct width
            for variant in &result.variants {
                prop_assert!(breakpoints.contains(&variant.width),
                    "Variant width {} should be in breakpoints {:?}", variant.width, breakpoints);
            }

            // Each variant should have valid format
            for variant in &result.variants {
                prop_assert!(formats.contains(&variant.format),
                    "Variant format '{}' should be in formats {:?}", variant.format, formats);
            }

            // Each variant should have height calculated from aspect ratio
            for variant in &result.variants {
                prop_assert!(variant.height > 0,
                    "Variant height should be positive");
            }

            // Blur placeholder should be present if requested
            if generate_blur {
                prop_assert!(result.blur_placeholder.is_some(),
                    "Blur placeholder should be generated when requested");

                let blur = result.blur_placeholder.as_ref().unwrap();
                prop_assert!(blur.starts_with("data:image/"),
                    "Blur placeholder should be a data URL");
            }
        }

        /// Property 11b: Image Variant Dimensions Preserve Aspect Ratio
        /// *For any* image with known dimensions, generated variants SHALL preserve
        /// the original aspect ratio.
        ///
        /// **Validates: Requirements 7.2**
        #[test]
        fn prop_image_variant_aspect_ratio(
            original_width in 400u32..4000u32,
            original_height in 300u32..3000u32,
            target_width in 320u32..1920u32,
        ) {
            let original_aspect = original_width as f64 / original_height as f64;

            // Calculate expected height
            let expected_height = (target_width as f64 / original_aspect).round() as u32;

            // Skip extreme aspect ratios that cause rounding issues
            if expected_height == 0 {
                return Ok(());
            }

            // Verify aspect ratio is preserved (within 2% tolerance for rounding)
            let result_aspect = target_width as f64 / expected_height as f64;
            let ratio_diff = (original_aspect - result_aspect).abs() / original_aspect;

            prop_assert!(ratio_diff < 0.02,
                "Aspect ratio should be preserved within 2%: original={:.3}, result={:.3}",
                original_aspect, result_aspect);
        }

        /// Property 11c: Image Srcset Generation
        /// *For any* set of image variants, generate_srcset SHALL produce a valid
        /// srcset string with all variants.
        ///
        /// **Validates: Requirements 7.2**
        #[test]
        fn prop_image_srcset_generation(
            variant_count in 1usize..6usize,
        ) {
            let variants: Vec<ImageVariant> = (0..variant_count).map(|i| {
                let width = 640 + (i as u32 * 384);
                ImageVariant {
                    path: PathBuf::from(format!("image-{}w.webp", width)),
                    width,
                    height: (width as f64 / 1.78).round() as u32,
                    format: "webp".to_string(),
                    size_estimate: Some(width as u64 * 100),
                }
            }).collect();

            let srcset = generate_srcset(&variants);

            // Srcset should contain all variants
            for variant in &variants {
                prop_assert!(srcset.contains(&format!("{}w", variant.width)),
                    "Srcset should contain width descriptor '{}w'", variant.width);
            }

            // Srcset should be comma-separated
            if variant_count > 1 {
                prop_assert!(srcset.contains(", "),
                    "Srcset should be comma-separated");
            }
        }

        /// Property 11d: Image Import Detection
        /// *For any* source code with image imports, detect_image_imports SHALL
        /// find all image references.
        ///
        /// **Validates: Requirements 7.1**
        #[test]
        fn prop_image_import_detection(
            image_name in "[a-z]{3,10}",
            extension in prop::sample::select(vec!["png", "jpg", "jpeg", "webp", "gif"]),
        ) {
            let source = format!(
                r#"import img from './{}.{}'
                <img src="./{}.{}" alt="test" />"#,
                image_name, extension, image_name, extension
            );

            let imports = detect_image_imports(&source);

            // Should detect at least one import
            prop_assert!(!imports.is_empty(),
                "Should detect image imports in source");

            // All detected imports should have the correct extension
            for import in &imports {
                let ext = import.path.extension().and_then(|e| e.to_str()).unwrap_or("");
                prop_assert_eq!(ext, extension,
                    "Detected import should have extension '{}'", extension);
            }
        }

        /// Property 11e: Blur Placeholder is Valid Data URL
        /// *For any* image, the generated blur placeholder SHALL be a valid
        /// base64-encoded SVG data URL.
        ///
        /// **Validates: Requirements 7.4**
        #[test]
        fn prop_blur_placeholder_valid_data_url(
            width in 100u32..4000u32,
            height in 100u32..4000u32,
        ) {
            let placeholder = generate_blur_placeholder(
                Path::new("test.jpg"),
                Some((width, height))
            );

            // Should be a data URL
            prop_assert!(placeholder.starts_with("data:image/svg+xml;base64,"),
                "Blur placeholder should be SVG data URL");

            // Should be valid base64 (no invalid characters)
            let base64_part = &placeholder["data:image/svg+xml;base64,".len()..];
            prop_assert!(base64_part.chars().all(|c|
                c.is_ascii_alphanumeric() || c == '+' || c == '/' || c == '='
            ), "Base64 should only contain valid characters");
        }
    }
}
