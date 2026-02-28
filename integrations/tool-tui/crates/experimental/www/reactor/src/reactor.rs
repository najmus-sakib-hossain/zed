//! DxReactor - Main entry point for Binary Dawn.

use crate::core_state::CoreState;
use crate::io::{PlatformReactor, Reactor, ReactorConfig};
use crate::protocol::HbtpProtocol;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::{self, JoinHandle};
use std::time::Duration;

/// Worker thread strategy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WorkerStrategy {
    /// One worker thread per CPU core (default).
    #[default]
    ThreadPerCore,
    /// Fixed number of worker threads.
    Fixed(usize),
}

/// I/O backend selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum IoBackend {
    /// io_uring (Linux 5.1+).
    IoUring,
    /// epoll (Linux fallback).
    Epoll,
    /// kqueue (macOS/BSD).
    Kqueue,
    /// IOCP (Windows).
    Iocp,
    /// Automatically select the best backend.
    #[default]
    Auto,
}

/// Builder for DxReactor.
pub struct ReactorBuilder {
    workers: WorkerStrategy,
    io_backend: IoBackend,
    teleport: bool,
    hbtp: bool,
    buffer_size: usize,
    buffer_count: usize,
}

impl ReactorBuilder {
    /// Create a new builder with default settings.
    pub fn new() -> Self {
        Self {
            workers: WorkerStrategy::default(),
            io_backend: IoBackend::default(),
            teleport: true,
            hbtp: true,
            buffer_size: 4096,
            buffer_count: 1024,
        }
    }

    /// Set the worker strategy.
    pub fn workers(mut self, strategy: WorkerStrategy) -> Self {
        self.workers = strategy;
        self
    }

    /// Set the I/O backend.
    pub fn io_backend(mut self, backend: IoBackend) -> Self {
        self.io_backend = backend;
        self
    }

    /// Enable or disable memory teleportation.
    pub fn teleport(mut self, enabled: bool) -> Self {
        self.teleport = enabled;
        self
    }

    /// Enable or disable HBTP protocol.
    pub fn hbtp(mut self, enabled: bool) -> Self {
        self.hbtp = enabled;
        self
    }

    /// Set the buffer size for I/O operations.
    pub fn buffer_size(mut self, size: usize) -> Self {
        self.buffer_size = size;
        self
    }

    /// Set the number of pre-allocated buffers.
    pub fn buffer_count(mut self, count: usize) -> Self {
        self.buffer_count = count;
        self
    }

    /// Build the DxReactor.
    pub fn build(self) -> DxReactor {
        let num_cores = match self.workers {
            WorkerStrategy::ThreadPerCore => num_cpus::get(),
            WorkerStrategy::Fixed(n) => n,
        };

        let config = ReactorConfig::default()
            .buffer_size(self.buffer_size)
            .buffer_count(self.buffer_count);

        let cores: Vec<CoreState> =
            (0..num_cores).map(|id| CoreState::new(id, config.clone())).collect();

        DxReactor {
            config,
            cores,
            protocol: Arc::new(HbtpProtocol::new()),
            worker_strategy: self.workers,
            io_backend: self.io_backend,
            shutdown: Arc::new(AtomicBool::new(false)),
        }
    }
}

impl Default for ReactorBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// The main DxReactor - Binary Dawn's core.
pub struct DxReactor {
    /// Reactor configuration.
    config: ReactorConfig,
    /// Per-core state.
    cores: Vec<CoreState>,
    /// HBTP protocol handler.
    protocol: Arc<HbtpProtocol>,
    /// Worker strategy used.
    worker_strategy: WorkerStrategy,
    /// I/O backend used.
    io_backend: IoBackend,
    /// Shutdown flag.
    shutdown: Arc<AtomicBool>,
}

impl DxReactor {
    /// Create a new ReactorBuilder.
    pub fn build() -> ReactorBuilder {
        ReactorBuilder::new()
    }

    /// Get the number of cores/workers.
    pub fn num_cores(&self) -> usize {
        self.cores.len()
    }

    /// Get the worker strategy.
    pub fn worker_strategy(&self) -> WorkerStrategy {
        self.worker_strategy
    }

    /// Get the I/O backend.
    pub fn io_backend(&self) -> IoBackend {
        self.io_backend
    }

