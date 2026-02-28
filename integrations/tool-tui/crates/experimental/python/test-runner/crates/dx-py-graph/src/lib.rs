//! Dependency graph for smart change detection
//!
//! This crate builds and maintains an import graph of Python files
//! to identify which tests are affected by file changes.

pub use dx_py_core::{GraphError, TestId};

use dx_py_core::TestCase;
use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::Direction;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};
use tree_sitter::{Node, Parser};

/// Magic bytes for graph cache files
const GRAPH_MAGIC: &[u8; 4] = b"DXGR";
const GRAPH_VERSION: u16 = 1;

/// A node in the import graph representing a Python file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileNode {
    pub path: PathBuf,
    pub content_hash: [u8; 32],
    pub tests: Vec<TestId>,
}

/// Import graph for tracking file dependencies
#[derive(Debug)]
pub struct ImportGraph {
    graph: DiGraph<PathBuf, ()>,
    path_to_node: HashMap<PathBuf, NodeIndex>,
    file_hashes: HashMap<PathBuf, [u8; 32]>,
    file_tests: HashMap<PathBuf, Vec<TestId>>,
}

impl ImportGraph {
    /// Create a new empty import graph
    pub fn new() -> Self {
        Self {
            graph: DiGraph::new(),
            path_to_node: HashMap::new(),
            file_hashes: HashMap::new(),
            file_tests: HashMap::new(),
        }
    }

    /// Add a file to the graph
    pub fn add_file(&mut self, path: &Path) -> NodeIndex {
        if let Some(&idx) = self.path_to_node.get(path) {
            return idx;
        }

        let idx = self.graph.add_node(path.to_owned());
        self.path_to_node.insert(path.to_owned(), idx);
        idx
    }

    /// Add an import edge from importer to imported
    pub fn add_import(&mut self, importer: &Path, imported: &Path) {
        let from_idx = self.add_file(importer);
        let to_idx = self.add_file(imported);

        // Check if edge already exists
        if !self.graph.contains_edge(from_idx, to_idx) {
            self.graph.add_edge(from_idx, to_idx, ());
        }
    }

    /// Associate tests with a file
    pub fn set_file_tests(&mut self, path: &Path, tests: Vec<TestId>) {
        self.file_tests.insert(path.to_owned(), tests);
    }

    /// Set the content hash for a file
    pub fn set_file_hash(&mut self, path: &Path, hash: [u8; 32]) {
        self.file_hashes.insert(path.to_owned(), hash);
    }

    /// Get all files that depend on the given file (transitively)
    pub fn get_dependents(&self, path: &Path) -> HashSet<PathBuf> {
        let mut dependents = HashSet::new();

        if let Some(&start_idx) = self.path_to_node.get(path) {
            // Use BFS to find all nodes that can reach this node
            // We need to traverse incoming edges
            let mut visited = HashSet::new();
            let mut stack = vec![start_idx];

            while let Some(current) = stack.pop() {
                if visited.contains(&current) {
                    continue;
                }
                visited.insert(current);

                // Get all nodes that import this one (incoming edges)
                for neighbor in self.graph.neighbors_directed(current, Direction::Incoming) {
                    if !visited.contains(&neighbor) {
                        stack.push(neighbor);
                        if let Some(path) = self.graph.node_weight(neighbor) {
                            dependents.insert(path.clone());
                        }
                    }
                }
            }
        }

        dependents
    }

    /// Get all tests affected by changes to the given file
    pub fn get_affected_tests(&self, changed_file: &Path) -> Vec<TestId> {
        let mut affected = Vec::new();

        // Include tests from the changed file itself
        if let Some(tests) = self.file_tests.get(changed_file) {
            affected.extend(tests.iter().cloned());
        }

        // Include tests from all dependent files
        for dependent in self.get_dependents(changed_file) {
            if let Some(tests) = self.file_tests.get(&dependent) {
                affected.extend(tests.iter().cloned());
            }
        }

        affected
    }

    /// Get the number of files in the graph
    pub fn file_count(&self) -> usize {
        self.graph.node_count()
    }

