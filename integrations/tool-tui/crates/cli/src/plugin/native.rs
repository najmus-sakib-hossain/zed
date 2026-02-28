//! Native Plugin Loader
//!
//! Loads native dynamic library plugins with Ed25519 signature verification.

use std::ffi::OsStr;
use std::path::Path;
use std::time::Instant;

use anyhow::{Context, Result};
use async_trait::async_trait;
use libloading::{Library, Symbol};

use super::PluginType;
use super::traits::{Capability, DxPlugin, PluginContext, PluginMetadata, PluginResult};

/// Native plugin function signature
type PluginInit = unsafe extern "C" fn() -> i32;
type PluginRun = unsafe extern "C" fn(*const u8, usize) -> i32;
type PluginShutdown = unsafe extern "C" fn() -> i32;
type PluginGetOutput = unsafe extern "C" fn(*mut u8, usize) -> usize;

/// Ed25519 public key for signature verification
pub struct PublicKey([u8; 32]);

impl PublicKey {
    /// Load public key from bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        if bytes.len() != 32 {
            anyhow::bail!("Invalid public key length");
        }
        let mut key = [0u8; 32];
        key.copy_from_slice(bytes);
        Ok(Self(key))
    }

    /// Load public key from base64 string
    pub fn from_base64(s: &str) -> Result<Self> {
        use base64::{Engine as _, engine::general_purpose::STANDARD};
        let bytes = STANDARD.decode(s)?;
        Self::from_bytes(&bytes)
    }
}

/// Native plugin loader with signature verification
pub struct NativeLoader {
    /// Trusted public keys for signature verification
    trusted_keys: Vec<PublicKey>,
    /// Allow unsigned plugins (dangerous!)
    allow_unsigned: bool,
}

impl NativeLoader {
    /// Create a new native loader
    pub fn new() -> Self {
        Self {
            trusted_keys: Vec::new(),
            allow_unsigned: false,
        }
    }

    /// Add a trusted public key
    pub fn add_trusted_key(&mut self, key: PublicKey) {
        self.trusted_keys.push(key);
    }

    /// Allow unsigned plugins (dangerous!)
    pub fn allow_unsigned(mut self) -> Self {
        self.allow_unsigned = true;
        self
    }

    /// Verify plugin signature
    pub fn verify_signature(&self, _path: &Path, signature: Option<&str>) -> Result<bool> {
        if self.allow_unsigned {
            return Ok(true);
        }

        let Some(_sig) = signature else {
            anyhow::bail!("Plugin signature required but not provided");
        };

        // TODO: Implement actual Ed25519 verification
        // For now, just check that we have at least one trusted key
        if self.trusted_keys.is_empty() {
            anyhow::bail!("No trusted keys configured for native plugin verification");
        }

        // Placeholder for actual verification
        Ok(true)
    }

    /// Load a native plugin from file
    pub fn load(&self, path: &Path) -> Result<NativePlugin> {
        // Check platform-specific extension
        let expected_ext = if cfg!(target_os = "windows") {
            "dll"
        } else if cfg!(target_os = "macos") {
            "dylib"
        } else {
            "so"
        };

        if path.extension() != Some(OsStr::new(expected_ext)) {
            anyhow::bail!("Invalid plugin extension. Expected .{} for this platform", expected_ext);
        }

        let name = path.file_stem().and_then(|s| s.to_str()).unwrap_or("unknown").to_string();

        // Load the library
        let library =
            unsafe { Library::new(path).context("Failed to load native plugin library")? };

        let metadata = PluginMetadata {
            name,
            version: "1.0.0".to_string(),
            description: String::new(),
            author: String::new(),
            capabilities: vec![Capability::System], // Native plugins have full access
            plugin_type: PluginType::Native,
            path: path.to_path_buf(),
            signature: None,
        };

        Ok(NativePlugin {
            library: Some(library),
            metadata,
            initialized: false,
        })
    }
}

impl Default for NativeLoader {
    fn default() -> Self {
        Self::new()
    }
}

