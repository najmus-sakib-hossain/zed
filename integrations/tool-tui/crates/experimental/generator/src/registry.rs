//! Template Registry - Feature #5
//!
//! Local and remote template registry for discovering, installing,
//! and managing templates.
//!
//! ## Features
//!
//! - Local template discovery from `.dx/templates/`
//! - Template metadata with parameter schemas
//! - Search by name, tags, and description
//! - Ed25519 signature verification for trusted templates
//! - Remote registry support (future)

use crate::error::{GeneratorError, Result};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

// ============================================================================
// Serde Helper for Ed25519 Signatures (64-byte arrays)
// ============================================================================

/// Custom serde module for `Option<[u8; 64]>` signature fields.
/// Serde doesn't support arrays > 32 elements by default.
#[cfg(feature = "serde-compat")]
mod serde_signature {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S>(signature: &Option<[u8; 64]>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match signature {
            Some(sig) => {
                // Encode as hex string for readability
                let hex = sig.iter().map(|b| format!("{:02x}", b)).collect::<String>();
                Some(hex).serialize(serializer)
            }
            None => None::<String>.serialize(serializer),
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<[u8; 64]>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let opt: Option<String> = Option::deserialize(deserializer)?;
        match opt {
            Some(hex) => {
                if hex.len() != 128 {
                    return Err(serde::de::Error::custom(format!(
                        "signature hex string must be 128 characters, got {}",
                        hex.len()
                    )));
                }
                let mut sig = [0u8; 64];
                for (i, chunk) in hex.as_bytes().chunks(2).enumerate() {
                    let byte_str = std::str::from_utf8(chunk)
                        .map_err(|_| serde::de::Error::custom("invalid UTF-8 in hex string"))?;
                    sig[i] = u8::from_str_radix(byte_str, 16)
                        .map_err(|_| serde::de::Error::custom("invalid hex character"))?;
                }
                Ok(Some(sig))
            }
            None => Ok(None),
        }
    }
}

// ============================================================================
// Template Metadata
// ============================================================================

/// Template metadata for registry.
///
/// Contains all information needed to discover, validate, and use a template.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde-compat", derive(serde::Serialize, serde::Deserialize))]
pub struct TemplateMetadata {
    /// Unique template identifier (e.g., "component", "rust-crate")
    pub id: String,
    /// Human-readable name (e.g., "React Component")
    pub name: String,
    /// Description of what the template generates
    pub description: String,
    /// Version (semver format)
    pub version: String,
    /// Author information
    pub author: Option<String>,
    /// Category tags for filtering
    pub tags: Vec<String>,
    /// Parameter schema for validation and documentation
    pub parameters: Vec<ParameterSchema>,
    /// Output file pattern (e.g., "{{name}}.tsx")
    pub output_pattern: String,
    /// Dependencies on other templates
    pub dependencies: Vec<String>,
    /// Ed25519 signature (64 bytes)
    #[cfg_attr(feature = "serde-compat", serde(with = "serde_signature"))]
    pub signature: Option<[u8; 64]>,
    /// Path to the template file
    pub path: PathBuf,
}

impl TemplateMetadata {
    /// Create new template metadata with required fields.
    #[must_use]
    pub fn new(id: impl Into<String>, name: impl Into<String>, path: impl Into<PathBuf>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            description: String::new(),
            version: "0.1.0".to_string(),
            author: None,
            tags: Vec::new(),
            parameters: Vec::new(),
            output_pattern: String::new(),
            dependencies: Vec::new(),
            signature: None,
            path: path.into(),
        }
    }

    /// Set the description.
    #[must_use]
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self
    }

    /// Set the version.
    #[must_use]
    pub fn with_version(mut self, version: impl Into<String>) -> Self {
        self.version = version.into();
        self
    }

    /// Set the author.
    #[must_use]
    pub fn with_author(mut self, author: impl Into<String>) -> Self {
        self.author = Some(author.into());
        self
    }

    /// Add a tag.
    #[must_use]
    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    /// Add multiple tags.
    #[must_use]
    pub fn with_tags(mut self, tags: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.tags.extend(tags.into_iter().map(Into::into));
        self
    }

    /// Add a parameter schema.
    #[must_use]
    pub fn with_parameter(mut self, param: ParameterSchema) -> Self {
        self.parameters.push(param);
        self
    }

    /// Set the output pattern.
    #[must_use]
    pub fn with_output_pattern(mut self, pattern: impl Into<String>) -> Self {
        self.output_pattern = pattern.into();
        self
    }

    /// Add a dependency.
    #[must_use]
    pub fn with_dependency(mut self, dep: impl Into<String>) -> Self {
        self.dependencies.push(dep.into());
        self
    }

    /// Set the signature.
    #[must_use]
    pub fn with_signature(mut self, signature: [u8; 64]) -> Self {
        self.signature = Some(signature);
        self
    }

    /// Check if this template matches a search query.
    ///
    /// Matches against id, name, description, and tags (case-insensitive).
    #[must_use]
    pub fn matches_query(&self, query: &str) -> bool {
        let query_lower = query.to_lowercase();

        // Check id
        if self.id.to_lowercase().contains(&query_lower) {
            return true;
        }

        // Check name
        if self.name.to_lowercase().contains(&query_lower) {
            return true;
        }

        // Check description
        if self.description.to_lowercase().contains(&query_lower) {
            return true;
        }

        // Check tags
        for tag in &self.tags {
            if tag.to_lowercase().contains(&query_lower) {
                return true;
            }
        }

        false
    }

    /// Check if this template has a specific tag.
    #[must_use]
    pub fn has_tag(&self, tag: &str) -> bool {
        let tag_lower = tag.to_lowercase();
        self.tags.iter().any(|t| t.to_lowercase() == tag_lower)
    }

    /// Check if this template is signed.
    #[must_use]
    pub fn is_signed(&self) -> bool {
        self.signature.is_some()
    }
}

