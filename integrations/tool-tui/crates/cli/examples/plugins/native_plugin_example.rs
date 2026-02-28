//! Example Native Plugin for DX
//!
//! This demonstrates how to create a native (dynamically loaded) plugin
//! that can be loaded by the DX plugin system.
//!
//! # Building
//!
//! ```bash
//! cargo build --release --lib
//! ```
//!
//! # Signing (Required for native plugins)
//!
//! ```bash
//! dx forge sign target/release/libmy_plugin.so
//! ```
//!
//! # Plugin Structure
//!
//! ```
//! my-native-plugin/
//! ├── Cargo.toml
//! ├── plugin.sr
//! ├── signature.bin
//! └── src/
//!     └── lib.rs (this file)
//! ```

use std::collections::HashMap;
use std::ffi::{c_char, CStr, CString};
use std::ptr;

// ============================================================================
// C ABI Types (for FFI)
// ============================================================================

/// Plugin metadata returned to the host
#[repr(C)]
pub struct NativePluginMetadata {
    pub name: *const c_char,
    pub version: *const c_char,
    pub description: *const c_char,
    pub author: *const c_char,
    pub capabilities_count: usize,
    pub capabilities: *const *const c_char,
}

/// Plugin context passed from the host
#[repr(C)]
pub struct NativePluginContext {
    pub args_count: usize,
    pub args: *const *const c_char,
    pub working_dir: *const c_char,
}

/// Plugin result returned to the host
#[repr(C)]
pub struct NativePluginResult {
    pub success: bool,
    pub message: *const c_char,
    pub data: *const u8,
    pub data_len: usize,
}

// ============================================================================
// Required Exports (C ABI)
// ============================================================================

/// Plugin metadata - called once when plugin is loaded
#[no_mangle]
pub extern "C" fn dx_plugin_metadata() -> NativePluginMetadata {
    // These must be static or leaked to avoid dangling pointers
    static NAME: &str = "native-example\0";
    static VERSION: &str = "1.0.0\0";
    static DESCRIPTION: &str = "Example native plugin demonstrating C ABI\0";
    static AUTHOR: &str = "DX Team\0";
    static CAP_FS: &str = "fs:read\0";
    static CAPS: [*const c_char; 1] = [CAP_FS.as_ptr() as *const c_char];

    NativePluginMetadata {
        name: NAME.as_ptr() as *const c_char,
        version: VERSION.as_ptr() as *const c_char,
        description: DESCRIPTION.as_ptr() as *const c_char,
        author: AUTHOR.as_ptr() as *const c_char,
        capabilities_count: 1,
        capabilities: CAPS.as_ptr(),
    }
}

/// Plugin execution - called each time the plugin is invoked
#[no_mangle]
pub extern "C" fn dx_plugin_execute(ctx: *const NativePluginContext) -> NativePluginResult {
    let result = std::panic::catch_unwind(|| {
        unsafe { execute_impl(ctx) }
    });

    match result {
        Ok(r) => r,
        Err(_) => NativePluginResult {
            success: false,
            message: leak_string("Plugin panicked"),
            data: ptr::null(),
            data_len: 0,
        },
    }
}

/// Plugin cleanup - called when plugin is unloaded
#[no_mangle]
pub extern "C" fn dx_plugin_cleanup() {
    // Clean up any resources here
}

/// Free a result returned by the plugin
#[no_mangle]
pub extern "C" fn dx_plugin_free_result(result: NativePluginResult) {
    unsafe {
        if !result.message.is_null() {
            drop(CString::from_raw(result.message as *mut c_char));
        }
        if !result.data.is_null() {
            let data = Vec::from_raw_parts(
                result.data as *mut u8,
                result.data_len,
                result.data_len,
            );
            drop(data);
        }
    }
}

// ============================================================================
// Implementation
// ============================================================================

unsafe fn execute_impl(ctx: *const NativePluginContext) -> NativePluginResult {
    if ctx.is_null() {
        return NativePluginResult {
            success: false,
            message: leak_string("Null context"),
            data: ptr::null(),
            data_len: 0,
        };
    }

    let ctx = &*ctx;
    
    // Parse arguments
    let args = parse_args(ctx.args, ctx.args_count);
    let working_dir = if ctx.working_dir.is_null() {
        ".".to_string()
    } else {
        CStr::from_ptr(ctx.working_dir).to_string_lossy().to_string()
    };

    // Determine command
    let command = args.get(0).map(|s| s.as_str()).unwrap_or("help");

    match command {
        "list" => list_files(&working_dir, &args),
        "count" => count_lines(&args),
        "hash" => hash_file(&args),
        "help" | _ => show_help(),
    }
}

unsafe fn parse_args(args: *const *const c_char, count: usize) -> Vec<String> {
    if args.is_null() || count == 0 {
        return Vec::new();
    }

    (0..count)
        .filter_map(|i| {
            let arg_ptr = *args.add(i);
            if arg_ptr.is_null() {
                None
            } else {
                Some(CStr::from_ptr(arg_ptr).to_string_lossy().to_string())
            }
        })
        .collect()
}

fn leak_string(s: &str) -> *const c_char {
    CString::new(s).unwrap().into_raw()
}

