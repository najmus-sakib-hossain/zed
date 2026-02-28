//! Memory-optimized server configuration for dx-server
//!
//! Target: Beat Fiber's 5-15 MB per instance
//!
//! ## Techniques Used
//!
//! 1. **System Allocator** - Predictable, no jemalloc/mimalloc overhead
//! 2. **Pre-allocated Buffers** - Fixed-size buffer pools
//! 3. **Connection Limits** - Bounded memory growth
//! 4. **Static Responses** - Zero per-request allocations

use std::sync::atomic::{AtomicUsize, Ordering};

/// Memory-efficient server configuration
#[derive(Debug, Clone)]
pub struct MemoryConfig {
    /// Maximum concurrent connections
    pub max_connections: usize,
    /// Pre-allocated buffer pool size (number of buffers)
    pub buffer_pool_size: usize,
    /// Size of each pre-allocated buffer in bytes
    pub buffer_size: usize,
    /// Maximum request body size in bytes
    pub max_body_size: usize,
    /// Enable aggressive memory reclamation
    pub aggressive_gc: bool,
}

impl Default for MemoryConfig {
    fn default() -> Self {
        Self {
            max_connections: 10_000,
            buffer_pool_size: 256,
            buffer_size: 4096,          // 4KB buffers
            max_body_size: 1024 * 1024, // 1MB max body
            aggressive_gc: false,
        }
    }
}

impl MemoryConfig {
    /// Configuration optimized for minimum memory usage
    pub fn minimal() -> Self {
        Self {
            max_connections: 1_000,
            buffer_pool_size: 64,
            buffer_size: 1024,        // 1KB buffers
            max_body_size: 64 * 1024, // 64KB max body
            aggressive_gc: true,
        }
    }

    /// Configuration optimized for high throughput
    pub fn high_throughput() -> Self {
        Self {
            max_connections: 100_000,
            buffer_pool_size: 1024,
            buffer_size: 8192,               // 8KB buffers
            max_body_size: 10 * 1024 * 1024, // 10MB max body
            aggressive_gc: false,
        }
    }

    /// Estimated maximum memory usage in bytes
    pub fn estimated_max_memory(&self) -> usize {
        // Base overhead + buffer pool + per-connection overhead
        let base = 2 * 1024 * 1024; // ~2MB base
        let buffers = self.buffer_pool_size * self.buffer_size;
        let connections = self.max_connections * 512; // ~512 bytes per connection estimate
        base + buffers + connections
    }
}

/// Pre-computed static responses for zero-allocation paths
pub mod static_responses {
    /// Plaintext "Hello, World!" response
    pub const PLAINTEXT: &[u8] = b"Hello, World!";

    /// Pre-built HTTP response for plaintext (no allocation needed)
    pub const PLAINTEXT_HTTP: &[u8] = b"HTTP/1.1 200 OK\r\n\
        Content-Type: text/plain\r\n\
        Content-Length: 13\r\n\
        Server: dx-server/0.1\r\n\
        Connection: keep-alive\r\n\
        \r\n\
        Hello, World!";

    /// Health check response
    pub const HEALTH_HTTP: &[u8] = b"HTTP/1.1 200 OK\r\n\
        Content-Type: text/plain\r\n\
        Content-Length: 2\r\n\
        Connection: keep-alive\r\n\
        \r\n\
        OK";

    /// 404 response
    pub const NOT_FOUND_HTTP: &[u8] = b"HTTP/1.1 404 Not Found\r\n\
        Content-Type: text/plain\r\n\
        Content-Length: 9\r\n\
        Connection: keep-alive\r\n\
        \r\n\
        Not Found";
}

/// Connection counter for tracking active connections
pub struct ConnectionCounter {
    active: AtomicUsize,
    total: AtomicUsize,
    max_seen: AtomicUsize,
}

impl ConnectionCounter {
    pub const fn new() -> Self {
        Self {
            active: AtomicUsize::new(0),
            total: AtomicUsize::new(0),
            max_seen: AtomicUsize::new(0),
        }
    }

    /// Increment active connections
    pub fn connect(&self) {
        let prev = self.active.fetch_add(1, Ordering::Relaxed);
        self.total.fetch_add(1, Ordering::Relaxed);

        // Update max seen
        let current = prev + 1;
        let mut max = self.max_seen.load(Ordering::Relaxed);
        while current > max {
            match self.max_seen.compare_exchange_weak(
                max,
                current,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(m) => max = m,
            }
        }
    }

    /// Decrement active connections
    pub fn disconnect(&self) {
        self.active.fetch_sub(1, Ordering::Relaxed);
    }

    /// Get current active connection count
    pub fn active(&self) -> usize {
        self.active.load(Ordering::Relaxed)
    }

    /// Get total connections served
    pub fn total(&self) -> usize {
        self.total.load(Ordering::Relaxed)
    }

    /// Get maximum concurrent connections seen
    pub fn max_seen(&self) -> usize {
        self.max_seen.load(Ordering::Relaxed)
    }
}

impl Default for ConnectionCounter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_config_defaults() {
        let config = MemoryConfig::default();
        assert_eq!(config.max_connections, 10_000);
        assert_eq!(config.buffer_size, 4096);
    }

    #[test]
    fn test_memory_estimation() {
        let config = MemoryConfig::minimal();
        let estimated = config.estimated_max_memory();
        // Should be under 5MB for minimal config
        assert!(estimated < 5 * 1024 * 1024);
    }

    #[test]
    fn test_connection_counter() {
        let counter = ConnectionCounter::new();

        counter.connect();
        counter.connect();
        assert_eq!(counter.active(), 2);
        assert_eq!(counter.total(), 2);

        counter.disconnect();
        assert_eq!(counter.active(), 1);
        assert_eq!(counter.total(), 2);
        assert_eq!(counter.max_seen(), 2);
    }

    #[test]
    fn test_static_responses() {
        assert_eq!(static_responses::PLAINTEXT, b"Hello, World!");
        assert!(static_responses::PLAINTEXT_HTTP.starts_with(b"HTTP/1.1 200"));
    }
}
