//! # Build Pipeline
//!
//! This module implements the build pipeline for compiling DX WWW projects.
//! It transforms `.pg` and `.cp` source files into `.dxob` binary format.
//!
//! ## Build Process
//!
//! 1. Parse component files
//! 2. Compile scripts (Rust/Python/JS/Go → WASM/Native)
//! 3. Compile templates (HTML → Binary DOM instructions)
//! 4. Compile styles (CSS → Binary CSS via dx-style)
//! 5. Bundle dependencies
//! 6. Optimize (Tree-shake, Minify)
//! 7. Generate binary objects (.dxob)

mod binary;
mod cache;
mod manifest;
mod optimize;
mod script;
mod style;
mod template;

pub use binary::{BinaryObjectBuilder, DxobHeader, DxobSection};
pub use cache::{BuildCache, CacheEntry};
pub use manifest::{ManifestAsset, ManifestRoute, RouteManifest};
pub use optimize::{OptimizeOptions, Optimizer};
pub use script::ScriptCompiler;
pub use style::StyleCompiler;
pub use template::TemplateCompiler;

use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use dashmap::DashMap;
use tokio::sync::Semaphore;

use crate::config::{DxConfig, OptimizationLevel};
use crate::error::{DxError, DxResult};
use crate::parser::{ComponentParser, ParsedComponent};
use crate::project::Project;

// =============================================================================
// Build Output
// =============================================================================

/// Output of a build operation.
#[derive(Debug, Clone)]
pub struct BuildOutput {
    /// Generated binary objects
    pub binary_objects: Vec<BinaryObject>,

    /// Route manifest
    pub manifest: RouteManifest,

    /// Compiled assets
    pub assets: Vec<CompiledAsset>,

    /// Source maps (if enabled)
    pub source_maps: Vec<SourceMap>,

    /// Build statistics
    pub stats: BuildStats,
}

/// A compiled binary object.
#[derive(Debug, Clone)]
pub struct BinaryObject {
    /// Output file path
    pub path: PathBuf,

    /// Binary size in bytes
    pub size: usize,

    /// Content hash for caching
    pub hash: String,

    /// Dependencies
    pub dependencies: Vec<String>,

    /// Component type
    pub component_type: crate::parser::ComponentType,
}

/// A compiled asset.
#[derive(Debug, Clone)]
pub struct CompiledAsset {
    /// Original path
    pub source_path: PathBuf,

    /// Output path
    pub output_path: PathBuf,

    /// Content hash
    pub hash: String,

    /// Asset size
    pub size: usize,
}

/// A source map for debugging.
#[derive(Debug, Clone)]
pub struct SourceMap {
    /// Source file path
    pub source: PathBuf,

    /// Output file path
    pub output: PathBuf,

    /// Source map content
    pub content: String,
}

/// Build statistics.
#[derive(Debug, Clone, Default)]
pub struct BuildStats {
    /// Total build time
    pub duration: Duration,

    /// Number of files compiled
    pub files_compiled: usize,

    /// Number of files from cache
    pub files_cached: usize,

    /// Total output size in bytes
    pub total_size: usize,

    /// Number of errors
    pub errors: usize,

    /// Number of warnings
    pub warnings: usize,
}

// =============================================================================
// Build Pipeline
// =============================================================================

/// The main build pipeline for DX WWW projects.
pub struct BuildPipeline {
    /// Project configuration
    config: DxConfig,

    /// Build cache
    cache: BuildCache,

    /// Component parser
    parser: ComponentParser,

    /// Script compiler
    script_compiler: ScriptCompiler,

    /// Template compiler
    template_compiler: TemplateCompiler,

    /// Style compiler
    style_compiler: StyleCompiler,

    /// Optimizer
    optimizer: Optimizer,

    /// Compiled components cache
    #[allow(dead_code)]
    compiled: Arc<DashMap<PathBuf, CompiledComponent>>,

    /// Parallel build semaphore
    semaphore: Arc<Semaphore>,
}

