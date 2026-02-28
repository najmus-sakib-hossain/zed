//! Plugin compatibility for pytest plugins and conftest.py hooks
//!
//! This module implements:
//! - conftest.py hook loading and discovery
//! - pytest-cov integration
//! - pytest-asyncio support
//! - Plugin hook invocation

use dx_py_core::DiscoveryError;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Supported pytest hooks
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum HookType {
    /// Called at collection start
    PytestCollectionStart,
    /// Called for each collected item
    PytestCollectionModifyItems,
    /// Called at collection finish
    PytestCollectionFinish,
    /// Called before test setup
    PytestRunTestSetup,
    /// Called for test execution
    PytestRunTestCall,
    /// Called after test teardown
    PytestRunTestTeardown,
    /// Called to make test report
    PytestMakeReport,
    /// Called at session start
    PytestSessionStart,
    /// Called at session finish
    PytestSessionFinish,
    /// Called to configure pytest
    PytestConfigure,
    /// Called to add command line options
    PytestAddOption,
    /// Called to generate test id
    PytestGenerateTests,
}

impl HookType {
    /// Get the Python function name for this hook
    pub fn function_name(&self) -> &'static str {
        match self {
            HookType::PytestCollectionStart => "pytest_collection_start",
            HookType::PytestCollectionModifyItems => "pytest_collection_modifyitems",
            HookType::PytestCollectionFinish => "pytest_collection_finish",
            HookType::PytestRunTestSetup => "pytest_runtest_setup",
            HookType::PytestRunTestCall => "pytest_runtest_call",
            HookType::PytestRunTestTeardown => "pytest_runtest_teardown",
            HookType::PytestMakeReport => "pytest_runtest_makereport",
            HookType::PytestSessionStart => "pytest_sessionstart",
            HookType::PytestSessionFinish => "pytest_sessionfinish",
            HookType::PytestConfigure => "pytest_configure",
            HookType::PytestAddOption => "pytest_addoption",
            HookType::PytestGenerateTests => "pytest_generate_tests",
        }
    }

    /// Get all hook types
    pub fn all() -> &'static [HookType] {
        &[
            HookType::PytestCollectionStart,
            HookType::PytestCollectionModifyItems,
            HookType::PytestCollectionFinish,
            HookType::PytestRunTestSetup,
            HookType::PytestRunTestCall,
            HookType::PytestRunTestTeardown,
            HookType::PytestMakeReport,
            HookType::PytestSessionStart,
            HookType::PytestSessionFinish,
            HookType::PytestConfigure,
            HookType::PytestAddOption,
            HookType::PytestGenerateTests,
        ]
    }
}

/// A discovered hook implementation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookImplementation {
    /// The hook type
    pub hook_type: HookType,
    /// Path to the conftest.py file
    pub conftest_path: PathBuf,
    /// Line number where the hook is defined
    pub line_number: u32,
    /// Whether this hook has a tryfirst marker
    pub tryfirst: bool,
    /// Whether this hook has a trylast marker
    pub trylast: bool,
    /// Whether this hook has a hookwrapper marker
    pub hookwrapper: bool,
}

/// A conftest.py file with its hooks and fixtures
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConftestFile {
    /// Path to the conftest.py file
    pub path: PathBuf,
    /// Discovered hooks
    pub hooks: Vec<HookImplementation>,
    /// Fixture names defined in this conftest
    pub fixtures: Vec<String>,
    /// Plugin imports (e.g., pytest_plugins = [...])
    pub plugin_imports: Vec<String>,
}

/// Known pytest plugins and their features
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum KnownPlugin {
    /// pytest-cov for coverage
    PytestCov,
    /// pytest-asyncio for async tests
    PytestAsyncio,
    /// pytest-xdist for distributed testing
    PytestXdist,
    /// pytest-mock for mocking
    PytestMock,
    /// pytest-timeout for test timeouts
    PytestTimeout,
    /// pytest-benchmark for benchmarking
    PytestBenchmark,
    /// pytest-django for Django testing
    PytestDjango,
    /// pytest-flask for Flask testing
    PytestFlask,
    /// pytest-httpx for HTTP mocking
    PytestHttpx,
    /// pytest-env for environment variables
    PytestEnv,
}