fn list_files(working_dir: &str, _args: &[String]) -> NativePluginResult {
    match std::fs::read_dir(working_dir) {
        Ok(entries) => {
            let files: Vec<String> = entries
                .filter_map(|e| e.ok())
                .map(|e| {
                    let name = e.file_name().to_string_lossy().to_string();
                    let meta = e.metadata().ok();
                    let size = meta.as_ref().map(|m| m.len()).unwrap_or(0);
                    let is_dir = meta.as_ref().map(|m| m.is_dir()).unwrap_or(false);
                    if is_dir {
                        format!("📁 {}/", name)
                    } else {
                        format!("📄 {} ({} bytes)", name, size)
                    }
                })
                .collect();

            let output = files.join("\n");
            NativePluginResult {
                success: true,
                message: leak_string(&output),
                data: ptr::null(),
                data_len: 0,
            }
        }
        Err(e) => NativePluginResult {
            success: false,
            message: leak_string(&format!("Error: {}", e)),
            data: ptr::null(),
            data_len: 0,
        },
    }
}

fn count_lines(args: &[String]) -> NativePluginResult {
    let path = args.get(1).map(|s| s.as_str()).unwrap_or(".");
    
    match std::fs::read_to_string(path) {
        Ok(content) => {
            let lines = content.lines().count();
            let words = content.split_whitespace().count();
            let chars = content.chars().count();
            
            let output = format!(
                "File: {}\n  Lines: {}\n  Words: {}\n  Characters: {}",
                path, lines, words, chars
            );
            
            NativePluginResult {
                success: true,
                message: leak_string(&output),
                data: ptr::null(),
                data_len: 0,
            }
        }
        Err(e) => NativePluginResult {
            success: false,
            message: leak_string(&format!("Error reading {}: {}", path, e)),
            data: ptr::null(),
            data_len: 0,
        },
    }
}

fn hash_file(args: &[String]) -> NativePluginResult {
    let path = args.get(1).map(|s| s.as_str()).unwrap_or(".");
    
    match std::fs::read(path) {
        Ok(content) => {
            // Simple hash for demo (not cryptographically secure)
            let hash: u64 = content.iter().fold(0u64, |acc, &byte| {
                acc.wrapping_mul(31).wrapping_add(byte as u64)
            });
            
            let output = format!("Hash of {}: {:016x}", path, hash);
            
            // Also return raw hash as data
            let hash_bytes = hash.to_le_bytes().to_vec();
            let len = hash_bytes.len();
            let ptr = hash_bytes.as_ptr();
            std::mem::forget(hash_bytes);
            
            NativePluginResult {
                success: true,
                message: leak_string(&output),
                data: ptr,
                data_len: len,
            }
        }
        Err(e) => NativePluginResult {
            success: false,
            message: leak_string(&format!("Error reading {}: {}", path, e)),
            data: ptr::null(),
            data_len: 0,
        },
    }
}

fn show_help() -> NativePluginResult {
    let help = r#"Native Example Plugin

USAGE:
    dx native-example <command> [args...]

COMMANDS:
    list              List files in current directory
    count <file>      Count lines, words, and characters in a file
    hash <file>       Compute hash of a file
    help              Show this help message

EXAMPLES:
    dx native-example list
    dx native-example count README.md
    dx native-example hash Cargo.toml
"#;

    NativePluginResult {
        success: true,
        message: leak_string(help),
        data: ptr::null(),
        data_len: 0,
    }
}

// ============================================================================
// Plugin.sr manifest example
// ============================================================================

// ```sr
// [plugin]
// name = "native-example"
// version = "1.0.0"
// description = "Example native plugin demonstrating C ABI"
// author = "DX Team"
// license = "MIT"
// plugin_type = "Native"
// 
// [plugin.capabilities]
// required = ["fs:read"]
// optional = []
// 
// [plugin.commands]
// list = { description = "List files in current directory" }
// count = { description = "Count lines/words/chars", args = "<file>" }
// hash = { description = "Compute file hash", args = "<file>" }
// help = { description = "Show help message" }
// 
// [plugin.build]
// entry_point = "target/release/libnative_example.so"  # Linux
// # entry_point = "target/release/native_example.dll"  # Windows
// # entry_point = "target/release/libnative_example.dylib"  # macOS
// min_dx_version = "0.1.0"
// 
// [plugin.security]
// signature_file = "signature.bin"
// public_key = "..." # Base64 Ed25519 public key
// ```

// ============================================================================
// Cargo.toml example
// ============================================================================

// ```toml
// [package]
// name = "native-example"
// version = "1.0.0"
// edition = "2021"
// 
// [lib]
// crate-type = ["cdylib"]
// 
// [dependencies]
// # Add your dependencies here
// ```

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metadata() {
        let meta = dx_plugin_metadata();
        unsafe {
            assert_eq!(CStr::from_ptr(meta.name).to_str().unwrap(), "native-example");
            assert_eq!(CStr::from_ptr(meta.version).to_str().unwrap(), "1.0.0");
        }
    }

    #[test]
    fn test_help() {
        let result = show_help();
        assert!(result.success);
        unsafe {
            let msg = CStr::from_ptr(result.message).to_str().unwrap();
            assert!(msg.contains("Native Example Plugin"));
        }
    }
}
