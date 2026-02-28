use gpui::{AssetSource, Result, SharedString};
use rust_embed::RustEmbed;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[derive(RustEmbed)]
#[folder = "assets"]
pub struct Assets;

impl AssetSource for Assets {
    fn load(&self, path: &str) -> Result<Option<std::borrow::Cow<'static, [u8]>>> {
        if let Some(file) = Assets::get(path) {
            Ok(Some(file.data))
        } else {
            Ok(None)
        }
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        Ok(Assets::iter()
            .filter(|p| p.starts_with(path))
            .map(|p| SharedString::from(p.to_string()))
            .collect())
    }
}

// Dynamic SVG AssetSource for JSON-based icons
#[derive(Clone)]
pub struct DynamicSvgAssets {
    svgs: Arc<Mutex<HashMap<String, Vec<u8>>>>,
}

impl DynamicSvgAssets {
    pub fn new() -> Self {
        Self {
            svgs: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn register_svg(&self, path: String, svg_body: &str, width: f32, height: f32) {
        let full_svg = format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {} {}" width="{}" height="{}">{}</svg>"#,
            width, height, width, height, svg_body
        );
        let mut svgs = self.svgs.lock().unwrap();
        svgs.insert(path, full_svg.into_bytes());
    }
}

impl AssetSource for DynamicSvgAssets {
    fn load(&self, path: &str) -> Result<Option<std::borrow::Cow<'static, [u8]>>> {
        let svgs = self.svgs.lock().unwrap();
        Ok(svgs.get(path).map(|bytes| bytes.clone().into()))
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        let svgs = self.svgs.lock().unwrap();
        Ok(svgs
            .keys()
            .filter(|p| p.starts_with(path))
            .map(|p| SharedString::from(p.clone()))
            .collect())
    }
}

pub struct AppAssets {
    embedded: Assets,
    dynamic: DynamicSvgAssets,
}

impl AppAssets {
    pub fn new(dynamic: DynamicSvgAssets) -> Self {
        Self {
            embedded: Assets,
            dynamic,
        }
    }
}

impl AssetSource for AppAssets {
    fn load(&self, path: &str) -> Result<Option<std::borrow::Cow<'static, [u8]>>> {
        if let Some(bytes) = self.dynamic.load(path)? {
            return Ok(Some(bytes));
        }
        self.embedded.load(path)
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        let mut items = self.embedded.list(path)?;
        items.extend(self.dynamic.list(path)?);
        Ok(items)
    }
}

// Alternative: Directory-based AssetSource for development
#[allow(dead_code)]
pub struct DirectoryAssetSource {
    root: std::path::PathBuf,
}

#[allow(dead_code)]
impl DirectoryAssetSource {
    pub fn new(root: std::path::PathBuf) -> Self {
        Self { root }
    }
}

impl AssetSource for DirectoryAssetSource {
    fn load(&self, path: &str) -> Result<Option<std::borrow::Cow<'static, [u8]>>> {
        let full_path = self.root.join(path);
        match std::fs::read(full_path) {
            Ok(bytes) => Ok(Some(bytes.into())),
            Err(_) => Ok(None),
        }
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        let full_path = self.root.join(path);
        if let Ok(entries) = std::fs::read_dir(full_path) {
            Ok(entries
                .filter_map(|e| e.ok())
                .filter_map(|e| e.file_name().to_str().map(|s| SharedString::from(s.to_string())))
                .collect())
        } else {
            Ok(Vec::new())
        }
    }
}
