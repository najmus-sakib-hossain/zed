//! dx-js-runtime: A high-performance JavaScript/TypeScript runtime
//!
//! Designed for performance through:
//! - OXC for fast parsing
//! - Cranelift JIT for native code generation
//! - Type-directed compilation
//! - Arena-based execution
//! - Persistent code cache
//!
//! # Example
//!
//! ```no_run
//! use dx_js_runtime::DxRuntime;
//!
//! fn main() -> anyhow::Result<()> {
//!     let mut runtime = DxRuntime::new()?;
//!     let result = runtime.run_sync("console.log('Hello from dx!')", "hello.js")?;
//!     Ok(())
//! }
//! ```

pub mod compiler;
pub mod config;
pub mod constants;
pub mod crystallized;
pub mod debugger;
pub mod deploy;
pub mod distributed;
pub mod error;
pub mod features;
pub mod gc;
pub mod gpu;
pub mod io;
pub mod profiler;
pub mod runtime;
pub mod simd;
pub mod snapshot;
pub mod value;
pub mod wasm;
pub mod workers;
pub mod zero_copy;

pub use compiler::{
    clear_structured_exception, get_structured_exception, set_structured_exception,
    throw_range_error, throw_reference_error, throw_syntax_error, throw_type_error,
};
pub use compiler::{CompiledModule, Compiler, CompilerConfig};
pub use config::{
    load_config, load_config_from_file, merge_with_defaults, BundlerConfigFile, ConfigError,
    LoadedConfig, PackageManagerConfigFile, ProjectConfig, RuntimeConfigFile, TestRunnerConfigFile,
};
pub use error::{
    call_stack_depth,
    capture_stack_trace,
    capture_stack_trace_limited,
    clear_call_stack,
    create_error_with_snippet,
    create_exception_with_stack,
    // CLI helpers
    format_error_for_cli,
    format_error_plain,
    get_source_map,
    pop_call_frame,
    print_error,
    push_call_frame,
    range_error_with_stack,
    reference_error_with_snippet,
    reference_error_with_stack,
    register_source_map,
    syntax_error_with_snippet,
    syntax_error_with_stack,
    type_error_with_snippet,
    type_error_with_stack,
    CallFrame,
    CodeSnippet,
    ConsoleErrorFormatter,
    DefaultErrorFormatter,
    DxError,
    DxResult,
    ErrorFormatter,
    ExternalSourceMap,
    JsErrorType,
    JsException,
    ModuleSourceMap,
    OriginalLocation,
    SourceLocation,
    SourceMapEntry,
    StackFrame,
};
pub use runtime::{Runtime, RuntimeConfig};
pub use snapshot::ImmortalCache;
pub use value::Value;

use std::path::{Path, PathBuf};

/// The main dx JavaScript/TypeScript runtime.
///
/// `DxRuntime` provides a high-performance execution environment for JavaScript
/// and TypeScript code. It combines:
/// - OXC parser for fast parsing
/// - Cranelift JIT compiler for native code generation
/// - Persistent code cache for fast cold starts
/// - Arena-based memory management
///
/// # Example
///
/// ```no_run
/// use dx_js_runtime::{DxRuntime, DxConfig};
///
/// // Create runtime with default configuration
/// let mut runtime = DxRuntime::new().expect("Failed to create runtime");
///
/// // Run JavaScript code
/// let result = runtime.run_sync("1 + 2", "eval.js").expect("Execution failed");
/// println!("Result: {}", result);
///
/// // Or with custom configuration
/// let config = DxConfig {
///     max_heap_size_mb: 1024, // 1 GB heap
///     ..Default::default()
/// };
/// let mut runtime = DxRuntime::with_config(config).expect("Failed to create runtime");
/// ```
pub struct DxRuntime {
    /// Compiler for TypeScript/JavaScript
    compiler: Compiler,
    /// Runtime execution environment
    runtime: Runtime,
    /// Immortal code cache
    cache: ImmortalCache,
    /// Configuration - stored for future runtime reconfiguration support
    #[allow(dead_code)]
    config: DxConfig,
}

/// Configuration options for the DX runtime.
///
/// Use this struct to customize runtime behavior including cache location,
/// TypeScript type checking, worker threads, and memory limits.
///
/// # Example
///
/// ```
/// use dx_js_runtime::DxConfig;
/// use std::path::PathBuf;
///
/// let config = DxConfig {
///     cache_dir: PathBuf::from(".my-cache"),
///     type_check: false,  // Disable TypeScript type checking for faster startup
///     max_heap_size_mb: 256,  // Limit heap to 256 MB
///     ..Default::default()
/// };
/// ```
#[derive(Clone, Debug)]
pub struct DxConfig {
    /// Directory for the persistent code cache.
    /// Default: `.dx/cache`
    pub cache_dir: PathBuf,

    /// Enable TypeScript type checking during compilation.
    /// Default: `true`
    pub type_check: bool,

    /// Enable speculative execution optimizations.
    /// Default: `false`
    pub speculation: bool,

    /// Number of worker threads for parallel operations.
    /// Default: number of CPU cores
    pub workers: usize,

    /// Arena size per worker in bytes.
    /// Default: 256 MB
    pub arena_size: usize,

    /// Maximum heap size in megabytes.
    /// Valid range: 16 MB to 16 GB.
    /// Default: 512 MB
    pub max_heap_size_mb: usize,
}

