//! Sandboxing and isolation for DX CLI
//!
//! Provides secure execution environments for AI-generated code and untrusted operations.
//! Supports multiple backends: Docker, native OS sandboxing, and WASM.

pub mod backend;
pub mod config;
pub mod docker;
pub mod manager;
pub mod native;
pub mod resource;
pub mod wasm;

pub use backend::SandboxBackendType;
pub use config::{NetworkMode, ResourceLimits, SandboxConfig};
pub use manager::{SandboxManager, SandboxStatus};

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Sandbox instance representing an isolated environment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Sandbox {
    pub id: String,
    pub name: String,
    pub backend: SandboxBackendType,
    pub root_dir: PathBuf,
    pub config: SandboxConfig,
    pub status: SandboxStatus,
}

impl Sandbox {
    /// Create a new sandbox instance
    pub fn new(name: String, backend: SandboxBackendType, config: SandboxConfig) -> Result<Self> {
        let id = format!("dx-sandbox-{}", uuid::Uuid::new_v4());
        let root_dir = std::env::temp_dir().join(&id);

        Ok(Self {
            id,
            name,
            backend,
            root_dir,
            config,
            status: SandboxStatus::Created,
        })
    }
}

// UUID generation helper
mod uuid {
    use std::time::{SystemTime, UNIX_EPOCH};

    pub struct Uuid(String);

    impl Uuid {
        pub fn new_v4() -> Self {
            let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
            let random = rand::random::<u64>();
            Self(format!("{:x}-{:x}", timestamp, random))
        }
    }

    impl std::fmt::Display for Uuid {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{}", self.0)
        }
    }
}
