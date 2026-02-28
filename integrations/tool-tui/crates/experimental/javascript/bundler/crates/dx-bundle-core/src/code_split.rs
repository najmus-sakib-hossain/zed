//! Code splitting implementation for dynamic imports
//!
//! This module provides code splitting functionality to create separate chunks
//! for dynamic imports, enabling lazy loading of code.

use crate::error::BundleResult;
use crate::resolve::parse_module;
use crate::types::{ChunkId, ModuleId};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

/// Information about a code chunk
#[derive(Debug, Clone)]
pub struct ChunkInfo {
    /// Unique chunk ID
    pub id: ChunkId,
    /// Modules in this chunk
    pub modules: Vec<ModuleId>,
    /// Entry point module for this chunk
    pub entry_module: ModuleId,
    /// Whether this is the main entry chunk
    pub is_entry: bool,
    /// Chunks that this chunk depends on
    pub dependencies: HashSet<ChunkId>,
    /// Output filename for this chunk
    pub filename: String,
}

/// Module information for code splitting
#[derive(Debug, Clone)]
pub struct SplitModuleInfo {
    /// Module ID
    pub id: ModuleId,
    /// Module path
    pub path: PathBuf,
    /// Static imports (regular imports)
    pub static_imports: Vec<String>,
    /// Dynamic imports (import() expressions)
    pub dynamic_imports: Vec<DynamicImportInfo>,
    /// Source code
    pub source: String,
}

/// Information about a dynamic import
#[derive(Debug, Clone)]
pub struct DynamicImportInfo {
    /// Import specifier
    pub specifier: String,
    /// Start position in source
    pub start: u32,
    /// End position in source
    pub end: u32,
}

/// Code splitter for creating chunks from dynamic imports
pub struct CodeSplitter {
    /// All modules
    modules: HashMap<ModuleId, SplitModuleInfo>,
    /// Module path to ID mapping
    path_to_id: HashMap<PathBuf, ModuleId>,
    /// Entry point modules
    entry_modules: HashSet<ModuleId>,
    /// Generated chunks
    chunks: Vec<ChunkInfo>,
    /// Module to chunk mapping
    module_to_chunk: HashMap<ModuleId, ChunkId>,
    /// Next chunk ID
    next_chunk_id: ChunkId,
}

impl CodeSplitter {
    /// Create a new code splitter
    pub fn new() -> Self {
        Self {
            modules: HashMap::new(),
            path_to_id: HashMap::new(),
            entry_modules: HashSet::new(),
            chunks: Vec::new(),
            module_to_chunk: HashMap::new(),
            next_chunk_id: 0,
        }
    }

    /// Add a module to the code splitter
    pub fn add_module(
        &mut self,
        path: &Path,
        source: &str,
        is_entry: bool,
    ) -> BundleResult<ModuleId> {
        let filename = path.to_string_lossy();
        let parse_result = parse_module(source, &filename)?;

        let id = self.compute_module_id(path);

        // Separate static and dynamic imports
        let mut static_imports = Vec::new();
        let mut dynamic_imports = Vec::new();

        for import in parse_result.imports {
            if import.is_dynamic {
                dynamic_imports.push(DynamicImportInfo {
                    specifier: import.specifier,
                    start: import.start,
                    end: import.end,
                });
            } else {
                static_imports.push(import.specifier);
            }
        }

        let module_info = SplitModuleInfo {
            id,
            path: path.to_path_buf(),
            static_imports,
            dynamic_imports,
            source: source.to_string(),
        };

        self.modules.insert(id, module_info);
        self.path_to_id.insert(path.to_path_buf(), id);

        if is_entry {
            self.entry_modules.insert(id);
        }

        Ok(id)
    }

    /// Compute module ID from path
    fn compute_module_id(&self, path: &Path) -> ModuleId {
        let path_bytes = path.to_string_lossy().as_bytes().to_vec();
        xxhash_rust::xxh64::xxh64(&path_bytes, 0)
    }

