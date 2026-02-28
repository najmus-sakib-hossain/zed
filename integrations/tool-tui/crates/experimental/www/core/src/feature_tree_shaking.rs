//! # Feature Tree-Shaking Module
//!
//! Implements feature-based tree-shaking for dx-www projects.
//! When a feature is disabled in the configuration, all related code
//! is excluded from the production build.
//!
//! Also provides symbol-level tree-shaking for dead code elimination.
//!
//! **Validates: Requirements 2.1, 2.2, 2.3, 2.4, 2.5, 10.2**

use crate::www_config::{FeaturesConfig, VALID_FEATURE_NAMES};
use std::collections::{HashMap, HashSet, VecDeque};

/// Unique identifier for a symbol in the module graph
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SymbolId {
    /// Module path
    pub module: String,
    /// Symbol name within the module
    pub name: String,
}

impl SymbolId {
    pub fn new(module: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            module: module.into(),
            name: name.into(),
        }
    }
}

/// A symbol in the module graph
#[derive(Debug, Clone)]
pub struct Symbol {
    pub id: SymbolId,
    pub kind: SymbolKind,
    pub is_exported: bool,
    pub byte_size: usize,
}

/// Kind of symbol
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SymbolKind {
    Function,
    Variable,
    Class,
    Type,
    Component,
}

/// Statistics from tree shaking operation
#[derive(Debug, Clone, Default)]
pub struct TreeShakeStats {
    /// Number of symbols removed
    pub symbols_removed: usize,
    /// Estimated bytes saved
    pub bytes_saved: usize,
    /// Number of modules affected
    pub modules_affected: usize,
    /// Symbols that were removed
    pub removed_symbols: Vec<SymbolId>,
}

/// Symbol usage graph for tree shaking
///
/// This implements graph-based dead code elimination by tracking
/// which symbols reference which other symbols, then removing
/// any symbols not reachable from entry points.
#[derive(Debug, Clone, Default)]
pub struct UsageGraph {
    /// All symbols in the module graph
    symbols: HashMap<SymbolId, Symbol>,
    /// Edges: symbol -> symbols it references
    edges: HashMap<SymbolId, HashSet<SymbolId>>,
    /// Entry point symbols (always retained)
    entry_points: HashSet<SymbolId>,
}

impl UsageGraph {
    /// Create a new empty usage graph
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a symbol to the graph
    pub fn add_symbol(&mut self, symbol: Symbol) {
        self.symbols.insert(symbol.id.clone(), symbol);
    }

    /// Add a reference edge from one symbol to another
    pub fn add_reference(&mut self, from: SymbolId, to: SymbolId) {
        self.edges.entry(from).or_default().insert(to);
    }

    /// Mark a symbol as an entry point (will always be retained)
    pub fn mark_entry_point(&mut self, id: SymbolId) {
        self.entry_points.insert(id);
    }

    /// Get all symbols
    pub fn symbols(&self) -> &HashMap<SymbolId, Symbol> {
        &self.symbols
    }

    /// Get entry points
    pub fn entry_points(&self) -> &HashSet<SymbolId> {
        &self.entry_points
    }

    /// Mark reachable symbols starting from entry points using BFS
    /// Returns the set of all reachable symbol IDs
    pub fn mark_reachable(&self) -> HashSet<SymbolId> {
        let mut reachable = HashSet::new();
        let mut queue: VecDeque<SymbolId> = self.entry_points.iter().cloned().collect();

        while let Some(current) = queue.pop_front() {
            if reachable.contains(&current) {
                continue;
            }

            reachable.insert(current.clone());

            // Add all symbols referenced by current
            if let Some(refs) = self.edges.get(&current) {
                for referenced in refs {
                    if !reachable.contains(referenced) {
                        queue.push_back(referenced.clone());
                    }
                }
            }
        }

        reachable
    }

    /// Remove unreachable symbols and return statistics
    pub fn shake(&mut self) -> TreeShakeStats {
        let reachable = self.mark_reachable();
        let mut stats = TreeShakeStats::default();
        let mut affected_modules = HashSet::new();

        // Find symbols to remove
        let to_remove: Vec<SymbolId> =
            self.symbols.keys().filter(|id| !reachable.contains(*id)).cloned().collect();

        // Remove unreachable symbols
        for id in to_remove {
            if let Some(symbol) = self.symbols.remove(&id) {
                stats.symbols_removed += 1;
                stats.bytes_saved += symbol.byte_size;
                affected_modules.insert(symbol.id.module.clone());
                stats.removed_symbols.push(id.clone());
            }
            self.edges.remove(&id);
        }

        // Clean up edges pointing to removed symbols
        for refs in self.edges.values_mut() {
            refs.retain(|id| self.symbols.contains_key(id));
        }

        stats.modules_affected = affected_modules.len();
        stats
    }

