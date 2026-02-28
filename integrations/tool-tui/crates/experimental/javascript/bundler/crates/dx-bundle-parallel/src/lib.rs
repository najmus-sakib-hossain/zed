//! Speculative parallel processing
//!
//! Start transforming modules before resolution completes - 3x faster!

pub mod speculative;
pub mod worker;

pub use speculative::SpeculativeBundler;

use dx_bundle_core::{ModuleId, TransformedModule};

/// Bundle result from parallel processing
#[derive(Clone)]
pub struct ParallelBundle {
    /// All transformed modules
    pub modules: Vec<TransformedModule>,
    /// Entry points
    pub entries: Vec<ModuleId>,
    /// Total processing time (ms)
    pub time_ms: f64,
}

/// Parallel bundling options
#[derive(Clone, Debug)]
pub struct ParallelOptions {
    /// Number of worker threads (0 = auto)
    pub threads: usize,
    /// Enable speculative execution
    pub speculative: bool,
    /// Maximum modules to process in parallel
    pub max_parallel: usize,
}

impl Default for ParallelOptions {
    fn default() -> Self {
        Self {
            threads: 0, // Auto-detect
            speculative: true,
            max_parallel: 128,
        }
    }
}

impl ParallelOptions {
    /// Get actual thread count
    pub fn thread_count(&self) -> usize {
        if self.threads == 0 {
            num_cpus::get()
        } else {
            self.threads
        }
    }
}
