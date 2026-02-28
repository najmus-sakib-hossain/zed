//! # dx-security: The Binary Fortress
//!
//! Binary-level security scanner with SIMD acceleration, memory-mapped analysis,
//! and cryptographic attestation for the DX ecosystem.
//!
//! ## Architecture
//!
//! - **score**: Security score calculation (0-100)
//! - **index**: Binary Vulnerability Index (BVI) for O(1) CVE lookups
//! - **scanner**: SIMD-accelerated secret detection
//! - **mapper**: Memory-mapped file access
//! - **rules**: Binary Rule Fusion Engine (BRFE)
//! - **diff**: XOR differential scanning
//! - **graph**: Binary Dependency Graph (BDG)
//! - **signer**: Ed25519 cryptographic attestation
//! - **stream**: HBSP streaming protocol
//! - **pool**: Lock-free thread pool
//! - **cli**: CLI integration
//! - **report**: Binary report format (.sr)
//! - **extension**: VS Code extension integration

pub mod cli;
pub mod diff;
pub mod error;
pub mod extension;
pub mod graph;
pub mod index;
pub mod mapper;
pub mod pool;
pub mod report;
pub mod rules;
pub mod safety;
pub mod scanner;
pub mod score;
pub mod signer;
pub mod stream;

pub use error::{Result, SecurityError};
pub use safety::{SafetyError, check_alignment, check_bounds};

// Re-export score module types
pub use score::ScanFindings;
pub use score::SecurityScore;
pub use score::calculate_score;
pub use score::sign_score;
pub use score::verify_score;
