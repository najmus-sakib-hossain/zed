//! Framework Detection
//!
//! This module provides a dedicated `FrameworkDetector` component for detecting
//! frameworks in projects, with support for pattern matching and multiple frameworks
//! in monorepos.

use serde::Deserialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::project::{Framework, MonorepoInfo, PackageInfo};

/// Framework detection result
#[derive(Debug, Clone)]
pub struct FrameworkDetectionResult {
    /// Detected frameworks
    pub frameworks: Vec<Framework>,
    /// Framework-specific configurations
    pub configurations: HashMap<String, FrameworkConfig>,
    /// Confidence score (0.0 to 1.0)
    pub confidence: f64,
}

/// Framework-specific configuration
#[derive(Debug, Clone)]
pub struct FrameworkConfig {
    /// Framework version (if detected)
    pub version: Option<String>,
    /// Framework-specific settings
    pub settings: HashMap<String, String>,
    /// Detected features
    pub features: Vec<String>,
}

/// Framework pattern for detection
#[derive(Debug, Clone)]
pub struct FrameworkPattern {
    /// Framework identifier
    pub framework: Framework,
    /// Package dependencies to check
    pub dependencies: Vec<String>,
    /// Config files to look for
    pub config_files: Vec<String>,
    /// Directory patterns to check
    pub directory_patterns: Vec<String>,
    /// File patterns to check
    pub file_patterns: Vec<String>,
    /// Minimum confidence threshold
    pub min_confidence: f64,
}

/// Framework detector
pub struct FrameworkDetector {
    /// Registered framework patterns
    patterns: Vec<FrameworkPattern>,
    /// Cache of detection results
    cache: HashMap<PathBuf, FrameworkDetectionResult>,
}

impl FrameworkDetector {
    /// Create a new framework detector with default patterns
    #[must_use]
    pub fn new() -> Self {
        Self {
            patterns: Self::default_patterns(),
            cache: HashMap::new(),
        }
    }

    /// Create a new framework detector with custom patterns
    #[must_use]
    pub fn with_patterns(patterns: Vec<FrameworkPattern>) -> Self {
        Self {
            patterns,
            cache: HashMap::new(),
        }
    }