    /// Get the number of import edges in the graph
    pub fn edge_count(&self) -> usize {
        self.graph.edge_count()
    }

    /// Check if a file needs re-scanning based on content hash
    pub fn needs_rescan(&self, path: &Path, current_hash: &[u8; 32]) -> bool {
        match self.file_hashes.get(path) {
            None => true,
            Some(stored_hash) => stored_hash != current_hash,
        }
    }

    /// Save the graph to a binary cache file
    pub fn save(&self, path: &Path) -> Result<(), GraphError> {
        let file = File::create(path)?;
        let mut writer = BufWriter::new(file);

        // Write magic and version
        writer.write_all(GRAPH_MAGIC)?;
        writer.write_all(&GRAPH_VERSION.to_le_bytes())?;

        // Serialize graph data
        let data = GraphData {
            nodes: self.graph.node_weights().cloned().collect(),
            edges: self
                .graph
                .edge_indices()
                .filter_map(|e| {
                    let (a, b) = self.graph.edge_endpoints(e)?;
                    Some((a.index(), b.index()))
                })
                .collect(),
            file_hashes: self.file_hashes.clone(),
            file_tests: self.file_tests.clone(),
        };

        bincode::serialize_into(&mut writer, &data)
            .map_err(|e| GraphError::CacheCorrupted(e.to_string()))?;

        writer.flush()?;
        Ok(())
    }

    /// Load the graph from a binary cache file
    pub fn load(path: &Path) -> Result<Self, GraphError> {
        let file = File::open(path)?;
        let mut reader = BufReader::new(file);

        // Read and verify magic
        let mut magic = [0u8; 4];
        reader.read_exact(&mut magic)?;
        if &magic != GRAPH_MAGIC {
            return Err(GraphError::CacheCorrupted("Invalid magic bytes".into()));
        }

        // Read version
        let mut version_bytes = [0u8; 2];
        reader.read_exact(&mut version_bytes)?;
        let version = u16::from_le_bytes(version_bytes);
        if version != GRAPH_VERSION {
            return Err(GraphError::CacheCorrupted(format!("Unsupported version: {}", version)));
        }

        // Deserialize graph data
        let data: GraphData = bincode::deserialize_from(reader)
            .map_err(|e| GraphError::CacheCorrupted(e.to_string()))?;

        // Rebuild graph
        let mut graph = DiGraph::new();
        let mut path_to_node = HashMap::new();

        for node_path in &data.nodes {
            let idx = graph.add_node(node_path.clone());
            path_to_node.insert(node_path.clone(), idx);
        }

        for (from, to) in &data.edges {
            if *from < data.nodes.len() && *to < data.nodes.len() {
                let from_idx = NodeIndex::new(*from);
                let to_idx = NodeIndex::new(*to);
                graph.add_edge(from_idx, to_idx, ());
            }
        }

        Ok(Self {
            graph,
            path_to_node,
            file_hashes: data.file_hashes,
            file_tests: data.file_tests,
        })
    }
}

impl Default for ImportGraph {
    fn default() -> Self {
        Self::new()
    }
}

/// Serializable graph data
#[derive(Serialize, Deserialize)]
struct GraphData {
    nodes: Vec<PathBuf>,
    edges: Vec<(usize, usize)>,
    file_hashes: HashMap<PathBuf, [u8; 32]>,
    file_tests: HashMap<PathBuf, Vec<TestId>>,
}

/// Parser for extracting imports from Python files
pub struct ImportExtractor {
    parser: Parser,
}

impl ImportExtractor {
    /// Create a new import extractor
    pub fn new() -> Result<Self, GraphError> {
        let mut parser = Parser::new();
        let language = tree_sitter_python::language();
        parser
            .set_language(language)
            .map_err(|e| GraphError::ParseError(e.to_string()))?;
        Ok(Self { parser })
    }