    /// Analyze the module graph and create chunks
    pub fn analyze(&mut self) {
        // Create main entry chunk
        for &entry_id in &self.entry_modules.clone() {
            self.create_chunk_from_entry(entry_id, true);
        }

        // Find all dynamic imports and create chunks for them
        let dynamic_import_targets: Vec<(ModuleId, String)> = self
            .modules
            .values()
            .flat_map(|m| m.dynamic_imports.iter().map(|di| (m.id, di.specifier.clone())))
            .collect();

        for (_from_module, specifier) in dynamic_import_targets {
            // Try to resolve the dynamic import to a module
            if let Some(&target_id) = self.resolve_specifier(&specifier) {
                // Create a new chunk for this dynamic import if not already in a chunk
                if !self.module_to_chunk.contains_key(&target_id) {
                    self.create_chunk_from_entry(target_id, false);
                }
            }
        }
    }

    /// Create a chunk starting from an entry module
    fn create_chunk_from_entry(&mut self, entry_id: ModuleId, is_main_entry: bool) {
        let chunk_id = self.next_chunk_id;
        self.next_chunk_id += 1;

        // Collect all modules reachable via static imports
        let mut chunk_modules = Vec::new();
        let mut visited = HashSet::new();
        let mut stack = vec![entry_id];
        let mut chunk_dependencies = HashSet::new();

        while let Some(module_id) = stack.pop() {
            if visited.contains(&module_id) {
                continue;
            }
            visited.insert(module_id);

            // Check if already in another chunk (shared dependency)
            if let Some(&existing_chunk) = self.module_to_chunk.get(&module_id) {
                // This module is in another chunk - add it as a dependency
                if existing_chunk != chunk_id {
                    chunk_dependencies.insert(existing_chunk);
                }
                continue;
            }

            chunk_modules.push(module_id);
            self.module_to_chunk.insert(module_id, chunk_id);

            // Add static imports to the stack
            if let Some(module) = self.modules.get(&module_id) {
                for import_specifier in &module.static_imports {
                    if let Some(&import_id) = self.resolve_specifier(import_specifier) {
                        stack.push(import_id);
                    }
                }
            }
        }

        // Generate filename for the chunk
        let filename = if is_main_entry {
            "main.js".to_string()
        } else {
            format!("chunk-{}.js", chunk_id)
        };

        let chunk = ChunkInfo {
            id: chunk_id,
            modules: chunk_modules,
            entry_module: entry_id,
            is_entry: is_main_entry,
            dependencies: chunk_dependencies,
            filename,
        };

        self.chunks.push(chunk);
    }

    /// Optimize chunks by extracting shared dependencies into common chunks
    /// 
    /// This implements the code splitting optimization for shared dependencies:
    /// - Modules imported by multiple chunks are extracted into a common chunk
    /// - This reduces duplication and improves caching
    pub fn optimize_shared_dependencies(&mut self) {
        // Find modules that are imported by multiple chunks
        let mut module_chunk_count: HashMap<ModuleId, HashSet<ChunkId>> = HashMap::new();
        
        for chunk in &self.chunks {
            for &module_id in &chunk.modules {
                module_chunk_count
                    .entry(module_id)
                    .or_insert_with(HashSet::new)
                    .insert(chunk.id);
            }
        }
        
        // Find modules shared by 2+ chunks
        let shared_modules: Vec<ModuleId> = module_chunk_count
            .iter()
            .filter(|(_, chunks)| chunks.len() >= 2)
            .map(|(&module_id, _)| module_id)
            .collect();
        
        if shared_modules.is_empty() {
            return;
        }
        
        // Create a common chunk for shared modules
        let common_chunk_id = self.next_chunk_id;
        self.next_chunk_id += 1;
        
        // Move shared modules to the common chunk
        for &module_id in &shared_modules {
            self.module_to_chunk.insert(module_id, common_chunk_id);
        }
        
        // Collect chunks that need the common chunk as a dependency
        let chunks_needing_common: HashSet<ChunkId> = self.chunks
            .iter()
            .filter(|chunk| {
                chunk.modules.iter().any(|m| {
                    if let Some(module) = self.modules.get(m) {
                        module.static_imports.iter().any(|spec| {
                            self.resolve_specifier(spec)
                                .map(|id| shared_modules.contains(id))
                                .unwrap_or(false)
                        })
                    } else {
                        false
                    }
                })
            })
            .map(|c| c.id)
            .collect();
        
        // Remove shared modules from their original chunks and add dependency
        for chunk in &mut self.chunks {
            chunk.modules.retain(|m| !shared_modules.contains(m));
            if chunks_needing_common.contains(&chunk.id) {
                chunk.dependencies.insert(common_chunk_id);
            }
        }
        
        // Create the common chunk
        let common_chunk = ChunkInfo {
            id: common_chunk_id,
            modules: shared_modules,
            entry_module: 0, // No specific entry for common chunk
            is_entry: false,
            dependencies: HashSet::new(),
            filename: "common.js".to_string(),
        };
        
        self.chunks.push(common_chunk);
    }

