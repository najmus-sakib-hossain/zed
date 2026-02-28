#![allow(dead_code)]

use anyhow::Result;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use super::data::*;

/// Loads icons from all three data sources:
/// 1. apps/www/public/icons (Iconify JSON packs)
/// 2. apps/www/public/svgl (SVG files)  
/// 3. crates/icon/data (Iconify JSON packs)
pub struct IconDataLoader {
    /// Root path of the DX project
    project_root: PathBuf,
    /// All loaded icons
    icons: Vec<LoadedIcon>,
    /// Pack info summaries
    packs: Vec<IconPackInfo>,
    /// Index: pack name -> list of icon indices
    pack_index: HashMap<String, Vec<usize>>,
}

impl IconDataLoader {
    /// Create a new loader. `project_root` should be the DX monorepo root (e.g. f:\Dx)
    pub fn new(project_root: impl Into<PathBuf>) -> Self {
        Self {
            project_root: project_root.into(),
            icons: Vec::new(),
            packs: Vec::new(),
            pack_index: HashMap::new(),
        }
    }

    /// Load all icon data from every source
    pub fn load_all(&mut self) -> Result<()> {
        // Load from www/icons (Iconify JSON packs - monochrome, theme-friendly)
        let www_icons_dir = self.project_root.join("apps/www/public/icons");
        if www_icons_dir.exists() {
            self.load_iconify_dir(&www_icons_dir, IconSource::WwwIcons)?;
        }

        // Limit to 5000 icons max to prevent lag (will show paginated)
        self.icons.truncate(5000);

        // Build pack index
        self.build_pack_index();

        Ok(())
    }

    /// Load all Iconify JSON pack files from a directory
    fn load_iconify_dir(&mut self, dir: &Path, source: IconSource) -> Result<()> {
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) != Some("json") {
                continue;
            }

            match self.load_iconify_pack(&path, source.clone()) {
                Ok(_count) => {
                    // Pack loaded successfully
                }
                Err(e) => {
                    // Skip malformed packs silently
                    let _ = e;
                }
            }
        }

        Ok(())
    }

    /// Load a single Iconify JSON pack and add icons to the list
    fn load_iconify_pack(&mut self, path: &Path, source: IconSource) -> Result<usize> {
        let content = std::fs::read_to_string(path)?;
        let pack: IconifyPack = serde_json::from_str(&content)?;

        let default_width = pack.width.unwrap_or(24.0);
        let default_height = pack.height.unwrap_or(24.0);
        let pack_name = pack.prefix.clone();
        let display_name =
            pack.info.as_ref().map(|i| i.name.clone()).unwrap_or_else(|| pack_name.clone());

        let count = pack.icons.len();

        // Add pack info
        self.packs.push(IconPackInfo {
            prefix: pack_name.clone(),
            name: display_name,
            total: count as u32,
            source: source.clone(),
        });

        // Add icons
        for (icon_name, icon_data) in pack.icons {
            let width = icon_data.width.unwrap_or(default_width);
            let height = icon_data.height.unwrap_or(default_height);

            self.icons.push(LoadedIcon {
                id: format!("{}:{}", pack_name, icon_name),
                name: icon_name,
                pack: pack_name.clone(),
                source: source.clone(),
                svg_body: icon_data.body,
                width,
                height,
            });
        }

        Ok(count)
    }

    /// Load SVG files from the SVGL directory
    fn load_svgl_dir(&mut self, dir: &Path) -> Result<()> {
        let mut count = 0;

        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) != Some("svg") {
                continue;
            }

            let file_stem =
                path.file_stem().and_then(|s| s.to_str()).unwrap_or("unknown").to_string();

            let svg_content = match std::fs::read_to_string(&path) {
                Ok(c) => c,
                Err(_) => continue,
            };

            // Extract viewBox dimensions or use defaults
            let (width, height) = extract_svg_dimensions(&svg_content).unwrap_or((24.0, 24.0));

            // Extract inner body (content inside <svg>...</svg>)
            let body = extract_svg_body(&svg_content).unwrap_or_else(|| svg_content.clone());

            self.icons.push(LoadedIcon {
                id: format!("svgl:{}", file_stem),
                name: file_stem,
                pack: "svgl".to_string(),
                source: IconSource::WwwSvgl,
                svg_body: body,
                width,
                height,
            });

            count += 1;
        }

        self.packs.push(IconPackInfo {
            prefix: "svgl".to_string(),
            name: "SVGL Brand Icons".to_string(),
            total: count as u32,
            source: IconSource::WwwSvgl,
        });

        Ok(())
    }

    /// Build pack -> icon index for fast pack filtering
    fn build_pack_index(&mut self) {
        self.pack_index.clear();
        for (idx, icon) in self.icons.iter().enumerate() {
            self.pack_index.entry(icon.pack.clone()).or_default().push(idx);
        }
    }

    // -- Accessors --

    /// Get all loaded icons
    pub fn icons(&self) -> &[LoadedIcon] {
        &self.icons
    }

    /// Get loaded pack summaries
    pub fn packs(&self) -> &[IconPackInfo] {
        &self.packs
    }

    /// Get icons for a specific pack
    pub fn icons_for_pack(&self, pack: &str) -> Vec<&LoadedIcon> {
        self.pack_index
            .get(pack)
            .map(|indices| indices.iter().map(|&i| &self.icons[i]).collect())
            .unwrap_or_default()
    }

    /// Simple search by name substring (case-insensitive)
    pub fn search(&self, query: &str, limit: usize) -> Vec<&LoadedIcon> {
        if query.is_empty() {
            return self.icons.iter().take(limit).collect();
        }

        let query_lower = query.to_lowercase();
        let mut results: Vec<(&LoadedIcon, f32)> = Vec::new();

        for icon in &self.icons {
            let name_lower = icon.name.to_lowercase();

            if name_lower == query_lower {
                results.push((icon, 100.0));
            } else if name_lower.starts_with(&query_lower) {
                results.push((icon, 80.0));
            } else if name_lower.contains(&query_lower) {
                results.push((icon, 60.0));
            } else if fuzzy_contains(&name_lower, &query_lower) {
                results.push((icon, 30.0));
            }
        }

        results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        results.into_iter().take(limit).map(|(icon, _)| icon).collect()
    }

    /// Get total icon count
    pub fn total_icons(&self) -> usize {
        self.icons.len()
    }

    /// Get all unique pack names
    pub fn pack_names(&self) -> Vec<String> {
        let mut names: Vec<String> = self.pack_index.keys().cloned().collect();
        names.sort();
        names
    }
}

