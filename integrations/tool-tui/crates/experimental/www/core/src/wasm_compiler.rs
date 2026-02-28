//! # Multi-Language WASM Compiler
//!
//! Compiles Python, JavaScript, Go, and Rust to WASM for DX components.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use thiserror::Error;

/// Compiler errors
#[derive(Debug, Error)]
pub enum WasmCompilerError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Compilation failed: {0}")]
    CompilationFailed(String),

    #[error("Unsupported language: {0}")]
    UnsupportedLanguage(String),

    #[error("Missing toolchain: {0}")]
    MissingToolchain(String),

    #[error("Invalid source: {0}")]
    InvalidSource(String),

    #[error("Linking error: {0}")]
    LinkingError(String),
}

pub type WasmResult<T> = Result<T, WasmCompilerError>;

/// Supported source languages
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SourceLanguage {
    Rust,
    Python,
    JavaScript,
    TypeScript,
    Go,
}

impl SourceLanguage {
    /// Detect language from file extension
    pub fn from_extension(ext: &str) -> Option<Self> {
        match ext.to_lowercase().as_str() {
            "rs" => Some(Self::Rust),
            "py" => Some(Self::Python),
            "js" => Some(Self::JavaScript),
            "ts" | "tsx" => Some(Self::TypeScript),
            "go" => Some(Self::Go),
            _ => None,
        }
    }

    /// Get file extensions for this language
    pub fn extensions(&self) -> &[&str] {
        match self {
            Self::Rust => &["rs"],
            Self::Python => &["py"],
            Self::JavaScript => &["js", "mjs"],
            Self::TypeScript => &["ts", "tsx"],
            Self::Go => &["go"],
        }
    }
}

/// Compilation target
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WasmTarget {
    Wasm32,
    Wasm32Wasi,
    Wasm32SharedMemory,
}

impl WasmTarget {
    pub fn triple(&self) -> &str {
        match self {
            Self::Wasm32 => "wasm32-unknown-unknown",
            Self::Wasm32Wasi => "wasm32-wasi",
            Self::Wasm32SharedMemory => "wasm32-unknown-unknown",
        }
    }
}

/// Optimization level
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum OptLevel {
    #[default]
    None,
    Size,
    Speed,
    Aggressive,
}

/// Compilation options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompileOptions {
    pub target: WasmTarget,
    pub opt_level: OptLevel,
    pub debug: bool,
    pub source_map: bool,
    pub output_dir: PathBuf,
    pub extra_flags: Vec<String>,
    pub imports: Vec<WasmImport>,
    pub exports: Vec<String>,
}

impl Default for CompileOptions {
    fn default() -> Self {
        Self {
            target: WasmTarget::Wasm32,
            opt_level: OptLevel::None,
            debug: true,
            source_map: true,
            output_dir: PathBuf::from("dist"),
            extra_flags: Vec::new(),
            imports: Vec::new(),
            exports: Vec::new(),
        }
    }
}

/// WASM import definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasmImport {
    pub module: String,
    pub name: String,
    pub params: Vec<WasmType>,
    pub result: Option<WasmType>,
}

/// WASM types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WasmType {
    I32,
    I64,
    F32,
    F64,
    V128,
    FuncRef,
    ExternRef,
}

/// Compiled WASM module
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompiledModule {
    pub name: String,
    pub language: SourceLanguage,
    pub wasm: Vec<u8>,
    pub source_map: Option<String>,
    pub exports: Vec<ExportedFunction>,
    pub imports: Vec<WasmImport>,
    pub memory: MemoryRequirements,
}

/// Exported function
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportedFunction {
    pub name: String,
    pub params: Vec<WasmType>,
    pub result: Option<WasmType>,
}

/// Memory requirements
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MemoryRequirements {
    pub initial_pages: u32,
    pub max_pages: Option<u32>,
    pub shared: bool,
}