// ============================================================================
// Parameter Schema
// ============================================================================

/// Parameter schema for documentation and validation.
///
/// Describes a template parameter including its type, whether it's required,
/// default value, and usage examples.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde-compat", derive(serde::Serialize, serde::Deserialize))]
pub struct ParameterSchema {
    /// Parameter name
    pub name: String,
    /// Human-readable description
    pub description: String,
    /// Value type (string, integer, boolean, etc.)
    pub value_type: String,
    /// Whether this parameter is required
    pub required: bool,
    /// Default value (as string representation)
    pub default: Option<String>,
    /// Usage examples
    pub examples: Vec<String>,
}

impl ParameterSchema {
    /// Create a new parameter schema.
    #[must_use]
    pub fn new(name: impl Into<String>, value_type: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: String::new(),
            value_type: value_type.into(),
            required: true,
            default: None,
            examples: Vec::new(),
        }
    }

    /// Set the description.
    #[must_use]
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self
    }

    /// Mark as optional.
    #[must_use]
    pub fn optional(mut self) -> Self {
        self.required = false;
        self
    }

    /// Set the default value.
    #[must_use]
    pub fn with_default(mut self, default: impl Into<String>) -> Self {
        self.default = Some(default.into());
        self.required = false;
        self
    }

    /// Add an example.
    #[must_use]
    pub fn with_example(mut self, example: impl Into<String>) -> Self {
        self.examples.push(example.into());
        self
    }
}

// ============================================================================
// Template Registry
// ============================================================================

/// Local template registry for discovering and managing templates.
///
/// Scans `.dx/templates/` directories for template files and maintains
/// a cache of template metadata for fast lookup.
///
/// # Example
///
/// ```rust,ignore
/// use dx_generator::TemplateRegistry;
///
/// let mut registry = TemplateRegistry::new(".dx/templates");
/// registry.scan()?;
///
/// // Search for templates
/// let results = registry.search("component");
///
/// // Get a specific template
/// if let Some(template) = registry.get("react-component") {
///     println!("Found: {}", template.name);
/// }
/// ```
#[derive(Debug)]
pub struct TemplateRegistry {
    /// Base directory for templates
    base_dir: PathBuf,
    /// Additional search paths
    search_paths: Vec<PathBuf>,
    /// Cached template metadata (id -> metadata)
    templates: HashMap<String, TemplateMetadata>,
    /// Remote registry URL (for future use)
    remote_url: Option<String>,
    /// Whether to verify signatures
    verify_signatures: bool,
}

impl TemplateRegistry {
    /// Create a new registry with the given base directory.
    #[must_use]
    pub fn new(base_dir: impl Into<PathBuf>) -> Self {
        Self {
            base_dir: base_dir.into(),
            search_paths: Vec::new(),
            templates: HashMap::new(),
            remote_url: None,
            verify_signatures: false,
        }
    }