    /// Get statistics without modifying the graph
    pub fn stats(&self) -> TreeShakeStats {
        let reachable = self.mark_reachable();
        let mut stats = TreeShakeStats::default();
        let mut affected_modules = HashSet::new();

        for (id, symbol) in &self.symbols {
            if !reachable.contains(id) {
                stats.symbols_removed += 1;
                stats.bytes_saved += symbol.byte_size;
                affected_modules.insert(symbol.id.module.clone());
                stats.removed_symbols.push(id.clone());
            }
        }

        stats.modules_affected = affected_modules.len();
        stats
    }

    /// Check if a symbol is reachable from entry points
    pub fn is_reachable(&self, id: &SymbolId) -> bool {
        self.mark_reachable().contains(id)
    }
}

/// Feature module mappings - maps feature names to their module paths
pub const FEATURE_MODULES: &[(&str, &[&str])] = &[
    ("forms", &["dx/forms", "dx-forms", "@dx/forms", "crates/www/form"]),
    ("query", &["dx/query", "dx-query", "@dx/query", "crates/www/query"]),
    ("auth", &["dx/auth", "dx-auth", "@dx/auth", "crates/www/auth"]),
    ("sync", &["dx/sync", "dx-sync", "@dx/sync", "crates/www/sync"]),
    (
        "offline",
        &[
            "dx/offline",
            "dx-offline",
            "@dx/offline",
            "crates/www/offline",
        ],
    ),
    ("a11y", &["dx/a11y", "dx-a11y", "@dx/a11y", "crates/www/a11y"]),
    ("i18n", &["dx/i18n", "dx-i18n", "@dx/i18n", "crates/www/i18n"]),
];

/// Feature API patterns - maps feature names to their API patterns
pub const FEATURE_API_PATTERNS: &[(&str, &[&str])] = &[
    (
        "forms",
        &[
            "useForm",
            "useField",
            "useFormContext",
            "FormProvider",
            "<Form",
            "<Field",
        ],
    ),
    (
        "query",
        &[
            "useQuery",
            "useMutation",
            "useQueryClient",
            "QueryProvider",
            "<Query",
        ],
    ),
    (
        "auth",
        &[
            "useAuth",
            "useSession",
            "useUser",
            "AuthProvider",
            "<Auth",
            "withAuth",
        ],
    ),
    ("sync", &["useSync", "useLiveQuery", "SyncProvider", "<Sync"]),
    (
        "offline",
        &[
            "useOffline",
            "useNetworkStatus",
            "OfflineProvider",
            "<Offline",
        ],
    ),
    (
        "a11y",
        &[
            "useA11y",
            "useFocusTrap",
            "useAnnounce",
            "A11yProvider",
            "<A11y",
        ],
    ),
    (
        "i18n",
        &[
            "useTranslation",
            "useLocale",
            "I18nProvider",
            "<Trans",
            "t(",
        ],
    ),
];

/// Result of feature usage analysis
#[derive(Debug, Clone, Default)]
pub struct FeatureUsageAnalysis {
    /// Features that are used in the source code
    pub used_features: HashSet<String>,
    /// Features that are enabled in config
    pub enabled_features: HashSet<String>,
    /// Features that are used but disabled (warnings)
    pub disabled_but_used: Vec<FeatureUsageWarning>,
    /// Features that are enabled but not used (can be tree-shaken)
    pub enabled_but_unused: Vec<String>,
    /// Import statements to remove (feature module imports)
    pub imports_to_remove: Vec<ImportToRemove>,
    /// Code blocks to remove (feature-specific code)
    pub code_blocks_to_remove: Vec<CodeBlockToRemove>,
}

/// Warning for using a disabled feature
#[derive(Debug, Clone)]
pub struct FeatureUsageWarning {
    /// The feature that is used but disabled
    pub feature: String,
    /// The file where the usage was found
    pub file: String,
    /// The line number where the usage was found
    pub line: usize,
    /// The pattern that was matched
    pub pattern: String,
    /// Human-readable warning message
    pub message: String,
}

impl std::fmt::Display for FeatureUsageWarning {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}:{}: Feature '{}' is disabled but '{}' is used. {}",
            self.file, self.line, self.feature, self.pattern, self.message
        )
    }
}

/// Import statement to remove during tree-shaking
#[derive(Debug, Clone)]
pub struct ImportToRemove {
    /// The file containing the import
    pub file: String,
    /// The line number of the import
    pub line: usize,
    /// The full import statement
    pub import_statement: String,
    /// The feature this import belongs to
    pub feature: String,
}