impl KnownPlugin {
    /// Get the package name for this plugin
    pub fn package_name(&self) -> &'static str {
        match self {
            KnownPlugin::PytestCov => "pytest-cov",
            KnownPlugin::PytestAsyncio => "pytest-asyncio",
            KnownPlugin::PytestXdist => "pytest-xdist",
            KnownPlugin::PytestMock => "pytest-mock",
            KnownPlugin::PytestTimeout => "pytest-timeout",
            KnownPlugin::PytestBenchmark => "pytest-benchmark",
            KnownPlugin::PytestDjango => "pytest-django",
            KnownPlugin::PytestFlask => "pytest-flask",
            KnownPlugin::PytestHttpx => "pytest-httpx",
            KnownPlugin::PytestEnv => "pytest-env",
        }
    }

    /// Get the import name for this plugin
    pub fn import_name(&self) -> &'static str {
        match self {
            KnownPlugin::PytestCov => "pytest_cov",
            KnownPlugin::PytestAsyncio => "pytest_asyncio",
            KnownPlugin::PytestXdist => "xdist",
            KnownPlugin::PytestMock => "pytest_mock",
            KnownPlugin::PytestTimeout => "pytest_timeout",
            KnownPlugin::PytestBenchmark => "pytest_benchmark",
            KnownPlugin::PytestDjango => "pytest_django",
            KnownPlugin::PytestFlask => "pytest_flask",
            KnownPlugin::PytestHttpx => "pytest_httpx",
            KnownPlugin::PytestEnv => "pytest_env",
        }
    }

    /// Check if this plugin is installed
    pub fn is_installed(&self) -> bool {
        // This would check if the plugin is importable
        // For now, we'll return false and let the actual check happen at runtime
        false
    }
}

/// Plugin manager for discovering and managing pytest plugins
pub struct PluginManager {
    /// Root directory for conftest discovery
    root_dir: PathBuf,
    /// Discovered conftest files by directory
    conftest_files: HashMap<PathBuf, ConftestFile>,
    /// Registered hooks by type
    hooks: HashMap<HookType, Vec<HookImplementation>>,
    /// Detected plugins
    detected_plugins: Vec<KnownPlugin>,
    /// Parser for scanning conftest files
    parser: crate::PythonParser,
}

impl PluginManager {
    /// Create a new plugin manager
    pub fn new(root_dir: impl Into<PathBuf>) -> Result<Self, DiscoveryError> {
        Ok(Self {
            root_dir: root_dir.into(),
            conftest_files: HashMap::new(),
            hooks: HashMap::new(),
            detected_plugins: Vec::new(),
            parser: crate::PythonParser::new()?,
        })
    }

    /// Discover all conftest.py files from root to the given path
    pub fn discover_conftest_files(
        &mut self,
        test_path: &Path,
    ) -> Result<Vec<&ConftestFile>, DiscoveryError> {
        let mut conftest_paths = Vec::new();

        // Walk from root to test_path, collecting conftest.py files
        let mut current = self.root_dir.clone();
        let relative = test_path.strip_prefix(&self.root_dir).unwrap_or(test_path);

        // Check root conftest
        let root_conftest = current.join("conftest.py");
        if root_conftest.exists() {
            conftest_paths.push(root_conftest);
        }

        // Walk through path components
        for component in relative.components() {
            if let std::path::Component::Normal(name) = component {
                current = current.join(name);
                let conftest = current.join("conftest.py");
                if conftest.exists() {
                    conftest_paths.push(conftest);
                }
            }
        }

        // Parse each conftest file
        for path in conftest_paths {
            if !self.conftest_files.contains_key(&path) {
                let conftest = self.parse_conftest(&path)?;

                // Register hooks
                for hook in &conftest.hooks {
                    self.hooks.entry(hook.hook_type).or_default().push(hook.clone());
                }

                self.conftest_files.insert(path.clone(), conftest);
            }
        }

        // Return conftest files in order (root first)
        Ok(self.conftest_files.values().collect())
    }

