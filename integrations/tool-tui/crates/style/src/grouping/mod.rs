//! Auto-grouping module for CSS class pattern detection and classname generation
//!
//! This module provides functionality for automatically grouping similar CSS class
//! patterns and generating short, deterministic classnames for them.

pub mod auto;
pub mod classname;
pub mod optimizer;

pub use auto::{AutoGroupConfig, AutoGroupInfo, AutoGroupRewrite, AutoGrouper};
pub use classname::ClassnameGenerator;
pub use optimizer::{
    ApplyResult, FileScanner, GroupingCandidate, GroupingConfig, GroupingOptimizer, GroupingReport,
    SizeCalculator,
};