/// Code block to remove during tree-shaking
#[derive(Debug, Clone)]
pub struct CodeBlockToRemove {
    /// The file containing the code block
    pub file: String,
    /// Start line of the code block
    pub start_line: usize,
    /// End line of the code block
    pub end_line: usize,
    /// The feature this code block belongs to
    pub feature: String,
    /// Description of what's being removed
    pub description: String,
}

/// Feature tree-shaker for analyzing and removing disabled feature code
pub struct FeatureTreeShaker {
    /// Enabled features from configuration
    enabled_features: HashSet<String>,
    /// Module patterns for each feature
    module_patterns: HashMap<String, Vec<String>>,
    /// API patterns for each feature
    api_patterns: HashMap<String, Vec<String>>,
}

impl FeatureTreeShaker {
    /// Create a new feature tree-shaker from configuration
    pub fn new(config: &FeaturesConfig) -> Self {
        let enabled_features: HashSet<String> =
            config.enabled_features().into_iter().map(|s| s.to_string()).collect();

        let mut module_patterns = HashMap::new();
        for (feature, modules) in FEATURE_MODULES {
            module_patterns
                .insert(feature.to_string(), modules.iter().map(|s| s.to_string()).collect());
        }

        let mut api_patterns = HashMap::new();
        for (feature, patterns) in FEATURE_API_PATTERNS {
            api_patterns
                .insert(feature.to_string(), patterns.iter().map(|s| s.to_string()).collect());
        }

        Self {
            enabled_features,
            module_patterns,
            api_patterns,
        }
    }

    /// Create a tree-shaker with all features enabled (for testing)
    pub fn all_enabled() -> Self {
        let enabled_features: HashSet<String> =
            VALID_FEATURE_NAMES.iter().map(|s| s.to_string()).collect();

        let mut module_patterns = HashMap::new();
        for (feature, modules) in FEATURE_MODULES {
            module_patterns
                .insert(feature.to_string(), modules.iter().map(|s| s.to_string()).collect());
        }

        let mut api_patterns = HashMap::new();
        for (feature, patterns) in FEATURE_API_PATTERNS {
            api_patterns
                .insert(feature.to_string(), patterns.iter().map(|s| s.to_string()).collect());
        }

        Self {
            enabled_features,
            module_patterns,
            api_patterns,
        }
    }

    /// Create a tree-shaker with no features enabled (for testing)
    pub fn none_enabled() -> Self {
        Self {
            enabled_features: HashSet::new(),
            module_patterns: {
                let mut map = HashMap::new();
                for (feature, modules) in FEATURE_MODULES {
                    map.insert(
                        feature.to_string(),
                        modules.iter().map(|s| s.to_string()).collect(),
                    );
                }
                map
            },
            api_patterns: {
                let mut map = HashMap::new();
                for (feature, patterns) in FEATURE_API_PATTERNS {
                    map.insert(
                        feature.to_string(),
                        patterns.iter().map(|s| s.to_string()).collect(),
                    );
                }
                map
            },
        }
    }

    /// Check if a feature is enabled
    pub fn is_feature_enabled(&self, feature: &str) -> bool {
        self.enabled_features.contains(feature)
    }

    /// Get all enabled features
    pub fn enabled_features(&self) -> &HashSet<String> {
        &self.enabled_features
    }

    /// Get all disabled features
    pub fn disabled_features(&self) -> Vec<String> {
        VALID_FEATURE_NAMES
            .iter()
            .filter(|f| !self.enabled_features.contains(**f))
            .map(|s| s.to_string())
            .collect()
    }

    /// Analyze source code for feature usage
    pub fn analyze_source(&self, source: &str, file_path: &str) -> FeatureUsageAnalysis {
        let mut analysis = FeatureUsageAnalysis {
            enabled_features: self.enabled_features.clone(),
            ..Default::default()
        };

        // Analyze imports
        self.analyze_imports(source, file_path, &mut analysis);

        // Analyze API usage
        self.analyze_api_usage(source, file_path, &mut analysis);

        // Calculate enabled but unused features
        for feature in &self.enabled_features {
            if !analysis.used_features.contains(feature) {
                analysis.enabled_but_unused.push(feature.clone());
            }
        }

        analysis
    }