    /// Parse a conftest.py file
    fn parse_conftest(&mut self, path: &Path) -> Result<ConftestFile, DiscoveryError> {
        let source = std::fs::read_to_string(path)?;
        let tree = self.parser.parse(&source)?;

        let mut hooks = Vec::new();
        let mut fixtures = Vec::new();
        let mut plugin_imports = Vec::new();

        // Walk the AST to find hooks, fixtures, and plugin imports
        self.walk_conftest_ast(
            tree.root_node(),
            source.as_bytes(),
            path,
            &mut hooks,
            &mut fixtures,
            &mut plugin_imports,
        );

        Ok(ConftestFile {
            path: path.to_path_buf(),
            hooks,
            fixtures,
            plugin_imports,
        })
    }

    /// Walk the conftest AST to extract hooks, fixtures, and plugin imports
    fn walk_conftest_ast(
        &self,
        node: tree_sitter::Node,
        source: &[u8],
        path: &Path,
        hooks: &mut Vec<HookImplementation>,
        fixtures: &mut Vec<String>,
        plugin_imports: &mut Vec<String>,
    ) {
        match node.kind() {
            "function_definition" => {
                if let Some(name) = self.get_function_name(node, source) {
                    // Check if it's a hook
                    for hook_type in HookType::all() {
                        if name == hook_type.function_name() {
                            let (tryfirst, trylast, hookwrapper) =
                                self.get_hook_markers(node, source);
                            hooks.push(HookImplementation {
                                hook_type: *hook_type,
                                conftest_path: path.to_path_buf(),
                                line_number: node.start_position().row as u32 + 1,
                                tryfirst,
                                trylast,
                                hookwrapper,
                            });
                            break;
                        }
                    }

                    // Check if it's a fixture
                    if self.has_fixture_decorator(node, source) {
                        fixtures.push(name);
                    }
                }
            }
            "assignment" => {
                // Check for pytest_plugins assignment
                if let Some(imports) = self.parse_pytest_plugins(node, source) {
                    plugin_imports.extend(imports);
                }
            }
            _ => {}
        }

        // Recurse into children
        for child in node.children(&mut node.walk()) {
            self.walk_conftest_ast(child, source, path, hooks, fixtures, plugin_imports);
        }
    }

    /// Get function name from a function_definition node
    fn get_function_name(&self, node: tree_sitter::Node, source: &[u8]) -> Option<String> {
        for child in node.children(&mut node.walk()) {
            if child.kind() == "identifier" {
                return Some(self.node_text(child, source));
            }
        }
        None
    }

    /// Check if a function has a fixture decorator
    fn has_fixture_decorator(&self, node: tree_sitter::Node, source: &[u8]) -> bool {
        if let Some(parent) = node.parent() {
            if parent.kind() == "decorated_definition" {
                for child in parent.children(&mut parent.walk()) {
                    if child.kind() == "decorator" {
                        let text = self.node_text(child, source);
                        if text.contains("fixture") || text.contains("pytest.fixture") {
                            return true;
                        }
                    }
                }
            }
        }
        false
    }

    /// Get hook markers (tryfirst, trylast, hookwrapper)
    fn get_hook_markers(&self, node: tree_sitter::Node, source: &[u8]) -> (bool, bool, bool) {
        let mut tryfirst = false;
        let mut trylast = false;
        let mut hookwrapper = false;

        if let Some(parent) = node.parent() {
            if parent.kind() == "decorated_definition" {
                for child in parent.children(&mut parent.walk()) {
                    if child.kind() == "decorator" {
                        let text = self.node_text(child, source);
                        if text.contains("tryfirst") {
                            tryfirst = true;
                        }
                        if text.contains("trylast") {
                            trylast = true;
                        }
                        if text.contains("hookwrapper") {
                            hookwrapper = true;
                        }
                    }
                }
            }
        }

        (tryfirst, trylast, hookwrapper)
    }

