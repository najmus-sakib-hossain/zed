//! Node.js/Bun runtime integration for messaging channels
//! Executes OpenClaw messaging code via Bun for maximum performance

pub mod bridge;
pub mod installer;
pub mod runtime;

pub use bridge::{GatewayConfig, OpenClawBridge};
pub use installer::{check_bun, ensure_bun, install_bun};
pub use runtime::{BunConfig, BunRuntime};
