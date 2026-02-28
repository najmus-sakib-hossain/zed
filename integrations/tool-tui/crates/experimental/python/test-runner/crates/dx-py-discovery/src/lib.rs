//! Test discovery engine using tree-sitter for zero-import scanning
//!
//! This crate provides Rust-based AST scanning of Python files to discover
//! test functions without importing them, achieving 100x faster discovery.

mod asyncio_support;
mod coverage;
mod fixture_discovery;
mod index;
mod parametrize;
mod parser;
mod plugin;
mod scanner;

pub use asyncio_support::{AsyncTestDetector, AsyncioConfig, AsyncioMode, EventLoopScope};
pub use coverage::{
    CoverageArgs, CoverageConfig, CoverageData, CoverageReportFormat, FileCoverage,
};
pub use fixture_discovery::{DiscoveredFixture, FixtureDiscovery};
pub use index::{TestIndex, TestIndexBuilder};
pub use parametrize::{ExpandedTest, ParameterSet, ParametrizeExpander};
pub use parser::PythonParser;
pub use plugin::{ConftestFile, HookImplementation, HookType, KnownPlugin, PluginManager};
pub use scanner::{DiscoveredTest, TestScanner};

#[cfg(test)]
mod tests;