    /// Add an additional search path.
    #[must_use]
    pub fn with_search_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.search_paths.push(path.into());
        self
    }

    /// Set the remote registry URL.
    #[must_use]
    pub fn with_remote_url(mut self, url: impl Into<String>) -> Self {
        self.remote_url = Some(url.into());
        self
    }

    /// Enable signature verification.
    #[must_use]
    pub fn with_signature_verification(mut self, enabled: bool) -> Self {
        self.verify_signatures = enabled;
        self
    }

    /// Get the base directory.
    #[must_use]
    pub fn base_dir(&self) -> &Path {
        &self.base_dir
    }

    /// Get all search paths (including base directory).
    #[must_use]
    pub fn all_paths(&self) -> Vec<&Path> {
        let mut paths = vec![self.base_dir.as_path()];
        paths.extend(self.search_paths.iter().map(PathBuf::as_path));
        paths
    }

    /// Scan directories for templates and populate the registry.
    ///
    /// This scans the base directory and all search paths for `.dxt` and
    /// `.dxt.hbs` files, extracting metadata from each.
    pub fn scan(&mut self) -> Result<usize> {
        self.templates.clear();
        let mut count = 0;

        // Collect paths first to avoid borrow issues
        let paths: Vec<PathBuf> = self.all_paths().iter().map(|p| p.to_path_buf()).collect();

        for path in paths {
            if path.exists() && path.is_dir() {
                count += self.scan_directory(&path)?;
            }
        }

        Ok(count)
    }

    /// Scan a single directory for templates.
    fn scan_directory(&mut self, dir: &Path) -> Result<usize> {
        let mut count = 0;

        let entries = std::fs::read_dir(dir).map_err(|e| GeneratorError::Io(e))?;

        for entry in entries.flatten() {
            let path = entry.path();

            if path.is_dir() {
                // Recursively scan subdirectories
                count += self.scan_directory(&path)?;
            } else if Self::is_template_file(&path) {
                // Extract metadata from template file
                if let Ok(metadata) = self.extract_metadata(&path) {
                    self.templates.insert(metadata.id.clone(), metadata);
                    count += 1;
                }
            }
        }

        Ok(count)
    }

    /// Check if a file is a template file.
    fn is_template_file(path: &Path) -> bool {
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        name.ends_with(".dxt") || name.ends_with(".dxt.hbs") || name.ends_with(".hbs")
    }

    /// Extract metadata from a template file.
    ///
    /// For now, this creates basic metadata from the filename.
    /// In the future, this will parse frontmatter or companion .meta files.
    fn extract_metadata(&self, path: &Path) -> Result<TemplateMetadata> {
        let file_name = path.file_stem().and_then(|n| n.to_str()).ok_or_else(|| {
            GeneratorError::InvalidTemplate {
                reason: "Invalid file name".to_string(),
            }
        })?;

        // Remove .dxt suffix if present (for .dxt.hbs files)
        let id = file_name.trim_end_matches(".dxt");

        // Create basic metadata
        let metadata = TemplateMetadata::new(id, Self::id_to_name(id), path);

        Ok(metadata)
    }

    /// Convert an id to a human-readable name.
    fn id_to_name(id: &str) -> String {
        id.split(|c| c == '-' || c == '_')
            .map(|word| {
                let mut chars = word.chars();
                match chars.next() {
                    None => String::new(),
                    Some(first) => first.to_uppercase().chain(chars).collect(),
                }
            })
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// Register a template manually.
    pub fn register(&mut self, metadata: TemplateMetadata) {
        self.templates.insert(metadata.id.clone(), metadata);
    }

    /// Get a template by ID.
    #[must_use]
    pub fn get(&self, id: &str) -> Option<&TemplateMetadata> {
        self.templates.get(id)
    }

    /// Check if a template exists.
    #[must_use]
    pub fn contains(&self, id: &str) -> bool {
        self.templates.contains_key(id)
    }

    /// Get the number of registered templates.
    #[must_use]
    pub fn len(&self) -> usize {
        self.templates.len()
    }

    /// Check if the registry is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.templates.is_empty()
    }

    /// List all templates.
    #[must_use]
    pub fn list(&self) -> Vec<&TemplateMetadata> {
        let mut templates: Vec<_> = self.templates.values().collect();
        templates.sort_by(|a, b| a.id.cmp(&b.id));
        templates
    }

    /// Search for templates matching a query.
    ///
    /// Searches against id, name, description, and tags (case-insensitive).
    /// Returns all templates that match the query.
    #[must_use]
    pub fn search(&self, query: &str) -> Vec<&TemplateMetadata> {
        if query.is_empty() {
            return self.list();
        }

        let mut results: Vec<_> =
            self.templates.values().filter(|t| t.matches_query(query)).collect();

        results.sort_by(|a, b| a.id.cmp(&b.id));
        results
    }

    /// Filter templates by tag.
    #[must_use]
    pub fn filter_by_tag(&self, tag: &str) -> Vec<&TemplateMetadata> {
        let mut results: Vec<_> = self.templates.values().filter(|t| t.has_tag(tag)).collect();

        results.sort_by(|a, b| a.id.cmp(&b.id));
        results
    }

    /// Get all unique tags across all templates.
    #[must_use]
    pub fn all_tags(&self) -> Vec<String> {
        let mut tags: Vec<_> =
            self.templates.values().flat_map(|t| t.tags.iter().cloned()).collect();

        tags.sort();
        tags.dedup();
        tags
    }

    /// Verify a template's signature.
    ///
    /// Returns Ok(true) if signature is valid, Ok(false) if no signature,
    /// or Err if signature verification fails.
    pub fn verify_signature(
        &self,
        metadata: &TemplateMetadata,
        public_key: &[u8; 32],
    ) -> Result<bool> {
        use ed25519_dalek::{Signature, Verifier, VerifyingKey};

        let signature = match &metadata.signature {
            Some(sig) => sig,
            None => return Ok(false),
        };

        // Read template content
        let content = std::fs::read(&metadata.path).map_err(|e| GeneratorError::Io(e))?;

        // Verify signature
        let verifying_key =
            VerifyingKey::from_bytes(public_key).map_err(|_| GeneratorError::SignatureInvalid)?;

        let sig = Signature::from_bytes(signature);

        verifying_key
            .verify(&content, &sig)
            .map_err(|_| GeneratorError::SignatureInvalid)?;

        Ok(true)
    }

    /// Remove a template from the registry.
    pub fn remove(&mut self, id: &str) -> Option<TemplateMetadata> {
        self.templates.remove(id)
    }

    /// Clear all templates from the registry.
    pub fn clear(&mut self) {
        self.templates.clear();
    }

    /// Iterate over all templates.
    pub fn iter(&self) -> impl Iterator<Item = (&String, &TemplateMetadata)> {
        self.templates.iter()
    }
}

