//! Reactor configuration.

/// Configuration for reactor initialization.
#[derive(Debug, Clone)]
pub struct ReactorConfig {
    /// Number of submission queue entries (default: 16384).
    pub entries: u32,

    /// Enable kernel-side polling (Linux io_uring only).
    /// When enabled, the kernel polls for completions without syscalls.
    pub sqpoll: bool,

    /// SQPOLL idle timeout in milliseconds.
    /// The kernel thread will sleep after this many ms of inactivity.
    pub sqpoll_idle_ms: u32,

    /// CPU to pin SQPOLL thread to (Linux io_uring only).
    pub sqpoll_cpu: Option<u32>,

    /// Enable zero-copy I/O operations.
    pub zero_copy: bool,

    /// Buffer size for registered buffers (bytes).
    pub buffer_size: usize,

    /// Number of pre-registered buffers.
    pub buffer_count: usize,

    /// Concurrency hint for IOCP (Windows only).
    /// Typically set to the number of CPU cores.
    pub concurrency_hint: usize,
}

impl Default for ReactorConfig {
    fn default() -> Self {
        Self {
            entries: 16384,
            sqpoll: false,
            sqpoll_idle_ms: 1000,
            sqpoll_cpu: None,
            zero_copy: false,
            buffer_size: 4096,
            buffer_count: 1024,
            concurrency_hint: num_cpus::get(),
        }
    }
}

impl ReactorConfig {
    /// Create a new configuration with default values.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the number of submission queue entries.
    pub fn entries(mut self, entries: u32) -> Self {
        self.entries = entries;
        self
    }

    /// Enable kernel-side polling (Linux io_uring only).
    pub fn sqpoll(mut self, enabled: bool) -> Self {
        self.sqpoll = enabled;
        self
    }

    /// Set SQPOLL idle timeout in milliseconds.
    pub fn sqpoll_idle_ms(mut self, ms: u32) -> Self {
        self.sqpoll_idle_ms = ms;
        self
    }

    /// Set CPU affinity for SQPOLL thread.
    pub fn sqpoll_cpu(mut self, cpu: u32) -> Self {
        self.sqpoll_cpu = Some(cpu);
        self
    }

    /// Enable zero-copy I/O operations.
    pub fn zero_copy(mut self, enabled: bool) -> Self {
        self.zero_copy = enabled;
        self
    }

    /// Set buffer size for registered buffers.
    pub fn buffer_size(mut self, size: usize) -> Self {
        self.buffer_size = size;
        self
    }

    /// Set number of pre-registered buffers.
    pub fn buffer_count(mut self, count: usize) -> Self {
        self.buffer_count = count;
        self
    }

    /// Set concurrency hint for IOCP.
    pub fn concurrency_hint(mut self, hint: usize) -> Self {
        self.concurrency_hint = hint;
        self
    }
}