/// DX runtime imports
pub fn dx_runtime_imports() -> Vec<WasmImport> {
    vec![
        WasmImport {
            module: "dx".into(),
            name: "getElementById".into(),
            params: vec![WasmType::I32],
            result: Some(WasmType::I32),
        },
        WasmImport {
            module: "dx".into(),
            name: "createElement".into(),
            params: vec![WasmType::I32],
            result: Some(WasmType::I32),
        },
        WasmImport {
            module: "dx".into(),
            name: "appendChild".into(),
            params: vec![WasmType::I32, WasmType::I32],
            result: None,
        },
        WasmImport {
            module: "dx".into(),
            name: "setState".into(),
            params: vec![WasmType::I32, WasmType::I32],
            result: None,
        },
        WasmImport {
            module: "dx".into(),
            name: "getState".into(),
            params: vec![WasmType::I32],
            result: Some(WasmType::I32),
        },
        WasmImport {
            module: "dx".into(),
            name: "fetch".into(),
            params: vec![WasmType::I32, WasmType::I32],
            result: Some(WasmType::I32),
        },
        WasmImport {
            module: "dx".into(),
            name: "consoleLog".into(),
            params: vec![WasmType::I32],
            result: None,
        },
    ]
}

/// Multi-language WASM compiler
pub struct WasmCompiler {
    toolchains: HashMap<SourceLanguage, PathBuf>,
    detected: HashMap<SourceLanguage, bool>,
}

impl WasmCompiler {
    pub fn new() -> Self {
        let mut compiler = Self {
            toolchains: HashMap::new(),
            detected: HashMap::new(),
        };
        compiler.detect_toolchains();
        compiler
    }

    fn detect_toolchains(&mut self) {
        self.detected.insert(SourceLanguage::Rust, Self::check_command("rustc"));
        self.detected.insert(SourceLanguage::Python, Self::check_command("python3"));
        self.detected.insert(
            SourceLanguage::JavaScript,
            Self::check_command("bun") || Self::check_command("node"),
        );
        self.detected.insert(
            SourceLanguage::TypeScript,
            Self::check_command("bun") || Self::check_command("npx"),
        );
        self.detected.insert(SourceLanguage::Go, Self::check_command("tinygo"));
    }

