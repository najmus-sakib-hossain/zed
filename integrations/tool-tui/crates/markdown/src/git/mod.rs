//! Git integration for DXM (Holographic Git).
//!
//! This module provides Git clean/smudge filters for seamless DXM workflows
//! where developers work with rich `.dxm` files locally while GitHub sees
//! standard `.md` files.
//!
//! ## Architecture
//!
//! - **Clean Filter**: Converts DXM → Markdown when staging files
//! - **Smudge Filter**: Converts Markdown → DXM on checkout
//! - **Format Detection**: Automatically detects input format
//! - **Repository Init**: Configures Git filters and attributes
//! - **Sync**: Regenerates shadow .md files from .dxm sources
//!
//! ## Usage
//!
//! ```bash
//! # Initialize DXM support in a repository
//! dx dxm init
//!
//! # Git will automatically use filters for .dxm files
//! git add README.dxm  # Clean filter converts to MD
//! git checkout        # Smudge filter converts back to DXM
//! ```
//!
//! # Stability
//!
//! This module is **experimental** and not part of the stable API.
//! It may use `unwrap()` and `expect()` for convenience as it's not production code.

// Allow unwrap/expect in experimental git integration code
#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]

pub mod clean;
pub mod detect;
pub mod init;
pub mod repo;
pub mod smudge;
pub mod sync;

#[cfg(test)]
mod integration_tests;

// Re-export main types
pub use clean::{CleanFilter, FilterError};
pub use detect::{DetectedFormat, detect_format};
pub use init::{InitError, InitResult, RepoInitializer, find_repo_root};
pub use repo::{BundleResult, FileResult, RepoCompileResult, bundle_directory, process_directory};
pub use smudge::SmudgeFilter;
pub use sync::SyncManager;