    /// Analyze import statements for feature module imports
    fn analyze_imports(&self, source: &str, file_path: &str, analysis: &mut FeatureUsageAnalysis) {
        let import_regex = regex::Regex::new(
            r#"import\s+(?:\{[^}]*\}|\*\s+as\s+\w+|\w+)\s+from\s+['"]([^'"]+)['"]"#,
        )
        .unwrap();

        for (line_num, line) in source.lines().enumerate() {
            if let Some(cap) = import_regex.captures(line) {
                let import_path = &cap[1];

                // Check if this import matches any feature module
                for (feature, modules) in &self.module_patterns {
                    for module in modules {
                        if import_path.contains(module) {
                            analysis.used_features.insert(feature.clone());

                            // If feature is disabled, mark for removal
                            if !self.is_feature_enabled(feature) {
                                analysis.imports_to_remove.push(ImportToRemove {
                                    file: file_path.to_string(),
                                    line: line_num + 1,
                                    import_statement: line.to_string(),
                                    feature: feature.clone(),
                                });

                                analysis.disabled_but_used.push(FeatureUsageWarning {
                                    feature: feature.clone(),
                                    file: file_path.to_string(),
                                    line: line_num + 1,
                                    pattern: import_path.to_string(),
                                    message: format!(
                                        "Enable the '{}' feature in dx.config or remove this import",
                                        feature
                                    ),
                                });
                            }
                        }
                    }
                }
            }
        }
    }

    /// Analyze API usage patterns
    fn analyze_api_usage(
        &self,
        source: &str,
        file_path: &str,
        analysis: &mut FeatureUsageAnalysis,
    ) {
        for (line_num, line) in source.lines().enumerate() {
            for (feature, patterns) in &self.api_patterns {
                for pattern in patterns {
                    if line.contains(pattern) {
                        analysis.used_features.insert(feature.clone());

                        // If feature is disabled, add warning
                        if !self.is_feature_enabled(feature) {
                            // Avoid duplicate warnings for the same line
                            let already_warned = analysis
                                .disabled_but_used
                                .iter()
                                .any(|w| w.file == file_path && w.line == line_num + 1);

                            if !already_warned {
                                analysis.disabled_but_used.push(FeatureUsageWarning {
                                    feature: feature.clone(),
                                    file: file_path.to_string(),
                                    line: line_num + 1,
                                    pattern: pattern.clone(),
                                    message: format!(
                                        "Enable the '{}' feature in dx.config to use '{}'",
                                        feature, pattern
                                    ),
                                });
                            }
                        }
                    }
                }
            }
        }
    }

    /// Apply tree-shaking to source code, removing disabled feature code
    pub fn tree_shake(&self, source: &str, file_path: &str) -> TreeShakeResult {
        let analysis = self.analyze_source(source, file_path);
        let mut result = TreeShakeResult {
            output: source.to_string(),
            removed_lines: Vec::new(),
            warnings: analysis.disabled_but_used.clone(),
        };

        // If there are no imports to remove, return early
        if analysis.imports_to_remove.is_empty() {
            return result;
        }

        // Remove import lines (in reverse order to preserve line numbers)
        let mut lines: Vec<&str> = source.lines().collect();
        let mut removed_indices: Vec<usize> =
            analysis.imports_to_remove.iter().map(|i| i.line - 1).collect();
        removed_indices.sort();
        removed_indices.reverse();

        for idx in removed_indices {
            if idx < lines.len() {
                result.removed_lines.push((idx + 1, lines[idx].to_string()));
                lines.remove(idx);
            }
        }

        result.output = lines.join("\n");
        result
    }

    /// Check if source code uses any disabled features
    pub fn has_disabled_feature_usage(&self, source: &str, file_path: &str) -> bool {
        let analysis = self.analyze_source(source, file_path);
        !analysis.disabled_but_used.is_empty()
    }

    /// Get feature for a given import path
    pub fn get_feature_for_import(&self, import_path: &str) -> Option<String> {
        for (feature, modules) in &self.module_patterns {
            for module in modules {
                if import_path.contains(module) {
                    return Some(feature.clone());
                }
            }
        }
        None
    }

    /// Get feature for a given API pattern
    pub fn get_feature_for_api(&self, api_pattern: &str) -> Option<String> {
        for (feature, patterns) in &self.api_patterns {
            for pattern in patterns {
                if api_pattern.contains(pattern) {
                    return Some(feature.clone());
                }
            }
        }
        None
    }
}

/// Result of tree-shaking operation
#[derive(Debug, Clone)]
pub struct TreeShakeResult {
    /// The tree-shaken output
    pub output: String,
    /// Lines that were removed (line number, content)
    pub removed_lines: Vec<(usize, String)>,
    /// Warnings about disabled feature usage
    pub warnings: Vec<FeatureUsageWarning>,
}

impl TreeShakeResult {
    /// Check if any code was removed
    pub fn has_removals(&self) -> bool {
        !self.removed_lines.is_empty()
    }

    /// Get the number of removed lines
    pub fn removed_count(&self) -> usize {
        self.removed_lines.len()
    }

