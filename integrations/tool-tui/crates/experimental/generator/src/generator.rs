//! Main Generator - Unified Code Generation Engine
//!
//! The `Generator` struct ties all components together into a
//! single, cohesive code generation system.

use crate::binary::BinaryTemplate;
use crate::compiler::{CompileOptions, Compiler};
use crate::dirty::DirtyTracker;
use crate::error::{GeneratorError, Result};
use crate::fusion::{FusionBundle, FusionOutput};
use crate::params::Parameters;
use crate::pool::TemplatePool;
use crate::render::{RenderMode, RenderOutput, Renderer};
use crate::session::Session;
use crate::template::Template;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

// ============================================================================
// Generator Configuration
// ============================================================================

/// Generator configuration.
#[derive(Clone, Debug)]
pub struct GeneratorConfig {
    /// Maximum output size in bytes.
    pub max_output_size: usize,
    /// Cache capacity.
    pub cache_capacity: usize,
    /// Pool capacity.
    pub pool_capacity: usize,
    /// Enable dirty-bit tracking.
    pub enable_dirty_tracking: bool,
    /// Enable XOR patching.
    pub enable_patching: bool,
    /// Preferred render mode.
    pub render_mode: RenderMode,
    /// Enable security verification.
    pub verify_signatures: bool,
    /// Template search paths.
    pub template_paths: Vec<PathBuf>,
}

impl Default for GeneratorConfig {
    fn default() -> Self {
        Self {
            max_output_size: 16 * 1024 * 1024, // 16 MB
            cache_capacity: 256,
            pool_capacity: 1024,
            enable_dirty_tracking: true,
            enable_patching: true,
            render_mode: RenderMode::Auto,
            verify_signatures: true,
            template_paths: Vec::new(),
        }
    }
}

impl GeneratorConfig {
    /// Create a new configuration.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set maximum output size.
    #[must_use]
    pub fn max_output_size(mut self, size: usize) -> Self {
        self.max_output_size = size;
        self
    }

    /// Set cache capacity.
    #[must_use]
    pub fn cache_capacity(mut self, capacity: usize) -> Self {
        self.cache_capacity = capacity;
        self
    }

    /// Set pool capacity.
    #[must_use]
    pub fn pool_capacity(mut self, capacity: usize) -> Self {
        self.pool_capacity = capacity;
        self
    }

    /// Set render mode.
    #[must_use]
    pub fn render_mode(mut self, mode: RenderMode) -> Self {
        self.render_mode = mode;
        self
    }

    /// Disable signature verification (use with caution).
    #[must_use]
    pub fn skip_verification(mut self) -> Self {
        self.verify_signatures = false;
        self
    }

    /// Add a template search path.
    #[must_use]
    pub fn add_template_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.template_paths.push(path.into());
        self
    }
}

// ============================================================================
// Generator Statistics
// ============================================================================

/// Generation statistics.
#[derive(Clone, Debug, Default)]
pub struct GeneratorStats {
    /// Total templates loaded.
    pub templates_loaded: u64,
    /// Total renders.
    pub renders: u64,
    /// Cache hits.
    pub cache_hits: u64,
    /// Cache misses.
    pub cache_misses: u64,
    /// Dirty-bit skips (saved renders).
    pub dirty_skips: u64,
    /// Total bytes generated.
    pub bytes_generated: u64,
    /// Total time spent rendering (nanoseconds).
    pub render_time_ns: u64,
}

impl GeneratorStats {
    /// Get cache hit rate.
    #[must_use]
    pub fn cache_hit_rate(&self) -> f64 {
        let total = self.cache_hits + self.cache_misses;
        if total == 0 {
            0.0
        } else {
            self.cache_hits as f64 / total as f64
        }
    }

    /// Get average render time in microseconds.
    #[must_use]
    pub fn avg_render_time_us(&self) -> f64 {
        if self.renders == 0 {
            0.0
        } else {
            (self.render_time_ns as f64 / self.renders as f64) / 1000.0
        }
    }
}

// ============================================================================
// Generator
// ============================================================================

/// The main code generator engine.
///
/// # Example
///
/// ```rust,ignore
/// use dx_generator::{Generator, Parameters};
///
/// let mut generator = Generator::new();
///
/// // Load template
/// generator.load_template("component", "templates/component.dxt")?;
///
/// // Generate
/// let params = Parameters::new()
///     .set("name", "Counter")
///     .set("with_state", true);
///
/// let output = generator.generate("component", &params)?;
/// std::fs::write("src/counter.rs", output.as_bytes())?;
/// ```
pub struct Generator {
    /// Configuration.
    config: GeneratorConfig,
    /// Template storage (name -> template).
    templates: HashMap<String, BinaryTemplate>,
    /// Template pool.
    pool: TemplatePool,
    /// Compiler.
    compiler: Compiler,
    /// Renderer.
    renderer: Renderer,
    /// Dirty tracker.
    dirty: DirtyTracker,
    /// Statistics.
    stats: GeneratorStats,
    /// Loaded fusion bundles.
    bundles: HashMap<String, FusionBundle>,
    /// Active sessions.
    sessions: HashMap<String, Session>,
}