/// Simple fuzzy match: check if all chars of query appear in order in target
fn fuzzy_contains(target: &str, query: &str) -> bool {
    let mut target_chars = target.chars();
    for qc in query.chars() {
        loop {
            match target_chars.next() {
                Some(tc) if tc == qc => break,
                Some(_) => continue,
                None => return false,
            }
        }
    }
    true
}

/// Extract width/height from SVG viewBox or width/height attributes
fn extract_svg_dimensions(svg: &str) -> Option<(f32, f32)> {
    // Try viewBox first: viewBox="0 0 W H"
    if let Some(vb_start) = svg.find("viewBox=\"") {
        let vb_str = &svg[vb_start + 9..];
        if let Some(vb_end) = vb_str.find('"') {
            let parts: Vec<&str> = vb_str[..vb_end].split_whitespace().collect();
            if parts.len() == 4 {
                let w: f32 = parts[2].parse().ok()?;
                let h: f32 = parts[3].parse().ok()?;
                return Some((w, h));
            }
        }
    }

    // Try width/height attributes
    let w = extract_attr(svg, "width")?;
    let h = extract_attr(svg, "height")?;
    Some((w, h))
}

fn extract_attr(svg: &str, attr: &str) -> Option<f32> {
    let needle = format!("{}=\"", attr);
    let start = svg.find(&needle)? + needle.len();
    let rest = &svg[start..];
    let end = rest.find('"')?;
    rest[..end].trim_end_matches("px").parse().ok()
}

/// Extract the inner body content from a full SVG string
fn extract_svg_body(svg: &str) -> Option<String> {
    // Find the end of the opening <svg ...> tag
    let svg_open_end = svg.find('>')? + 1;
    // Find the closing </svg> tag
    let svg_close_start = svg.rfind("</svg>")?;
    if svg_open_end < svg_close_start {
        Some(svg[svg_open_end..svg_close_start].trim().to_string())
    } else {
        None
    }
}