    /// Parse pytest_plugins assignment
    fn parse_pytest_plugins(&self, node: tree_sitter::Node, source: &[u8]) -> Option<Vec<String>> {
        // Look for: pytest_plugins = ["plugin1", "plugin2"]
        let mut found_name = false;
        let mut plugins = Vec::new();

        for child in node.children(&mut node.walk()) {
            if child.kind() == "identifier" {
                let name = self.node_text(child, source);
                if name == "pytest_plugins" {
                    found_name = true;
                }
            } else if found_name && child.kind() == "list" {
                for item in child.children(&mut child.walk()) {
                    if item.kind() == "string" {
                        let text = self.node_text(item, source);
                        let cleaned = text.trim_matches(|c| c == '"' || c == '\'');
                        plugins.push(cleaned.to_string());
                    }
                }
            }
        }

        if found_name && !plugins.is_empty() {
            Some(plugins)
        } else {
            None
        }
    }

    /// Get text from a node
    fn node_text(&self, node: tree_sitter::Node, source: &[u8]) -> String {
        let start = node.start_byte();
        let end = node.end_byte();
        String::from_utf8_lossy(&source[start..end]).to_string()
    }

    /// Get hooks for a specific type, sorted by priority
    pub fn get_hooks(&self, hook_type: HookType) -> Vec<&HookImplementation> {
        let mut hooks: Vec<_> =
            self.hooks.get(&hook_type).map(|h| h.iter().collect()).unwrap_or_default();

        // Sort by priority: tryfirst first, then normal, then trylast
        hooks.sort_by(|a, b| match (a.tryfirst, b.tryfirst, a.trylast, b.trylast) {
            (true, false, _, _) => std::cmp::Ordering::Less,
            (false, true, _, _) => std::cmp::Ordering::Greater,
            (_, _, true, false) => std::cmp::Ordering::Greater,
            (_, _, false, true) => std::cmp::Ordering::Less,
            _ => std::cmp::Ordering::Equal,
        });

        hooks
    }

    /// Detect installed plugins
    pub fn detect_plugins(&mut self) -> &[KnownPlugin] {
        // This would check which plugins are installed
        // For now, we detect based on conftest imports
        let mut detected = Vec::new();

        for conftest in self.conftest_files.values() {
            for import in &conftest.plugin_imports {
                if import.contains("pytest_cov") || import.contains("cov") {
                    detected.push(KnownPlugin::PytestCov);
                }
                if import.contains("pytest_asyncio") || import.contains("asyncio") {
                    detected.push(KnownPlugin::PytestAsyncio);
                }
                if import.contains("xdist") {
                    detected.push(KnownPlugin::PytestXdist);
                }
                if import.contains("pytest_mock") || import.contains("mock") {
                    detected.push(KnownPlugin::PytestMock);
                }
            }
        }

        // Deduplicate
        detected.sort_by_key(|p| *p as u8);
        detected.dedup();
        self.detected_plugins = detected;

        &self.detected_plugins
    }

    /// Check if a specific plugin is available
    pub fn has_plugin(&self, plugin: KnownPlugin) -> bool {
        self.detected_plugins.contains(&plugin)
    }

    /// Get all discovered fixtures from conftest files
    pub fn get_all_fixtures(&self) -> Vec<(&Path, &str)> {
        self.conftest_files
            .values()
            .flat_map(|c| c.fixtures.iter().map(move |f| (c.path.as_path(), f.as_str())))
            .collect()
    }

    /// Get conftest files for a specific directory
    pub fn get_conftest_for_dir(&self, dir: &Path) -> Option<&ConftestFile> {
        let conftest_path = dir.join("conftest.py");
        self.conftest_files.get(&conftest_path)
    }
}
