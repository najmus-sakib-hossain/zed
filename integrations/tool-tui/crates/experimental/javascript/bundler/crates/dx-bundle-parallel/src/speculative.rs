//! Speculative parallel bundler
//!
//! Resolves and transforms modules in parallel with work-stealing

use crate::{ParallelBundle, ParallelOptions};
use dashmap::DashMap;
use dx_bundle_cache::WarmCache;
use dx_bundle_core::error::{BundleError, BundleResult};
use dx_bundle_core::hash::PathHasher;
use dx_bundle_core::{
    BundleConfig, ContentHash, ImportMap, ModuleFormat, ModuleId, ResolvedModule, TransformedModule,
};
use dx_bundle_pipeline::{transform, TransformOptions};
use dx_bundle_scanner::scan_source;
use rayon::prelude::*;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

/// Speculative bundler - processes modules in parallel
pub struct SpeculativeBundler {
    /// Configuration
    config: Arc<BundleConfig>,
    /// Resolved modules (thread-safe)
    resolved: Arc<DashMap<ModuleId, ResolvedModule>>,
    /// Transformed modules (thread-safe)
    transformed: Arc<DashMap<ModuleId, TransformedModule>>,
    /// Import map
    imports: Arc<ImportMap>,
    /// Warm cache
    cache: Arc<Option<WarmCache>>,
    /// Module queue for processing
    pending: Arc<DashMap<ModuleId, PathBuf>>,
}

impl SpeculativeBundler {
    /// Create new speculative bundler
    pub fn new(config: BundleConfig, cache: Option<WarmCache>) -> Self {
        Self {
            config: Arc::new(config),
            resolved: Arc::new(DashMap::new()),
            transformed: Arc::new(DashMap::new()),
            imports: Arc::new(ImportMap::new()),
            cache: Arc::new(cache),
            pending: Arc::new(DashMap::new()),
        }
    }

    /// Bundle with speculative parallelism
    pub fn bundle(
        &self,
        entries: &[PathBuf],
        options: &ParallelOptions,
    ) -> BundleResult<ParallelBundle> {
        let start = Instant::now();

        // Configure thread pool
        let thread_count = options.thread_count();
        rayon::ThreadPoolBuilder::new()
            .num_threads(thread_count)
            .build()
            .map_err(|e| BundleError::transform_error(e.to_string()))?
            .install(|| self.bundle_parallel(entries, options))?;

        let elapsed = start.elapsed();

        // Collect results
        let modules: Vec<TransformedModule> =
            self.transformed.iter().map(|entry| entry.value().clone()).collect();

        let entry_ids: Vec<ModuleId> = entries.iter().map(|path| PathHasher::hash(path)).collect();

        Ok(ParallelBundle {
            modules,
            entries: entry_ids,
            time_ms: elapsed.as_secs_f64() * 1000.0,
        })
    }

    fn bundle_parallel(&self, entries: &[PathBuf], options: &ParallelOptions) -> BundleResult<()> {
        // Phase 1: Scan all entry points for imports (SIMD accelerated)
        let scan_results: Vec<_> = entries
            .par_iter()
            .map(|path| {
                let source =
                    std::fs::read(path).map_err(|_e| BundleError::module_not_found(path))?;
                let scan = scan_source(&source);
                Ok((path.to_path_buf(), source, scan))
            })
            .collect::<BundleResult<Vec<_>>>()?;

        // Phase 2: Queue all modules for processing
        for (path, source, scan) in scan_results {
            let module_id = PathHasher::hash(&path);

            // Store resolved module
            self.resolved.insert(
                module_id,
                ResolvedModule {
                    id: module_id,
                    path: path.clone(),
                    source: source.clone(),
                    imports: vec![],
                    exports: vec![],
                    flags: 0,
                },
            );

            // Queue for transformation
            self.pending.insert(module_id, path.to_path_buf());

            // Extract and queue imports
            for import_pos in scan.imports {
                if let Some(import_path) = self.extract_import_path(&source, import_pos as usize) {
                    if let Some(resolved_path) = self.resolve_import(&import_path, &path) {
                        let import_id = PathHasher::hash(&resolved_path);
                        if !self.resolved.contains_key(&import_id) {
                            self.pending.insert(import_id, resolved_path);
                        }
                    }
                }
            }
        }

        // Phase 3: Process all modules in parallel
        let pending_list: Vec<_> =
            self.pending.iter().map(|entry| (*entry.key(), entry.value().clone())).collect();

        pending_list
            .par_iter()
            .try_for_each(|(module_id, path)| self.process_module(*module_id, path, options))?;

        Ok(())
    }

