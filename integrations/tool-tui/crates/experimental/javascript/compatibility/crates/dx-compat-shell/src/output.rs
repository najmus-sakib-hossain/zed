//! Shell command output.

use crate::error::{ShellError, ShellResult};
use serde::de::DeserializeOwned;

/// Shell command output.
#[derive(Debug, Clone)]
pub struct ShellOutput {
    /// Standard output
    pub stdout: Vec<u8>,
    /// Standard error
    pub stderr: Vec<u8>,
    /// Exit code
    pub exit_code: i32,
}

impl ShellOutput {
    /// Get stdout as text.
    pub fn text(&self) -> ShellResult<String> {
        String::from_utf8(self.stdout.clone())
            .map_err(|e| ShellError::ExecutionFailed(e.to_string()))
    }

    /// Parse stdout as JSON.
    pub fn json<T: DeserializeOwned>(&self) -> ShellResult<T> {
        serde_json::from_slice(&self.stdout).map_err(|e| ShellError::ExecutionFailed(e.to_string()))
    }

    /// Get stdout as lines.
    pub fn lines(&self) -> Vec<String> {
        self.text().unwrap_or_default().lines().map(String::from).collect()
    }

    /// Get stdout as bytes.
    pub fn bytes(&self) -> &[u8] {
        &self.stdout
    }
}
