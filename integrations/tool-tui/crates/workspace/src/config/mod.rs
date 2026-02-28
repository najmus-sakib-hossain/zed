//! Configuration types for dx-workspace.
//!
//! This module contains all the configuration structures that represent
//! a dx workspace's development environment settings.

mod debug;
mod editor;
mod extensions;
mod tasks;
mod workspace;

pub use debug::*;
pub use editor::*;
pub use extensions::*;
pub use tasks::*;
pub use workspace::*;
