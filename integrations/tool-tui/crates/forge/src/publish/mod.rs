//! # DX Forge Publish Module
//!
//! This module provides the complete publishing workflow for DX plugins and extensions.
//! It handles validation, packaging, signing, and submission to the dx-plugins repository.
//!
//! ## Features
//!
//! - **Validation Pipeline**: Comprehensive checks before publishing
//! - **Package Creation**: Generates distributable plugin packages
//! - **Ed25519 Signing**: Cryptographic signatures for plugin integrity
//! - **GitHub Integration**: Automated PR creation for dx-plugins repo
//! - **Auto-merge Support**: Green status → automatic merge
//!
//! ## Usage
//!
//! ```rust,ignore
//! use dx_forge::publish::{PublishConfig, Publisher, PublishResult};
//!
//! let config = PublishConfig::builder("my-plugin")
//!     .with_version("1.0.0")
//!     .with_author("developer@example.com")
//!     .build()?;
//!
//! let publisher = Publisher::new(config)?;
//! let result = publisher.publish().await?;
//!
//! match result {
//!     PublishResult::Success(pr_url) => println!("PR created: {}", pr_url),
//!     PublishResult::ValidationFailed(errors) => eprintln!("Errors: {:?}", errors),
//! }
//! ```

mod config;
mod package;
mod pipeline;
mod publisher;
mod signing;
mod submission;

pub use config::{PublishConfig, PublishConfigBuilder};
pub use package::{Package, PackageFormat, PackageManifest};
pub use pipeline::{PublishPipeline, ValidationStep};
pub use publisher::{PublishResult, Publisher};
pub use signing::{Ed25519Signer, SignatureInfo, SigningKey};
pub use submission::{GitHubSubmission, PullRequest, SubmissionStatus};