    /// Get a reference to the HBTP protocol handler.
    pub fn protocol(&self) -> &HbtpProtocol {
        &self.protocol
    }

    /// Get a mutable reference to the HBTP protocol handler.
    pub fn protocol_mut(&mut self) -> &mut HbtpProtocol {
        Arc::make_mut(&mut self.protocol)
    }

    /// Get a reference to a specific core's state.
    pub fn core(&self, id: usize) -> Option<&CoreState> {
        self.cores.get(id)
    }

    /// Get the reactor configuration.
    pub fn config(&self) -> &ReactorConfig {
        &self.config
    }

    /// Start the reactor (blocking).
    ///
    /// This spawns worker threads and runs the event loop.
    /// This function never returns under normal operation.
    ///
    /// # Panics
    ///
    /// Panics if worker threads cannot be spawned. This indicates a critical
    /// system resource issue (e.g., thread limit reached, out of memory).
    pub fn ignite(self) -> ! {
        let num_workers = self.cores.len();
        let shutdown = self.shutdown.clone();
        let config = self.config.clone();

        // Spawn worker threads
        let handles: Vec<JoinHandle<()>> = (0..num_workers)
            .map(|worker_id| {
                let shutdown = shutdown.clone();
                let config = config.clone();

                thread::Builder::new()
                    .name(format!("dx-worker-{}", worker_id))
                    .spawn(move || {
                        Self::run_worker(worker_id, config, shutdown);
                    })
                    .unwrap_or_else(|e| {
                        panic!(
                            "Failed to spawn worker thread {}: {}. \
                            This may indicate system resource exhaustion (thread limit, memory).",
                            worker_id, e
                        )
                    })
            })
            .collect();

        // Main thread waits for shutdown signal
        while !shutdown.load(Ordering::Relaxed) {
            thread::park_timeout(Duration::from_millis(100));
        }

        // Wait for all workers to finish
        for handle in handles {
            let _ = handle.join();
        }

        // This function is marked as never returning, so we loop forever
        // In practice, the process would be terminated by a signal handler
        loop {
            thread::park();
        }
    }

    /// Run a worker thread's event loop.
    fn run_worker(worker_id: usize, config: ReactorConfig, shutdown: Arc<AtomicBool>) {
        // Create platform-specific reactor for this worker
        let reactor = match PlatformReactor::new(config) {
            Ok(r) => r,
            Err(e) => {
                tracing::error!("Worker {} failed to create reactor: {}", worker_id, e);
                return;
            }
        };

        tracing::info!("Worker {} started with {} backend", worker_id, crate::io::best_available());

        // Event loop
        while !shutdown.load(Ordering::Relaxed) {
            // Submit any pending operations
            if let Err(e) = reactor.submit() {
                tracing::warn!("Worker {} submit error: {}", worker_id, e);
            }

            // Wait for completions with a timeout
            match reactor.wait(Some(Duration::from_millis(100))) {
                Ok(completions) => {
                    for completion in completions {
                        // Process each completion
                        Self::handle_completion(worker_id, &completion);
                    }
                }
                Err(e) => {
                    if e.kind() != std::io::ErrorKind::Interrupted {
                        tracing::warn!("Worker {} wait error: {}", worker_id, e);
                    }
                }
            }
        }

        tracing::info!("Worker {} shutting down", worker_id);
    }

    /// Handle a single I/O completion.
    fn handle_completion(worker_id: usize, completion: &crate::io::Completion) {
        if completion.is_error() {
            tracing::debug!(
                "Worker {} completion error: user_data={}, error={}",
                worker_id,
                completion.user_data,
                completion.error_code().unwrap_or(0)
            );
        } else {
            tracing::trace!(
                "Worker {} completion: user_data={}, bytes={}",
                worker_id,
                completion.user_data,
                completion.bytes_transferred().unwrap_or(0)
            );
        }
        // In a full implementation, this would dispatch to registered callbacks
        // based on user_data
    }

    /// Request shutdown of the reactor.
    pub fn shutdown(&self) {
        self.shutdown.store(true, Ordering::Relaxed);
    }

    /// Check if shutdown has been requested.
    pub fn is_shutdown(&self) -> bool {
        self.shutdown.load(Ordering::Relaxed)
    }
}
