//! # WASM Module
//!
//! Represents a loaded WASM module.

use crate::Result;

/// Represents exported functions from a WASM module
#[derive(Debug, Clone)]
pub struct ModuleExports {
    pub functions: Vec<String>,
    pub globals: Vec<String>,
    pub memories: Vec<String>,
    pub tables: Vec<String>,
}

/// A loaded WASM module
pub struct WasmModule {
    name: String,
    bytes: Vec<u8>,
    exports: Vec<String>,
}

impl WasmModule {
    pub fn new(name: &str, bytes: Vec<u8>) -> Result<Self> {
        // Parse exports from WASM binary
        let exports = Self::parse_exports(&bytes)?;

        Ok(Self {
            name: name.to_string(),
            bytes,
            exports,
        })
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    pub fn exports(&self) -> &[String] {
        &self.exports
    }

    /// Parse exports from WASM binary
    fn parse_exports(_bytes: &[u8]) -> Result<Vec<String>> {
        // In a full implementation, this would parse the WASM binary
        // to extract the export section

        // For now, return default exports that integrations typically have
        Ok(vec![
            "init".to_string(),
            "handle".to_string(),
            "cleanup".to_string(),
        ])
    }

    /// Check if the module exports a function
    pub fn has_export(&self, name: &str) -> bool {
        self.exports.contains(&name.to_string())
    }

    /// Get the size of the module in bytes
    pub fn size(&self) -> usize {
        self.bytes.len()
    }

    /// Convert module info to DX format
    pub fn to_dx(&self) -> String {
        format!(
            "module:1[name={} size={} exports[{}]={}]",
            self.name,
            self.bytes.len(),
            self.exports.len(),
            self.exports.join(" ")
        )
    }
}