impl Generator {
    /// Create a new generator with default configuration.
    #[must_use]
    pub fn new() -> Self {
        Self::with_config(GeneratorConfig::default())
    }

    /// Create a generator with custom configuration.
    #[must_use]
    pub fn with_config(config: GeneratorConfig) -> Self {
        Self {
            templates: HashMap::new(),
            pool: TemplatePool::new(config.pool_capacity),
            compiler: Compiler::new(),
            renderer: Renderer::with_mode(config.render_mode),
            dirty: DirtyTracker::new(),
            stats: GeneratorStats::default(),
            bundles: HashMap::new(),
            sessions: HashMap::new(),
            config,
        }
    }

    // ========================================================================
    // Template Loading
    // ========================================================================

    /// Load a template from file.
    pub fn load_template(&mut self, name: impl Into<String>, path: impl AsRef<Path>) -> Result<()> {
        let name = name.into();
        let path = path.as_ref();

        // Check if already loaded
        if self.templates.contains_key(&name) {
            self.stats.cache_hits += 1;
            return Ok(());
        }
        self.stats.cache_misses += 1;

        // Load template
        let template = Template::load(path)?;
        self.templates.insert(name, template.inner().clone());
        self.stats.templates_loaded += 1;

        Ok(())
    }

    /// Load a template from bytes.
    pub fn load_template_bytes(&mut self, name: impl Into<String>, data: Vec<u8>) -> Result<()> {
        let name = name.into();
        let template = Template::from_bytes(data)?;
        self.templates.insert(name, template.inner().clone());
        self.stats.templates_loaded += 1;
        Ok(())
    }

    /// Compile and load a text template.
    pub fn compile_template(&mut self, name: impl Into<String>, source: &str) -> Result<()> {
        let name = name.into();
        let options = CompileOptions::default();
        let compiled = self.compiler.compile(source.as_bytes(), options)?;
        self.templates.insert(name, compiled);
        self.stats.templates_loaded += 1;
        Ok(())
    }

    /// Get a loaded template.
    #[must_use]
    pub fn get_template(&self, name: &str) -> Option<&BinaryTemplate> {
        self.templates.get(name)
    }

    // ========================================================================
    // Generation
    // ========================================================================

    /// Generate output from a template.
    pub fn generate(
        &mut self,
        template_name: &str,
        params: &Parameters<'_>,
    ) -> Result<RenderOutput> {
        let start = std::time::Instant::now();

        // Get template
        let template =
            self.templates
                .get(template_name)
                .ok_or_else(|| GeneratorError::TemplateNotFound {
                    path: template_name.to_string(),
                })?;

        // Render
        let output = self.renderer.render(template, params)?;

        // Update stats
        self.stats.renders += 1;
        self.stats.bytes_generated += output.len() as u64;
        self.stats.render_time_ns += start.elapsed().as_nanos() as u64;

        Ok(output.clone())
    }

    /// Generate to a file.
    pub fn generate_to_file(
        &mut self,
        template_name: &str,
        params: &Parameters<'_>,
        output_path: impl AsRef<Path>,
    ) -> Result<()> {
        let output = self.generate(template_name, params)?;
        let path = output_path.as_ref();

        // Create parent directories
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // Write output
        std::fs::write(path, output.as_bytes())?;

        Ok(())
    }

    /// Generate with dirty-bit tracking.
    ///
    /// Only regenerates if parameters have changed.
    pub fn generate_tracked(
        &mut self,
        template_name: &str,
        params: &Parameters<'_>,
    ) -> Result<Option<RenderOutput>> {
        // Check if dirty
        if !self.dirty.is_dirty() {
            self.stats.dirty_skips += 1;
            return Ok(None);
        }

        let output = self.generate(template_name, params)?;
        self.dirty.clear();

        Ok(Some(output))
    }

    // ========================================================================
    // Fusion Bundles
    // ========================================================================

    /// Register a fusion bundle.
    pub fn register_bundle(&mut self, bundle: FusionBundle) {
        self.bundles.insert(bundle.name.clone(), bundle);
    }

    /// Generate a fusion bundle.
    pub fn generate_bundle(
        &mut self,
        bundle_name: &str,
        params: &Parameters<'_>,
    ) -> Result<Vec<FusionOutput>> {
        let bundle =
            self.bundles.get(bundle_name).ok_or_else(|| GeneratorError::TemplateNotFound {
                path: bundle_name.to_string(),
            })?;

        bundle.generate(params)
    }