impl Default for TemplateRegistry {
    fn default() -> Self {
        Self::new(".dx/templates")
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_template_metadata_new() {
        let meta = TemplateMetadata::new("component", "React Component", "templates/component.dxt");

        assert_eq!(meta.id, "component");
        assert_eq!(meta.name, "React Component");
        assert_eq!(meta.version, "0.1.0");
        assert!(meta.description.is_empty());
        assert!(meta.author.is_none());
        assert!(meta.tags.is_empty());
    }

    #[test]
    fn test_template_metadata_builder() {
        let meta = TemplateMetadata::new("model", "Data Model", "templates/model.dxt")
            .with_description("Generate a data model")
            .with_version("1.0.0")
            .with_author("Test Author")
            .with_tags(["rust", "model"])
            .with_output_pattern("{{name}}.rs");

        assert_eq!(meta.description, "Generate a data model");
        assert_eq!(meta.version, "1.0.0");
        assert_eq!(meta.author, Some("Test Author".to_string()));
        assert_eq!(meta.tags, vec!["rust", "model"]);
        assert_eq!(meta.output_pattern, "{{name}}.rs");
    }

    #[test]
    fn test_template_metadata_matches_query() {
        let meta = TemplateMetadata::new("react-component", "React Component", "path")
            .with_description("Generate a React component with hooks")
            .with_tags(["react", "typescript", "frontend"]);

        // Match by id
        assert!(meta.matches_query("react"));
        assert!(meta.matches_query("component"));

        // Match by name
        assert!(meta.matches_query("React"));
        assert!(meta.matches_query("COMPONENT"));

        // Match by description
        assert!(meta.matches_query("hooks"));

        // Match by tag
        assert!(meta.matches_query("typescript"));
        assert!(meta.matches_query("frontend"));

        // No match
        assert!(!meta.matches_query("python"));
        assert!(!meta.matches_query("backend"));
    }

    #[test]
    fn test_template_metadata_has_tag() {
        let meta = TemplateMetadata::new("test", "Test", "path").with_tags(["Rust", "Backend"]);

        assert!(meta.has_tag("rust"));
        assert!(meta.has_tag("RUST"));
        assert!(meta.has_tag("backend"));
        assert!(!meta.has_tag("frontend"));
    }

    #[test]
    fn test_parameter_schema() {
        let param = ParameterSchema::new("name", "string")
            .with_description("Component name")
            .with_default("MyComponent")
            .with_example("UserProfile");

        assert_eq!(param.name, "name");
        assert_eq!(param.value_type, "string");
        assert_eq!(param.description, "Component name");
        assert!(!param.required); // default makes it optional
        assert_eq!(param.default, Some("MyComponent".to_string()));
        assert_eq!(param.examples, vec!["UserProfile"]);
    }

    #[test]
    fn test_parameter_schema_optional() {
        let param = ParameterSchema::new("count", "integer").optional();

        assert!(!param.required);
        assert!(param.default.is_none());
    }

    #[test]
    fn test_registry_new() {
        let registry = TemplateRegistry::new(".dx/templates");

        assert_eq!(registry.base_dir(), Path::new(".dx/templates"));
        assert!(registry.is_empty());
        assert_eq!(registry.len(), 0);
    }

    #[test]
    fn test_registry_register_and_get() {
        let mut registry = TemplateRegistry::new(".dx/templates");

        let meta = TemplateMetadata::new("test", "Test Template", "path/test.dxt");
        registry.register(meta);

        assert_eq!(registry.len(), 1);
        assert!(registry.contains("test"));
        assert!(!registry.contains("other"));

        let retrieved = registry.get("test").unwrap();
        assert_eq!(retrieved.id, "test");
        assert_eq!(retrieved.name, "Test Template");
    }

    #[test]
    fn test_registry_list() {
        let mut registry = TemplateRegistry::new(".dx/templates");

        registry.register(TemplateMetadata::new("c", "C", "c.dxt"));
        registry.register(TemplateMetadata::new("a", "A", "a.dxt"));
        registry.register(TemplateMetadata::new("b", "B", "b.dxt"));

        let list = registry.list();
        assert_eq!(list.len(), 3);
        // Should be sorted by id
        assert_eq!(list[0].id, "a");
        assert_eq!(list[1].id, "b");
        assert_eq!(list[2].id, "c");
    }

    #[test]
    fn test_registry_search() {
        let mut registry = TemplateRegistry::new(".dx/templates");

        registry.register(
            TemplateMetadata::new("react-component", "React Component", "path")
                .with_tags(["react", "frontend"]),
        );
        registry.register(
            TemplateMetadata::new("vue-component", "Vue Component", "path")
                .with_tags(["vue", "frontend"]),
        );
        registry.register(
            TemplateMetadata::new("rust-model", "Rust Model", "path")
                .with_tags(["rust", "backend"]),
        );

        // Search by name
        let results = registry.search("component");
        assert_eq!(results.len(), 2);

        // Search by tag
        let results = registry.search("frontend");
        assert_eq!(results.len(), 2);

        // Search by specific framework
        let results = registry.search("react");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "react-component");

        // Empty search returns all
        let results = registry.search("");
        assert_eq!(results.len(), 3);

        // No match
        let results = registry.search("python");
        assert!(results.is_empty());
    }