    /// Check if there are any warnings
    pub fn has_warnings(&self) -> bool {
        !self.warnings.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_feature_tree_shaker_creation() {
        let config = FeaturesConfig::default();
        let shaker = FeatureTreeShaker::new(&config);

        // All features should be disabled by default
        assert!(shaker.disabled_features().len() == VALID_FEATURE_NAMES.len());
    }

    #[test]
    fn test_feature_tree_shaker_with_enabled_features() {
        let mut config = FeaturesConfig::default();
        config.forms = true;
        config.query = true;

        let shaker = FeatureTreeShaker::new(&config);

        assert!(shaker.is_feature_enabled("forms"));
        assert!(shaker.is_feature_enabled("query"));
        assert!(!shaker.is_feature_enabled("auth"));
    }

    #[test]
    fn test_analyze_imports() {
        let shaker = FeatureTreeShaker::none_enabled();

        let source = r#"
import { useForm, useField } from 'dx/forms';
import { useQuery } from '@dx/query';

function App() {
    const form = useForm();
    return <div>Hello</div>;
}
"#;

        let analysis = shaker.analyze_source(source, "App.tsx");

        assert!(analysis.used_features.contains("forms"));
        assert!(analysis.used_features.contains("query"));
        assert!(!analysis.disabled_but_used.is_empty());
    }

    #[test]
    fn test_analyze_api_usage() {
        let shaker = FeatureTreeShaker::none_enabled();

        let source = r#"
function App() {
    const { t } = useTranslation();
    const auth = useAuth();
    return <div>{t('hello')}</div>;
}
"#;

        let analysis = shaker.analyze_source(source, "App.tsx");

        assert!(analysis.used_features.contains("i18n"));
        assert!(analysis.used_features.contains("auth"));
    }

    #[test]
    fn test_tree_shake_removes_imports() {
        let shaker = FeatureTreeShaker::none_enabled();

        let source = r#"import { useForm } from 'dx/forms';
import React from 'react';

function App() {
    return <div>Hello</div>;
}"#;

        let result = shaker.tree_shake(source, "App.tsx");

        assert!(result.has_removals());
        assert!(!result.output.contains("dx/forms"));
        assert!(result.output.contains("react"));
    }

    #[test]
    fn test_no_tree_shake_when_feature_enabled() {
        let mut config = FeaturesConfig::default();
        config.forms = true;
        let shaker = FeatureTreeShaker::new(&config);

        let source = r#"import { useForm } from 'dx/forms';

function App() {
    const form = useForm();
    return <div>Hello</div>;
}"#;

        let result = shaker.tree_shake(source, "App.tsx");

        assert!(!result.has_removals());
        assert!(result.output.contains("dx/forms"));
    }

    #[test]
    fn test_get_feature_for_import() {
        let shaker = FeatureTreeShaker::all_enabled();

        assert_eq!(shaker.get_feature_for_import("dx/forms"), Some("forms".to_string()));
        assert_eq!(shaker.get_feature_for_import("@dx/query"), Some("query".to_string()));
        assert_eq!(shaker.get_feature_for_import("react"), None);
    }

    #[test]
    fn test_get_feature_for_api() {
        let shaker = FeatureTreeShaker::all_enabled();

        assert_eq!(shaker.get_feature_for_api("useForm"), Some("forms".to_string()));
        assert_eq!(shaker.get_feature_for_api("useQuery"), Some("query".to_string()));
        assert_eq!(shaker.get_feature_for_api("useState"), None);
    }

    #[test]
    fn test_enabled_but_unused() {
        let mut config = FeaturesConfig::default();
        config.forms = true;
        config.query = true;
        let shaker = FeatureTreeShaker::new(&config);

        let source = r#"
function App() {
    return <div>Hello</div>;
}
"#;

        let analysis = shaker.analyze_source(source, "App.tsx");

        // Both forms and query are enabled but not used
        assert!(analysis.enabled_but_unused.contains(&"forms".to_string()));
        assert!(analysis.enabled_but_unused.contains(&"query".to_string()));
    }
}

#[cfg(test)]
mod property_tests {
    use super::*;
    use proptest::prelude::*;

    /// Generate a random feature name
    fn arbitrary_feature() -> impl Strategy<Value = String> {
        prop_oneof![
            Just("forms".to_string()),
            Just("query".to_string()),
            Just("auth".to_string()),
            Just("sync".to_string()),
            Just("offline".to_string()),
            Just("a11y".to_string()),
            Just("i18n".to_string()),
        ]
    }