/// A compiled component ready for binary generation.
#[derive(Debug, Clone)]
pub struct CompiledComponent {
    /// Parsed component
    pub parsed: ParsedComponent,

    /// Compiled script bytes
    pub script_bytes: Vec<u8>,

    /// Compiled template bytes
    pub template_bytes: Vec<u8>,

    /// Compiled style bytes
    pub style_bytes: Vec<u8>,

    /// Dependencies
    pub dependencies: Vec<String>,

    /// Content hash
    pub hash: String,
}

impl BuildPipeline {
    /// Create a new build pipeline.
    pub fn new(config: &DxConfig) -> Self {
        let parallel_jobs = config.build.parallel_jobs.unwrap_or_else(num_cpus::get);

        Self {
            config: config.clone(),
            cache: BuildCache::new(),
            parser: ComponentParser::new(),
            script_compiler: ScriptCompiler::new(config),
            template_compiler: TemplateCompiler::new(),
            style_compiler: StyleCompiler::new(),
            optimizer: Optimizer::new(config),
            compiled: Arc::new(DashMap::new()),
            semaphore: Arc::new(Semaphore::new(parallel_jobs)),
        }
    }

    /// Build the entire project.
    pub async fn build(&mut self, project: &Project) -> DxResult<BuildOutput> {
        let start = Instant::now();
        let mut stats = BuildStats::default();

        // Ensure output directory exists
        let output_dir = project.output_dir();
        tokio::fs::create_dir_all(&output_dir).await?;

        // Load cache
        self.cache.load(&project.cache_dir()).await?;

        // Compile all components in parallel
        let mut binary_objects = Vec::new();
        let mut source_maps = Vec::new();

        // Compile pages
        for page in &project.pages {
            match self.compile_component(&page.path, project).await {
                Ok((obj, map)) => {
                    stats.files_compiled += 1;
                    stats.total_size += obj.size;
                    binary_objects.push(obj);
                    if let Some(m) = map {
                        source_maps.push(m);
                    }
                }
                Err(e) => {
                    stats.errors += 1;
                    tracing::error!("Failed to compile {}: {}", page.path.display(), e);
                }
            }
        }

        // Compile components
        for component in &project.components {
            match self.compile_component(&component.path, project).await {
                Ok((obj, map)) => {
                    stats.files_compiled += 1;
                    stats.total_size += obj.size;
                    binary_objects.push(obj);
                    if let Some(m) = map {
                        source_maps.push(m);
                    }
                }
                Err(e) => {
                    stats.errors += 1;
                    tracing::error!("Failed to compile {}: {}", component.path.display(), e);
                }
            }
        }

        // Compile layouts
        for layout in &project.layouts {
            match self.compile_component(&layout.path, project).await {
                Ok((obj, map)) => {
                    stats.files_compiled += 1;
                    stats.total_size += obj.size;
                    binary_objects.push(obj);
                    if let Some(m) = map {
                        source_maps.push(m);
                    }
                }
                Err(e) => {
                    stats.errors += 1;
                    tracing::error!("Failed to compile {}: {}", layout.path.display(), e);
                }
            }
        }

        // Process assets
        let assets = self.process_assets(project).await?;

        // Generate manifest
        let manifest = self.generate_manifest(project, &binary_objects, &assets)?;

        // Save cache
        self.cache.save(&project.cache_dir()).await?;

        stats.duration = start.elapsed();

        Ok(BuildOutput {
            binary_objects,
            manifest,
            assets,
            source_maps,
            stats,
        })
    }

