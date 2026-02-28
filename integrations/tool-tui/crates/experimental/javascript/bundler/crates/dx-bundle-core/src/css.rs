//! CSS bundling support
//!
//! This module provides CSS bundling functionality including:
//! - Detecting CSS imports in JavaScript
//! - Bundling CSS files together
//! - CSS modules support (scoped class names)
//! - Asset URL rewriting

use crate::error::BundleResult;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

/// CSS import information
#[derive(Debug, Clone)]
pub struct CssImport {
    /// Path to the CSS file
    pub path: PathBuf,
    /// Original import specifier
    pub specifier: String,
    /// Whether this is a CSS module import
    pub is_module: bool,
    /// Start position in source
    pub start: u32,
    /// End position in source
    pub end: u32,
}

/// CSS module export (class name mapping)
#[derive(Debug, Clone)]
pub struct CssModuleExport {
    /// Original class name
    pub original: String,
    /// Scoped class name
    pub scoped: String,
}

/// Bundled CSS output
#[derive(Debug, Clone)]
pub struct CssBundleOutput {
    /// Concatenated CSS content
    pub css: String,
    /// CSS module exports (file -> class mappings)
    pub module_exports: HashMap<PathBuf, Vec<CssModuleExport>>,
    /// Asset URLs that need to be copied
    pub assets: Vec<AssetReference>,
}

/// Reference to an asset (image, font, etc.)
#[derive(Debug, Clone)]
pub struct AssetReference {
    /// Original URL in CSS
    pub original_url: String,
    /// Resolved file path
    pub file_path: PathBuf,
    /// New URL in output
    pub output_url: String,
}

/// CSS bundler
pub struct CssBundler {
    /// CSS files to bundle
    css_files: Vec<CssFileInfo>,
    /// Base directory for resolving paths
    _base_dir: PathBuf,
    /// Output directory for assets
    _output_dir: PathBuf,
    /// Whether to enable CSS modules
    css_modules: bool,
    /// Hash for scoping class names
    _scope_hash: String,
}

/// Information about a CSS file
#[derive(Debug, Clone)]
struct CssFileInfo {
    path: PathBuf,
    content: String,
    is_module: bool,
}

impl CssBundler {
    /// Create a new CSS bundler
    pub fn new(base_dir: PathBuf, output_dir: PathBuf) -> Self {
        Self {
            css_files: Vec::new(),
            _base_dir: base_dir,
            _output_dir: output_dir,
            css_modules: true,
            _scope_hash: generate_scope_hash(),
        }
    }

    /// Enable or disable CSS modules
    pub fn with_css_modules(mut self, enabled: bool) -> Self {
        self.css_modules = enabled;
        self
    }

    /// Add a CSS file to the bundle
    pub fn add_css_file(&mut self, path: &Path, content: &str, is_module: bool) {
        self.css_files.push(CssFileInfo {
            path: path.to_path_buf(),
            content: content.to_string(),
            is_module,
        });
    }

    /// Detect CSS imports in JavaScript source
    pub fn detect_css_imports(source: &str, file_path: &Path) -> Vec<CssImport> {
        let mut imports = Vec::new();

        // Pattern for import statements
        let import_patterns = [
            r#"import\s+['"]([^'"]+\.css)['"]\s*;"#,
            r#"import\s+\w+\s+from\s+['"]([^'"]+\.css)['"]\s*;"#,
            r#"import\s+\*\s+as\s+\w+\s+from\s+['"]([^'"]+\.css)['"]\s*;"#,
            r#"import\s+\{[^}]*\}\s+from\s+['"]([^'"]+\.css)['"]\s*;"#,
            r#"require\s*\(\s*['"]([^'"]+\.css)['"]\s*\)"#,
        ];

        for pattern in &import_patterns {
            if let Ok(regex) = regex::Regex::new(pattern) {
                for cap in regex.captures_iter(source) {
                    if let Some(specifier_match) = cap.get(1) {
                        let specifier = specifier_match.as_str().to_string();
                        let is_module = specifier.ends_with(".module.css");

                        // Resolve the path
                        let css_path = if specifier.starts_with('.') {
                            file_path.parent().unwrap_or(Path::new(".")).join(&specifier)
                        } else {
                            PathBuf::from(&specifier)
                        };

                        imports.push(CssImport {
                            path: css_path,
                            specifier,
                            is_module,
                            start: cap.get(0).map(|m| m.start() as u32).unwrap_or(0),
                            end: cap.get(0).map(|m| m.end() as u32).unwrap_or(0),
                        });
                    }
                }
            }
        }

        imports
    }