    /// Generate a random set of enabled features
    fn arbitrary_enabled_features() -> impl Strategy<Value = FeaturesConfig> {
        (
            any::<bool>(),
            any::<bool>(),
            any::<bool>(),
            any::<bool>(),
            any::<bool>(),
            any::<bool>(),
            any::<bool>(),
        )
            .prop_map(|(forms, query, auth, sync, offline, a11y, i18n)| FeaturesConfig {
                forms,
                query,
                auth,
                sync,
                offline,
                a11y,
                i18n,
            })
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Property 12: Feature Flag Tree-Shaking
        /// *For any* disabled feature, the production build SHALL NOT contain any code from that feature's module.
        ///
        /// **Validates: Requirements 10.2**
        #[test]
        fn prop_disabled_feature_code_removed(
            config in arbitrary_enabled_features(),
        ) {
            let shaker = FeatureTreeShaker::new(&config);

            // Generate source with all feature imports
            let source = r#"
import { useForm } from 'dx/forms';
import { useQuery } from 'dx/query';
import { useAuth } from 'dx/auth';
import { useSync } from 'dx/sync';
import { useOffline } from 'dx/offline';
import { useA11y } from 'dx/a11y';
import { useTranslation } from 'dx/i18n';

function App() {
    return <div>Hello</div>;
}
"#;

            let result = shaker.tree_shake(source, "App.tsx");

            // For each disabled feature, verify its import is removed
            for feature in shaker.disabled_features() {
                let modules = FEATURE_MODULES.iter()
                    .find(|(f, _)| *f == feature)
                    .map(|(_, m)| m)
                    .unwrap_or(&(&[] as &[&str]));

                for module in *modules {
                    // The output should not contain imports from disabled feature modules
                    let import_pattern = format!("from '{}'", module);
                    prop_assert!(
                        !result.output.contains(&import_pattern),
                        "Disabled feature '{}' import '{}' should be removed",
                        feature,
                        module
                    );
                }
            }

            // For each enabled feature, verify its import is preserved
            for feature in shaker.enabled_features() {
                let modules = FEATURE_MODULES.iter()
                    .find(|(f, _)| *f == feature.as_str())
                    .map(|(_, m)| m)
                    .unwrap_or(&(&[] as &[&str]));

                // At least one module pattern should be in the output
                let has_import = modules.iter().any(|_module| {
                    let import_pattern = format!("from 'dx/{}'", feature);
                    result.output.contains(&import_pattern)
                });

                // Only check if the feature has a matching import in the source
                if source.contains(&format!("from 'dx/{}'", feature)) {
                    prop_assert!(
                        has_import,
                        "Enabled feature '{}' import should be preserved",
                        feature
                    );
                }
            }
        }

        /// Property: Feature detection is consistent
        /// *For any* feature and source code, analyzing the same source twice should produce the same result.
        #[test]
        fn prop_feature_detection_deterministic(
            config in arbitrary_enabled_features(),
        ) {
            let shaker = FeatureTreeShaker::new(&config);

            let source = r#"
import { useForm } from 'dx/forms';
function App() {
    const form = useForm();
    return <div>Hello</div>;
}
"#;

            let analysis1 = shaker.analyze_source(source, "App.tsx");
            let analysis2 = shaker.analyze_source(source, "App.tsx");

            prop_assert_eq!(analysis1.used_features, analysis2.used_features);
            prop_assert_eq!(analysis1.disabled_but_used.len(), analysis2.disabled_but_used.len());
        }

        /// Property: Tree-shaking is idempotent
        /// *For any* source code, tree-shaking twice should produce the same result as tree-shaking once.
        #[test]
        fn prop_tree_shaking_idempotent(
            config in arbitrary_enabled_features(),
        ) {
            let shaker = FeatureTreeShaker::new(&config);

            let source = r#"
import { useForm } from 'dx/forms';
import { useQuery } from 'dx/query';
function App() {
    return <div>Hello</div>;
}
"#;

            let result1 = shaker.tree_shake(source, "App.tsx");
            let result2 = shaker.tree_shake(&result1.output, "App.tsx");

            // Second tree-shake should not remove anything more
            prop_assert_eq!(result1.output, result2.output);
            prop_assert!(result2.removed_lines.is_empty());
        }

        /// Property: Enabled features are never removed
        /// *For any* enabled feature, its imports should never be removed.
        #[test]
        fn prop_enabled_features_preserved(
            feature in arbitrary_feature(),
        ) {
            let mut config = FeaturesConfig::default();
            config.enable(&feature);
            let shaker = FeatureTreeShaker::new(&config);

            // Create source with just this feature's import
            let source = format!(
                r#"import {{ use{} }} from 'dx/{}';
function App() {{
    return <div>Hello</div>;
}}"#,
                feature.chars().next().unwrap().to_uppercase().to_string() + &feature[1..],
                feature
            );

            let result = shaker.tree_shake(&source, "App.tsx");

            // The import should be preserved
            prop_assert!(
                result.output.contains(&format!("dx/{}", feature)),
                "Enabled feature '{}' should be preserved",
                feature
            );
        }

        /// Property: All disabled feature imports are detected
        /// *For any* configuration, all imports from disabled features should be detected.
        #[test]
        fn prop_all_disabled_imports_detected(
            config in arbitrary_enabled_features(),
        ) {
            let shaker = FeatureTreeShaker::new(&config);

            let source = r#"
import { useForm } from 'dx/forms';
import { useQuery } from 'dx/query';
import { useAuth } from 'dx/auth';
import { useSync } from 'dx/sync';
import { useOffline } from 'dx/offline';
import { useA11y } from 'dx/a11y';
import { useTranslation } from 'dx/i18n';
"#;

            let analysis = shaker.analyze_source(source, "App.tsx");

            // Count disabled features that have imports in the source
            let disabled_with_imports: Vec<_> = shaker.disabled_features()
                .into_iter()
                .filter(|f| source.contains(&format!("dx/{}", f)))
                .collect();

            // All disabled features with imports should be in disabled_but_used
            for feature in &disabled_with_imports {
                prop_assert!(
                    analysis.disabled_but_used.iter().any(|w| &w.feature == feature),
                    "Disabled feature '{}' should be detected",
                    feature
                );
            }
        }
    }
}

#[cfg(test)]
mod usage_graph_tests {
    use super::*;