    /// Build only changed files (incremental).
    pub async fn build_incremental(
        &mut self,
        project: &Project,
        changed_files: &[PathBuf],
    ) -> DxResult<BuildOutput> {
        let start = Instant::now();
        let mut stats = BuildStats::default();

        // Load cache
        self.cache.load(&project.cache_dir()).await?;

        // Find affected files
        let affected = self.find_affected_files(project, changed_files)?;

        // Compile affected files
        let mut binary_objects = Vec::new();
        let mut source_maps = Vec::new();

        for path in &affected {
            match self.compile_component(path, project).await {
                Ok((obj, map)) => {
                    stats.files_compiled += 1;
                    stats.total_size += obj.size;
                    binary_objects.push(obj);
                    if let Some(m) = map {
                        source_maps.push(m);
                    }
                }
                Err(e) => {
                    stats.errors += 1;
                    tracing::error!("Failed to compile {}: {}", path.display(), e);
                }
            }
        }

        // Add unchanged files from cache
        for page in &project.pages {
            if !affected.contains(&page.path) {
                if let Some(cached) = self.cache.get(&page.path) {
                    stats.files_cached += 1;
                    binary_objects.push(cached.to_binary_object());
                }
            }
        }

        // Process assets
        let assets = self.process_assets(project).await?;

        // Generate manifest
        let manifest = self.generate_manifest(project, &binary_objects, &assets)?;

        // Save cache
        self.cache.save(&project.cache_dir()).await?;

        stats.duration = start.elapsed();

        Ok(BuildOutput {
            binary_objects,
            manifest,
            assets,
            source_maps,
            stats,
        })
    }

    /// Compile a single component.
    async fn compile_component(
        &mut self,
        path: &PathBuf,
        project: &Project,
    ) -> DxResult<(BinaryObject, Option<SourceMap>)> {
        // Acquire semaphore permit for parallel limiting
        let _permit = self.semaphore.acquire().await.map_err(|_| DxError::InternalError {
            message: "Failed to acquire build semaphore".to_string(),
        })?;

        // Check cache
        if let Some(cached) = self.cache.check(path).await? {
            return Ok((cached.to_binary_object(), None));
        }

        // Parse component
        let parsed = self.parser.parse_file(path)?;

        // Compile script
        let script_bytes = if let Some(ref script) = parsed.script {
            self.script_compiler.compile(script)?
        } else {
            Vec::new()
        };

        // Compile template
        let template_bytes = self.template_compiler.compile(&parsed.template)?;

        // Compile style
        let style_bytes = if let Some(ref style) = parsed.style {
            self.style_compiler.compile(style)?
        } else {
            Vec::new()
        };

        // Calculate hash
        let hash = self.calculate_hash(&script_bytes, &template_bytes, &style_bytes);

        // Build binary object
        let builder = BinaryObjectBuilder::new()
            .component_type(parsed.component_type)
            .script(script_bytes.clone())
            .template(template_bytes.clone())
            .style(style_bytes.clone());

        let binary = builder.build()?;

        // Determine output path
        let output_path = self.compute_output_path(path, project);

        // Ensure parent directory exists
        if let Some(parent) = output_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        // Write binary file
        tokio::fs::write(&output_path, &binary).await?;

        // Optimize if needed
        let final_binary = if self.config.build.optimization_level != OptimizationLevel::Debug {
            let optimized = self.optimizer.optimize(&binary)?;
            tokio::fs::write(&output_path, &optimized).await?;
            optimized
        } else {
            binary
        };

        // Generate source map if enabled
        let source_map = if self.config.build.source_maps {
            Some(SourceMap {
                source: path.clone(),
                output: output_path.with_extension("dxob.map"),
                content: self.generate_source_map(path, &parsed)?,
            })
        } else {
            None
        };

        // Update cache
        let compiled = CompiledComponent {
            parsed: parsed.clone(),
            script_bytes,
            template_bytes,
            style_bytes,
            dependencies: Vec::new(),
            hash: hash.clone(),
        };
        self.cache.set(path.clone(), compiled).await;

        let binary_object = BinaryObject {
            path: output_path,
            size: final_binary.len(),
            hash,
            dependencies: Vec::new(),
            component_type: parsed.component_type,
        };

        Ok((binary_object, source_map))
    }

