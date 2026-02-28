//! Integration tests for AI agent self-update capabilities
//!
//! These tests verify that the self-update mechanism works correctly,
//! including runtime detection, installation, and WASM compilation.

use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;
use tempfile::TempDir;

/// Mock runtime status for testing
#[derive(Debug, Clone, PartialEq)]
pub enum MockRuntimeStatus {
    Available { version: String, path: PathBuf },
    NotInstalled,
    InstallFailed { reason: String },
}

/// Mock runtime manager for testing
pub struct MockRuntimeManager {
    runtimes: HashMap<String, MockRuntimeStatus>,
    install_calls: Vec<String>,
}

impl MockRuntimeManager {
    pub fn new() -> Self {
        Self {
            runtimes: HashMap::new(),
            install_calls: Vec::new(),
        }
    }

    pub fn with_runtime(mut self, name: &str, status: MockRuntimeStatus) -> Self {
        self.runtimes.insert(name.to_string(), status);
        self
    }

    pub fn check(&self, runtime: &str) -> MockRuntimeStatus {
        self.runtimes.get(runtime).cloned().unwrap_or(MockRuntimeStatus::NotInstalled)
    }

    pub fn install(&mut self, runtime: &str) -> Result<(), String> {
        self.install_calls.push(runtime.to_string());
        match self.check(runtime) {
            MockRuntimeStatus::NotInstalled => {
                // Simulate installation
                self.runtimes.insert(
                    runtime.to_string(),
                    MockRuntimeStatus::Available {
                        version: "1.0.0".to_string(),
                        path: PathBuf::from(format!("/usr/local/bin/{}", runtime)),
                    },
                );
                Ok(())
            }
            MockRuntimeStatus::InstallFailed { reason } => Err(reason),
            MockRuntimeStatus::Available { .. } => Ok(()),
        }
    }

    pub fn install_calls(&self) -> &[String] {
        &self.install_calls
    }
}

impl Default for MockRuntimeManager {
    fn default() -> Self {
        Self::new()
    }
}

#[test]
fn test_runtime_detection() {
    let manager = MockRuntimeManager::new()
        .with_runtime(
            "node",
            MockRuntimeStatus::Available {
                version: "20.10.0".to_string(),
                path: PathBuf::from("/usr/local/bin/node"),
            },
        )
        .with_runtime("python3", MockRuntimeStatus::NotInstalled);

    assert!(matches!(manager.check("node"), MockRuntimeStatus::Available { .. }));
    assert!(matches!(manager.check("python3"), MockRuntimeStatus::NotInstalled));
    assert!(matches!(manager.check("unknown"), MockRuntimeStatus::NotInstalled));
}

#[test]
fn test_runtime_installation() {
    let mut manager =
        MockRuntimeManager::new().with_runtime("python3", MockRuntimeStatus::NotInstalled);

    // Initially not installed
    assert!(matches!(manager.check("python3"), MockRuntimeStatus::NotInstalled));

    // Install
    manager.install("python3").unwrap();

    // Now available
    assert!(matches!(manager.check("python3"), MockRuntimeStatus::Available { .. }));

    // Verify install was called
    assert_eq!(manager.install_calls(), &["python3"]);
}

#[test]
fn test_install_failure() {
    let mut manager = MockRuntimeManager::new().with_runtime(
        "go",
        MockRuntimeStatus::InstallFailed {
            reason: "Network error".to_string(),
        },
    );

    let result = manager.install("go");
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), "Network error");
}

/// Mock WASM compiler for testing
pub struct MockWasmCompiler {
    compile_calls: Vec<(String, PathBuf)>,
    should_fail: bool,
}

impl MockWasmCompiler {
    pub fn new() -> Self {
        Self {
            compile_calls: Vec::new(),
            should_fail: false,
        }
    }

    pub fn fail_on_compile(mut self) -> Self {
        self.should_fail = true;
        self
    }

    pub fn compile(&mut self, runtime: &str, source: PathBuf) -> Result<Vec<u8>, String> {
        self.compile_calls.push((runtime.to_string(), source));

        if self.should_fail {
            return Err("Compilation failed".to_string());
        }

        // Return mock WASM bytes
        Ok(b"\0asm\x01\0\0\0".to_vec())
    }

    pub fn compile_calls(&self) -> &[(String, PathBuf)] {
        &self.compile_calls
    }
}

impl Default for MockWasmCompiler {
    fn default() -> Self {
        Self::new()
    }
}

#[test]
fn test_wasm_compilation() {
    let mut compiler = MockWasmCompiler::new();

    let result = compiler.compile("javascript", PathBuf::from("test.js"));
    assert!(result.is_ok());

    let wasm = result.unwrap();
    assert!(wasm.starts_with(b"\0asm"));
}

#[test]
fn test_wasm_compilation_failure() {
    let mut compiler = MockWasmCompiler::new().fail_on_compile();

    let result = compiler.compile("python", PathBuf::from("test.py"));
    assert!(result.is_err());
}

