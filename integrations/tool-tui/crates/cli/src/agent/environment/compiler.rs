//! Compilation Pipeline
//!
//! Handles compiling source code from various runtimes into WASM components.

use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;

use super::{
    EnvironmentConfig, EnvironmentError, EnvironmentResult, Runtime,
    manager::{EnvironmentManager, ProgressCallback},
};

/// Target for WASM compilation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompilationTarget {
    /// Standard WASM module
    Wasm32,
    /// WASI component (with interface types)
    WasiComponent,
    /// WASI preview 2 (component model)
    WasiP2,
}

impl Default for CompilationTarget {
    fn default() -> Self {
        Self::WasiP2
    }
}

/// Result of a compilation
#[derive(Debug, Clone)]
pub struct CompilationResult {
    /// Path to the compiled WASM
    pub wasm_path: PathBuf,
    /// Size of the compiled WASM in bytes
    pub size: u64,
    /// Compilation duration in milliseconds
    pub duration_ms: u64,
    /// Whether this was a cache hit
    pub cached: bool,
    /// Hash of the source
    pub source_hash: [u8; 32],
    /// Optimization level used
    pub optimization_level: u8,
}

/// Configuration for a compilation
#[derive(Debug, Clone)]
pub struct CompilationConfig {
    /// Target WASM format
    pub target: CompilationTarget,
    /// Optimization level (0-4)
    pub optimization_level: u8,
    /// Enable debug symbols
    pub debug: bool,
    /// Additional compiler flags
    pub extra_flags: Vec<String>,
    /// Environment variables for compilation
    pub env_vars: HashMap<String, String>,
}

impl Default for CompilationConfig {
    fn default() -> Self {
        Self {
            target: CompilationTarget::default(),
            optimization_level: 2,
            debug: false,
            extra_flags: Vec::new(),
            env_vars: HashMap::new(),
        }
    }
}

/// Pipeline for compiling code to WASM
pub struct CompilationPipeline {
    env_config: EnvironmentConfig,
    manager: Option<EnvironmentManager>,
}

impl CompilationPipeline {
    /// Create a new compilation pipeline
    pub fn new(config: EnvironmentConfig) -> Self {
        Self {
            env_config: config,
            manager: None,
        }
    }

    /// Create with an environment manager
    pub fn with_manager(config: EnvironmentConfig, manager: EnvironmentManager) -> Self {
        Self {
            env_config: config,
            manager: Some(manager),
        }
    }

    /// Compile source file to WASM
    pub async fn compile(
        &self,
        source: &Path,
        runtime: Runtime,
        config: CompilationConfig,
        progress: Option<ProgressCallback>,
    ) -> EnvironmentResult<CompilationResult> {
        let start = std::time::Instant::now();

        let report = |pct: f32, msg: &str| {
            if let Some(ref cb) = progress {
                cb(pct, msg);
            }
        };

        report(0.0, "Starting compilation...");

        // Compute source hash for caching
        report(0.05, "Computing source hash...");
        let source_hash = self.hash_source(source).await?;

        // Check cache
        if self.env_config.cache_enabled {
            report(0.1, "Checking cache...");
            if let Some(cached) = self.check_cache(&source_hash).await? {
                let cached_size = tokio::fs::metadata(&cached).await?.len();
                report(1.0, "Cache hit!");
                return Ok(CompilationResult {
                    wasm_path: cached,
                    size: cached_size,
                    duration_ms: start.elapsed().as_millis() as u64,
                    cached: true,
                    source_hash,
                    optimization_level: config.optimization_level,
                });
            }
        }

        // Compile based on runtime
        report(0.2, &format!("Compiling {} source...", runtime));
        let wasm_path = match runtime {
            Runtime::NodeJs | Runtime::Bun => {
                self.compile_javascript(source, &config, progress.as_ref()).await?
            }
            Runtime::Python => self.compile_python(source, &config, progress.as_ref()).await?,
            Runtime::Go => self.compile_go(source, &config, progress.as_ref()).await?,
            Runtime::Rust => self.compile_rust(source, &config, progress.as_ref()).await?,
            Runtime::Deno => self.compile_deno(source, &config, progress.as_ref()).await?,
        };

        // Validate WASM
        report(0.8, "Validating WASM...");
        self.validate_wasm(&wasm_path).await?;

        // Optimize if requested
        if config.optimization_level > 0 {
            report(0.85, "Optimizing WASM...");
            self.optimize_wasm(&wasm_path, config.optimization_level).await?;
        }

        // Cache the result
        if self.env_config.cache_enabled {
            report(0.95, "Caching result...");
            self.cache_result(&source_hash, &wasm_path).await?;
        }

        let size = tokio::fs::metadata(&wasm_path).await?.len();
        let duration_ms = start.elapsed().as_millis() as u64;

        report(1.0, "Compilation complete!");

        Ok(CompilationResult {
            wasm_path,
            size,
            duration_ms,
            cached: false,
            source_hash,
            optimization_level: config.optimization_level,
        })
    }

