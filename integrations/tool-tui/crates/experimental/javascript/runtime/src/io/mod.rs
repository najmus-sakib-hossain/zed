//! Async I/O with io_uring support

pub mod env;
pub mod platform;

pub use env::{load_dotenv, load_dotenv_from, EnvLoader};
pub use platform::{
    arch_name, is_linux, is_macos, is_windows, join_paths, line_ending, normalize_line_endings,
    normalize_path, path_separator, platform_name, to_native_line_endings, to_unix_path,
};

use crate::error::{DxError, DxResult};
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;

pub struct AsyncIO {
    io_uring_available: bool,
}

impl AsyncIO {
    pub fn new() -> Self {
        Self {
            io_uring_available: cfg!(target_os = "linux"),
        }
    }

    pub fn is_io_uring_available(&self) -> bool {
        self.io_uring_available
    }

    pub async fn read_file(&self, path: &Path) -> DxResult<Vec<u8>> {
        let mut file = File::open(path)
            .map_err(|e| DxError::RuntimeError(format!("File open failed: {}", e)))?;
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)
            .map_err(|e| DxError::RuntimeError(format!("File read failed: {}", e)))?;
        Ok(buffer)
    }

    pub async fn write_file(&self, path: &Path, data: &[u8]) -> DxResult<()> {
        let mut file = File::create(path)
            .map_err(|e| DxError::RuntimeError(format!("File create failed: {}", e)))?;
        file.write_all(data)
            .map_err(|e| DxError::RuntimeError(format!("File write failed: {}", e)))?;
        Ok(())
    }

    pub async fn batch_read(&self, paths: &[&Path]) -> DxResult<Vec<Vec<u8>>> {
        let mut results = Vec::new();
        for path in paths {
            results.push(self.read_file(path).await?);
        }
        Ok(results)
    }
}

impl Default for AsyncIO {
    fn default() -> Self {
        Self::new()
    }
}

pub struct IOQueue {
    pending: Vec<IORequest>,
}

pub struct IORequest {
    pub id: u64,
    pub path: String,
    pub operation: IOOperation,
}

pub enum IOOperation {
    Read,
    Write(Vec<u8>),
}

impl IOQueue {
    pub fn new() -> Self {
        Self {
            pending: Vec::new(),
        }
    }

    pub fn submit(&mut self, request: IORequest) {
        self.pending.push(request);
    }

    pub fn process_batch(&mut self) -> Vec<(u64, DxResult<Vec<u8>>)> {
        let requests = std::mem::take(&mut self.pending);
        requests
            .into_iter()
            .map(|req| {
                let result = match req.operation {
                    IOOperation::Read => {
                        std::fs::read(&req.path).map_err(|e| DxError::RuntimeError(e.to_string()))
                    }
                    IOOperation::Write(data) => std::fs::write(&req.path, data)
                        .map(|_| Vec::new())
                        .map_err(|e| DxError::RuntimeError(e.to_string())),
                };
                (req.id, result)
            })
            .collect()
    }
}

impl Default for IOQueue {
    fn default() -> Self {
        Self::new()
    }
}
