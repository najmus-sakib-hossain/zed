//! Comparative benchmarking framework for DX-Py components
//!
//! This crate provides a comprehensive benchmarking framework for comparing
//! DX-Py components against industry-standard tools (CPython, UV, pytest/unittest).

pub mod analysis;
pub mod core;
pub mod data;
pub mod report;
pub mod suites;

pub use analysis::*;
pub use core::*;
pub use data::*;
pub use report::*;
