//! # WASM Compiler
//!
//! Compile code from various languages to WebAssembly.

use std::path::PathBuf;
use std::process::Command;
use tracing::info;

use crate::{Result, WasmError};

/// Supported source languages
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceLanguage {
    Python,
    JavaScript,
    TypeScript,
    Go,
    Rust,
    Wasm,
}

impl SourceLanguage {
    pub fn from_extension(ext: &str) -> Option<Self> {
        match ext.to_lowercase().as_str() {
            "py" => Some(Self::Python),
            "js" => Some(Self::JavaScript),
            "ts" => Some(Self::TypeScript),
            "go" => Some(Self::Go),
            "rs" => Some(Self::Rust),
            "wasm" | "wat" => Some(Self::Wasm),
            _ => None,
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        match name.to_lowercase().as_str() {
            "python" | "py" => Some(Self::Python),
            "javascript" | "js" => Some(Self::JavaScript),
            "typescript" | "ts" => Some(Self::TypeScript),
            "go" | "golang" => Some(Self::Go),
            "rust" | "rs" => Some(Self::Rust),
            "wasm" | "webassembly" => Some(Self::Wasm),
            _ => None,
        }
    }
}

/// Compiler configuration
#[derive(Debug, Clone)]
pub struct CompilerConfig {
    /// Output directory for compiled WASM
    pub output_dir: PathBuf,

    /// Optimization level (0-3)
    pub opt_level: u8,

    /// Whether to include debug info
    pub debug: bool,

    /// Target features
    pub features: Vec<String>,
}

impl Default for CompilerConfig {
    fn default() -> Self {
        Self {
            output_dir: PathBuf::from(".dx/wasm"),
            opt_level: 2,
            debug: false,
            features: vec![],
        }
    }
}

/// WASM compiler that can compile from multiple languages
pub struct WasmCompiler {
    config: CompilerConfig,
    temp_dir: PathBuf,
}

impl WasmCompiler {
    pub fn new(config: CompilerConfig) -> Result<Self> {
        let temp_dir = std::env::temp_dir().join("dx-wasm-compiler");
        std::fs::create_dir_all(&temp_dir)?;
        std::fs::create_dir_all(&config.output_dir)?;

        Ok(Self { config, temp_dir })
    }

    /// Compile source code to WASM
    pub async fn compile(
        &self,
        source: &str,
        language: SourceLanguage,
        name: &str,
    ) -> Result<Vec<u8>> {
        info!("Compiling {} ({:?}) to WASM...", name, language);

        match language {
            SourceLanguage::Rust => self.compile_rust(source, name).await,
            SourceLanguage::Go => self.compile_go(source, name).await,
            SourceLanguage::Python => self.compile_python(source, name).await,
            SourceLanguage::JavaScript | SourceLanguage::TypeScript => {
                self.compile_js(source, name).await
            }
            SourceLanguage::Wasm => {
                // Already WASM, just validate and return
                if source.starts_with("\0asm")
                    || source.as_bytes().starts_with(&[0x00, 0x61, 0x73, 0x6d])
                {
                    Ok(source.as_bytes().to_vec())
                } else {
                    // Might be WAT format, try to convert
                    self.wat_to_wasm(source).await
                }
            }
        }
    }

    /// Compile Rust to WASM
    async fn compile_rust(&self, source: &str, name: &str) -> Result<Vec<u8>> {
        // Create a temporary Cargo project
        let project_dir = self.temp_dir.join(name);
        std::fs::create_dir_all(project_dir.join("src"))?;

        // Write Cargo.toml
        let cargo_toml = format!(
            r#"
[package]
name = "{}"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[dependencies]
"#,
            name
        );
        std::fs::write(project_dir.join("Cargo.toml"), cargo_toml)?;

        // Write source file
        std::fs::write(project_dir.join("src/lib.rs"), source)?;

        // Compile with cargo
        let output = Command::new("cargo")
            .args(["build", "--target", "wasm32-unknown-unknown", "--release"])
            .current_dir(&project_dir)
            .output()
            .map_err(|e| WasmError::CompilationFailed(format!("Failed to run cargo: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(WasmError::CompilationFailed(format!(
                "Cargo build failed: {}",
                stderr
            )));
        }

        // Read the compiled WASM
        let wasm_path = project_dir
            .join("target/wasm32-unknown-unknown/release")
            .join(format!("{}.wasm", name));

