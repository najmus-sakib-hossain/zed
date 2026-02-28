//! Node.js API Compatibility Layer
//!
//! This module provides implementations of core Node.js APIs to ensure
//! compatibility with existing Node.js applications.

use crate::error::{DxError, DxResult};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Node.js API registry
pub struct NodeAPIs {
    /// File system module
    pub fs: FileSystemAPI,
    /// Path module
    pub path: PathAPI,
    /// Process module
    pub process: ProcessAPI,
    /// Buffer module
    pub buffer: BufferAPI,
}

impl NodeAPIs {
    pub fn new() -> Self {
        Self {
            fs: FileSystemAPI::new(),
            path: PathAPI::new(),
            process: ProcessAPI::new(),
            buffer: BufferAPI::new(),
        }
    }

    /// Get all Node.js built-in modules
    pub fn get_builtin_modules(&self) -> Vec<String> {
        vec![
            "fs".to_string(),
            "path".to_string(),
            "process".to_string(),
            "buffer".to_string(),
        ]
    }
}

impl Default for NodeAPIs {
    fn default() -> Self {
        Self::new()
    }
}

/// File System API (fs module)
pub struct FileSystemAPI {
    /// Current working directory - reserved for relative path resolution
    #[allow(dead_code)]
    cwd: PathBuf,
}

impl FileSystemAPI {
    pub fn new() -> Self {
        Self {
            cwd: std::env::current_dir().unwrap_or_else(|_| PathBuf::from("/")),
        }
    }

    /// Read file synchronously
    pub fn read_file_sync(&self, path: &str) -> DxResult<Vec<u8>> {
        fs::read(path).map_err(|e| DxError::IoError(e.to_string()))
    }

    /// Write file synchronously
    pub fn write_file_sync(&self, path: &str, data: &[u8]) -> DxResult<()> {
        fs::write(path, data).map_err(|e| DxError::IoError(e.to_string()))
    }

    /// Read file asynchronously (returns Promise)
    pub fn read_file(&self, path: String) -> DxResult<Vec<u8>> {
        // For now, just do sync read
        // In future, integrate with async runtime
        fs::read(&path).map_err(|e| DxError::IoError(e.to_string()))
    }

    /// Write file asynchronously
    pub fn write_file(&self, path: String, data: Vec<u8>) -> DxResult<()> {
        fs::write(&path, data).map_err(|e| DxError::IoError(e.to_string()))
    }

    /// Check if file exists
    pub fn exists_sync(&self, path: &str) -> bool {
        Path::new(path).exists()
    }

    /// Read directory
    pub fn read_dir_sync(&self, path: &str) -> DxResult<Vec<String>> {
        let entries = fs::read_dir(path).map_err(|e| DxError::IoError(e.to_string()))?;

        let mut result = Vec::new();
        for entry in entries.flatten() {
            if let Some(name) = entry.file_name().to_str() {
                result.push(name.to_string());
            }
        }

        Ok(result)
    }

    /// Create directory
    pub fn mkdir_sync(&self, path: &str) -> DxResult<()> {
        fs::create_dir_all(path).map_err(|e| DxError::IoError(e.to_string()))
    }

    /// Remove file
    pub fn unlink_sync(&self, path: &str) -> DxResult<()> {
        fs::remove_file(path).map_err(|e| DxError::IoError(e.to_string()))
    }

    /// Remove directory
    pub fn rmdir_sync(&self, path: &str) -> DxResult<()> {
        fs::remove_dir_all(path).map_err(|e| DxError::IoError(e.to_string()))
    }

    /// Get file stats
    pub fn stat_sync(&self, path: &str) -> DxResult<FileStats> {
        let metadata = fs::metadata(path).map_err(|e| DxError::IoError(e.to_string()))?;

        Ok(FileStats {
            size: metadata.len(),
            is_file: metadata.is_file(),
            is_directory: metadata.is_dir(),
            is_symlink: metadata.file_type().is_symlink(),
        })
    }
}

impl Default for FileSystemAPI {
    fn default() -> Self {
        Self::new()
    }
}

/// File statistics
#[derive(Debug, Clone)]
pub struct FileStats {
    pub size: u64,
    pub is_file: bool,
    pub is_directory: bool,
    pub is_symlink: bool,
}

/// Path API (path module)
pub struct PathAPI;

impl PathAPI {
    pub fn new() -> Self {
        Self
    }

    /// Join path segments
    pub fn join(&self, segments: &[&str]) -> String {
        let mut path = PathBuf::new();
        for segment in segments {
            path.push(segment);
        }
        path.to_string_lossy().to_string()
    }

