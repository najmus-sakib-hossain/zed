//! # DX Forge Credits Module
//!
//! This module provides contribution tracking and credit management for DX plugins.
//! It extracts git user information, maintains contributor metadata, and generates
//! CONTRIBUTORS.md entries automatically.
//!
//! ## Features
//!
//! - **Git Integration**: Extract author info from git history
//! - **GitHub Linking**: Associate contributions with GitHub profiles
//! - **Credit Tracking**: Track contributions per plugin
//! - **CONTRIBUTORS.md**: Auto-generate contributor files
//!
//! ## Usage
//!
//! ```rust,ignore
//! use dx_forge::credits::{CreditsManager, Contributor};
//!
//! let manager = CreditsManager::from_git_repo(".")?;
//! let contributors = manager.get_contributors()?;
//!
//! // Generate CONTRIBUTORS.md
//! let markdown = manager.generate_contributors_md()?;
//! std::fs::write("CONTRIBUTORS.md", markdown)?;
//! ```

mod contributor;
mod git;
mod manager;
mod markdown;

pub use contributor::{ContributionStats, Contributor, ContributorRole};
pub use git::{GitAuthor, GitExtractor};
pub use manager::CreditsManager;
pub use markdown::ContributorsMarkdown;