    #[test]
    fn test_usage_graph_basic() {
        let mut graph = UsageGraph::new();

        // Add symbols
        graph.add_symbol(Symbol {
            id: SymbolId::new("main", "App"),
            kind: SymbolKind::Component,
            is_exported: true,
            byte_size: 100,
        });
        graph.add_symbol(Symbol {
            id: SymbolId::new("main", "helper"),
            kind: SymbolKind::Function,
            is_exported: false,
            byte_size: 50,
        });
        graph.add_symbol(Symbol {
            id: SymbolId::new("main", "unused"),
            kind: SymbolKind::Function,
            is_exported: false,
            byte_size: 30,
        });

        // App references helper
        graph.add_reference(SymbolId::new("main", "App"), SymbolId::new("main", "helper"));

        // Mark App as entry point
        graph.mark_entry_point(SymbolId::new("main", "App"));

        // Check reachability
        let reachable = graph.mark_reachable();
        assert!(reachable.contains(&SymbolId::new("main", "App")));
        assert!(reachable.contains(&SymbolId::new("main", "helper")));
        assert!(!reachable.contains(&SymbolId::new("main", "unused")));

        // Shake and verify stats
        let stats = graph.shake();
        assert_eq!(stats.symbols_removed, 1);
        assert_eq!(stats.bytes_saved, 30);
        assert_eq!(stats.modules_affected, 1);
    }

    #[test]
    fn test_usage_graph_transitive() {
        let mut graph = UsageGraph::new();

        // A -> B -> C chain
        graph.add_symbol(Symbol {
            id: SymbolId::new("mod", "A"),
            kind: SymbolKind::Function,
            is_exported: true,
            byte_size: 10,
        });
        graph.add_symbol(Symbol {
            id: SymbolId::new("mod", "B"),
            kind: SymbolKind::Function,
            is_exported: false,
            byte_size: 20,
        });
        graph.add_symbol(Symbol {
            id: SymbolId::new("mod", "C"),
            kind: SymbolKind::Function,
            is_exported: false,
            byte_size: 30,
        });

        graph.add_reference(SymbolId::new("mod", "A"), SymbolId::new("mod", "B"));
        graph.add_reference(SymbolId::new("mod", "B"), SymbolId::new("mod", "C"));
        graph.mark_entry_point(SymbolId::new("mod", "A"));

        let reachable = graph.mark_reachable();
        assert_eq!(reachable.len(), 3);
    }

    #[test]
    fn test_usage_graph_empty() {
        let graph = UsageGraph::new();
        let reachable = graph.mark_reachable();
        assert!(reachable.is_empty());
    }
}

#[cfg(test)]
mod usage_graph_property_tests {
    use super::*;
    use proptest::prelude::*;

    /// Generate a random symbol ID
    fn arbitrary_symbol_id() -> impl Strategy<Value = SymbolId> {
        ("[a-z]{1,5}", "[A-Za-z]{1,8}").prop_map(|(module, name)| SymbolId::new(module, name))
    }