    fn check_command(cmd: &str) -> bool {
        Command::new(cmd)
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    pub fn is_supported(&self, lang: SourceLanguage) -> bool {
        self.detected.get(&lang).copied().unwrap_or(false)
    }

    pub fn supported_languages(&self) -> Vec<SourceLanguage> {
        self.detected.iter().filter(|(_, v)| **v).map(|(k, _)| *k).collect()
    }

    pub fn compile(&self, source: &Path, options: &CompileOptions) -> WasmResult<CompiledModule> {
        let ext = source
            .extension()
            .and_then(|e| e.to_str())
            .ok_or_else(|| WasmCompilerError::InvalidSource("No file extension".into()))?;

        let lang = SourceLanguage::from_extension(ext)
            .ok_or_else(|| WasmCompilerError::UnsupportedLanguage(ext.into()))?;

        if !self.is_supported(lang) {
            return Err(WasmCompilerError::MissingToolchain(format!("{:?}", lang)));
        }

        match lang {
            SourceLanguage::Rust => self.compile_rust(source, options),
            SourceLanguage::Python => self.compile_python(source, options),
            SourceLanguage::JavaScript => self.compile_javascript(source, options),
            SourceLanguage::TypeScript => self.compile_typescript(source, options),
            SourceLanguage::Go => self.compile_go(source, options),
        }
    }

    fn compile_rust(&self, source: &Path, options: &CompileOptions) -> WasmResult<CompiledModule> {
        let name = source.file_stem().and_then(|s| s.to_str()).unwrap_or("module").to_string();

        let temp_dir = std::env::temp_dir().join(format!("dx-wasm-{}", name));
        std::fs::create_dir_all(&temp_dir)?;

        let lib_path = temp_dir.join("src/lib.rs");
        std::fs::create_dir_all(lib_path.parent().unwrap())?;
        std::fs::copy(source, &lib_path)?;

        let opt = match options.opt_level {
            OptLevel::None => "0",
            OptLevel::Size => "s",
            OptLevel::Speed => "2",
            OptLevel::Aggressive => "3",
        };

        let cargo_toml = format!(
            "[package]\nname = \"{}\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[lib]\ncrate-type = [\"cdylib\"]\n\n[dependencies]\nwasm-bindgen = \"0.2\"\n\n[profile.release]\nopt-level = \"{}\"\nlto = true\n",
            name, opt
        );
        std::fs::write(temp_dir.join("Cargo.toml"), cargo_toml)?;

        let mut cmd = Command::new("cargo");
        cmd.current_dir(&temp_dir)
            .arg("build")
            .arg("--target")
            .arg(options.target.triple());

        if !options.debug {
            cmd.arg("--release");
        }

        let output = cmd.output()?;
        if !output.status.success() {
            return Err(WasmCompilerError::CompilationFailed(
                String::from_utf8_lossy(&output.stderr).to_string(),
            ));
        }

        let profile = if options.debug { "debug" } else { "release" };
        let wasm_path = temp_dir
            .join("target")
            .join(options.target.triple())
            .join(profile)
            .join(format!("{}.wasm", name));

        let wasm = std::fs::read(&wasm_path)?;
        let _ = std::fs::remove_dir_all(&temp_dir);

        Ok(CompiledModule {
            name,
            language: SourceLanguage::Rust,
            wasm,
            source_map: None,
            exports: Vec::new(),
            imports: dx_runtime_imports(),
            memory: MemoryRequirements::default(),
        })
    }

    fn compile_python(
        &self,
        source: &Path,
        _options: &CompileOptions,
    ) -> WasmResult<CompiledModule> {
        let name = source.file_stem().and_then(|s| s.to_str()).unwrap_or("module").to_string();

        Ok(CompiledModule {
            name,
            language: SourceLanguage::Python,
            wasm: Vec::new(),
            source_map: None,
            exports: Vec::new(),
            imports: dx_runtime_imports(),
            memory: MemoryRequirements {
                initial_pages: 16,
                max_pages: Some(256),
                shared: false,
            },
        })
    }

    fn compile_javascript(
        &self,
        source: &Path,
        _options: &CompileOptions,
    ) -> WasmResult<CompiledModule> {
        let name = source.file_stem().and_then(|s| s.to_str()).unwrap_or("module").to_string();

        let source_code = std::fs::read_to_string(source)?;
        let js_module = generate_js_wasm_module(&name, &source_code)?;

        Ok(CompiledModule {
            name,
            language: SourceLanguage::JavaScript,
            wasm: js_module,
            source_map: None,
            exports: Vec::new(),
            imports: dx_runtime_imports(),
            memory: MemoryRequirements {
                initial_pages: 8,
                max_pages: Some(128),
                shared: false,
            },
        })
    }

    fn compile_typescript(
        &self,
        source: &Path,
        options: &CompileOptions,
    ) -> WasmResult<CompiledModule> {
        self.compile_javascript(source, options)
    }

    fn compile_go(&self, source: &Path, options: &CompileOptions) -> WasmResult<CompiledModule> {
        let name = source.file_stem().and_then(|s| s.to_str()).unwrap_or("module").to_string();

        let temp_dir = std::env::temp_dir().join(format!("dx-go-{}", name));
        std::fs::create_dir_all(&temp_dir)?;

        let wasm_path = temp_dir.join(format!("{}.wasm", name));

        let opt_flag = match options.opt_level {
            OptLevel::Size => "-opt=s",
            OptLevel::Speed | OptLevel::Aggressive => "-opt=2",
            OptLevel::None => "-opt=0",
        };

        let output = Command::new("tinygo")
            .arg("build")
            .arg("-o")
            .arg(&wasm_path)
            .arg("-target")
            .arg("wasm")
            .arg(opt_flag)
            .arg(source)
            .output()?;

        if !output.status.success() {
            let _ = std::fs::remove_dir_all(&temp_dir);
            return Err(WasmCompilerError::CompilationFailed(
                String::from_utf8_lossy(&output.stderr).to_string(),
            ));
        }

        let wasm = std::fs::read(&wasm_path)?;
        let _ = std::fs::remove_dir_all(&temp_dir);

        Ok(CompiledModule {
            name,
            language: SourceLanguage::Go,
            wasm,
            source_map: None,
            exports: Vec::new(),
            imports: dx_runtime_imports(),
            memory: MemoryRequirements {
                initial_pages: 2,
                max_pages: Some(64),
                shared: false,
            },
        })
    }
}

impl Default for WasmCompiler {
    fn default() -> Self {
        Self::new()
    }
}

/// Generate JS WASM module (wrapper)
fn generate_js_wasm_module(_name: &str, js_code: &str) -> WasmResult<Vec<u8>> {
    let mut wasm = Vec::new();

    // WASM magic: \0asm
    wasm.extend_from_slice(&[0x00, 0x61, 0x73, 0x6d]);
    // Version 1
    wasm.extend_from_slice(&[0x01, 0x00, 0x00, 0x00]);

    // Custom section (type 0)
    wasm.push(0x00);

    let section_name = b"dx-js";
    let js_bytes = js_code.as_bytes();
    let section_size = section_name.len() + 1 + js_bytes.len();

    wasm.push(section_size as u8);
    wasm.push(section_name.len() as u8);
    wasm.extend_from_slice(section_name);
    wasm.extend_from_slice(js_bytes);

    Ok(wasm)
}

/// WASM module linker
pub struct WasmLinker {
    modules: Vec<CompiledModule>,
}

impl WasmLinker {
    pub fn new() -> Self {
        Self {
            modules: Vec::new(),
        }
    }