    /// Extract imports from Python source code
    pub fn extract_imports(&mut self, source: &str) -> Result<Vec<ImportInfo>, GraphError> {
        let tree = self
            .parser
            .parse(source, None)
            .ok_or_else(|| GraphError::ParseError("Failed to parse source".into()))?;

        let mut imports = Vec::new();
        self.walk_tree(tree.root_node(), source.as_bytes(), &mut imports);
        Ok(imports)
    }

    fn walk_tree(&self, node: Node, source: &[u8], imports: &mut Vec<ImportInfo>) {
        match node.kind() {
            "import_statement" => {
                if let Some(import) = self.parse_import(node, source) {
                    imports.push(import);
                }
            }
            "import_from_statement" => {
                if let Some(import) = self.parse_import_from(node, source) {
                    imports.push(import);
                }
            }
            _ => {
                for child in node.children(&mut node.walk()) {
                    self.walk_tree(child, source, imports);
                }
            }
        }
    }

    fn parse_import(&self, node: Node, source: &[u8]) -> Option<ImportInfo> {
        for child in node.children(&mut node.walk()) {
            if child.kind() == "dotted_name" {
                let module = self.node_text(child, source);
                return Some(ImportInfo {
                    module,
                    is_relative: false,
                    level: 0,
                });
            }
        }
        None
    }

    fn parse_import_from(&self, node: Node, source: &[u8]) -> Option<ImportInfo> {
        let mut module = String::new();
        let mut level = 0;
        let mut found_module = false;

        for child in node.children(&mut node.walk()) {
            match child.kind() {
                "import_prefix" => {
                    // Count dots for relative imports
                    let text = self.node_text(child, source);
                    level = text.chars().filter(|c| *c == '.').count();
                }
                "dotted_name" => {
                    if !found_module {
                        module = self.node_text(child, source);
                        found_module = true;
                    }
                }
                "relative_import" => {
                    // Handle relative imports
                    for subchild in child.children(&mut child.walk()) {
                        match subchild.kind() {
                            "import_prefix" => {
                                let text = self.node_text(subchild, source);
                                level = text.chars().filter(|c| *c == '.').count();
                            }
                            "dotted_name" => {
                                module = self.node_text(subchild, source);
                            }
                            _ => {}
                        }
                    }
                }
                _ => {}
            }
        }

        Some(ImportInfo {
            module,
            is_relative: level > 0,
            level,
        })
    }

    fn node_text(&self, node: Node, source: &[u8]) -> String {
        let start = node.start_byte();
        let end = node.end_byte();
        String::from_utf8_lossy(&source[start..end]).to_string()
    }
}

impl Default for ImportExtractor {
    fn default() -> Self {
        Self::new().expect("Failed to create ImportExtractor")
    }
}

/// Information about an import statement
#[derive(Debug, Clone)]
pub struct ImportInfo {
    pub module: String,
    pub is_relative: bool,
    pub level: usize,
}

impl ImportInfo {
    /// Resolve a relative import to an absolute module path
    pub fn resolve(&self, current_file: &Path) -> Option<PathBuf> {
        if self.is_relative {
            let mut base = current_file.parent()?.to_owned();
            for _ in 1..self.level {
                base = base.parent()?.to_owned();
            }
            if self.module.is_empty() {
                Some(base.join("__init__.py"))
            } else {
                let module_path = self.module.replace('.', "/");
                Some(base.join(format!("{}.py", module_path)))
            }
        } else {
            let module_path = self.module.replace('.', "/");
            Some(PathBuf::from(format!("{}.py", module_path)))
        }
    }
}

/// Builder for constructing an ImportGraph from a directory
pub struct ImportGraphBuilder {
    graph: ImportGraph,
    extractor: ImportExtractor,
    root: PathBuf,
}

impl ImportGraphBuilder {
    pub fn new(root: impl Into<PathBuf>) -> Result<Self, GraphError> {
        Ok(Self {
            graph: ImportGraph::new(),
            extractor: ImportExtractor::new()?,
            root: root.into(),
        })
    }