    /// Get default framework patterns
    fn default_patterns() -> Vec<FrameworkPattern> {
        vec![
            // Next.js
            FrameworkPattern {
                framework: Framework::Next,
                dependencies: vec!["next".to_string()],
                config_files: vec![
                    "next.config.js".to_string(),
                    "next.config.mjs".to_string(),
                    "next.config.ts".to_string(),
                ],
                directory_patterns: vec!["pages".to_string(), "app".to_string()],
                file_patterns: vec!["**/_app.tsx".to_string(), "**/_document.tsx".to_string()],
                min_confidence: 0.7,
            },
            // React
            FrameworkPattern {
                framework: Framework::React,
                dependencies: vec!["react".to_string(), "react-dom".to_string()],
                config_files: vec!["vite.config.ts".to_string(), "vite.config.js".to_string()],
                directory_patterns: vec!["src".to_string(), "public".to_string()],
                file_patterns: vec!["**/*.jsx".to_string(), "**/*.tsx".to_string()],
                min_confidence: 0.6,
            },
            // Vue
            FrameworkPattern {
                framework: Framework::Vue,
                dependencies: vec!["vue".to_string()],
                config_files: vec!["vue.config.js".to_string(), "vite.config.ts".to_string()],
                directory_patterns: vec!["src".to_string()],
                file_patterns: vec!["**/*.vue".to_string()],
                min_confidence: 0.6,
            },
            // Nuxt
            FrameworkPattern {
                framework: Framework::Nuxt,
                dependencies: vec!["nuxt".to_string()],
                config_files: vec!["nuxt.config.ts".to_string(), "nuxt.config.js".to_string()],
                directory_patterns: vec!["pages".to_string(), "components".to_string()],
                file_patterns: vec!["**/*.vue".to_string()],
                min_confidence: 0.7,
            },
            // Svelte
            FrameworkPattern {
                framework: Framework::Svelte,
                dependencies: vec!["svelte".to_string()],
                config_files: vec![
                    "svelte.config.js".to_string(),
                    "svelte.config.ts".to_string(),
                ],
                directory_patterns: vec!["src".to_string()],
                file_patterns: vec!["**/*.svelte".to_string()],
                min_confidence: 0.6,
            },
            // SvelteKit
            FrameworkPattern {
                framework: Framework::SvelteKit,
                dependencies: vec!["@sveltejs/kit".to_string()],
                config_files: vec![
                    "svelte.config.js".to_string(),
                    "svelte.config.ts".to_string(),
                ],
                directory_patterns: vec!["src/routes".to_string(), "src/lib".to_string()],
                file_patterns: vec!["**/*.svelte".to_string()],
                min_confidence: 0.7,
            },
            // Angular
            FrameworkPattern {
                framework: Framework::Angular,
                dependencies: vec!["@angular/core".to_string()],
                config_files: vec!["angular.json".to_string()],
                directory_patterns: vec!["src/app".to_string()],
                file_patterns: vec![
                    "**/*.component.ts".to_string(),
                    "**/*.module.ts".to_string(),
                ],
                min_confidence: 0.7,
            },
            // Solid
            FrameworkPattern {
                framework: Framework::Solid,
                dependencies: vec!["solid-js".to_string()],
                config_files: vec!["vite.config.ts".to_string()],
                directory_patterns: vec!["src".to_string()],
                file_patterns: vec!["**/*.jsx".to_string(), "**/*.tsx".to_string()],
                min_confidence: 0.6,
            },
            // Qwik
            FrameworkPattern {
                framework: Framework::Qwik,
                dependencies: vec!["@builder.io/qwik".to_string()],
                config_files: vec!["vite.config.ts".to_string()],
                directory_patterns: vec!["src".to_string()],
                file_patterns: vec!["**/*.tsx".to_string()],
                min_confidence: 0.7,
            },
            // Remix
            FrameworkPattern {
                framework: Framework::Remix,
                dependencies: vec!["@remix-run/react".to_string()],
                config_files: vec!["remix.config.js".to_string()],
                directory_patterns: vec!["app/routes".to_string()],
                file_patterns: vec!["**/*.tsx".to_string()],
                min_confidence: 0.7,
            },
            // Astro
            FrameworkPattern {
                framework: Framework::Astro,
                dependencies: vec!["astro".to_string()],
                config_files: vec![
                    "astro.config.mjs".to_string(),
                    "astro.config.ts".to_string(),
                ],
                directory_patterns: vec!["src/pages".to_string()],
                file_patterns: vec!["**/*.astro".to_string()],
                min_confidence: 0.7,
            },
            // Express
            FrameworkPattern {
                framework: Framework::Express,
                dependencies: vec!["express".to_string()],
                config_files: vec![],
                directory_patterns: vec!["src".to_string()],
                file_patterns: vec!["**/app.js".to_string(), "**/server.js".to_string()],
                min_confidence: 0.5,
            },
            // Fastify
            FrameworkPattern {
                framework: Framework::Fastify,
                dependencies: vec!["fastify".to_string()],
                config_files: vec![],
                directory_patterns: vec!["src".to_string()],
                file_patterns: vec!["**/app.js".to_string(), "**/server.js".to_string()],
                min_confidence: 0.5,
            },
            // Hono
            FrameworkPattern {
                framework: Framework::Hono,
                dependencies: vec!["hono".to_string()],
                config_files: vec![],
                directory_patterns: vec!["src".to_string()],
                file_patterns: vec!["**/index.ts".to_string()],
                min_confidence: 0.5,
            },
            // NestJS
            FrameworkPattern {
                framework: Framework::Nest,
                dependencies: vec!["@nestjs/core".to_string()],
                config_files: vec!["nest-cli.json".to_string()],
                directory_patterns: vec!["src".to_string()],
                file_patterns: vec![
                    "**/*.module.ts".to_string(),
                    "**/*.controller.ts".to_string(),
                ],
                min_confidence: 0.7,
            },
        ]
    }