impl Default for DxConfig {
    fn default() -> Self {
        Self {
            cache_dir: PathBuf::from(".dx/cache"),
            type_check: true,
            speculation: false,
            workers: num_cpus::get(),
            arena_size: 256 * 1024 * 1024, // 256MB
            max_heap_size_mb: 512,         // 512 MB default
        }
    }
}

impl DxRuntime {
    /// Create a new dx runtime with default configuration.
    ///
    /// This is equivalent to calling `DxRuntime::with_config(DxConfig::default())`.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The cache directory cannot be created
    /// - The compiler fails to initialize
    /// - The runtime fails to initialize
    ///
    /// # Example
    ///
    /// ```no_run
    /// use dx_js_runtime::DxRuntime;
    ///
    /// let mut runtime = DxRuntime::new()?;
    /// # Ok::<(), dx_js_runtime::DxError>(())
    /// ```
    pub fn new() -> DxResult<Self> {
        Self::with_config(DxConfig::default())
    }

    /// Create a new dx runtime with custom configuration.
    ///
    /// # Arguments
    ///
    /// * `config` - Configuration options for the runtime
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The cache directory cannot be created
    /// - The compiler fails to initialize
    /// - The runtime fails to initialize
    ///
    /// # Example
    ///
    /// ```no_run
    /// use dx_js_runtime::{DxRuntime, DxConfig};
    ///
    /// let config = DxConfig {
    ///     max_heap_size_mb: 1024,
    ///     ..Default::default()
    /// };
    /// let mut runtime = DxRuntime::with_config(config)?;
    /// # Ok::<(), dx_js_runtime::DxError>(())
    /// ```
    pub fn with_config(config: DxConfig) -> DxResult<Self> {
        // Ensure cache directory exists
        std::fs::create_dir_all(&config.cache_dir)?;

        // Initialize compiler
        let compiler = Compiler::new(CompilerConfig {
            type_check: config.type_check,
            optimization_level: compiler::OptLevel::Aggressive,
        })?;

        // Initialize runtime
        let runtime = Runtime::new(RuntimeConfig {
            arena_size: config.arena_size,
        })?;

        // Load or create immortal cache
        let cache = ImmortalCache::open_or_create(&config.cache_dir)?;

        Ok(Self {
            compiler,
            runtime,
            cache,
            config,
        })
    }

    /// Run a JavaScript/TypeScript file.
    ///
    /// Reads the file from disk and executes it. The file extension determines
    /// whether TypeScript processing is applied.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the JavaScript or TypeScript file
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The file cannot be read
    /// - The code contains syntax errors
    /// - A runtime error occurs during execution
    ///
    /// # Example
    ///
    /// ```no_run
    /// use dx_js_runtime::DxRuntime;
    ///
    /// let mut runtime = DxRuntime::new()?;
    /// let result = runtime.run_file("app.js")?;
    /// # Ok::<(), dx_js_runtime::DxError>(())
    /// ```
    pub fn run_file(&mut self, path: impl AsRef<Path>) -> DxResult<Value> {
        let path = path.as_ref();
        let source = std::fs::read_to_string(path)?;
        let filename = path.to_string_lossy();
        self.run_sync(&source, &filename)
    }

    /// Run JavaScript/TypeScript source code synchronously.
    ///
    /// Compiles and executes the provided source code. Results are cached
    /// based on source hash for faster subsequent executions.
    ///
    /// # Arguments
    ///
    /// * `source` - JavaScript or TypeScript source code
    /// * `filename` - Filename for error messages and source maps
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The code contains syntax errors
    /// - A runtime error occurs during execution
    ///
    /// # Example
    ///
    /// ```no_run
    /// use dx_js_runtime::DxRuntime;
    ///
    /// let mut runtime = DxRuntime::new()?;
    /// let result = runtime.run_sync("console.log('Hello!')", "hello.js")?;
    /// # Ok::<(), dx_js_runtime::DxError>(())
    /// ```
    pub fn run_sync(&mut self, source: &str, filename: &str) -> DxResult<Value> {
        // Check immortal cache first
        let source_hash = self.cache.hash_source(source);

        let module = if let Some(cached) = self.cache.get(&source_hash)? {
            // Cache hit - use pre-compiled native code
            cached
        } else {
            // Cache miss - compile and store
            let module = self.compiler.compile(source, filename)?;
            self.cache.store(&source_hash, &module)?;
            module
        };

        // Execute in runtime
        self.runtime.execute(&module)
    }

    /// Compile source code without executing it.
    ///
    /// Useful for benchmarking compilation time or pre-compiling modules.
    ///
    /// # Arguments
    ///
    /// * `source` - JavaScript or TypeScript source code
    /// * `filename` - Filename for error messages and source maps
    ///
    /// # Errors
    ///
    /// Returns an error if the code contains syntax errors.
    pub fn compile(&mut self, source: &str, filename: &str) -> DxResult<CompiledModule> {
        self.compiler.compile(source, filename)
    }

    /// Get statistics about the code cache.
    ///
    /// Returns information about cache hits, misses, and storage usage.
    pub fn cache_stats(&self) -> CacheStats {
        self.cache.stats()
    }
}

/// Statistics about the persistent code cache.
///
/// These statistics help monitor cache effectiveness and storage usage.
#[derive(Debug, Clone, Default)]
pub struct CacheStats {
    /// Number of cache hits (code found in cache)
    pub hits: u64,
    /// Number of cache misses (code needed compilation)
    pub misses: u64,
    /// Number of modules currently cached
    pub modules_cached: usize,
    /// Total size of cached data in bytes
    pub total_size_bytes: u64,
}