    /// Add a file and its imports to the graph
    pub fn add_file(&mut self, path: &Path, tests: Vec<TestCase>) -> Result<(), GraphError> {
        let content = std::fs::read(path)?;
        let hash = blake3::hash(&content);
        let source = String::from_utf8_lossy(&content);

        self.graph.add_file(path);
        self.graph.set_file_hash(path, *hash.as_bytes());
        self.graph.set_file_tests(path, tests.iter().map(|t| t.id).collect());

        let imports = self.extractor.extract_imports(&source)?;
        for import in imports {
            if let Some(imported_path) = import.resolve(path) {
                // Try to find the actual file
                let full_path = self.root.join(&imported_path);
                if full_path.exists() {
                    self.graph.add_import(path, &full_path);
                }
            }
        }

        Ok(())
    }

    /// Build the final graph
    pub fn build(self) -> ImportGraph {
        self.graph
    }
}

/// Dependency graph for watch mode
///
/// This provides a higher-level API for tracking file dependencies
/// and computing affected tests when files change.
#[derive(Debug)]
pub struct DependencyGraph {
    /// The underlying import graph
    import_graph: ImportGraph,
    /// Reverse dependency map: file -> files that depend on it
    reverse_deps: HashMap<PathBuf, HashSet<PathBuf>>,
    /// Test files in the project
    test_files: HashSet<PathBuf>,
}

impl DependencyGraph {
    /// Create a new dependency graph
    pub fn new() -> Self {
        Self {
            import_graph: ImportGraph::new(),
            reverse_deps: HashMap::new(),
            test_files: HashSet::new(),
        }
    }

    /// Create from an existing import graph
    pub fn from_import_graph(import_graph: ImportGraph) -> Self {
        let mut graph = Self {
            import_graph,
            reverse_deps: HashMap::new(),
            test_files: HashSet::new(),
        };
        graph.rebuild_reverse_deps();
        graph
    }

    /// Add a file to the graph
    pub fn add_file(&mut self, path: &Path, is_test_file: bool) {
        self.import_graph.add_file(path);
        if is_test_file {
            self.test_files.insert(path.to_owned());
        }
    }

    /// Add an import relationship
    pub fn add_import(&mut self, importer: &Path, imported: &Path) {
        self.import_graph.add_import(importer, imported);
        // Update reverse deps
        self.reverse_deps
            .entry(imported.to_owned())
            .or_default()
            .insert(importer.to_owned());
    }

    /// Set tests for a file
    pub fn set_file_tests(&mut self, path: &Path, tests: Vec<TestId>) {
        self.import_graph.set_file_tests(path, tests);
    }

    /// Set the content hash for a file
    pub fn set_file_hash(&mut self, path: &Path, hash: [u8; 32]) {
        self.import_graph.set_file_hash(path, hash);
    }

    /// Check if a file needs re-scanning
    pub fn needs_rescan(&self, path: &Path, current_hash: &[u8; 32]) -> bool {
        self.import_graph.needs_rescan(path, current_hash)
    }

    /// Get all files that directly depend on the given file
    pub fn get_direct_dependents(&self, path: &Path) -> HashSet<PathBuf> {
        self.reverse_deps.get(path).cloned().unwrap_or_default()
    }

    /// Get all files that transitively depend on the given file
    pub fn get_transitive_dependents(&self, path: &Path) -> HashSet<PathBuf> {
        self.import_graph.get_dependents(path)
    }

    /// Get all test files that are affected by changes to the given file
    pub fn get_affected_test_files(&self, changed_file: &Path) -> HashSet<PathBuf> {
        let mut affected = HashSet::new();

        // If the changed file is a test file, include it
        if self.test_files.contains(changed_file) {
            affected.insert(changed_file.to_owned());
        }

        // Get all transitive dependents
        let dependents = self.get_transitive_dependents(changed_file);

        // Filter to only test files
        for dep in dependents {
            if self.test_files.contains(&dep) {
                affected.insert(dep);
            }
        }

        affected
    }

    /// Get all tests affected by changes to the given file
    pub fn get_affected_tests(&self, changed_file: &Path) -> Vec<TestId> {
        self.import_graph.get_affected_tests(changed_file)
    }