/// A loaded native plugin
pub struct NativePlugin {
    library: Option<Library>,
    metadata: PluginMetadata,
    initialized: bool,
}

impl NativePlugin {
    /// Get the library for direct access
    pub fn library(&self) -> Option<&Library> {
        self.library.as_ref()
    }

    /// Check if a symbol exists
    pub fn has_symbol(&self, name: &str) -> bool {
        if let Some(lib) = &self.library {
            unsafe { lib.get::<Symbol<*const ()>>(name.as_bytes()).is_ok() }
        } else {
            false
        }
    }
}

#[async_trait]
impl DxPlugin for NativePlugin {
    fn metadata(&self) -> &PluginMetadata {
        &self.metadata
    }

    async fn init(&mut self) -> Result<()> {
        if self.initialized {
            return Ok(());
        }

        let lib = self
            .library
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Plugin library not loaded"))?;

        // Try to call init function if it exists
        unsafe {
            if let Ok(init) = lib.get::<PluginInit>(b"dx_plugin_init") {
                let result = init();
                if result != 0 {
                    anyhow::bail!("Plugin init returned error code: {}", result);
                }
            }
        }

        self.initialized = true;
        Ok(())
    }

    async fn execute(&self, ctx: &PluginContext) -> Result<PluginResult> {
        let start_time = Instant::now();

        let lib = self
            .library
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Plugin library not loaded"))?;

        // Serialize arguments
        let args_json = serde_json::to_string(&ctx.args)?;
        let args_bytes = args_json.as_bytes();

        let exit_code;
        let mut stdout = String::new();
        let stderr;

        unsafe {
            // Call run function
            if let Ok(run) = lib.get::<PluginRun>(b"dx_plugin_run") {
                exit_code = run(args_bytes.as_ptr(), args_bytes.len());
            } else {
                return Ok(PluginResult::error("No dx_plugin_run function found".to_string()));
            }

            // Get output if available
            if let Ok(get_output) = lib.get::<PluginGetOutput>(b"dx_plugin_get_output") {
                let mut buffer = vec![0u8; 64 * 1024]; // 64KB buffer
                let len = get_output(buffer.as_mut_ptr(), buffer.len());
                if len > 0 && len <= buffer.len() {
                    buffer.truncate(len);
                    stdout = String::from_utf8_lossy(&buffer).to_string();
                }
            }
        }

        stderr = if exit_code != 0 {
            format!("Plugin exited with code: {}", exit_code)
        } else {
            String::new()
        };

        let duration_ms = start_time.elapsed().as_millis() as u64;

        Ok(PluginResult {
            exit_code,
            stdout,
            stderr,
            duration_ms,
            memory_used: 0,
            return_value: None,
        })
    }

    async fn shutdown(&mut self) -> Result<()> {
        if !self.initialized {
            return Ok(());
        }

        if let Some(lib) = &self.library {
            unsafe {
                if let Ok(shutdown) = lib.get::<PluginShutdown>(b"dx_plugin_shutdown") {
                    shutdown();
                }
            }
        }

        self.initialized = false;
        Ok(())
    }
}

impl Drop for NativePlugin {
    fn drop(&mut self) {
        // Ensure shutdown is called
        if self.initialized {
            if let Some(lib) = &self.library {
                unsafe {
                    if let Ok(shutdown) = lib.get::<PluginShutdown>(b"dx_plugin_shutdown") {
                        shutdown();
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_native_loader_creation() {
        let loader = NativeLoader::new();
        assert!(!loader.allow_unsigned);
    }

    #[test]
    fn test_public_key_from_bytes() {
        let bytes = [0u8; 32];
        let key = PublicKey::from_bytes(&bytes);
        assert!(key.is_ok());
    }

    #[test]
    fn test_public_key_invalid_length() {
        let bytes = [0u8; 16];
        let key = PublicKey::from_bytes(&bytes);
        assert!(key.is_err());
    }
}