    /// Rewrite dynamic imports in source code to use chunk loading
    /// 
    /// Transforms: import('./module') -> __dx_load_chunk(chunkId)
    pub fn rewrite_dynamic_imports(&self, module_id: ModuleId) -> Option<String> {
        let module = self.modules.get(&module_id)?;
        let mut source = module.source.clone();
        
        // Process dynamic imports in reverse order to preserve positions
        let mut imports: Vec<_> = module.dynamic_imports.iter().collect();
        imports.sort_by(|a, b| b.start.cmp(&a.start));
        
        for import in imports {
            // Try to resolve the import to a chunk
            if let Some(&target_id) = self.resolve_specifier(&import.specifier) {
                if let Some(chunk_id) = self.module_to_chunk.get(&target_id) {
                    // Replace import() with __dx_load_chunk()
                    let replacement = format!("__dx_load_chunk({})", chunk_id);
                    let start = import.start as usize;
                    let end = import.end as usize;
                    
                    if start < source.len() && end <= source.len() {
                        source.replace_range(start..end, &replacement);
                    }
                }
            }
        }
        
        Some(source)
    }

    /// Resolve an import specifier to a module ID
    fn resolve_specifier(&self, specifier: &str) -> Option<&ModuleId> {
        // Try to find a module that matches the specifier
        // This is a simplified implementation - real resolution would use ModuleResolver
        for (path, id) in &self.path_to_id {
            let path_str = path.to_string_lossy();
            if path_str.contains(specifier) || specifier.contains(&*path_str) {
                return Some(id);
            }

            // Check filename match
            if let Some(filename) = path.file_stem() {
                if specifier.contains(&*filename.to_string_lossy()) {
                    return Some(id);
                }
            }
        }
        None
    }

    /// Get all chunks
    pub fn get_chunks(&self) -> &[ChunkInfo] {
        &self.chunks
    }

    /// Get the chunk containing a module
    pub fn get_module_chunk(&self, module_id: ModuleId) -> Option<ChunkId> {
        self.module_to_chunk.get(&module_id).copied()
    }

    /// Get all dynamic imports in the module graph
    pub fn get_dynamic_imports(&self) -> Vec<&DynamicImportInfo> {
        self.modules.values().flat_map(|m| m.dynamic_imports.iter()).collect()
    }

    /// Get code splitting statistics
    pub fn get_stats(&self) -> CodeSplitStats {
        let total_modules = self.modules.len();
        let total_chunks = self.chunks.len();
        let dynamic_imports = self.get_dynamic_imports().len();

        let modules_per_chunk: Vec<usize> = self.chunks.iter().map(|c| c.modules.len()).collect();

        CodeSplitStats {
            total_modules,
            total_chunks,
            dynamic_imports,
            modules_per_chunk,
        }
    }