    #[test]
    fn test_registry_filter_by_tag() {
        let mut registry = TemplateRegistry::new(".dx/templates");

        registry.register(TemplateMetadata::new("a", "A", "path").with_tags(["frontend", "react"]));
        registry.register(TemplateMetadata::new("b", "B", "path").with_tags(["frontend", "vue"]));
        registry.register(TemplateMetadata::new("c", "C", "path").with_tags(["backend", "rust"]));

        let frontend = registry.filter_by_tag("frontend");
        assert_eq!(frontend.len(), 2);

        let backend = registry.filter_by_tag("backend");
        assert_eq!(backend.len(), 1);
        assert_eq!(backend[0].id, "c");
    }

    #[test]
    fn test_registry_all_tags() {
        let mut registry = TemplateRegistry::new(".dx/templates");

        registry.register(TemplateMetadata::new("a", "A", "path").with_tags(["rust", "backend"]));
        registry.register(TemplateMetadata::new("b", "B", "path").with_tags(["rust", "frontend"]));

        let tags = registry.all_tags();
        assert_eq!(tags.len(), 3);
        assert!(tags.contains(&"rust".to_string()));
        assert!(tags.contains(&"backend".to_string()));
        assert!(tags.contains(&"frontend".to_string()));
    }

    #[test]
    fn test_registry_remove() {
        let mut registry = TemplateRegistry::new(".dx/templates");

        registry.register(TemplateMetadata::new("test", "Test", "path"));
        assert!(registry.contains("test"));

        let removed = registry.remove("test");
        assert!(removed.is_some());
        assert!(!registry.contains("test"));

        let removed_again = registry.remove("test");
        assert!(removed_again.is_none());
    }

    #[test]
    fn test_registry_clear() {
        let mut registry = TemplateRegistry::new(".dx/templates");

        registry.register(TemplateMetadata::new("a", "A", "path"));
        registry.register(TemplateMetadata::new("b", "B", "path"));
        assert_eq!(registry.len(), 2);

        registry.clear();
        assert!(registry.is_empty());
    }

    #[test]
    fn test_registry_with_search_paths() {
        let registry = TemplateRegistry::new(".dx/templates")
            .with_search_path("~/.dx/templates")
            .with_search_path("/usr/share/dx/templates");

        let paths = registry.all_paths();
        assert_eq!(paths.len(), 3);
    }

    #[test]
    fn test_id_to_name() {
        assert_eq!(TemplateRegistry::id_to_name("component"), "Component");
        assert_eq!(TemplateRegistry::id_to_name("react-component"), "React Component");
        assert_eq!(TemplateRegistry::id_to_name("rust_model"), "Rust Model");
        assert_eq!(TemplateRegistry::id_to_name("api-client-v2"), "Api Client V2");
    }

    #[test]
    fn test_is_template_file() {
        assert!(TemplateRegistry::is_template_file(Path::new("component.dxt")));
        assert!(TemplateRegistry::is_template_file(Path::new("component.dxt.hbs")));
        assert!(TemplateRegistry::is_template_file(Path::new("component.hbs")));
        assert!(!TemplateRegistry::is_template_file(Path::new("component.txt")));
        assert!(!TemplateRegistry::is_template_file(Path::new("component.rs")));
    }

