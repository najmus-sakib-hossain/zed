//! CSS Property Database Generator
//!
//! Generates the CSS property database in DX Serializer format.
//! Creates `.sr` source file and corresponding `.human`/`.machine` files.
//!
//! **Validates: Requirements 6.1, 6.2, 6.3, 6.4, 6.5, 6.6, 6.8, 6.9**

use super::database::{CssPropertyDef, get_all_css_properties};
use serializer::{DxDocument, DxLlmValue, document_to_human, document_to_machine};
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

/// Error type for generator operations
#[derive(Debug)]
pub enum GeneratorError {
    IoError(std::io::Error),
    SerializerError(String),
}

impl std::fmt::Display for GeneratorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::IoError(e) => write!(f, "IO error: {}", e),
            Self::SerializerError(msg) => write!(f, "Serializer error: {}", msg),
        }
    }
}

impl std::error::Error for GeneratorError {}

impl From<std::io::Error> for GeneratorError {
    fn from(e: std::io::Error) -> Self {
        Self::IoError(e)
    }
}

/// Result of generating the CSS property database
#[derive(Debug)]
pub struct GeneratorResult {
    pub dxs_path: std::path::PathBuf,
    pub human_path: std::path::PathBuf,
    pub machine_path: std::path::PathBuf,
    pub property_count: usize,
    pub category_count: usize,
}

/// CSS Property Database Generator
///
/// Generates the CSS property database from the built-in definitions
/// and outputs in DX Serializer format.
pub struct CssPropertyGenerator {
    style_dir: std::path::PathBuf,
    serializer_dir: std::path::PathBuf,
}

impl CssPropertyGenerator {
    /// Create a new generator with default paths
    pub fn new() -> Self {
        Self {
            style_dir: std::path::PathBuf::from(".dx/style"),
            serializer_dir: std::path::PathBuf::from(".dx/serializer"),
        }
    }

    /// Create a generator with custom paths
    pub fn with_paths(style_dir: impl AsRef<Path>, serializer_dir: impl AsRef<Path>) -> Self {
        Self {
            style_dir: style_dir.as_ref().to_path_buf(),
            serializer_dir: serializer_dir.as_ref().to_path_buf(),
        }
    }

    /// Generate the CSS property database files
    ///
    /// Creates:
    /// - `.dx/style/css-properties.sr` - DX Serializer source file
    /// - `.dx/serializer/css-properties.human` - Human-readable format
    /// - `.dx/serializer/css-properties.machine` - Binary machine format
    pub fn generate(&self) -> Result<GeneratorResult, GeneratorError> {
        let properties = get_all_css_properties();

        // Ensure directories exist
        fs::create_dir_all(&self.style_dir)?;
        fs::create_dir_all(&self.serializer_dir)?;

        // Generate DX document
        let doc = self.properties_to_document(&properties);

        // Generate .sr source file
        let dxs_content = self.generate_dxs_content(&properties);
        let dxs_path = self.style_dir.join("css-properties.sr");
        fs::write(&dxs_path, &dxs_content)?;

        // Generate .human file
        let human_content = document_to_human(&doc);
        let human_path = self.serializer_dir.join("css-properties.human");
        fs::write(&human_path, &human_content)?;

        // Generate .machine file
        let machine = document_to_machine(&doc);
        let machine_path = self.serializer_dir.join("css-properties.machine");
        fs::write(&machine_path, &machine.data)?;

        // Count categories
        let categories: std::collections::HashSet<_> =
            properties.iter().map(|p| &p.category).collect();

        Ok(GeneratorResult {
            dxs_path,
            human_path,
            machine_path,
            property_count: properties.len(),
            category_count: categories.len(),
        })
    }