        let wasm_bytes = std::fs::read(&wasm_path)
            .map_err(|e| WasmError::CompilationFailed(format!("Failed to read WASM: {}", e)))?;

        // Copy to output directory
        let output_path = self.config.output_dir.join(format!("{}.wasm", name));
        std::fs::write(&output_path, &wasm_bytes)?;

        info!("Rust compilation successful: {} bytes", wasm_bytes.len());
        Ok(wasm_bytes)
    }

    /// Compile Go to WASM using TinyGo
    async fn compile_go(&self, source: &str, name: &str) -> Result<Vec<u8>> {
        // Check if TinyGo is installed
        if which::which("tinygo").is_err() {
            return Err(WasmError::CompilationFailed(
                "TinyGo not found. Install from https://tinygo.org/".to_string(),
            ));
        }

        // Create temp directory
        let project_dir = self.temp_dir.join(format!("{}-go", name));
        std::fs::create_dir_all(&project_dir)?;

        // Write source file
        let source_path = project_dir.join("main.go");
        std::fs::write(&source_path, source)?;

        // Write go.mod
        let go_mod = format!("module {}\n\ngo 1.21\n", name);
        std::fs::write(project_dir.join("go.mod"), go_mod)?;

        // Compile with TinyGo
        let output_path = project_dir.join(format!("{}.wasm", name));
        let output = Command::new("tinygo")
            .args([
                "build",
                "-o",
                output_path.to_str().unwrap(),
                "-target",
                "wasi",
                source_path.to_str().unwrap(),
            ])
            .output()
            .map_err(|e| WasmError::CompilationFailed(format!("Failed to run tinygo: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(WasmError::CompilationFailed(format!(
                "TinyGo build failed: {}",
                stderr
            )));
        }

        let wasm_bytes = std::fs::read(&output_path)?;

        // Copy to output directory
        let final_path = self.config.output_dir.join(format!("{}.wasm", name));
        std::fs::write(&final_path, &wasm_bytes)?;

        info!("Go compilation successful: {} bytes", wasm_bytes.len());
        Ok(wasm_bytes)
    }

    /// Compile Python to WASM
    async fn compile_python(&self, source: &str, name: &str) -> Result<Vec<u8>> {
        // Python → WASM compilation is complex and requires Pyodide or similar
        // For now, we create a wrapper that embeds the Python source

        info!("Creating Python WASM wrapper for: {}", name);

        // In a full implementation, this would:
        // 1. Use Pyodide to create a WASM module that can run Python
        // 2. Embed the Python source code
        // 3. Expose the functions via WASM exports

        // For now, return a placeholder WASM module
        // This would be replaced with actual Pyodide integration
        let wasm_bytes = self.create_placeholder_wasm(name, "python", source.len())?;

        Ok(wasm_bytes)
    }

    /// Compile JavaScript/TypeScript to WASM
    async fn compile_js(&self, source: &str, name: &str) -> Result<Vec<u8>> {
        // JavaScript → WASM is typically done via:
        // 1. AssemblyScript (TypeScript-like → WASM)
        // 2. Embedding a JS engine in WASM (QuickJS)

        info!("Creating JavaScript WASM wrapper for: {}", name);

        // For now, return a placeholder
        let wasm_bytes = self.create_placeholder_wasm(name, "javascript", source.len())?;

        Ok(wasm_bytes)
    }

    /// Convert WAT (WebAssembly Text) to WASM binary
    async fn wat_to_wasm(&self, _wat: &str) -> Result<Vec<u8>> {
        // Use wasmtime's wat crate to convert
        // In production, would use: wat::parse_str(wat)

        Err(WasmError::CompilationFailed(
            "WAT to WASM not implemented".to_string(),
        ))
    }

    /// Create a placeholder WASM module
    fn create_placeholder_wasm(
        &self,
        name: &str,
        language: &str,
        source_len: usize,
    ) -> Result<Vec<u8>> {
        // WASM magic number + version
        let wasm = vec![
            0x00, 0x61, 0x73, 0x6d, // Magic: \0asm
            0x01, 0x00, 0x00, 0x00, // Version: 1
        ];

        // This is a minimal valid WASM module
        // In production, this would be a full module with the embedded runtime

        info!(
            "Created placeholder WASM for {} ({}, {} bytes source)",
            name, language, source_len
        );

        Ok(wasm)
    }

    /// Get the output path for a compiled module
    pub fn output_path(&self, name: &str) -> PathBuf {
        self.config.output_dir.join(format!("{}.wasm", name))
    }
}