    fn process_module(
        &self,
        module_id: ModuleId,
        path: &Path,
        _options: &ParallelOptions,
    ) -> BundleResult<()> {
        // Check cache first
        if let Some(ref cache) = *self.cache {
            if let Some(cached) = cache.get(path) {
                self.transformed.insert(
                    module_id,
                    TransformedModule {
                        id: module_id,
                        content: cached.transformed,
                        source_map: None,
                        imports: cached.imports,
                    },
                );
                return Ok(());
            }
        }

        // Read source
        let source = std::fs::read(path).map_err(|_| BundleError::module_not_found(path))?;

        // Scan for patterns (SIMD)
        let scan = scan_source(&source);

        // Transform using unified pipeline
        let transform_opts = TransformOptions {
            strip_typescript: self.config.preserve_jsx,
            transform_jsx: !self.config.preserve_jsx,
            jsx_factory: self.config.jsx_factory.clone(),
            jsx_fragment: self.config.jsx_fragment.clone(),
            transform_es6: self.config.format == ModuleFormat::CJS,
            minify: self.config.minify,
            preserve_comments: false,
        };

        let transformed = transform(&source, module_id, &self.imports, &transform_opts)?;

        // Extract imports for dependency tracking
        let import_ids: Vec<ModuleId> = scan
            .imports
            .iter()
            .filter_map(|&pos| {
                self.extract_import_path(&source, pos as usize)
                    .and_then(|import_path| self.resolve_import(&import_path, path))
                    .map(|resolved| PathHasher::hash(&resolved))
            })
            .collect();

        // Store transformed module
        let transformed_module = TransformedModule {
            id: module_id,
            content: transformed.clone(),
            source_map: None,
            imports: import_ids.clone(),
        };

        self.transformed.insert(module_id, transformed_module);

        // Update cache
        if let Some(ref cache) = *self.cache {
            let hash = ContentHash::xxh3(&source);
            cache.put(
                path,
                dx_bundle_cache::CachedTransform {
                    content_hash: hash,
                    transformed,
                    imports: import_ids,
                    mtime: get_mtime(path),
                },
            );
        }

        Ok(())
    }

    fn extract_import_path(&self, source: &[u8], pos: usize) -> Option<String> {
        // Find " from " or "import("
        let mut i = pos + 7; // Skip "import "

        // Find "from"
        while i + 5 < source.len() {
            if &source[i..i + 5] == b" from" || &source[i..i + 4] == b"from" {
                i += 5;
                break;
            }
            i += 1;
        }

        // Skip whitespace
        while i < source.len() && (source[i] == b' ' || source[i] == b'\t') {
            i += 1;
        }

        // Find quote
        if i >= source.len() {
            return None;
        }

        let quote = source[i];
        if quote != b'"' && quote != b'\'' {
            return None;
        }

        i += 1;
        let start = i;

        // Find closing quote
        while i < source.len() && source[i] != quote {
            i += 1;
        }

        String::from_utf8(source[start..i].to_vec()).ok()
    }

    fn resolve_import(&self, import_path: &str, from: &Path) -> Option<PathBuf> {
        // Simple resolution (can be enhanced with node_modules resolution)
        if import_path.starts_with('.') {
            // Relative import
            let base = from.parent()?;
            let resolved = base.join(import_path);

            // Try with extensions
            for ext in &["", ".js", ".ts", ".jsx", ".tsx"] {
                let with_ext = if ext.is_empty() {
                    resolved.clone()
                } else {
                    resolved.with_extension(&ext[1..])
                };

                if with_ext.exists() {
                    return Some(with_ext);
                }
            }

            None
        } else {
            // Node modules or absolute - skip for now
            None
        }
    }
}

fn get_mtime(path: &Path) -> u64 {
    std::fs::metadata(path)
        .and_then(|m| m.modified())
        .and_then(|t| {
            t.duration_since(std::time::UNIX_EPOCH)
                .map_err(|_| std::io::Error::other("time error"))
        })
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parallel_bundler() {
        // Create test files
        let temp_dir = std::env::temp_dir().join("dx-parallel-test");
        std::fs::create_dir_all(&temp_dir).unwrap();

        let entry = temp_dir.join("entry.js");
        std::fs::write(&entry, b"import { foo } from './foo';\nconsole.log(foo);").unwrap();

        let foo = temp_dir.join("foo.js");
        std::fs::write(&foo, b"export const foo = 'bar';").unwrap();

        // Bundle
        let config = BundleConfig::default();
        let bundler = SpeculativeBundler::new(config, None);
        let result = bundler.bundle(&[entry], &ParallelOptions::default()).unwrap();

        assert!(!result.modules.is_empty());

        // Clean up
        std::fs::remove_dir_all(&temp_dir).ok();
    }
}