    /// Process static assets.
    async fn process_assets(&self, project: &Project) -> DxResult<Vec<CompiledAsset>> {
        let mut assets = Vec::new();

        for asset in &project.assets {
            let output_path = if self.config.assets.content_hash {
                // Add content hash to filename
                let hash = self.hash_file(&asset.path).await?;
                let stem = asset.path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
                let ext = asset.path.extension().and_then(|e| e.to_str()).unwrap_or("");
                let hashed_name = format!("{stem}-{hash}.{ext}");
                project
                    .output_dir()
                    .join("assets")
                    .join(&asset.relative_path)
                    .with_file_name(hashed_name)
            } else {
                project.output_dir().join("assets").join(&asset.relative_path)
            };

            // Ensure parent directory exists
            if let Some(parent) = output_path.parent() {
                tokio::fs::create_dir_all(parent).await?;
            }

            // Copy or optimize asset
            if self.config.assets.optimize_images && asset.asset_type.is_optimizable() {
                // TODO: Implement image optimization
                tokio::fs::copy(&asset.path, &output_path).await?;
            } else {
                tokio::fs::copy(&asset.path, &output_path).await?;
            }

            let metadata = tokio::fs::metadata(&output_path).await?;
            let hash = self.hash_file(&asset.path).await?;

            assets.push(CompiledAsset {
                source_path: asset.path.clone(),
                output_path,
                hash,
                size: metadata.len() as usize,
            });
        }

        Ok(assets)
    }

    /// Generate route manifest.
    fn generate_manifest(
        &self,
        project: &Project,
        binary_objects: &[BinaryObject],
        _assets: &[CompiledAsset],
    ) -> DxResult<RouteManifest> {
        Ok(RouteManifest::generate(project, binary_objects))
    }

    /// Compute output path for a component.
    fn compute_output_path(&self, source: &PathBuf, project: &Project) -> PathBuf {
        let relative = source.strip_prefix(&project.root).unwrap_or(source);
        let mut output = project.output_dir().join(relative);
        output.set_extension(crate::BINARY_EXTENSION);
        output
    }

    /// Calculate content hash.
    fn calculate_hash(&self, script: &[u8], template: &[u8], style: &[u8]) -> String {
        use blake3::Hasher;
        let mut hasher = Hasher::new();
        hasher.update(script);
        hasher.update(template);
        hasher.update(style);
        hasher.finalize().to_hex()[..16].to_string()
    }

    /// Hash a file.
    async fn hash_file(&self, path: &PathBuf) -> DxResult<String> {
        let content = tokio::fs::read(path).await?;
        let hash = blake3::hash(&content);
        Ok(hash.to_hex()[..8].to_string())
    }

    /// Find files affected by changes.
    fn find_affected_files(
        &self,
        _project: &Project,
        changed: &[PathBuf],
    ) -> DxResult<Vec<PathBuf>> {
        // For now, just return the changed files
        // TODO: Implement dependency tracking for proper affected file detection
        Ok(changed.to_vec())
    }

    /// Generate source map for a component.
    fn generate_source_map(&self, _path: &PathBuf, _parsed: &ParsedComponent) -> DxResult<String> {
        // Simplified source map generation
        Ok(serde_json::json!({
            "version": 3,
            "sources": [],
            "mappings": ""
        })
        .to_string())
    }
}

// =============================================================================
// Helper to get CPU count
// =============================================================================

mod num_cpus {
    pub fn get() -> usize {
        std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4)
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_stats_default() {
        let stats = BuildStats::default();
        assert_eq!(stats.files_compiled, 0);
        assert_eq!(stats.errors, 0);
    }

    #[test]
    fn test_calculate_hash() {
        let config = DxConfig::default();
        let pipeline = BuildPipeline::new(&config);

        let hash1 = pipeline.calculate_hash(b"script1", b"template1", b"style1");
        let hash2 = pipeline.calculate_hash(b"script1", b"template1", b"style1");
        let hash3 = pipeline.calculate_hash(b"script2", b"template1", b"style1");

        assert_eq!(hash1, hash2);
        assert_ne!(hash1, hash3);
    }
}