    /// Get file name from path
    pub fn basename(&self, path: &str) -> Option<String> {
        Path::new(path).file_name().and_then(|s| s.to_str()).map(String::from)
    }

    /// Get directory name from path
    pub fn dirname(&self, path: &str) -> Option<String> {
        Path::new(path).parent().and_then(|p| p.to_str()).map(String::from)
    }

    /// Get file extension
    pub fn extname(&self, path: &str) -> Option<String> {
        Path::new(path).extension().and_then(|s| s.to_str()).map(|s| format!(".{}", s))
    }

    /// Resolve path to absolute
    pub fn resolve(&self, path: &str) -> DxResult<String> {
        let absolute = if Path::new(path).is_absolute() {
            PathBuf::from(path)
        } else {
            std::env::current_dir().map_err(|e| DxError::IoError(e.to_string()))?.join(path)
        };

        Ok(absolute.to_string_lossy().to_string())
    }

    /// Check if path is absolute
    pub fn is_absolute(&self, path: &str) -> bool {
        Path::new(path).is_absolute()
    }

    /// Normalize path (remove . and ..)
    pub fn normalize(&self, path: &str) -> String {
        let mut components = Vec::new();

        for component in Path::new(path).components() {
            use std::path::Component;
            match component {
                Component::ParentDir => {
                    components.pop();
                }
                Component::CurDir => {}
                _ => components.push(component.as_os_str().to_string_lossy().to_string()),
            }
        }

        components.join(std::path::MAIN_SEPARATOR_STR)
    }
}

impl Default for PathAPI {
    fn default() -> Self {
        Self::new()
    }
}

/// Process API (process module)
pub struct ProcessAPI {
    /// Environment variables
    env: HashMap<String, String>,
    /// Process arguments
    argv: Vec<String>,
    /// Exit code
    exit_code: i32,
}

impl ProcessAPI {
    pub fn new() -> Self {
        // Collect environment variables from system
        let mut env: HashMap<String, String> = std::env::vars().collect();

        // Load .env files and merge (dotenv values don't override system env)
        let dotenv_vars = super::dotenv::load_dotenv(None);
        for (key, value) in dotenv_vars {
            env.entry(key).or_insert(value);
        }

        // Collect command line arguments
        let argv = std::env::args().collect();

        Self {
            env,
            argv,
            exit_code: 0,
        }
    }

    /// Create ProcessAPI with custom base path for .env files
    pub fn with_dotenv_path(base_path: &std::path::Path) -> Self {
        // Collect environment variables from system
        let mut env: HashMap<String, String> = std::env::vars().collect();

        // Load .env files from specified path
        let dotenv_vars = super::dotenv::load_dotenv(Some(base_path));
        for (key, value) in dotenv_vars {
            env.entry(key).or_insert(value);
        }

        // Collect command line arguments
        let argv = std::env::args().collect();

        Self {
            env,
            argv,
            exit_code: 0,
        }
    }

    /// Get environment variable
    pub fn env_get(&self, key: &str) -> Option<&String> {
        self.env.get(key)
    }

    /// Set environment variable
    pub fn env_set(&mut self, key: String, value: String) {
        self.env.insert(key, value);
    }

    /// Get all environment variables
    pub fn env_all(&self) -> &HashMap<String, String> {
        &self.env
    }

    /// Get command line arguments
    pub fn argv(&self) -> &[String] {
        &self.argv
    }

    /// Get current working directory
    pub fn cwd(&self) -> DxResult<String> {
        std::env::current_dir()
            .map(|p| p.to_string_lossy().to_string())
            .map_err(|e| DxError::IoError(e.to_string()))
    }

    /// Change working directory
    pub fn chdir(&self, path: &str) -> DxResult<()> {
        std::env::set_current_dir(path).map_err(|e| DxError::IoError(e.to_string()))
    }

    /// Exit process
    pub fn exit(&mut self, code: i32) -> ! {
        std::process::exit(code)
    }

    /// Set exit code
    pub fn set_exit_code(&mut self, code: i32) {
        self.exit_code = code;
    }

    /// Get platform
    pub fn platform(&self) -> &str {
        if cfg!(target_os = "windows") {
            "win32"
        } else if cfg!(target_os = "macos") {
            "darwin"
        } else if cfg!(target_os = "linux") {
            "linux"
        } else {
            "unknown"
        }
    }

    /// Get architecture
    pub fn arch(&self) -> &str {
        if cfg!(target_arch = "x86_64") {
            "x64"
        } else if cfg!(target_arch = "aarch64") {
            "arm64"
        } else {
            "unknown"
        }
    }

    /// Get process ID
    pub fn pid(&self) -> u32 {
        std::process::id()
    }
}

