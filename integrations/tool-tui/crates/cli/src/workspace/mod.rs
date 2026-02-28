//! Workspace Module
//!
//! Multi-workspace management for concurrent task execution.

pub mod manager;

pub use manager::{
    ManagerConfig, ManagerStatus, MessageType, Priority, Task, TaskId, TaskResult, TaskState,
    TaskType, Workspace, WorkspaceConfig, WorkspaceId, WorkspaceManager, WorkspaceMessage,
    WorkspaceState, WorkspaceStatus,
};