    #[test]
    fn test_template_metadata_is_signed() {
        let unsigned = TemplateMetadata::new("test", "Test", "path");
        assert!(!unsigned.is_signed());

        let signed = TemplateMetadata::new("test", "Test", "path").with_signature([0u8; 64]);
        assert!(signed.is_signed());
    }

    #[test]
    fn test_registry_iter() {
        let mut registry = TemplateRegistry::new(".dx/templates");

        registry.register(TemplateMetadata::new("a", "A", "path"));
        registry.register(TemplateMetadata::new("b", "B", "path"));

        let ids: Vec<_> = registry.iter().map(|(id, _)| id.clone()).collect();
        assert_eq!(ids.len(), 2);
        assert!(ids.contains(&"a".to_string()));
        assert!(ids.contains(&"b".to_string()));
    }

    #[test]
    fn test_verify_signature_no_signature() {
        let registry = TemplateRegistry::new(".dx/templates");
        let metadata = TemplateMetadata::new("test", "Test", "path");
        let public_key = [0u8; 32];

        // No signature returns Ok(false)
        let result = registry.verify_signature(&metadata, &public_key);
        assert!(result.is_ok());
        assert!(!result.unwrap());
    }
}

// ============================================================================
// Property-Based Tests for Template Registry
// ============================================================================

#[cfg(test)]
mod property_tests {
    use super::*;
    use proptest::prelude::*;

    // ========================================================================
    // Feature: dx-generator-production
    // Property 4: Registry Search Completeness
    // Validates: Requirements 5.1, 5.4
    // ========================================================================

    /// Strategy for generating template IDs
    fn template_id_strategy() -> impl Strategy<Value = String> {
        "[a-z][a-z0-9-]{0,15}".prop_map(|s| s.to_string())
    }