    /// Detect frameworks in a project
    pub fn detect(&mut self, root: &Path) -> FrameworkDetectionResult {
        // Check cache first
        if let Some(cached) = self.cache.get(root) {
            return cached.clone();
        }

        let mut frameworks = Vec::new();
        let mut configurations = HashMap::new();
        let mut total_confidence = 0.0;
        let mut pattern_count = 0;

        // Read package.json if it exists
        let package_json = read_package_json(root);
        let dependencies =
            package_json.as_ref().map(|pkg| pkg.all_dependencies()).unwrap_or_default();

        // Check each pattern
        for pattern in &self.patterns {
            let confidence = self.calculate_confidence(root, pattern, &dependencies);

            if confidence >= pattern.min_confidence {
                frameworks.push(pattern.framework);

                // Extract framework configuration
                let config = self.extract_framework_config(root, pattern);
                configurations.insert(pattern.framework.as_str().to_string(), config);

                total_confidence += confidence;
                pattern_count += 1;
            }
        }

        // Calculate overall confidence
        let overall_confidence = if pattern_count > 0 {
            total_confidence / f64::from(pattern_count)
        } else {
            0.0
        };

        let result = FrameworkDetectionResult {
            frameworks,
            configurations,
            confidence: overall_confidence,
        };

        // Cache the result
        self.cache.insert(root.to_path_buf(), result.clone());

        result
    }

    /// Calculate confidence score for a pattern
    fn calculate_confidence(
        &self,
        root: &Path,
        pattern: &FrameworkPattern,
        dependencies: &std::collections::HashSet<&str>,
    ) -> f64 {
        let mut confidence = 0.0;

        // Check dependencies
        if !pattern.dependencies.is_empty() {
            let dep_matches = pattern
                .dependencies
                .iter()
                .filter(|dep| dependencies.contains(dep.as_str()))
                .count();

            if dep_matches > 0 {
                confidence += (dep_matches as f64 / pattern.dependencies.len() as f64) * 0.4;
            }
        }

        // Check config files
        if !pattern.config_files.is_empty() {
            let config_matches =
                pattern.config_files.iter().filter(|config| root.join(config).exists()).count();

            if config_matches > 0 {
                confidence += (config_matches as f64 / pattern.config_files.len() as f64) * 0.3;
            }
        }

        // Check directory patterns
        if !pattern.directory_patterns.is_empty() {
            let dir_matches =
                pattern.directory_patterns.iter().filter(|dir| root.join(dir).is_dir()).count();

            if dir_matches > 0 {
                confidence += (dir_matches as f64 / pattern.directory_patterns.len() as f64) * 0.2;
            }
        }

        // Check file patterns
        if !pattern.file_patterns.is_empty() {
            let file_matches = pattern
                .file_patterns
                .iter()
                .filter(|pattern| self.matches_file_pattern(root, pattern))
                .count();

            if file_matches > 0 {
                confidence += (file_matches as f64 / pattern.file_patterns.len() as f64) * 0.1;
            }
        }

        confidence
    }

    /// Check if a file pattern matches in the project
    fn matches_file_pattern(&self, root: &Path, pattern: &str) -> bool {
        // Convert glob pattern to path
        let pattern_path = if pattern.starts_with("**/") {
            root.join(&pattern[3..])
        } else {
            root.join(pattern)
        };

        // Check if pattern contains wildcards
        if pattern.contains('*') {
            // Simple glob matching - in production, use a proper glob library
            self.glob_match(root, pattern)
        } else {
            pattern_path.exists()
        }
    }

