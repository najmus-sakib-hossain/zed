//! Resource Limiter
//!
//! Enforces memory and CPU limits for plugin execution. Works with both
//! WASM fuel metering and wall-clock timeouts for native plugins.
//!
//! # Limits
//!
//! - **Memory**: Configurable per-plugin byte limit, tracked via WASM linear memory pages
//! - **CPU**: Fuel-based metering for WASM; wall-clock timeout for native
//! - **Execution time**: Hard wall-clock timeout as a safety net

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use anyhow::Result;

/// Resource limits configuration
#[derive(Debug, Clone)]
pub struct ResourceLimits {
    /// Maximum memory in bytes (0 = unlimited)
    pub max_memory_bytes: usize,
    /// Maximum CPU fuel (WASM instruction budget, 0 = unlimited)
    pub max_fuel: u64,
    /// Maximum wall-clock time
    pub max_duration: Duration,
    /// Maximum output size in bytes
    pub max_output_bytes: usize,
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            max_memory_bytes: 256 * 1024 * 1024, // 256 MB
            max_fuel: 1_000_000_000,             // 1 billion instructions
            max_duration: Duration::from_secs(30),
            max_output_bytes: 10 * 1024 * 1024, // 10 MB
        }
    }
}

impl ResourceLimits {
    /// Create restrictive limits (e.g., for untrusted plugins)
    pub fn restrictive() -> Self {
        Self {
            max_memory_bytes: 32 * 1024 * 1024, // 32 MB
            max_fuel: 100_000_000,              // 100M instructions
            max_duration: Duration::from_secs(5),
            max_output_bytes: 1024 * 1024, // 1 MB
        }
    }

    /// Create generous limits (e.g., for trusted/signed plugins)
    pub fn generous() -> Self {
        Self {
            max_memory_bytes: 1024 * 1024 * 1024, // 1 GB
            max_fuel: 10_000_000_000,             // 10B instructions
            max_duration: Duration::from_secs(300),
            max_output_bytes: 100 * 1024 * 1024, // 100 MB
        }
    }

    /// Convert memory limit to WASM pages (64 KiB each)
    pub fn max_wasm_pages(&self) -> u32 {
        if self.max_memory_bytes == 0 {
            u32::MAX
        } else {
            (self.max_memory_bytes / (64 * 1024)) as u32
        }
    }
}

/// Live resource usage tracker for a single plugin execution
pub struct ResourceTracker {
    limits: ResourceLimits,
    /// Bytes of memory currently allocated
    memory_used: AtomicUsize,
    /// Fuel consumed so far
    fuel_consumed: AtomicU64,
    /// Execution start time
    started_at: Instant,
    /// Whether the execution has been killed
    killed: AtomicBool,
    /// Reason for kill (if any)
    kill_reason: std::sync::Mutex<Option<String>>,
}

impl ResourceTracker {
    /// Create a new tracker with the given limits
    pub fn new(limits: ResourceLimits) -> Self {
        Self {
            limits,
            memory_used: AtomicUsize::new(0),
            fuel_consumed: AtomicU64::new(0),
            started_at: Instant::now(),
            killed: AtomicBool::new(false),
            kill_reason: std::sync::Mutex::new(None),
        }
    }

    /// Record memory allocation. Returns Err if limit exceeded.
    pub fn alloc_memory(&self, bytes: usize) -> Result<()> {
        let new_total = self.memory_used.fetch_add(bytes, Ordering::SeqCst) + bytes;
        if self.limits.max_memory_bytes > 0 && new_total > self.limits.max_memory_bytes {
            self.kill("Memory limit exceeded");
            anyhow::bail!(
                "Memory limit exceeded: {} > {} bytes",
                new_total,
                self.limits.max_memory_bytes
            );
        }
        Ok(())
    }

    /// Record memory deallocation
    pub fn dealloc_memory(&self, bytes: usize) {
        self.memory_used
            .fetch_sub(bytes.min(self.memory_used.load(Ordering::SeqCst)), Ordering::SeqCst);
    }

    /// Record fuel consumption. Returns Err if limit exceeded.
    pub fn consume_fuel(&self, fuel: u64) -> Result<()> {
        let new_total = self.fuel_consumed.fetch_add(fuel, Ordering::SeqCst) + fuel;
        if self.limits.max_fuel > 0 && new_total > self.limits.max_fuel {
            self.kill("CPU fuel limit exceeded");
            anyhow::bail!("CPU fuel limit exceeded: {} > {}", new_total, self.limits.max_fuel);
        }
        Ok(())
    }

    /// Check if wall-clock timeout has been exceeded
    pub fn check_timeout(&self) -> Result<()> {
        let elapsed = self.started_at.elapsed();
        if elapsed > self.limits.max_duration {
            self.kill("Execution timeout");
            anyhow::bail!("Execution timeout: {:?} > {:?}", elapsed, self.limits.max_duration);
        }
        Ok(())
    }

    /// Comprehensive check — memory + fuel + timeout
    pub fn check_all(&self) -> Result<()> {
        if self.is_killed() {
            let reason = self.kill_reason.lock().unwrap().clone().unwrap_or_default();
            anyhow::bail!("Execution killed: {}", reason);
        }
        self.check_timeout()
    }

    /// Kill the execution
    pub fn kill(&self, reason: &str) {
        self.killed.store(true, Ordering::SeqCst);
        *self.kill_reason.lock().unwrap() = Some(reason.to_string());
    }

    /// Whether execution has been killed
    pub fn is_killed(&self) -> bool {
        self.killed.load(Ordering::SeqCst)
    }