    /// Generate and write a fusion bundle.
    pub fn generate_bundle_to_disk(
        &mut self,
        bundle_name: &str,
        params: &Parameters<'_>,
    ) -> Result<Vec<PathBuf>> {
        let bundle = self
            .bundles
            .get(bundle_name)
            .ok_or_else(|| GeneratorError::TemplateNotFound {
                path: bundle_name.to_string(),
            })?
            .clone();

        bundle.generate_and_write(params)
    }

    // ========================================================================
    // Sessions
    // ========================================================================

    /// Start a new interactive session.
    pub fn start_session(&mut self, template_name: impl Into<String>) -> &mut Session {
        let template_name = template_name.into();
        let session = Session::new(&template_name);
        let id = session.id().to_string();
        self.sessions.insert(id.clone(), session);
        self.sessions.get_mut(&id).unwrap()
    }

    /// Get an active session.
    #[must_use]
    pub fn get_session(&self, session_id: &str) -> Option<&Session> {
        self.sessions.get(session_id)
    }

    /// Get a mutable session.
    #[must_use]
    pub fn get_session_mut(&mut self, session_id: &str) -> Option<&mut Session> {
        self.sessions.get_mut(session_id)
    }

    /// Complete a session and generate output.
    pub fn complete_session(&mut self, session_id: &str) -> Result<RenderOutput> {
        let session =
            self.sessions.get(session_id).ok_or_else(|| GeneratorError::SessionCorrupted {
                reason: "Session not found".to_string(),
            })?;

        if !session.is_complete() {
            return Err(GeneratorError::SessionCorrupted {
                reason: "Session not complete".to_string(),
            });
        }

        let template_name = session.template_name().to_string();
        let params = session.params().clone();

        self.generate(&template_name, &params)
    }

    // ========================================================================
    // Utilities
    // ========================================================================

    /// Get current statistics.
    #[must_use]
    pub fn stats(&self) -> &GeneratorStats {
        &self.stats
    }

    /// Clear all caches.
    pub fn clear_caches(&mut self) {
        self.templates.clear();
        self.pool.clear();
    }

    /// Get the dirty tracker for parameter updates.
    #[must_use]
    pub fn dirty_tracker(&self) -> &DirtyTracker {
        &self.dirty
    }

    /// Get mutable dirty tracker.
    pub fn dirty_tracker_mut(&mut self) -> &mut DirtyTracker {
        &mut self.dirty
    }

    /// Get configuration.
    #[must_use]
    pub fn config(&self) -> &GeneratorConfig {
        &self.config
    }
}

impl Default for Generator {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Shared Generator
// ============================================================================

/// Thread-safe shared generator.
pub type SharedGenerator = Arc<parking_lot::RwLock<Generator>>;

/// Create a new shared generator.
#[must_use]
pub fn shared_generator() -> SharedGenerator {
    Arc::new(parking_lot::RwLock::new(Generator::new()))
}

/// Create a shared generator with config.
#[must_use]
pub fn shared_generator_with_config(config: GeneratorConfig) -> SharedGenerator {
    Arc::new(parking_lot::RwLock::new(Generator::with_config(config)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generator_new() {
        let generator = Generator::new();
        assert_eq!(generator.stats().templates_loaded, 0);
        assert_eq!(generator.stats().renders, 0);
    }

    #[test]
    fn test_generator_compile_and_generate() {
        let mut generator = Generator::new();

        // Compile template
        generator.compile_template("greeting", "Hello, {{name}}!").unwrap();

        // Generate
        let params = Parameters::new().set("name", "World");
        let _output = generator.generate("greeting", &params).unwrap();

        // Note: Actual output depends on renderer implementation
        assert!(generator.stats().renders > 0);
    }

    #[test]
    fn test_generator_stats() {
        let mut generator = Generator::new();

        generator.compile_template("test", "{{x}}").unwrap();

        assert_eq!(generator.stats().templates_loaded, 1);

        let params = Parameters::new().set("x", "value");
        let _ = generator.generate("test", &params);

        assert_eq!(generator.stats().renders, 1);
    }

    #[test]
    fn test_generator_config() {
        let config = GeneratorConfig::new()
            .max_output_size(1024)
            .cache_capacity(64)
            .render_mode(RenderMode::Micro);

        let generator = Generator::with_config(config);

        assert_eq!(generator.config().max_output_size, 1024);
        assert_eq!(generator.config().cache_capacity, 64);
    }

    #[test]
    fn test_shared_generator() {
        let generator = shared_generator();

        {
            let mut w = generator.write();
            w.compile_template("test", "{{x}}").unwrap();
        }

        {
            let r = generator.read();
            assert_eq!(r.stats().templates_loaded, 1);
        }
    }
}