    /// Simple glob pattern matching
    fn glob_match(&self, root: &Path, pattern: &str) -> bool {
        // This is a simplified implementation
        // In production, use the glob crate for proper pattern matching

        let pattern_stripped = pattern.strip_prefix("**/").unwrap_or(pattern);
        let parts: Vec<&str> = pattern_stripped.split('/').collect();

        if parts.is_empty() {
            return false;
        }

        // Get the file extension from the pattern
        if let Some(ext) = parts.last().and_then(|p| p.split('.').next_back()) {
            // Search for files with this extension
            if let Ok(entries) = std::fs::read_dir(root) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_file()
                        && let Some(file_ext) = path.extension().and_then(|e| e.to_str())
                        && file_ext == ext
                    {
                        return true;
                    }
                }
            }
        }

        false
    }

    /// Extract framework-specific configuration
    fn extract_framework_config(&self, root: &Path, pattern: &FrameworkPattern) -> FrameworkConfig {
        let mut config = FrameworkConfig {
            version: None,
            settings: HashMap::new(),
            features: Vec::new(),
        };

        // Try to read version from package.json
        if let Some(pkg) = read_package_json(root) {
            for dep_name in &pattern.dependencies {
                if let Some(version) =
                    pkg.all_dependencies().iter().find(|dep| **dep == dep_name.as_str()).and_then(
                        |_| {
                            pkg.dependencies
                                .get(dep_name)
                                .or_else(|| pkg.dev_dependencies.get(dep_name))
                        },
                    )
                {
                    config.version = Some(version.clone());
                    break;
                }
            }
        }

        // Detect features based on directory structure
        for dir_pattern in &pattern.directory_patterns {
            let dir_path = root.join(dir_pattern);
            if dir_path.is_dir() {
                config.features.push(dir_pattern.clone());
            }
        }

        // Add config file paths as settings
        for config_file in &pattern.config_files {
            let config_path = root.join(config_file);
            if config_path.exists() {
                config
                    .settings
                    .insert("config_file".to_string(), config_path.to_string_lossy().to_string());
            }
        }

        config
    }

    /// Detect frameworks in a monorepo
    pub fn detect_monorepo(
        &mut self,
        monorepo: &MonorepoInfo,
    ) -> Vec<(PackageInfo, FrameworkDetectionResult)> {
        let mut results = Vec::new();

        for package in &monorepo.packages {
            let detection = self.detect(&package.path);
            if !detection.frameworks.is_empty() {
                results.push((package.clone(), detection));
            }
        }

        results
    }

    /// Clear the detection cache
    pub fn clear_cache(&mut self) {
        self.cache.clear();
    }

    /// Add a custom framework pattern
    pub fn add_pattern(&mut self, pattern: FrameworkPattern) {
        self.patterns.push(pattern);
    }

    /// Get all registered patterns
    #[must_use]
    pub fn patterns(&self) -> &[FrameworkPattern] {
        &self.patterns
    }
}

impl Default for FrameworkDetector {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper function to read package.json
fn read_package_json(root: &Path) -> Option<PackageJson> {
    let path = root.join("package.json");
    let content = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&content).ok()
}

#[derive(Debug, Deserialize)]
struct PackageJson {
    #[serde(default)]
    dependencies: HashMap<String, String>,
    #[serde(default, rename = "devDependencies")]
    dev_dependencies: HashMap<String, String>,
    #[serde(default, rename = "peerDependencies")]
    peer_dependencies: HashMap<String, String>,
}

impl PackageJson {
    fn all_dependencies(&self) -> std::collections::HashSet<&str> {
        self.dependencies
            .keys()
            .chain(self.dev_dependencies.keys())
            .chain(self.peer_dependencies.keys())
            .map(std::string::String::as_str)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_framework_detector_new() {
        let detector = FrameworkDetector::new();
        assert!(!detector.patterns().is_empty());
    }

    #[test]
    fn test_framework_detector_default() {
        let detector = FrameworkDetector::default();
        assert!(!detector.patterns().is_empty());
    }

    #[test]
    fn test_framework_pattern_confidence() {
        let mut detector = FrameworkDetector::new();
        let temp_dir = std::env::temp_dir();

        // Test with non-existent directory
        let result = detector.detect(&temp_dir.join("nonexistent"));
        assert_eq!(result.frameworks.len(), 0);
        assert_eq!(result.confidence, 0.0);
    }

    #[test]
    fn test_add_pattern() {
        let mut detector = FrameworkDetector::new();
        let initial_count = detector.patterns().len();

        let pattern = FrameworkPattern {
            framework: Framework::React,
            dependencies: vec!["react".to_string()],
            config_files: vec![],
            directory_patterns: vec![],
            file_patterns: vec![],
            min_confidence: 0.5,
        };

        detector.add_pattern(pattern);
        assert_eq!(detector.patterns().len(), initial_count + 1);
    }

    #[test]
    fn test_clear_cache() {
        let mut detector = FrameworkDetector::new();
        let temp_dir = std::env::temp_dir();

        // Run detection to populate cache
        detector.detect(&temp_dir);

        // Clear cache
        detector.clear_cache();

        // Cache should be empty
        assert!(detector.cache.is_empty());
    }
}