    /// Convert properties to DX document for serialization
    fn properties_to_document(&self, properties: &[CssPropertyDef]) -> DxDocument {
        let mut doc = DxDocument::new();

        for prop in properties {
            // Store property definition: p:{name}|cat -> category
            doc.context
                .insert(format!("p:{}|cat", prop.name), DxLlmValue::Str(prop.category.clone()));

            // Store values: p:{name}|val -> [values]
            if !prop.values.is_empty() {
                doc.context.insert(
                    format!("p:{}|val", prop.name),
                    DxLlmValue::Arr(
                        prop.values.iter().map(|v| DxLlmValue::Str(v.clone())).collect(),
                    ),
                );
            }

            // Store numeric flag: p:{name}|num -> bool
            doc.context
                .insert(format!("p:{}|num", prop.name), DxLlmValue::Bool(prop.accepts_numeric));

            // Store valid units: p:{name}|units -> [units]
            if !prop.valid_units.is_empty() {
                doc.context.insert(
                    format!("p:{}|units", prop.name),
                    DxLlmValue::Arr(
                        prop.valid_units.iter().map(|u| DxLlmValue::Str(u.clone())).collect(),
                    ),
                );
            }

            // Store browser support: p:{name}|bs -> "chrome,firefox,safari,edge"
            let bs = &prop.browser_support;
            let bs_str = format!(
                "{},{},{},{}",
                bs.chrome.map_or("-".to_string(), |v| v.to_string()),
                bs.firefox.map_or("-".to_string(), |v| v.to_string()),
                bs.safari.map_or("-".to_string(), |v| v.to_string()),
                bs.edge.map_or("-".to_string(), |v| v.to_string()),
            );
            doc.context.insert(format!("p:{}|bs", prop.name), DxLlmValue::Str(bs_str));
        }

        // Store category index for quick lookup
        let mut categories: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for prop in properties {
            categories.entry(prop.category.clone()).or_default().push(prop.name.clone());
        }
        for (cat, props) in categories {
            doc.context.insert(
                format!("cat:{}", cat),
                DxLlmValue::Arr(props.into_iter().map(DxLlmValue::Str).collect()),
            );
        }

        doc
    }

    /// Generate human-readable .sr source content
    fn generate_dxs_content(&self, properties: &[CssPropertyDef]) -> String {
        let mut out = String::new();
        out.push_str("// CSS Property Database\n");
        out.push_str("// Generated from CSS specification\n");
        out.push_str("// Format: DX Serializer (.sr)\n\n");

        // Group by category
        let mut by_category: BTreeMap<&str, Vec<&CssPropertyDef>> = BTreeMap::new();
        for prop in properties {
            by_category.entry(&prop.category).or_default().push(prop);
        }

        for (category, props) in by_category {
            out.push_str(&format!("// === {} ===\n", category.to_uppercase()));
            for prop in props {
                out.push_str(&format!("p:{}|cat={}\n", prop.name, prop.category));
                if !prop.values.is_empty() {
                    out.push_str(&format!("p:{}|val=[{}]\n", prop.name, prop.values.join(",")));
                }
                if prop.accepts_numeric {
                    out.push_str(&format!("p:{}|num=true\n", prop.name));
                    if !prop.valid_units.is_empty() {
                        out.push_str(&format!(
                            "p:{}|units=[{}]\n",
                            prop.name,
                            prop.valid_units.join(",")
                        ));
                    }
                }
            }
            out.push('\n');
        }

        out
    }
}

impl Default for CssPropertyGenerator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_generate_creates_files() {
        let temp = tempdir().unwrap();
        let style_dir = temp.path().join("style");
        let serializer_dir = temp.path().join("serializer");

        let generator = CssPropertyGenerator::with_paths(&style_dir, &serializer_dir);
        let result = generator.generate().expect("Generation should succeed");

        assert!(result.dxs_path.exists(), ".sr file should exist");
        assert!(result.human_path.exists(), ".human file should exist");
        assert!(result.machine_path.exists(), ".machine file should exist");
        assert!(result.property_count > 0, "Should have properties");
        assert!(result.category_count > 0, "Should have categories");
    }

    #[test]
    fn test_dxs_content_format() {
        let generator = CssPropertyGenerator::new();
        let properties = get_all_css_properties();
        let content = generator.generate_dxs_content(&properties);

        assert!(content.contains("CSS Property Database"));
        assert!(content.contains("display"));
        assert!(content.contains("layout"));
    }
}
