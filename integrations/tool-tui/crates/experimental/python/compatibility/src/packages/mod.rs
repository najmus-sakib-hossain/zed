//! Package compatibility verification module
//!
//! This module provides functionality to verify that popular Python packages
//! work correctly with DX-Py runtime.

mod compatibility_report;
mod import_checker;
mod verifier;

pub use compatibility_report::{CompatibilityLevel, CompatibilityReport, PackageCompatibility};
pub use import_checker::{ImportChecker, ImportError, ImportResult};
pub use verifier::{PackageVerifier, VerificationResult, VerificationStatus};
