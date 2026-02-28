//! Runtime capability detection
//!
//! Detects available capabilities of a Python runtime.

use serde::{Deserialize, Serialize};

/// Runtime capabilities
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct RuntimeCapabilities {
    /// Whether pip is available
    pub has_pip: bool,
    /// Whether venv module is available
    pub has_venv: bool,
    /// Whether SSL support is available
    pub has_ssl: bool,
    /// Whether SQLite support is available
    pub has_sqlite: bool,
    /// ABI tag for wheel compatibility
    pub abi_tag: String,
}

impl RuntimeCapabilities {
    /// Create new capabilities with all features enabled
    pub fn full() -> Self {
        Self {
            has_pip: true,
            has_venv: true,
            has_ssl: true,
            has_sqlite: true,
            abi_tag: String::new(),
        }
    }

    /// Set the ABI tag
    pub fn with_abi_tag(mut self, abi_tag: impl Into<String>) -> Self {
        self.abi_tag = abi_tag.into();
        self
    }
}
