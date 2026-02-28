//! Test Runner Module
//!
//! Provides test discovery, execution, and coverage reporting
//! with output in DX Serializer format for 50-70% token savings.

mod coverage;
mod discovery;
mod runner;

pub use coverage::{CoverageReport, FileCoverage, LineCoverage};
pub use discovery::{TestCase, TestDiscovery, TestFile, TestFramework};
pub use runner::{TestOutput, TestResult, TestRunner, TestStatus, TestSuite};