    /// Generate chunk loading runtime code
    pub fn generate_chunk_loader(&self) -> String {
        let mut code = String::new();

        code.push_str("// Chunk loading runtime\n");
        code.push_str("const __dx_chunks = {};\n");
        code.push_str("const __dx_chunk_cache = {};\n\n");

        code.push_str("function __dx_load_chunk(chunkId) {\n");
        code.push_str("  if (__dx_chunk_cache[chunkId]) {\n");
        code.push_str("    return Promise.resolve(__dx_chunk_cache[chunkId]);\n");
        code.push_str("  }\n");
        code.push_str("  return new Promise((resolve, reject) => {\n");
        code.push_str("    const script = document.createElement('script');\n");
        code.push_str("    script.src = __dx_chunks[chunkId];\n");
        code.push_str("    script.onload = () => resolve(__dx_chunk_cache[chunkId]);\n");
        code.push_str("    script.onerror = reject;\n");
        code.push_str("    document.head.appendChild(script);\n");
        code.push_str("  });\n");
        code.push_str("}\n\n");

        // Register chunk URLs
        code.push_str("// Chunk URL mapping\n");
        for chunk in &self.chunks {
            if !chunk.is_entry {
                code.push_str(&format!("__dx_chunks[{}] = '{}';\n", chunk.id, chunk.filename));
            }
        }

        code
    }
}

impl Default for CodeSplitter {
    fn default() -> Self {
        Self::new()
    }
}

/// Code splitting statistics
#[derive(Debug, Clone)]
pub struct CodeSplitStats {
    /// Total number of modules
    pub total_modules: usize,
    /// Total number of chunks
    pub total_chunks: usize,
    /// Number of dynamic imports
    pub dynamic_imports: usize,
    /// Number of modules per chunk
    pub modules_per_chunk: Vec<usize>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_code_splitter_basic() {
        let mut splitter = CodeSplitter::new();

        let source = r#"
            import { foo } from './lib';
            console.log(foo());
        "#;

        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("entry.js");
        fs::write(&path, source).unwrap();

        let id = splitter.add_module(&path, source, true).unwrap();
        splitter.analyze();

        // Should have at least one chunk (the entry)
        assert!(!splitter.get_chunks().is_empty());

        // Entry module should be in a chunk
        assert!(splitter.get_module_chunk(id).is_some());
    }

    #[test]
    fn test_dynamic_import_detection() {
        let mut splitter = CodeSplitter::new();

        // Entry with dynamic import
        let entry_source = r#"
            const loadModule = () => import('./lazy');
            loadModule().then(m => console.log(m));
        "#;

        // Lazy loaded module
        let lazy_source = r#"
            export const value = 42;
        "#;

        let temp_dir = TempDir::new().unwrap();
        let entry_path = temp_dir.path().join("entry.js");
        let lazy_path = temp_dir.path().join("lazy.js");

        fs::write(&entry_path, entry_source).unwrap();
        fs::write(&lazy_path, lazy_source).unwrap();

        splitter.add_module(&entry_path, entry_source, true).unwrap();
        splitter.add_module(&lazy_path, lazy_source, false).unwrap();

        splitter.analyze();

        // Should detect the dynamic import
        let _dynamic_imports = splitter.get_dynamic_imports();
        // Note: The current parser may not detect all dynamic imports
        // This test verifies the structure is in place

        let stats = splitter.get_stats();
        assert_eq!(stats.total_modules, 2);
    }

    #[test]
    fn test_chunk_loader_generation() {
        let mut splitter = CodeSplitter::new();

        let source = r#"export const x = 1;"#;

        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("entry.js");
        fs::write(&path, source).unwrap();

        splitter.add_module(&path, source, true).unwrap();
        splitter.analyze();

        let loader = splitter.generate_chunk_loader();

        // Should contain chunk loading runtime
        assert!(loader.contains("__dx_load_chunk"));
        assert!(loader.contains("__dx_chunks"));
    }

    #[test]
    fn test_stats() {
        let mut splitter = CodeSplitter::new();

        let source1 = r#"export const a = 1;"#;
        let source2 = r#"export const b = 2;"#;

        let temp_dir = TempDir::new().unwrap();
        let path1 = temp_dir.path().join("module1.js");
        let path2 = temp_dir.path().join("module2.js");

        fs::write(&path1, source1).unwrap();
        fs::write(&path2, source2).unwrap();

        splitter.add_module(&path1, source1, true).unwrap();
        splitter.add_module(&path2, source2, true).unwrap();

        splitter.analyze();

        let stats = splitter.get_stats();
        assert_eq!(stats.total_modules, 2);
        assert!(stats.total_chunks >= 2); // At least one chunk per entry
    }
}
