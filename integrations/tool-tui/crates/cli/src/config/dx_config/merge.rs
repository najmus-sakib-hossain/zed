//! Configuration merging logic

use crate::utils::error::DxError;
use std::path::PathBuf;

use super::types::{
    BuildConfig, DEFAULT_CONFIG_FILE, DevConfig, DxConfig, ProjectConfig, RuntimeConfig,
    ToolsConfig, default_jsx, default_out_dir, default_port, default_target, default_version,
};

impl DxConfig {
    /// Load and merge global and local configurations
    pub fn load_merged() -> Result<Self, DxError> {
        let global_config = Self::load_global().ok();
        let local_config = Self::load_default().ok();

        match (global_config, local_config) {
            (Some(global), Some(local)) => Ok(Self::merge(global, local)),
            (Some(global), None) => Ok(global),
            (None, Some(local)) => Ok(local),
            (None, None) => Err(DxError::ConfigNotFound {
                path: PathBuf::from(DEFAULT_CONFIG_FILE),
            }),
        }
    }

    /// Merge two configurations (local overrides global)
    pub(crate) fn merge(global: Self, local: Self) -> Self {
        Self {
            project: ProjectConfig {
                name: if local.project.name.is_empty() {
                    global.project.name
                } else {
                    local.project.name
                },
                version: if local.project.version == default_version() {
                    global.project.version
                } else {
                    local.project.version
                },
                description: local.project.description.or(global.project.description),
            },
            build: BuildConfig {
                target: if local.build.target == default_target() {
                    global.build.target
                } else {
                    local.build.target
                },
                minify: local.build.minify,
                sourcemap: local.build.sourcemap || global.build.sourcemap,
                out_dir: if local.build.out_dir == default_out_dir() {
                    global.build.out_dir
                } else {
                    local.build.out_dir
                },
            },
            dev: DevConfig {
                port: if local.dev.port == default_port() {
                    global.dev.port
                } else {
                    local.dev.port
                },
                open: local.dev.open || global.dev.open,
                https: local.dev.https || global.dev.https,
            },
            runtime: RuntimeConfig {
                jsx: if local.runtime.jsx == default_jsx() {
                    global.runtime.jsx
                } else {
                    local.runtime.jsx
                },
                typescript: local.runtime.typescript,
            },
            tools: ToolsConfig {
                style: local.tools.style.or(global.tools.style),
                media: local.tools.media.or(global.tools.media),
                font: local.tools.font.or(global.tools.font),
                icon: local.tools.icon.or(global.tools.icon),
            },
        }
    }
}
