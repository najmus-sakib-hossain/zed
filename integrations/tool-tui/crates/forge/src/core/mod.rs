//! Core Forge functionality
//!
//! This module contains the main `Forge` struct that provides a unified API
//! for DX tools to manage their lifecycle, version control, and code generation.

pub mod branching_engine;
pub mod editor_integration;
pub mod event_bus;
pub mod forge;
pub mod lifecycle;
pub mod tracking;

pub use branching_engine::{
    ApplicationRecord, BranchColor, BranchingEngine, BranchingVote, FileChange,
};
pub use editor_integration::{EditorInfo, EditorIntegration, EditorType, OutputStrategy};
pub use event_bus::{EventBus, ForgeEvent};
pub use forge::{Forge, ForgeConfig};
pub use lifecycle::{LifecycleEvent, LifecycleManager, ToolId, ToolState, ToolStatus};
pub use tracking::{GeneratedCodeTracker, GeneratedFileInfo};