    /// Current memory usage
    pub fn memory_used(&self) -> usize {
        self.memory_used.load(Ordering::SeqCst)
    }

    /// Current fuel consumed
    pub fn fuel_consumed(&self) -> u64 {
        self.fuel_consumed.load(Ordering::SeqCst)
    }

    /// Elapsed wall-clock time
    pub fn elapsed(&self) -> Duration {
        self.started_at.elapsed()
    }

    /// Get a snapshot of current resource usage
    pub fn snapshot(&self) -> ResourceSnapshot {
        ResourceSnapshot {
            memory_used: self.memory_used(),
            fuel_consumed: self.fuel_consumed(),
            elapsed: self.elapsed(),
            killed: self.is_killed(),
        }
    }
}

/// Point-in-time snapshot of resource usage
#[derive(Debug, Clone)]
pub struct ResourceSnapshot {
    pub memory_used: usize,
    pub fuel_consumed: u64,
    pub elapsed: Duration,
    pub killed: bool,
}

impl std::fmt::Display for ResourceSnapshot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "mem={}KB fuel={} elapsed={:?}{}",
            self.memory_used / 1024,
            self.fuel_consumed,
            self.elapsed,
            if self.killed { " KILLED" } else { "" }
        )
    }
}

/// Spawn a background watchdog task that periodically checks resource limits
/// and kills the execution if any limit is exceeded.
pub fn spawn_watchdog(
    tracker: Arc<ResourceTracker>,
    check_interval: Duration,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(check_interval).await;

            if tracker.is_killed() {
                break;
            }

            if let Err(e) = tracker.check_all() {
                tracing::warn!("Resource watchdog triggered: {}", e);
                break;
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_limits() {
        let limits = ResourceLimits::default();
        assert_eq!(limits.max_memory_bytes, 256 * 1024 * 1024);
        assert_eq!(limits.max_fuel, 1_000_000_000);
    }

    #[test]
    fn test_restrictive_limits() {
        let limits = ResourceLimits::restrictive();
        assert_eq!(limits.max_memory_bytes, 32 * 1024 * 1024);
    }

    #[test]
    fn test_wasm_pages() {
        let limits = ResourceLimits {
            max_memory_bytes: 64 * 1024 * 1024, // 64 MB
            ..Default::default()
        };
        assert_eq!(limits.max_wasm_pages(), 1024); // 64MB / 64KB = 1024 pages
    }

    #[test]
    fn test_tracker_alloc_within_limit() {
        let limits = ResourceLimits {
            max_memory_bytes: 1024,
            ..Default::default()
        };
        let tracker = ResourceTracker::new(limits);

        assert!(tracker.alloc_memory(512).is_ok());
        assert_eq!(tracker.memory_used(), 512);
        assert!(tracker.alloc_memory(256).is_ok());
        assert_eq!(tracker.memory_used(), 768);
    }

    #[test]
    fn test_tracker_alloc_exceeds_limit() {
        let limits = ResourceLimits {
            max_memory_bytes: 1024,
            ..Default::default()
        };
        let tracker = ResourceTracker::new(limits);

        assert!(tracker.alloc_memory(512).is_ok());
        let result = tracker.alloc_memory(1024);
        assert!(result.is_err());
        assert!(tracker.is_killed());
    }

    #[test]
    fn test_tracker_dealloc() {
        let limits = ResourceLimits::default();
        let tracker = ResourceTracker::new(limits);

        tracker.alloc_memory(1024).unwrap();
        tracker.dealloc_memory(512);
        assert_eq!(tracker.memory_used(), 512);
    }

    #[test]
    fn test_tracker_fuel_within_limit() {
        let limits = ResourceLimits {
            max_fuel: 1000,
            ..Default::default()
        };
        let tracker = ResourceTracker::new(limits);

        assert!(tracker.consume_fuel(500).is_ok());
        assert_eq!(tracker.fuel_consumed(), 500);
    }

    #[test]
    fn test_tracker_fuel_exceeds_limit() {
        let limits = ResourceLimits {
            max_fuel: 1000,
            ..Default::default()
        };
        let tracker = ResourceTracker::new(limits);

        assert!(tracker.consume_fuel(500).is_ok());
        let result = tracker.consume_fuel(600);
        assert!(result.is_err());
        assert!(tracker.is_killed());
    }

    #[test]
    fn test_tracker_timeout() {
        let limits = ResourceLimits {
            max_duration: Duration::from_millis(1),
            ..Default::default()
        };
        let tracker = ResourceTracker::new(limits);

        std::thread::sleep(Duration::from_millis(10));
        assert!(tracker.check_timeout().is_err());
    }

    #[test]
    fn test_tracker_kill() {
        let tracker = ResourceTracker::new(ResourceLimits::default());
        assert!(!tracker.is_killed());

        tracker.kill("Test kill");
        assert!(tracker.is_killed());
        assert!(tracker.check_all().is_err());
    }

    #[test]
    fn test_resource_snapshot() {
        let tracker = ResourceTracker::new(ResourceLimits::default());
        tracker.alloc_memory(4096).unwrap();
        tracker.consume_fuel(1000).unwrap();

        let snap = tracker.snapshot();
        assert_eq!(snap.memory_used, 4096);
        assert_eq!(snap.fuel_consumed, 1000);
        assert!(!snap.killed);

        let display = format!("{}", snap);
        assert!(display.contains("mem=4KB"));
        assert!(display.contains("fuel=1000"));
    }
}