    /// Bundle all CSS files
    pub fn bundle(&self) -> BundleResult<CssBundleOutput> {
        let mut css_output = String::new();
        let mut module_exports: HashMap<PathBuf, Vec<CssModuleExport>> = HashMap::new();
        let mut assets: Vec<AssetReference> = Vec::new();
        let mut seen_assets: HashSet<String> = HashSet::new();

        for file in &self.css_files {
            let (processed_css, exports, file_assets) = self.process_css_file(file)?;

            // Add comment for source tracking
            css_output.push_str(&format!("/* {} */\n", file.path.display()));
            css_output.push_str(&processed_css);
            css_output.push('\n');

            if !exports.is_empty() {
                module_exports.insert(file.path.clone(), exports);
            }

            // Deduplicate assets
            for asset in file_assets {
                if !seen_assets.contains(&asset.original_url) {
                    seen_assets.insert(asset.original_url.clone());
                    assets.push(asset);
                }
            }
        }

        Ok(CssBundleOutput {
            css: css_output,
            module_exports,
            assets,
        })
    }

    /// Process a single CSS file
    fn process_css_file(
        &self,
        file: &CssFileInfo,
    ) -> BundleResult<(String, Vec<CssModuleExport>, Vec<AssetReference>)> {
        let mut css = file.content.clone();
        let mut exports = Vec::new();

        // Process CSS modules (scope class names)
        if file.is_module && self.css_modules {
            let (scoped_css, class_exports) = self.scope_class_names(&css, &file.path);
            css = scoped_css;
            exports = class_exports;
        }

        // Process asset URLs
        let (processed_css, assets) = self.process_asset_urls(&css, &file.path);
        css = processed_css;

        Ok((css, exports, assets))
    }

    /// Scope class names for CSS modules
    fn scope_class_names(&self, css: &str, file_path: &Path) -> (String, Vec<CssModuleExport>) {
        let mut result = css.to_string();
        let mut exports = Vec::new();

        // Generate a unique hash for this file
        let file_hash = generate_file_hash(file_path);

        // Find and replace class selectors
        // Pattern: .className
        if let Ok(regex) = regex::Regex::new(r"\.([a-zA-Z_][a-zA-Z0-9_-]*)") {
            let mut replacements: Vec<(String, String)> = Vec::new();

            for cap in regex.captures_iter(css) {
                if let Some(class_match) = cap.get(1) {
                    let original = class_match.as_str().to_string();
                    let scoped = format!("{}_{}", original, file_hash);

                    // Check if we already have this class
                    if !replacements.iter().any(|(o, _)| o == &original) {
                        replacements.push((original.clone(), scoped.clone()));
                        exports.push(CssModuleExport { original, scoped });
                    }
                }
            }

            // Apply replacements
            for (original, scoped) in replacements {
                result = result.replace(&format!(".{}", original), &format!(".{}", scoped));
            }
        }

        (result, exports)
    }

    /// Process asset URLs in CSS
    fn process_asset_urls(&self, css: &str, file_path: &Path) -> (String, Vec<AssetReference>) {
        let mut result = css.to_string();
        let mut assets = Vec::new();

        // Pattern: url("path") or url('path') or url(path)
        if let Ok(regex) = regex::Regex::new(r#"url\s*\(\s*['"]?([^'")]+)['"]?\s*\)"#) {
            for cap in regex.captures_iter(css) {
                if let Some(url_match) = cap.get(1) {
                    let original_url = url_match.as_str().to_string();

                    // Skip data URLs and absolute URLs
                    if original_url.starts_with("data:")
                        || original_url.starts_with("http://")
                        || original_url.starts_with("https://")
                        || original_url.starts_with("//")
                    {
                        continue;
                    }

                    // Resolve the asset path
                    let asset_path =
                        file_path.parent().unwrap_or(Path::new(".")).join(&original_url);

                    // Generate output URL
                    let file_name = asset_path
                        .file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_else(|| "asset".to_string());
                    let output_url = format!("assets/{}", file_name);

                    // Replace in CSS
                    result = result.replace(
                        &format!("url({})", original_url),
                        &format!("url({})", output_url),
                    );
                    result = result.replace(
                        &format!("url(\"{}\")", original_url),
                        &format!("url(\"{}\")", output_url),
                    );
                    result = result.replace(
                        &format!("url('{}')", original_url),
                        &format!("url('{}')", output_url),
                    );

                    assets.push(AssetReference {
                        original_url,
                        file_path: asset_path,
                        output_url,
                    });
                }
            }
        }

        (result, assets)
    }

