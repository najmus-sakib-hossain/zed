//! Integration tests for DX-Py runtime
//!
//! This module contains integration tests that verify end-to-end behavior:
//! - CPython compatibility tests
//! - Real-world package tests (Flask, requests, click, NumPy)
//! - Performance benchmarks
//! - Test runner integration tests
//!
//! Requirements: 10.1, 10.2, 10.3, 10.4, 10.5, 12.1, 12.2, 12.3, 12.4, 12.5

pub mod cpython_compat_tests;
pub mod performance_benchmarks;
pub mod real_world_tests;
pub mod test_runner_integration;

// Re-export key types for use in other test modules
pub use real_world_tests::{
    execute_python_script,
    execute_python_command,
    execute_python_script_with_timeout,
    is_package_installed,
    ExecutionResult,
    PythonRuntime,
};
