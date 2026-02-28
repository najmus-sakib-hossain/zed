//! dx-py-workspace: Python project lifecycle and workspace management
//!
//! This crate provides project management functionality including:
//! - Python version management (discovery, installation, pinning)
//! - Virtual environment creation and management
//! - Workspace/monorepo support
//! - Global tool management
//! - Project initialization

pub mod init;
pub mod python;
pub mod tool;
pub mod venv;
pub mod workspace;

pub use dx_py_core::{Error, Result};
pub use init::{InitOptions, InitResult, ProjectInitializer, ProjectLayout, ProjectTemplate};
pub use python::{PythonInstall, PythonManager, PythonRelease, RealPythonManager};
pub use tool::{InstalledTool, ToolManager};
pub use venv::{RealVenvManager, Venv, VenvManager};
pub use workspace::{PathDependency, WorkspaceConfig, WorkspaceManager, WorkspaceMember};
