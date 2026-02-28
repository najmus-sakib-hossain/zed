//! DX Test Executor - Parallel Test Execution
//!
//! Work-stealing parallel test executor with dynamic load balancing.

use dx_test_cache::CachedLayout;
use dx_test_core::*;
use dx_test_vm::TestVM;
use rayon::prelude::*;

/// Parallel test executor
pub struct TestExecutor {
    parallel: bool,
}

impl TestExecutor {
    pub fn new(parallel: bool) -> Self {
        Self { parallel }
    }

    /// Execute tests in parallel
    pub fn execute(&self, tests: &[FlatTestEntry], layout: &CachedLayout) -> Vec<TestResult> {
        if self.parallel && tests.len() > 1 {
            // Parallel execution with rayon
            tests
                .par_iter()
                .map(|test| {
                    let mut vm = TestVM::new();
                    let bytecode = layout.get_bytecode(test);
                    vm.execute(bytecode)
                })
                .collect()
        } else {
            // Sequential execution
            let mut vm = TestVM::new();
            tests
                .iter()
                .map(|test| {
                    let bytecode = layout.get_bytecode(test);
                    let result = vm.execute(bytecode);
                    vm.reset();
                    result
                })
                .collect()
        }
    }

    /// Execute with filtering
    pub fn execute_filtered<F>(
        &self,
        tests: &[FlatTestEntry],
        layout: &CachedLayout,
        filter: F,
    ) -> Vec<(usize, TestResult)>
    where
        F: Fn(&FlatTestEntry) -> bool + Send + Sync,
    {
        if self.parallel && tests.len() > 1 {
            tests
                .par_iter()
                .enumerate()
                .filter(|(_, test)| filter(test))
                .map(|(idx, test)| {
                    let mut vm = TestVM::new();
                    let bytecode = layout.get_bytecode(test);
                    let result = vm.execute(bytecode);
                    (idx, result)
                })
                .collect()
        } else {
            let mut vm = TestVM::new();
            tests
                .iter()
                .enumerate()
                .filter(|(_, test)| filter(test))
                .map(|(idx, test)| {
                    let bytecode = layout.get_bytecode(test);
                    let result = vm.execute(bytecode);
                    vm.reset();
                    (idx, result)
                })
                .collect()
        }
    }

    /// Get optimal thread count
    pub fn thread_count() -> usize {
        num_cpus::get()
    }
}

impl Default for TestExecutor {
    fn default() -> Self {
        Self::new(true)
    }
}
