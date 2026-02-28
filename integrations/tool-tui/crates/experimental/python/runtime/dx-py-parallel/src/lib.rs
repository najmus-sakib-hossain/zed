//! DX-Py Parallel - Thread-Per-Core Parallel Executor
//!
//! This crate implements a high-performance parallel executor with:
//! - One thread per physical CPU core
//! - Core pinning for cache locality
//! - Work-stealing for load balancing

pub mod executor;
pub mod parallel_object;
pub mod task;
pub mod worker;

pub use executor::{ExecutorError, ParallelExecutor};
pub use parallel_object::ParallelPyObject;
pub use task::Task;
pub use worker::{Worker, WorkerError};