    /// Generate JavaScript code for CSS module exports
    pub fn generate_css_module_js(exports: &[CssModuleExport]) -> String {
        let mut js = String::from("export default {\n");

        for export in exports {
            js.push_str(&format!("  \"{}\": \"{}\",\n", export.original, export.scoped));
        }

        js.push_str("};\n");
        js
    }
}

/// Generate a random scope hash
fn generate_scope_hash() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_nanos();
    format!("{:x}", timestamp & 0xFFFFFF)
}

/// Generate a hash for a file path
fn generate_file_hash(path: &Path) -> String {
    let path_str = path.to_string_lossy();
    let hash = xxhash_rust::xxh64::xxh64(path_str.as_bytes(), 0);
    format!("{:x}", hash & 0xFFFFFF)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_detect_css_imports() {
        let source = r#"
            import './styles.css';
            import styles from './component.module.css';
            const x = require('./other.css');
        "#;

        let imports = CssBundler::detect_css_imports(source, Path::new("src/index.js"));

        assert!(!imports.is_empty());
        assert!(imports.iter().any(|i| i.specifier.contains("styles.css")));
    }

    #[test]
    fn test_css_bundling() {
        let temp_dir = TempDir::new().unwrap();
        let output_dir = temp_dir.path().join("dist");

        let mut bundler = CssBundler::new(temp_dir.path().to_path_buf(), output_dir);

        bundler.add_css_file(Path::new("src/a.css"), ".button { color: red; }", false);
        bundler.add_css_file(Path::new("src/b.css"), ".link { color: blue; }", false);

        let output = bundler.bundle().unwrap();

        assert!(output.css.contains(".button"));
        assert!(output.css.contains(".link"));
    }

    #[test]
    fn test_css_modules() {
        let temp_dir = TempDir::new().unwrap();
        let output_dir = temp_dir.path().join("dist");

        let mut bundler = CssBundler::new(temp_dir.path().to_path_buf(), output_dir);

        bundler.add_css_file(
            Path::new("src/component.module.css"),
            ".container { display: flex; }\n.title { font-size: 24px; }",
            true,
        );

        let output = bundler.bundle().unwrap();

        // Class names should be scoped
        assert!(!output.css.contains(".container {"));
        assert!(output.css.contains("container_"));

        // Should have exports
        let exports = output.module_exports.get(Path::new("src/component.module.css"));
        assert!(exports.is_some());

        let exports = exports.unwrap();
        assert!(exports.iter().any(|e| e.original == "container"));
        assert!(exports.iter().any(|e| e.original == "title"));
    }

    #[test]
    fn test_asset_url_processing() {
        let temp_dir = TempDir::new().unwrap();
        let output_dir = temp_dir.path().join("dist");

        let mut bundler = CssBundler::new(temp_dir.path().to_path_buf(), output_dir);

        bundler.add_css_file(
            Path::new("src/styles.css"),
            r#"
                .bg { background: url('./images/bg.png'); }
                .icon { background: url("../icons/icon.svg"); }
                .data { background: url(data:image/png;base64,abc); }
            "#,
            false,
        );

        let output = bundler.bundle().unwrap();

        // Should have asset references (excluding data URLs)
        assert!(output.assets.len() >= 2);

        // URLs should be rewritten
        assert!(output.css.contains("assets/"));

        // Data URLs should be preserved
        assert!(output.css.contains("data:image/png"));
    }

    #[test]
    fn test_generate_css_module_js() {
        let exports = vec![
            CssModuleExport {
                original: "container".to_string(),
                scoped: "container_abc123".to_string(),
            },
            CssModuleExport {
                original: "title".to_string(),
                scoped: "title_abc123".to_string(),
            },
        ];

        let js = CssBundler::generate_css_module_js(&exports);

        assert!(js.contains("export default"));
        assert!(js.contains("\"container\": \"container_abc123\""));
        assert!(js.contains("\"title\": \"title_abc123\""));
    }
}