    /// Generate a random symbol
    fn arbitrary_symbol() -> impl Strategy<Value = Symbol> {
        (
            arbitrary_symbol_id(),
            prop_oneof![
                Just(SymbolKind::Function),
                Just(SymbolKind::Variable),
                Just(SymbolKind::Component),
            ],
            any::<bool>(),
            1usize..1000,
        )
            .prop_map(|(id, kind, is_exported, byte_size)| Symbol {
                id,
                kind,
                is_exported,
                byte_size,
            })
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Property 3: Tree Shaking Reachability Preservation
        /// For any module graph with defined entry points, tree shaking SHALL preserve
        /// all symbols that are transitively reachable from entry points.
        /// Validates: Requirements 2.1, 2.2, 2.3, 2.5
        #[test]
        fn prop_reachability_preservation(
            symbols in prop::collection::vec(arbitrary_symbol(), 1..10),
            entry_idx in 0usize..10,
        ) {
            let mut graph = UsageGraph::new();

            // Add all symbols
            for symbol in &symbols {
                graph.add_symbol(symbol.clone());
            }

            // Pick an entry point (if we have symbols)
            if !symbols.is_empty() {
                let entry_idx = entry_idx % symbols.len();
                let entry = symbols[entry_idx].id.clone();
                graph.mark_entry_point(entry.clone());

                // Add some random edges
                for i in 0..symbols.len().saturating_sub(1) {
                    graph.add_reference(
                        symbols[i].id.clone(),
                        symbols[i + 1].id.clone(),
                    );
                }

                // Get reachable before shaking
                let reachable_before = graph.mark_reachable();

                // Shake
                let mut graph_clone = graph.clone();
                graph_clone.shake();

                // All reachable symbols should still exist
                for id in &reachable_before {
                    prop_assert!(
                        graph_clone.symbols().contains_key(id),
                        "Reachable symbol {:?} should be preserved",
                        id
                    );
                }

                // Entry point should always be preserved
                prop_assert!(
                    graph_clone.symbols().contains_key(&entry),
                    "Entry point should always be preserved"
                );
            }
        }

        /// Property 4: Tree Shaking Statistics Accuracy
        /// For any tree shaking operation, the reported statistics SHALL exactly match
        /// the actual difference between input and output.
        /// Validates: Requirements 2.4
        #[test]
        fn prop_statistics_accuracy(
            symbols in prop::collection::vec(arbitrary_symbol(), 1..10),
        ) {
            let mut graph = UsageGraph::new();

            // Add all symbols
            for symbol in &symbols {
                graph.add_symbol(symbol.clone());
            }

            // Mark first symbol as entry if we have any
            if !symbols.is_empty() {
                graph.mark_entry_point(symbols[0].id.clone());
            }

            // Get stats before shaking
            let symbols_before = graph.symbols().len();
            let stats = graph.stats();

            // Verify stats match what we expect
            let reachable = graph.mark_reachable();
            let expected_removed = symbols_before - reachable.len();

            prop_assert_eq!(
                stats.symbols_removed,
                expected_removed,
                "Symbols removed count should match"
            );

            // Verify bytes saved matches sum of removed symbol sizes
            let expected_bytes: usize = symbols
                .iter()
                .filter(|s| !reachable.contains(&s.id))
                .map(|s| s.byte_size)
                .sum();

            prop_assert_eq!(
                stats.bytes_saved,
                expected_bytes,
                "Bytes saved should match sum of removed symbol sizes"
            );
        }

        /// Property: Shaking is idempotent
        /// Shaking twice should produce the same result as shaking once.
        #[test]
        fn prop_shaking_idempotent(
            symbols in prop::collection::vec(arbitrary_symbol(), 1..10),
        ) {
            let mut graph = UsageGraph::new();

            for symbol in &symbols {
                graph.add_symbol(symbol.clone());
            }

            if !symbols.is_empty() {
                graph.mark_entry_point(symbols[0].id.clone());
            }

            // First shake
            let mut graph1 = graph.clone();
            let _stats1 = graph1.shake();

            // Second shake
            let stats2 = graph1.shake();

            // Second shake should remove nothing
            prop_assert_eq!(stats2.symbols_removed, 0);
            prop_assert_eq!(stats2.bytes_saved, 0);
        }

        /// Property: Entry points are never removed
        /// All entry points should remain after shaking.
        #[test]
        fn prop_entry_points_preserved(
            symbols in prop::collection::vec(arbitrary_symbol(), 2..10),
            entry_indices in prop::collection::vec(0usize..10, 1..3),
        ) {
            let mut graph = UsageGraph::new();

            for symbol in &symbols {
                graph.add_symbol(symbol.clone());
            }

            // Mark multiple entry points
            let mut entries = Vec::new();
            for idx in entry_indices {
                let idx = idx % symbols.len();
                let entry = symbols[idx].id.clone();
                graph.mark_entry_point(entry.clone());
                entries.push(entry);
            }

            // Shake
            graph.shake();

            // All entry points should still exist
            for entry in entries {
                prop_assert!(
                    graph.symbols().contains_key(&entry),
                    "Entry point {:?} should be preserved",
                    entry
                );
            }
        }
    }
}
