//! WebAssembly module loader
//!
//! Provides functionality to load and instantiate WebAssembly modules
//! from .wasm files.

use super::{WasmExport, WasmMemory, WasmModule, WasmType};
use crate::error::{DxError, DxResult};
use std::path::Path;

/// WebAssembly module loader
pub struct WasmLoader {
    /// Loaded modules cache
    modules: std::collections::HashMap<String, WasmModule>,
}

impl WasmLoader {
    /// Create a new WASM loader
    pub fn new() -> Self {
        Self {
            modules: std::collections::HashMap::new(),
        }
    }

    /// Load a WebAssembly module from a file
    pub fn load_file(&mut self, path: &Path) -> DxResult<&WasmModule> {
        let path_str = path.to_string_lossy().to_string();

        // Check if already loaded
        if self.modules.contains_key(&path_str) {
            // Safe: we just verified the key exists
            return Ok(self.modules.get(&path_str).expect("key verified to exist"));
        }

        let bytes = std::fs::read(path)
            .map_err(|e| DxError::RuntimeError(format!("Failed to read WASM file: {}", e)))?;

        let module = self.parse_wasm(&bytes, &path_str)?;
        self.modules.insert(path_str.clone(), module);
        // Safe: we just inserted the key
        Ok(self.modules.get(&path_str).expect("key was just inserted"))
    }

    /// Load a WebAssembly module from bytes
    pub fn load_bytes(&mut self, bytes: &[u8], name: &str) -> DxResult<&WasmModule> {
        // Check if already loaded
        if self.modules.contains_key(name) {
            // Safe: we just verified the key exists
            return Ok(self.modules.get(name).expect("key verified to exist"));
        }

        let module = self.parse_wasm(bytes, name)?;
        self.modules.insert(name.to_string(), module);
        // Safe: we just inserted the key
        Ok(self.modules.get(name).expect("key was just inserted"))
    }

    /// Parse WebAssembly binary format
    fn parse_wasm(&self, bytes: &[u8], name: &str) -> DxResult<WasmModule> {
        // Validate magic number and version
        if bytes.len() < 8 {
            return Err(DxError::RuntimeError("Invalid WASM: too short".to_string()));
        }

        let magic = &bytes[0..4];
        if magic != b"\0asm" {
            return Err(DxError::RuntimeError("Invalid WASM: bad magic number".to_string()));
        }

        let version = u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);
        if version != 1 {
            return Err(DxError::RuntimeError(format!("Unsupported WASM version: {}", version)));
        }

        let mut module = WasmModule::new(name.to_string());

        // Parse sections
        let mut offset = 8;
        while offset < bytes.len() {
            let section_id = bytes[offset];
            offset += 1;

            let (section_size, bytes_read) = read_leb128_u32(&bytes[offset..])?;
            offset += bytes_read;

            let section_end = offset + section_size as usize;
            if section_end > bytes.len() {
                return Err(DxError::RuntimeError("Invalid WASM: section overflow".to_string()));
            }

            match section_id {
                // Type section (1)
                1 => {
                    // Parse function types - simplified
                }
                // Import section (2)
                2 => {
                    // Parse imports - simplified
                }
                // Function section (3)
                3 => {
                    // Parse function declarations - simplified
                }
                // Memory section (5)
                5 => {
                    let (count, bytes_read) = read_leb128_u32(&bytes[offset..])?;
                    let mut mem_offset = offset + bytes_read;

                    for _ in 0..count {
                        let flags = bytes[mem_offset];
                        mem_offset += 1;

                        let (initial, bytes_read) = read_leb128_u32(&bytes[mem_offset..])?;
                        mem_offset += bytes_read;

                        let maximum = if flags & 1 != 0 {
                            let (max, bytes_read) = read_leb128_u32(&bytes[mem_offset..])?;
                            mem_offset += bytes_read;
                            Some(max)
                        } else {
                            None
                        };

                        module.set_memory(WasmMemory::new(initial));
                        module.add_export(
                            "memory".to_string(),
                            WasmExport::Memory { initial, maximum },
                        );
                    }
                }
                // Export section (7)
                7 => {
                    let (count, bytes_read) = read_leb128_u32(&bytes[offset..])?;
                    let mut exp_offset = offset + bytes_read;

                    for _ in 0..count {
                        let (name_len, bytes_read) = read_leb128_u32(&bytes[exp_offset..])?;
                        exp_offset += bytes_read;

                        let export_name = String::from_utf8_lossy(
                            &bytes[exp_offset..exp_offset + name_len as usize],
                        )
                        .to_string();
                        exp_offset += name_len as usize;

                        let kind = bytes[exp_offset];
                        exp_offset += 1;

                        let (_index, bytes_read) = read_leb128_u32(&bytes[exp_offset..])?;
                        exp_offset += bytes_read;

                        let export = match kind {
                            0 => WasmExport::Function {
                                params: vec![],
                                results: vec![],
                            },
                            1 => continue, // Table
                            2 => WasmExport::Memory {
                                initial: 1,
                                maximum: None,
                            },
                            3 => WasmExport::Global {
                                value_type: WasmType::I32,
                                mutable: false,
                            },
                            _ => continue,
                        };

                        module.add_export(export_name, export);
                    }
                }
                // Other sections - skip
                _ => {}
            }

            offset = section_end;
        }

        Ok(module)
    }

    /// Get a loaded module by name
    pub fn get(&self, name: &str) -> Option<&WasmModule> {
        self.modules.get(name)
    }
}

impl Default for WasmLoader {
    fn default() -> Self {
        Self::new()
    }
}

/// Read a LEB128-encoded unsigned 32-bit integer
fn read_leb128_u32(bytes: &[u8]) -> DxResult<(u32, usize)> {
    let mut result: u32 = 0;
    let mut shift = 0;
    let mut bytes_read = 0;

    for &byte in bytes {
        bytes_read += 1;
        result |= ((byte & 0x7F) as u32) << shift;

        if byte & 0x80 == 0 {
            return Ok((result, bytes_read));
        }

        shift += 7;
        if shift >= 32 {
            return Err(DxError::RuntimeError("Invalid LEB128 encoding".to_string()));
        }
    }

    Err(DxError::RuntimeError("Unexpected end of LEB128".to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_leb128_decode() {
        // 0 -> 0x00
        assert_eq!(read_leb128_u32(&[0x00]).unwrap(), (0, 1));
        // 1 -> 0x01
        assert_eq!(read_leb128_u32(&[0x01]).unwrap(), (1, 1));
        // 127 -> 0x7F
        assert_eq!(read_leb128_u32(&[0x7F]).unwrap(), (127, 1));
        // 128 -> 0x80 0x01
        assert_eq!(read_leb128_u32(&[0x80, 0x01]).unwrap(), (128, 2));
        // 624485 -> 0xE5 0x8E 0x26
        assert_eq!(read_leb128_u32(&[0xE5, 0x8E, 0x26]).unwrap(), (624485, 3));
    }
}