impl Default for ProcessAPI {
    fn default() -> Self {
        Self::new()
    }
}

/// Buffer API (buffer module)
pub struct BufferAPI;

impl BufferAPI {
    pub fn new() -> Self {
        Self
    }

    /// Create buffer from bytes
    pub fn from_bytes(&self, data: Vec<u8>) -> Buffer {
        Buffer { data }
    }

    /// Create buffer from string
    pub fn from_string(&self, s: &str) -> Buffer {
        Buffer {
            data: s.as_bytes().to_vec(),
        }
    }

    /// Allocate buffer of size
    pub fn alloc(&self, size: usize) -> Buffer {
        Buffer {
            data: vec![0; size],
        }
    }

    /// Allocate unsafe buffer (uninitialized)
    ///
    /// # Safety
    /// This creates an uninitialized buffer, matching Node.js Buffer.allocUnsafe behavior.
    /// The caller is responsible for initializing the buffer before reading from it.
    #[allow(clippy::uninit_vec)]
    pub fn alloc_unsafe(&self, size: usize) -> Buffer {
        let mut data = Vec::with_capacity(size);
        unsafe {
            data.set_len(size);
        }
        Buffer { data }
    }

    /// Concatenate buffers
    pub fn concat(&self, buffers: &[Buffer]) -> Buffer {
        let total_size: usize = buffers.iter().map(|b| b.data.len()).sum();
        let mut result = Vec::with_capacity(total_size);

        for buffer in buffers {
            result.extend_from_slice(&buffer.data);
        }

        Buffer { data: result }
    }
}

impl Default for BufferAPI {
    fn default() -> Self {
        Self::new()
    }
}

/// Buffer type
#[derive(Debug, Clone)]
pub struct Buffer {
    pub data: Vec<u8>,
}

impl Buffer {
    /// Get buffer length
    pub fn length(&self) -> usize {
        self.data.len()
    }

    /// Convert to string
    pub fn to_string(&self, encoding: &str) -> DxResult<String> {
        match encoding {
            "utf8" | "utf-8" => String::from_utf8(self.data.clone())
                .map_err(|e| DxError::RuntimeError(e.to_string())),
            "hex" => Ok(hex::encode(&self.data)),
            "base64" => Ok(base64::encode(&self.data)),
            _ => Err(DxError::RuntimeError(format!("Unknown encoding: {}", encoding))),
        }
    }

    /// Slice buffer
    pub fn slice(&self, start: usize, end: usize) -> Buffer {
        Buffer {
            data: self.data[start..end].to_vec(),
        }
    }

    /// Write to buffer
    pub fn write(&mut self, data: &[u8], offset: usize) -> usize {
        let len = data.len().min(self.data.len() - offset);
        self.data[offset..offset + len].copy_from_slice(&data[..len]);
        len
    }

    /// Read from buffer
    pub fn read(&self, offset: usize, length: usize) -> &[u8] {
        let end = (offset + length).min(self.data.len());
        &self.data[offset..end]
    }
}

// Placeholder for hex and base64 encoding
mod hex {
    pub fn encode(data: &[u8]) -> String {
        data.iter().map(|b| format!("{:02x}", b)).collect::<String>()
    }
}

mod base64 {
    pub fn encode(data: &[u8]) -> String {
        // Simple base64 implementation
        const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let mut result = String::new();

        for chunk in data.chunks(3) {
            let mut buf = [0u8; 3];
            for (i, &byte) in chunk.iter().enumerate() {
                buf[i] = byte;
            }

            let b1 = (buf[0] >> 2) as usize;
            let b2 = (((buf[0] & 0x03) << 4) | (buf[1] >> 4)) as usize;
            let b3 = (((buf[1] & 0x0F) << 2) | (buf[2] >> 6)) as usize;
            let b4 = (buf[2] & 0x3F) as usize;

            result.push(CHARSET[b1] as char);
            result.push(CHARSET[b2] as char);
            result.push(if chunk.len() > 1 {
                CHARSET[b3] as char
            } else {
                '='
            });
            result.push(if chunk.len() > 2 {
                CHARSET[b4] as char
            } else {
                '='
            });
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_path_join() {
        let path = PathAPI::new();
        let result = path.join(&["foo", "bar", "baz.txt"]);
        assert!(result.contains("foo"));
        assert!(result.contains("bar"));
        assert!(result.contains("baz.txt"));
    }

    #[test]
    fn test_buffer_operations() {
        let buffer_api = BufferAPI::new();
        let buf = buffer_api.from_string("Hello");
        assert_eq!(buf.length(), 5);

        let slice = buf.slice(0, 2);
        assert_eq!(slice.length(), 2);
    }
}