    /// Strategy for generating tags
    fn tag_strategy() -> impl Strategy<Value = String> {
        "[a-z]{3,10}".prop_map(|s| s.to_string())
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Property 4.1: Search by ID returns matching templates
        /// For any template with ID containing the query, search must return it.
        #[test]
        fn prop_search_finds_by_id(
            id in template_id_strategy(),
            query in "[a-z]{2,5}".prop_map(|s| s.to_string())
        ) {
            let mut registry = TemplateRegistry::new(".dx/templates");

            // Create template with ID containing the query
            let full_id = format!("{}-{}", query, id);
            registry.register(TemplateMetadata::new(&full_id, "Test", "path"));

            let results = registry.search(&query);

            // Property: template with matching ID is found
            prop_assert!(
                results.iter().any(|t| t.id == full_id),
                "Template with ID '{}' not found when searching for '{}'",
                full_id, query
            );
        }

        /// Property 4.2: Search by name returns matching templates
        /// For any template with name containing the query, search must return it.
        #[test]
        fn prop_search_finds_by_name(
            id in template_id_strategy(),
            name_prefix in "[A-Z][a-z]{2,5}".prop_map(|s| s.to_string()),
            name_suffix in "[A-Z][a-z]{2,5}".prop_map(|s| s.to_string())
        ) {
            let mut registry = TemplateRegistry::new(".dx/templates");

            let full_name = format!("{} {}", name_prefix, name_suffix);
            registry.register(TemplateMetadata::new(&id, &full_name, "path"));

            // Search by part of the name
            let results = registry.search(&name_prefix);

            // Property: template with matching name is found
            prop_assert!(
                results.iter().any(|t| t.name == full_name),
                "Template with name '{}' not found when searching for '{}'",
                full_name, name_prefix
            );
        }

        /// Property 4.3: Search by tag returns matching templates
        /// For any template with a tag containing the query, search must return it.
        #[test]
        fn prop_search_finds_by_tag(
            id in template_id_strategy(),
            tag in tag_strategy()
        ) {
            let mut registry = TemplateRegistry::new(".dx/templates");

            registry.register(
                TemplateMetadata::new(&id, "Test", "path")
                    .with_tag(&tag)
            );

            let results = registry.search(&tag);

            // Property: template with matching tag is found
            prop_assert!(
                results.iter().any(|t| t.id == id),
                "Template with tag '{}' not found when searching for '{}'",
                tag, tag
            );
        }

        /// Property 4.4: Search is case-insensitive
        /// Searching with different cases should return the same results.
        #[test]
        fn prop_search_case_insensitive(
            query in "[a-z]{3,8}".prop_map(|s| s.to_string())
        ) {
            let mut registry = TemplateRegistry::new(".dx/templates");

            let full_id = format!("{}-test", query);
            registry.register(TemplateMetadata::new(&full_id, "Test", "path"));

            let lower_results = registry.search(&query.to_lowercase());
            let upper_results = registry.search(&query.to_uppercase());

            // Property: case doesn't affect results
            prop_assert_eq!(
                lower_results.len(), upper_results.len(),
                "Case-insensitive search failed: lowercase found {}, uppercase found {}",
                lower_results.len(), upper_results.len()
            );
        }

        /// Property 4.5: Empty search returns all templates
        /// Searching with empty string should return all registered templates.
        #[test]
        fn prop_empty_search_returns_all(
            ids in proptest::collection::vec(template_id_strategy(), 1..5)
        ) {
            let mut registry = TemplateRegistry::new(".dx/templates");

            // Deduplicate IDs
            let unique_ids: Vec<_> = ids.into_iter().collect::<std::collections::HashSet<_>>().into_iter().collect();

            for id in &unique_ids {
                registry.register(TemplateMetadata::new(id, "Test", "path"));
            }

            let results = registry.search("");

            // Property: all templates are returned
            prop_assert_eq!(
                results.len(), unique_ids.len(),
                "Empty search returned {} templates, expected {}",
                results.len(), unique_ids.len()
            );
        }

        /// Property 4.6: Search results are sorted by ID
        /// Results should always be sorted alphabetically by ID.
        #[test]
        fn prop_search_results_sorted(
            ids in proptest::collection::vec(template_id_strategy(), 2..6)
        ) {
            let mut registry = TemplateRegistry::new(".dx/templates");

            for id in &ids {
                registry.register(TemplateMetadata::new(id, "Test", "path"));
            }

            let results = registry.search("");
            let result_ids: Vec<_> = results.iter().map(|t| &t.id).collect();

            // Property: results are sorted
            for i in 1..result_ids.len() {
                prop_assert!(
                    result_ids[i-1] <= result_ids[i],
                    "Results not sorted: '{}' should come before '{}'",
                    result_ids[i-1], result_ids[i]
                );
            }
        }

        /// Property 4.7: Filter by tag returns only matching templates
        /// Templates without the specified tag should not be returned.
        #[test]
        fn prop_filter_by_tag_complete(
            id1 in template_id_strategy(),
            id2 in template_id_strategy(),
            tag1 in tag_strategy(),
            tag2 in tag_strategy()
        ) {
            prop_assume!(id1 != id2);
            prop_assume!(tag1 != tag2);

            let mut registry = TemplateRegistry::new(".dx/templates");

            registry.register(TemplateMetadata::new(&id1, "Test1", "path").with_tag(&tag1));
            registry.register(TemplateMetadata::new(&id2, "Test2", "path").with_tag(&tag2));

            let results = registry.filter_by_tag(&tag1);

            // Property: only templates with the tag are returned
            prop_assert_eq!(results.len(), 1);
            prop_assert_eq!(&results[0].id, &id1);
        }

        /// Property 4.8: Search completeness - no false negatives
        /// If a template matches the query, it must be in the results.
        #[test]
        fn prop_search_no_false_negatives(
            id in template_id_strategy(),
            query in "[a-z]{3,6}".prop_map(|s| s.to_string())
        ) {
            let mut registry = TemplateRegistry::new(".dx/templates");

            // Create template with query in description
            let full_desc = format!("This {} template", query);
            registry.register(
                TemplateMetadata::new(&id, "Test", "path")
                    .with_description(&full_desc)
            );

            let results = registry.search(&query);

            // Property: template is found via description match
            prop_assert!(
                results.iter().any(|t| t.id == id),
                "Template with description containing '{}' not found",
                query
            );
        }

        /// Property 4.9: All tags are collected
        /// all_tags() should return every unique tag from all templates.
        #[test]
        fn prop_all_tags_complete(
            tags1 in proptest::collection::vec(tag_strategy(), 1..3),
            tags2 in proptest::collection::vec(tag_strategy(), 1..3)
        ) {
            let mut registry = TemplateRegistry::new(".dx/templates");

            registry.register(
                TemplateMetadata::new("t1", "Test1", "path")
                    .with_tags(tags1.clone())
            );
            registry.register(
                TemplateMetadata::new("t2", "Test2", "path")
                    .with_tags(tags2.clone())
            );

            let all_tags = registry.all_tags();

            // Property: all tags from both templates are present
            for tag in tags1.iter().chain(tags2.iter()) {
                prop_assert!(
                    all_tags.contains(tag),
                    "Tag '{}' missing from all_tags()",
                    tag
                );
            }
        }

        /// Property 4.10: Registry operations are idempotent
        /// Registering the same template twice should not create duplicates.
        #[test]
        fn prop_register_idempotent(
            id in template_id_strategy()
        ) {
            let mut registry = TemplateRegistry::new(".dx/templates");

            registry.register(TemplateMetadata::new(&id, "Test1", "path1"));
            registry.register(TemplateMetadata::new(&id, "Test2", "path2"));

            // Property: only one template with this ID exists
            prop_assert_eq!(registry.len(), 1);

            // Property: the second registration overwrites the first
            let template = registry.get(&id).unwrap();
            prop_assert_eq!(&template.name, "Test2");
        }
    }