    /// Get all tests affected by changes to multiple files
    pub fn get_affected_tests_batch(&self, changed_files: &[PathBuf]) -> Vec<TestId> {
        let mut affected = HashSet::new();

        for file in changed_files {
            for test_id in self.import_graph.get_affected_tests(file) {
                affected.insert(test_id);
            }
        }

        affected.into_iter().collect()
    }

    /// Rebuild the reverse dependency map from the import graph
    fn rebuild_reverse_deps(&mut self) {
        self.reverse_deps.clear();
        // This would require access to the graph edges, which we don't have directly
        // For now, we build it incrementally via add_import
    }

    /// Get the number of files in the graph
    pub fn file_count(&self) -> usize {
        self.import_graph.file_count()
    }

    /// Get the number of test files
    pub fn test_file_count(&self) -> usize {
        self.test_files.len()
    }

    /// Check if a file is a test file
    pub fn is_test_file(&self, path: &Path) -> bool {
        self.test_files.contains(path)
    }

    /// Get the underlying import graph
    pub fn import_graph(&self) -> &ImportGraph {
        &self.import_graph
    }

    /// Save the graph to a cache file
    pub fn save(&self, path: &Path) -> Result<(), GraphError> {
        self.import_graph.save(path)
    }

    /// Load the graph from a cache file
    pub fn load(path: &Path) -> Result<Self, GraphError> {
        let import_graph = ImportGraph::load(path)?;
        Ok(Self::from_import_graph(import_graph))
    }
}

impl Default for DependencyGraph {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder for constructing a DependencyGraph from a directory
pub struct DependencyGraphBuilder {
    graph: DependencyGraph,
    extractor: ImportExtractor,
    root: PathBuf,
}

impl DependencyGraphBuilder {
    /// Create a new builder
    pub fn new(root: impl Into<PathBuf>) -> Result<Self, GraphError> {
        Ok(Self {
            graph: DependencyGraph::new(),
            extractor: ImportExtractor::new()?,
            root: root.into(),
        })
    }

    /// Add a file and its imports to the graph
    pub fn add_file(&mut self, path: &Path, tests: Vec<TestCase>) -> Result<(), GraphError> {
        let content = std::fs::read(path)?;
        let hash = blake3::hash(&content);
        let source = String::from_utf8_lossy(&content);

        // Determine if this is a test file
        let is_test_file = path
            .file_name()
            .and_then(|n| n.to_str())
            .map(|n| n.starts_with("test_") || n.ends_with("_test.py"))
            .unwrap_or(false);

        self.graph.add_file(path, is_test_file);
        self.graph.set_file_hash(path, *hash.as_bytes());
        self.graph.set_file_tests(path, tests.iter().map(|t| t.id).collect());

        let imports = self.extractor.extract_imports(&source)?;
        for import in imports {
            if let Some(imported_path) = import.resolve(path) {
                // Try to find the actual file
                let full_path = self.root.join(&imported_path);
                if full_path.exists() {
                    self.graph.add_import(path, &full_path);
                }
            }
        }

        Ok(())
    }

    /// Scan a directory and add all Python files
    pub fn scan_directory(&mut self, dir: &Path) -> Result<(), GraphError> {
        for entry in walkdir::WalkDir::new(dir).into_iter().filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.extension().is_some_and(|e| e == "py") {
                // Skip __pycache__ and hidden directories
                if path.components().any(|c| {
                    c.as_os_str()
                        .to_str()
                        .map(|s| s.starts_with('.') || s == "__pycache__")
                        .unwrap_or(false)
                }) {
                    continue;
                }

                // Add file with empty tests (tests will be discovered separately)
                if let Err(e) = self.add_file(path, Vec::new()) {
                    // Log but continue on parse errors
                    eprintln!("Warning: Failed to parse {}: {}", path.display(), e);
                }
            }
        }
        Ok(())
    }

    /// Build the final graph
    pub fn build(self) -> DependencyGraph {
        self.graph
    }
}

#[cfg(test)]
mod tests;