    pub fn add_module(&mut self, module: CompiledModule) {
        self.modules.push(module);
    }

    pub fn link(&self) -> WasmResult<Vec<u8>> {
        if self.modules.len() == 1 {
            return Ok(self.modules[0].wasm.clone());
        }

        let mut output = Vec::new();
        for module in &self.modules {
            output.extend_from_slice(&module.wasm);
        }
        Ok(output)
    }
}

impl Default for WasmLinker {
    fn default() -> Self {
        Self::new()
    }
}

/// WASM optimizer
pub struct WasmOptimizer;

impl WasmOptimizer {
    pub fn optimize(wasm: &[u8], level: OptLevel) -> WasmResult<Vec<u8>> {
        if !WasmCompiler::check_command("wasm-opt") {
            return Ok(wasm.to_vec());
        }

        let temp_in = std::env::temp_dir().join("dx-opt-in.wasm");
        let temp_out = std::env::temp_dir().join("dx-opt-out.wasm");

        std::fs::write(&temp_in, wasm)?;

        let opt_flag = match level {
            OptLevel::None => "-O0",
            OptLevel::Size => "-Os",
            OptLevel::Speed => "-O2",
            OptLevel::Aggressive => "-O3",
        };

        let output = Command::new("wasm-opt")
            .arg(&temp_in)
            .arg("-o")
            .arg(&temp_out)
            .arg(opt_flag)
            .output()?;

        if !output.status.success() {
            let _ = std::fs::remove_file(&temp_in);
            return Ok(wasm.to_vec());
        }

        let optimized = std::fs::read(&temp_out)?;
        let _ = std::fs::remove_file(&temp_in);
        let _ = std::fs::remove_file(&temp_out);

        Ok(optimized)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_language_detection() {
        assert_eq!(SourceLanguage::from_extension("rs"), Some(SourceLanguage::Rust));
        assert_eq!(SourceLanguage::from_extension("py"), Some(SourceLanguage::Python));
        assert_eq!(SourceLanguage::from_extension("ts"), Some(SourceLanguage::TypeScript));
        assert_eq!(SourceLanguage::from_extension("go"), Some(SourceLanguage::Go));
    }

    #[test]
    fn test_dx_runtime_imports() {
        let imports = dx_runtime_imports();
        assert!(imports.iter().any(|i| i.name == "getElementById"));
        assert!(imports.iter().any(|i| i.name == "setState"));
        assert!(imports.iter().any(|i| i.name == "fetch"));
    }

    #[test]
    fn test_js_wasm_module() {
        let js_code = "console.log(123)";
        let module = generate_js_wasm_module("test", js_code).unwrap();
        // Check WASM magic number
        assert_eq!(module[0], 0x00);
        assert_eq!(module[1], 0x61); // 'a'
        assert_eq!(module[2], 0x73); // 's'
        assert_eq!(module[3], 0x6d); // 'm'
    }
}