    // ========================================================================
    // Feature: dx-generator-production
    // Property 11: Signature Verification
    // Validates: Requirements 5.5
    // ========================================================================

    use ed25519_dalek::SigningKey;
    use rand::rngs::OsRng;
    use std::io::Write;
    use tempfile::NamedTempFile;

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Property 11.1: Valid signatures verify successfully
        /// A signature created with a key should verify with the same key.
        #[test]
        fn prop_valid_signature_verifies(
            content in proptest::collection::vec(proptest::num::u8::ANY, 10..100)
        ) {
            use ed25519_dalek::{Signer, Signature};

            // Create a temporary file with content
            let mut temp_file = NamedTempFile::new().unwrap();
            temp_file.write_all(&content).unwrap();
            let path = temp_file.path().to_path_buf();

            // Generate key pair
            let signing_key = SigningKey::generate(&mut OsRng);
            let public_key = signing_key.verifying_key().to_bytes();

            // Sign the content
            let signature: Signature = signing_key.sign(&content);

            // Create metadata with signature
            let metadata = TemplateMetadata::new("test", "Test", &path)
                .with_signature(signature.to_bytes());

            // Verify
            let registry = TemplateRegistry::new(".dx/templates");
            let result = registry.verify_signature(&metadata, &public_key);

            // Property: valid signature verifies
            prop_assert!(result.is_ok());
            prop_assert!(result.unwrap());
        }

        /// Property 11.2: Tampered content fails verification
        /// If content is modified after signing, verification should fail.
        #[test]
        fn prop_tampered_content_fails(
            content in proptest::collection::vec(proptest::num::u8::ANY, 10..100),
            tamper_index in 0usize..10usize
        ) {
            use ed25519_dalek::{Signer, Signature};

            // Generate key pair
            let signing_key = SigningKey::generate(&mut OsRng);
            let public_key = signing_key.verifying_key().to_bytes();

            // Sign the original content
            let signature: Signature = signing_key.sign(&content);

            // Tamper with content
            let mut tampered = content.clone();
            let idx = tamper_index % tampered.len();
            tampered[idx] ^= 0xFF;

            // Create a temporary file with tampered content
            let mut temp_file = NamedTempFile::new().unwrap();
            temp_file.write_all(&tampered).unwrap();
            let path = temp_file.path().to_path_buf();

            // Create metadata with original signature
            let metadata = TemplateMetadata::new("test", "Test", &path)
                .with_signature(signature.to_bytes());

            // Verify
            let registry = TemplateRegistry::new(".dx/templates");
            let result = registry.verify_signature(&metadata, &public_key);

            // Property: tampered content fails verification
            prop_assert!(result.is_err());
        }

        /// Property 11.3: Wrong key fails verification
        /// A signature should not verify with a different public key.
        #[test]
        fn prop_wrong_key_fails(
            content in proptest::collection::vec(proptest::num::u8::ANY, 10..100)
        ) {
            use ed25519_dalek::{Signer, Signature};

            // Create a temporary file with content
            let mut temp_file = NamedTempFile::new().unwrap();
            temp_file.write_all(&content).unwrap();
            let path = temp_file.path().to_path_buf();

            // Generate two different key pairs
            let signing_key1 = SigningKey::generate(&mut OsRng);
            let signing_key2 = SigningKey::generate(&mut OsRng);
            let wrong_public_key = signing_key2.verifying_key().to_bytes();

            // Sign with key1
            let signature: Signature = signing_key1.sign(&content);

            // Create metadata with signature from key1
            let metadata = TemplateMetadata::new("test", "Test", &path)
                .with_signature(signature.to_bytes());

            // Verify with key2 (wrong key)
            let registry = TemplateRegistry::new(".dx/templates");
            let result = registry.verify_signature(&metadata, &wrong_public_key);

            // Property: wrong key fails verification
            prop_assert!(result.is_err());
        }

        /// Property 11.4: Unsigned templates return false
        /// Templates without signatures should return Ok(false), not error.
        #[test]
        fn prop_unsigned_returns_false(
            id in template_id_strategy()
        ) {
            let metadata = TemplateMetadata::new(&id, "Test", "path");
            let public_key = [0u8; 32];

            let registry = TemplateRegistry::new(".dx/templates");
            let result = registry.verify_signature(&metadata, &public_key);

            // Property: unsigned template returns Ok(false)
            prop_assert!(result.is_ok());
            prop_assert!(!result.unwrap());
        }
    }
}
