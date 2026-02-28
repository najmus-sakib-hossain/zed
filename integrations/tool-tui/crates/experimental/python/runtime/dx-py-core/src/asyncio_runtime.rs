//! Asyncio runtime for executing coroutines
//!
//! This module provides a simple event loop implementation for running coroutines.

use crate::pygenerator::{CoroutineResult, CoroutineState, PyCoroutine};
use crate::pylist::PyValue;
use crate::RuntimeError;
use std::collections::VecDeque;
use std::sync::Arc;

/// Simple event loop for running coroutines
pub struct EventLoop {
    /// Queue of coroutines ready to run
    ready: VecDeque<Arc<PyCoroutine>>,
    /// Queue of coroutines waiting (suspended)
    waiting: VecDeque<Arc<PyCoroutine>>,
}

impl EventLoop {
    /// Create a new event loop
    pub fn new() -> Self {
        Self {
            ready: VecDeque::new(),
            waiting: VecDeque::new(),
        }
    }

    /// Run a coroutine until it completes
    /// This is the implementation of asyncio.run()
    pub fn run_until_complete(&mut self, coro: Arc<PyCoroutine>) -> Result<PyValue, String> {
        // Add the coroutine to the ready queue
        self.ready.push_back(coro);

        // Run the event loop until all coroutines are done
        while !self.ready.is_empty() || !self.waiting.is_empty() {
            // Process all ready coroutines
            while let Some(coro) = self.ready.pop_front() {
                // Check if coroutine is done
                if coro.is_done() {
                    continue;
                }

                // Send None to the coroutine to resume it
                let result = coro.send(PyValue::None);

                match result {
                    CoroutineResult::NeedExecution => {
                        // Coroutine needs to be executed by the VM
                        // For now, we'll just mark it as waiting
                        // In a real implementation, this would be handled by the dispatcher
                        self.waiting.push_back(coro);
                    }
                    CoroutineResult::Awaiting(_value) => {
                        // Coroutine is awaiting something
                        // Add it back to the waiting queue
                        self.waiting.push_back(coro);
                    }
                    CoroutineResult::StopIteration(value) => {
                        // Coroutine completed successfully
                        // If this was the main coroutine, return its value
                        if self.ready.is_empty() && self.waiting.is_empty() {
                            return Ok(value);
                        }
                    }
                    CoroutineResult::Error(msg) => {
                        return Err(msg);
                    }
                    CoroutineResult::Closed => {
                        // Coroutine was closed
                        continue;
                    }
                }
            }

            // Move waiting coroutines back to ready queue
            // In a real implementation, we would check if they're actually ready
            while let Some(coro) = self.waiting.pop_front() {
                self.ready.push_back(coro);
            }

            // If we have no ready coroutines but have waiting ones, break to avoid infinite loop
            if self.ready.is_empty() && !self.waiting.is_empty() {
                return Err("Event loop stuck: all coroutines are waiting".to_string());
            }
        }

        // If we get here, all coroutines completed but we didn't return a value
        Ok(PyValue::None)
    }

    /// Run multiple coroutines concurrently (asyncio.gather)
    pub fn gather(&mut self, coros: Vec<Arc<PyCoroutine>>) -> Result<Vec<PyValue>, String> {
        // Add all coroutines to the ready queue
        for coro in &coros {
            self.ready.push_back(Arc::clone(coro));
        }

        // Track which coroutines have completed
        let mut completed = vec![false; coros.len()];
        let mut coro_results: Vec<PyValue> = vec![PyValue::None; coros.len()];

        // Run until all coroutines complete
        while completed.iter().any(|&c| !c) {
            // Process ready coroutines
            let ready_count = self.ready.len();
            for _ in 0..ready_count {
                if let Some(coro) = self.ready.pop_front() {
                    // Find which coroutine this is
                    let idx = coros
                        .iter()
                        .position(|c| Arc::ptr_eq(c, &coro))
                        .unwrap_or(coros.len());

                    if idx < coros.len() && !completed[idx] {
                        let result = coro.send(PyValue::None);

                        match result {
                            CoroutineResult::NeedExecution => {
                                self.waiting.push_back(coro);
                            }
                            CoroutineResult::Awaiting(_) => {
                                self.waiting.push_back(coro);
                            }
                            CoroutineResult::StopIteration(value) => {
                                completed[idx] = true;
                                coro_results[idx] = value;
                            }
                            CoroutineResult::Error(msg) => {
                                return Err(msg);
                            }
                            CoroutineResult::Closed => {
                                completed[idx] = true;
                            }
                        }
                    }
                }
            }

            // Move waiting back to ready
            while let Some(coro) = self.waiting.pop_front() {
                self.ready.push_back(coro);
            }

            // Prevent infinite loop
            if self.ready.is_empty() && !self.waiting.is_empty() {
                return Err("Event loop stuck in gather".to_string());
            }
        }

        Ok(coro_results)
    }
}

impl Default for EventLoop {
    fn default() -> Self {
        Self::new()
    }
}

/// Run a coroutine to completion (asyncio.run implementation)
pub fn run_coroutine(coro: Arc<PyCoroutine>) -> Result<PyValue, RuntimeError> {
    let mut event_loop = EventLoop::new();
    event_loop
        .run_until_complete(coro)
        .map_err(|e| RuntimeError::ValueError { message: e })
}

/// Run multiple coroutines concurrently (asyncio.gather implementation)
pub fn gather_coroutines(coros: Vec<Arc<PyCoroutine>>) -> Result<Vec<PyValue>, RuntimeError> {
    let mut event_loop = EventLoop::new();
    event_loop
        .gather(coros)
        .map_err(|e| RuntimeError::ValueError { message: e })
}