    /// Compile JavaScript/TypeScript to WASM using Javy
    async fn compile_javascript(
        &self,
        source: &Path,
        config: &CompilationConfig,
        progress: Option<&ProgressCallback>,
    ) -> EnvironmentResult<PathBuf> {
        let report = |pct: f32, msg: &str| {
            if let Some(cb) = progress {
                cb(pct, msg);
            }
        };

        let output_path = self.get_output_path(source, "wasm");

        report(0.3, "Running javy compile...");

        let mut cmd = Command::new("javy");
        cmd.arg("compile").arg(source).arg("-o").arg(&output_path);

        // Add optimization flags
        if config.optimization_level >= 2 {
            cmd.arg("--optimize");
        }

        for flag in &config.extra_flags {
            cmd.arg(flag);
        }

        for (key, value) in &config.env_vars {
            cmd.env(key, value);
        }

        let output = cmd.output().await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(EnvironmentError::CompilationFailed {
                message: format!("javy compile failed: {}", stderr),
            });
        }

        Ok(output_path)
    }

    /// Compile Python to WASM using componentize-py
    async fn compile_python(
        &self,
        source: &Path,
        config: &CompilationConfig,
        progress: Option<&ProgressCallback>,
    ) -> EnvironmentResult<PathBuf> {
        let report = |pct: f32, msg: &str| {
            if let Some(cb) = progress {
                cb(pct, msg);
            }
        };

        let output_path = self.get_output_path(source, "wasm");

        report(0.3, "Running componentize-py...");

        let mut cmd = Command::new("componentize-py");
        cmd.arg("-d")
            .arg("wit/") // WIT directory
            .arg("-w")
            .arg("world") // World name
            .arg("componentize")
            .arg(source)
            .arg("-o")
            .arg(&output_path);

        for flag in &config.extra_flags {
            cmd.arg(flag);
        }

        let output = cmd.output().await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(EnvironmentError::CompilationFailed {
                message: format!("componentize-py failed: {}", stderr),
            });
        }

        Ok(output_path)
    }

    /// Compile Go to WASM using TinyGo
    async fn compile_go(
        &self,
        source: &Path,
        config: &CompilationConfig,
        progress: Option<&ProgressCallback>,
    ) -> EnvironmentResult<PathBuf> {
        let report = |pct: f32, msg: &str| {
            if let Some(cb) = progress {
                cb(pct, msg);
            }
        };

        let output_path = self.get_output_path(source, "wasm");

        report(0.3, "Running tinygo build...");

        let target = match config.target {
            CompilationTarget::Wasm32 => "wasm",
            CompilationTarget::WasiComponent | CompilationTarget::WasiP2 => "wasip2",
        };

        let mut cmd = Command::new("tinygo");
        cmd.arg("build")
            .arg("-target")
            .arg(target)
            .arg("-o")
            .arg(&output_path)
            .arg(source);

        // Optimization
        let opt = match config.optimization_level {
            0 => "0",
            1 => "1",
            2 => "2",
            3 => "s",
            _ => "z",
        };
        cmd.arg("-opt").arg(opt);

        for flag in &config.extra_flags {
            cmd.arg(flag);
        }

        let output = cmd.output().await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(EnvironmentError::CompilationFailed {
                message: format!("tinygo build failed: {}", stderr),
            });
        }

        Ok(output_path)
    }

    /// Compile Rust to WASM using cargo-component
    async fn compile_rust(
        &self,
        source: &Path,
        config: &CompilationConfig,
        progress: Option<&ProgressCallback>,
    ) -> EnvironmentResult<PathBuf> {
        let report = |pct: f32, msg: &str| {
            if let Some(cb) = progress {
                cb(pct, msg);
            }
        };

        // For Rust, source should be a Cargo.toml directory
        let project_dir = if source.is_file() {
            source.parent().unwrap_or(source)
        } else {
            source
        };

        report(0.3, "Running cargo component build...");

        let mut cmd = Command::new("cargo");
        cmd.arg("component").arg("build").current_dir(project_dir);

        if config.optimization_level >= 2 {
            cmd.arg("--release");
        }

        for flag in &config.extra_flags {
            cmd.arg(flag);
        }

        let output = cmd.output().await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(EnvironmentError::CompilationFailed {
                message: format!("cargo component build failed: {}", stderr),
            });
        }

        // Find the output WASM file
        let target_dir = project_dir.join("target");
        let profile = if config.optimization_level >= 2 {
            "release"
        } else {
            "debug"
        };

        let wasm_dir = target_dir.join("wasm32-wasip2").join(profile);

        // Find .wasm file in the directory
        let mut entries = tokio::fs::read_dir(&wasm_dir).await?;
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.extension().map(|e| e == "wasm").unwrap_or(false) {
                return Ok(path);
            }
        }

        Err(EnvironmentError::CompilationFailed {
            message: "Could not find compiled WASM file".into(),
        })
    }

    /// Compile Deno/TypeScript to WASM
    async fn compile_deno(
        &self,
        source: &Path,
        config: &CompilationConfig,
        progress: Option<&ProgressCallback>,
    ) -> EnvironmentResult<PathBuf> {
        let report = |pct: f32, msg: &str| {
            if let Some(cb) = progress {
                cb(pct, msg);
            }
        };

        let output_path = self.get_output_path(source, "wasm");

        report(0.3, "Compiling TypeScript to JavaScript...");

        // First, bundle TypeScript to JavaScript
        let bundle_path = self.get_output_path(source, "js");

        let mut cmd = Command::new("deno");
        cmd.arg("bundle").arg(source).arg(&bundle_path);

        let output = cmd.output().await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(EnvironmentError::CompilationFailed {
                message: format!("deno bundle failed: {}", stderr),
            });
        }

        report(0.5, "Compiling JavaScript to WASM via javy...");

        // Then compile bundled JS to WASM via javy
        self.compile_javascript(&bundle_path, config, progress).await?;

        // Clean up intermediate file
        let _ = tokio::fs::remove_file(&bundle_path).await;

        Ok(output_path)
    }

    /// Validate a WASM file
    async fn validate_wasm(&self, wasm_path: &Path) -> EnvironmentResult<()> {
        // Try wasm-tools validate
        let output = Command::new("wasm-tools").arg("validate").arg(wasm_path).output().await;

        match output {
            Ok(out) if out.status.success() => Ok(()),
            Ok(out) => {
                let stderr = String::from_utf8_lossy(&out.stderr);
                Err(EnvironmentError::InvalidWasm {
                    reason: stderr.to_string(),
                })
            }
            Err(_) => {
                // wasm-tools not available, do basic validation
                let bytes = tokio::fs::read(wasm_path).await?;

                // Check WASM magic number
                if bytes.len() < 8 || &bytes[0..4] != b"\0asm" {
                    return Err(EnvironmentError::InvalidWasm {
                        reason: "Invalid WASM magic number".into(),
                    });
                }

                Ok(())
            }
        }
    }

    /// Optimize WASM using wasm-opt
    async fn optimize_wasm(&self, wasm_path: &Path, level: u8) -> EnvironmentResult<()> {
        let opt_flag = match level {
            0 => return Ok(()), // No optimization
            1 => "-O1",
            2 => "-O2",
            3 => "-O3",
            4 => "-O4",
            _ => "-Oz",
        };

        let output = Command::new("wasm-opt")
            .arg(opt_flag)
            .arg(wasm_path)
            .arg("-o")
            .arg(wasm_path)
            .output()
            .await;

        match output {
            Ok(out) if out.status.success() => Ok(()),
            Ok(_) | Err(_) => {
                // wasm-opt not available, skip optimization
                Ok(())
            }
        }
    }

    /// Hash source file for caching
    async fn hash_source(&self, source: &Path) -> EnvironmentResult<[u8; 32]> {
        let content = tokio::fs::read(source).await?;
        let mut hasher = Sha256::new();
        hasher.update(&content);
        Ok(hasher.finalize().into())
    }

    /// Check if compiled WASM is in cache
    async fn check_cache(&self, hash: &[u8; 32]) -> EnvironmentResult<Option<PathBuf>> {
        let hash_hex = hex::encode(hash);
        let cache_path = self.env_config.cache_dir.join(format!("{}.wasm", hash_hex));

        if cache_path.exists() {
            Ok(Some(cache_path))
        } else {
            Ok(None)
        }
    }

    /// Cache compiled WASM
    async fn cache_result(&self, hash: &[u8; 32], wasm_path: &Path) -> EnvironmentResult<()> {
        let hash_hex = hex::encode(hash);
        let cache_path = self.env_config.cache_dir.join(format!("{}.wasm", hash_hex));

        tokio::fs::copy(wasm_path, cache_path).await?;

        // TODO: Prune cache if over size limit

        Ok(())
    }

    /// Get output path for compiled file
    fn get_output_path(&self, source: &Path, ext: &str) -> PathBuf {
        let stem = source.file_stem().unwrap_or_default();
        self.env_config.cache_dir.join(format!(
            "{}-{}.{}",
            stem.to_string_lossy(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis(),
            ext
        ))
    }

    /// Stream compilation output
    pub async fn compile_with_streaming(
        &self,
        source: &Path,
        runtime: Runtime,
        config: CompilationConfig,
    ) -> EnvironmentResult<impl tokio_stream::Stream<Item = String>> {
        use tokio_stream::StreamExt;

        let cmd_args = self.get_compile_command(source, runtime, &config)?;

        let mut child = Command::new(&cmd_args[0])
            .args(&cmd_args[1..])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        let stdout = child.stdout.take().ok_or_else(|| EnvironmentError::CompilationFailed {
            message: "Failed to capture stdout".into(),
        })?;

        let reader = BufReader::new(stdout);
        let lines = tokio_stream::wrappers::LinesStream::new(reader.lines());

        Ok(lines.filter_map(|line| line.ok()))
    }

    /// Get the compile command for a runtime
    fn get_compile_command(
        &self,
        source: &Path,
        runtime: Runtime,
        config: &CompilationConfig,
    ) -> EnvironmentResult<Vec<String>> {
        let output_path = self.get_output_path(source, "wasm");

        let cmd: Vec<String> = match runtime {
            Runtime::NodeJs | Runtime::Bun => {
                vec![
                    "javy".to_string(),
                    "compile".to_string(),
                    source.to_string_lossy().into_owned(),
                    "-o".to_string(),
                    output_path.to_string_lossy().into_owned(),
                ]
            }
            Runtime::Python => {
                vec![
                    "componentize-py".to_string(),
                    "componentize".to_string(),
                    source.to_string_lossy().into_owned(),
                    "-o".to_string(),
                    output_path.to_string_lossy().into_owned(),
                ]
            }
            Runtime::Go => {
                vec![
                    "tinygo".to_string(),
                    "build".to_string(),
                    "-target".to_string(),
                    "wasip2".to_string(),
                    "-o".to_string(),
                    output_path.to_string_lossy().into_owned(),
                    source.to_string_lossy().into_owned(),
                ]
            }
            Runtime::Rust => {
                vec![
                    "cargo".to_string(),
                    "component".to_string(),
                    "build".to_string(),
                    if config.optimization_level >= 2 {
                        "--release".to_string()
                    } else {
                        String::new()
                    },
                ]
            }
            Runtime::Deno => {
                vec![
                    "deno".to_string(),
                    "bundle".to_string(),
                    source.to_string_lossy().into_owned(),
                ]
            }
        };

        Ok(cmd.into_iter().filter(|s| !s.is_empty()).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compilation_target_default() {
        assert_eq!(CompilationTarget::default(), CompilationTarget::WasiP2);
    }

    #[test]
    fn test_compilation_config_default() {
        let config = CompilationConfig::default();
        assert_eq!(config.optimization_level, 2);
        assert!(!config.debug);
    }

    #[tokio::test]
    async fn test_hash_source() {
        let pipeline = CompilationPipeline::new(EnvironmentConfig::default());

        // Create a temp file
        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("test_hash.js");
        tokio::fs::write(&test_file, b"console.log('hello');").await.unwrap();

        let hash = pipeline.hash_source(&test_file).await.unwrap();
        assert_eq!(hash.len(), 32);

        // Same content should produce same hash
        let hash2 = pipeline.hash_source(&test_file).await.unwrap();
        assert_eq!(hash, hash2);

        tokio::fs::remove_file(test_file).await.ok();
    }
}