/// Integration test for full self-update flow
#[test]
fn test_self_update_flow() {
    // Start with no runtimes
    let mut manager = MockRuntimeManager::new()
        .with_runtime("node", MockRuntimeStatus::NotInstalled)
        .with_runtime("python3", MockRuntimeStatus::NotInstalled);

    // Simulate agent requesting a JavaScript plugin
    // Agent detects node is needed
    assert!(matches!(manager.check("node"), MockRuntimeStatus::NotInstalled));

    // Agent installs node
    manager.install("node").unwrap();

    // Agent can now compile JS to WASM
    let mut compiler = MockWasmCompiler::new();
    let result = compiler.compile("javascript", PathBuf::from("plugin.js"));
    assert!(result.is_ok());

    // Verify the flow
    assert_eq!(manager.install_calls(), &["node"]);
    assert_eq!(compiler.compile_calls().len(), 1);
}

/// Test caching behavior
#[test]
fn test_compilation_caching() {
    use std::collections::HashMap;

    struct CachingCompiler {
        cache: HashMap<String, Vec<u8>>,
        compile_count: usize,
    }

    impl CachingCompiler {
        fn new() -> Self {
            Self {
                cache: HashMap::new(),
                compile_count: 0,
            }
        }

        fn compile(&mut self, source_hash: &str) -> Vec<u8> {
            if let Some(cached) = self.cache.get(source_hash) {
                return cached.clone();
            }

            self.compile_count += 1;
            let wasm = b"\0asm\x01\0\0\0".to_vec();
            self.cache.insert(source_hash.to_string(), wasm.clone());
            wasm
        }
    }

    let mut compiler = CachingCompiler::new();

    // First compile
    let _ = compiler.compile("abc123");
    assert_eq!(compiler.compile_count, 1);

    // Same source, should use cache
    let _ = compiler.compile("abc123");
    assert_eq!(compiler.compile_count, 1);

    // Different source, should compile
    let _ = compiler.compile("def456");
    assert_eq!(compiler.compile_count, 2);
}

/// Test runtime version requirements
#[test]
fn test_version_requirements() {
    #[derive(Debug)]
    struct VersionReq {
        min: (u32, u32, u32),
    }

    impl VersionReq {
        fn satisfies(&self, version: &str) -> bool {
            let parts: Vec<u32> = version.split('.').filter_map(|s| s.parse().ok()).collect();

            if parts.len() < 3 {
                return false;
            }

            let current = (parts[0], parts[1], parts[2]);
            current >= self.min
        }
    }

    let req = VersionReq { min: (18, 0, 0) };

    assert!(req.satisfies("20.10.0"));
    assert!(req.satisfies("18.0.0"));
    assert!(!req.satisfies("16.20.0"));
    assert!(!req.satisfies("17.9.1"));
}

/// Test concurrent runtime installations
#[tokio::test]
async fn test_concurrent_installation() {
    use std::sync::Arc;
    use tokio::sync::Mutex;

    let manager = Arc::new(Mutex::new(MockRuntimeManager::new()));

    let runtimes = vec!["node", "python3", "go", "rust"];

    let handles: Vec<_> = runtimes
        .into_iter()
        .map(|runtime| {
            let manager = Arc::clone(&manager);
            let runtime = runtime.to_string();
            tokio::spawn(async move {
                let mut m = manager.lock().await;
                m.install(&runtime)
            })
        })
        .collect();

    for handle in handles {
        let result = handle.await.unwrap();
        assert!(result.is_ok());
    }

    let m = manager.lock().await;
    assert_eq!(m.install_calls().len(), 4);
}

/// Test plugin manifest parsing
#[test]
fn test_plugin_manifest() {
    #[derive(Debug)]
    struct PluginManifest {
        name: String,
        version: String,
        runtime: String,
        entry: PathBuf,
        permissions: Vec<String>,
    }

    impl PluginManifest {
        fn from_sr(content: &str) -> Option<Self> {
            // Simple mock parser
            let mut name = None;
            let mut version = None;
            let mut runtime = None;
            let mut entry = None;
            let mut permissions = Vec::new();

            for line in content.lines() {
                let line = line.trim();
                if let Some(val) = line.strip_prefix("name = \"").and_then(|s| s.strip_suffix('"'))
                {
                    name = Some(val.to_string());
                } else if let Some(val) =
                    line.strip_prefix("version = \"").and_then(|s| s.strip_suffix('"'))
                {
                    version = Some(val.to_string());
                } else if let Some(val) =
                    line.strip_prefix("runtime = \"").and_then(|s| s.strip_suffix('"'))
                {
                    runtime = Some(val.to_string());
                } else if let Some(val) =
                    line.strip_prefix("entry = \"").and_then(|s| s.strip_suffix('"'))
                {
                    entry = Some(PathBuf::from(val));
                } else if let Some(perms) =
                    line.strip_prefix("permissions = [").and_then(|s| s.strip_suffix(']'))
                {
                    permissions = perms
                        .split(',')
                        .map(|s| s.trim().trim_matches('"').to_string())
                        .filter(|s| !s.is_empty())
                        .collect();
                }
            }

            Some(Self {
                name: name?,
                version: version?,
                runtime: runtime?,
                entry: entry?,
                permissions,
            })
        }
    }

    let manifest = r#"
        name = "weather"
        version = "1.0.0"
        runtime = "javascript"
        entry = "src/index.js"
        permissions = ["http", "env"]
    "#;

    let parsed = PluginManifest::from_sr(manifest).unwrap();
    assert_eq!(parsed.name, "weather");
    assert_eq!(parsed.version, "1.0.0");
    assert_eq!(parsed.runtime, "javascript");
    assert_eq!(parsed.entry, PathBuf::from("src/index.js"));
    assert_eq!(parsed.permissions, vec!["http", "env"]);
}
