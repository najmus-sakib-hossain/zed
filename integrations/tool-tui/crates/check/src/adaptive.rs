//! Adaptive Performance Optimizations
//!
//! Smart, context-aware optimizations based on workload characteristics:
//! - Adaptive parallelism (sequential for <10 files, parallel for larger)
//! - Adaptive caching (skip overhead for small files <1KB)
//! - Adaptive SIMD (use only for strings >64 bytes)
//! - Adaptive I/O (memory-mapped files on Windows for >10MB files)

use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};

/// Workload characteristics detector
pub struct WorkloadDetector {
    file_count: AtomicUsize,
    total_size: AtomicUsize,
    small_files: AtomicUsize,
    large_files: AtomicUsize,
}

impl WorkloadDetector {
    #[must_use]
    pub fn new() -> Self {
        Self {
            file_count: AtomicUsize::new(0),
            total_size: AtomicUsize::new(0),
            small_files: AtomicUsize::new(0),
            large_files: AtomicUsize::new(0),
        }
    }

    pub fn record_file(&self, size: usize) {
        self.file_count.fetch_add(1, Ordering::Relaxed);
        self.total_size.fetch_add(size, Ordering::Relaxed);

        if size < 1024 {
            self.small_files.fetch_add(1, Ordering::Relaxed);
        } else if size > 10 * 1024 * 1024 {
            self.large_files.fetch_add(1, Ordering::Relaxed);
        }
    }

    pub fn get_stats(&self) -> WorkloadStats {
        let file_count = self.file_count.load(Ordering::Relaxed);
        let total_size = self.total_size.load(Ordering::Relaxed);
        let small_files = self.small_files.load(Ordering::Relaxed);
        let large_files = self.large_files.load(Ordering::Relaxed);

        WorkloadStats {
            file_count,
            total_size,
            small_files,
            large_files,
            avg_file_size: if file_count > 0 {
                total_size / file_count
            } else {
                0
            },
        }
    }
}

impl Default for WorkloadDetector {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct WorkloadStats {
    pub file_count: usize,
    pub total_size: usize,
    pub small_files: usize,
    pub large_files: usize,
    pub avg_file_size: usize,
}

/// Adaptive parallelism strategy
pub struct AdaptiveParallel {
    threshold: usize,
}

impl AdaptiveParallel {
    #[must_use]
    pub fn new() -> Self {
        Self { threshold: 10 }
    }

    #[must_use]
    pub fn with_threshold(mut self, threshold: usize) -> Self {
        self.threshold = threshold;
        self
    }

    /// Decide whether to use parallel processing
    #[must_use]
    pub fn should_parallelize(&self, stats: &WorkloadStats) -> bool {
        stats.file_count >= self.threshold
    }

    /// Calculate optimal batch size based on workload
    #[must_use]
    pub fn optimal_batch_size(&self, stats: &WorkloadStats) -> usize {
        if stats.avg_file_size < 1024 {
            // Small files: larger batches
            500
        } else if stats.avg_file_size > 100_000 {
            // Large files: smaller batches
            10
        } else {
            // Medium files: default
            100
        }
    }
}

impl Default for AdaptiveParallel {
    fn default() -> Self {
        Self::new()
    }
}

/// Adaptive caching strategy
pub struct AdaptiveCache {
    small_file_threshold: usize,
    cache_warmth_threshold: usize,
}

impl AdaptiveCache {
    #[must_use]
    pub fn new() -> Self {
        Self {
            small_file_threshold: 1024,
            cache_warmth_threshold: 3,
        }
    }

    /// Decide whether to use cache for a file
    #[must_use]
    pub fn should_cache(&self, file_size: usize, access_count: usize) -> bool {
        // Skip cache overhead for very small files
        if file_size < self.small_file_threshold {
            return false;
        }

        // Always cache frequently accessed files
        if access_count >= self.cache_warmth_threshold {
            return true;
        }

        // Cache medium to large files
        file_size >= self.small_file_threshold
    }
}

impl Default for AdaptiveCache {
    fn default() -> Self {
        Self::new()
    }
}

/// Adaptive SIMD strategy
pub struct AdaptiveSIMD {
    string_threshold: usize,
    pattern_complexity_threshold: usize,
}

impl AdaptiveSIMD {
    #[must_use]
    pub fn new() -> Self {
        Self {
            string_threshold: 64,
            pattern_complexity_threshold: 10,
        }
    }

    /// Decide whether to use SIMD for string scanning
    #[must_use]
    pub fn should_use_simd(&self, string_len: usize, pattern_complexity: usize) -> bool {
        // Use SIMD only for strings >64 bytes
        if string_len < self.string_threshold {
            return false;
        }

        // Use SIMD for complex patterns
        if pattern_complexity >= self.pattern_complexity_threshold {
            return true;
        }

        // Use SIMD for long strings with any pattern
        string_len >= self.string_threshold * 2
    }
}

impl Default for AdaptiveSIMD {
    fn default() -> Self {
        Self::new()
    }
}

/// Adaptive I/O strategy
pub struct AdaptiveIO {
    mmap_threshold: usize,
}

impl AdaptiveIO {
    #[must_use]
    pub fn new() -> Self {
        Self {
            mmap_threshold: 10 * 1024 * 1024, // 10MB
        }
    }

    /// Decide whether to use memory-mapped I/O
    #[must_use]
    pub fn should_use_mmap(&self, file_size: usize) -> bool {
        // Use memory-mapped files on Windows for large files (>10MB)
        #[cfg(target_os = "windows")]
        {
            file_size > self.mmap_threshold
        }

        // On other platforms, use standard I/O
        #[cfg(not(target_os = "windows"))]
        {
            let _ = file_size;
            false
        }
    }

    /// Read file with adaptive I/O strategy
    pub fn read_file(&self, path: &Path) -> std::io::Result<Vec<u8>> {
        let metadata = std::fs::metadata(path)?;
        let file_size = metadata.len() as usize;

        if self.should_use_mmap(file_size) {
            self.read_mmap(path)
        } else {
            std::fs::read(path)
        }
    }

    #[cfg(target_os = "windows")]
    fn read_mmap(&self, path: &Path) -> std::io::Result<Vec<u8>> {
        use memmap2::Mmap;
        let file = std::fs::File::open(path)?;
        // SAFETY: We're only reading the file, not modifying it
        let mmap = unsafe { Mmap::map(&file)? };
        Ok(mmap.to_vec())
    }

    #[cfg(not(target_os = "windows"))]
    fn read_mmap(&self, path: &Path) -> std::io::Result<Vec<u8>> {
        std::fs::read(path)
    }
}

impl Default for AdaptiveIO {
    fn default() -> Self {
        Self::new()
    }
}

/// Adaptive memory management
pub struct AdaptiveMemory {
    pressure_threshold: f64,
    batch_size_min: usize,
    batch_size_max: usize,
}

impl AdaptiveMemory {
    #[must_use]
    pub fn new() -> Self {
        Self {
            pressure_threshold: 0.8,
            batch_size_min: 10,
            batch_size_max: 1000,
        }
    }

    /// Get current memory pressure (0.0 to 1.0)
    #[must_use]
    pub fn memory_pressure(&self) -> f64 {
        // Simplified: would use actual system memory stats in production
        0.5
    }

    /// Adjust batch size based on memory pressure
    #[must_use]
    pub fn adaptive_batch_size(&self, default_size: usize) -> usize {
        let pressure = self.memory_pressure();

        if pressure > self.pressure_threshold {
            // High memory pressure: reduce batch size
            self.batch_size_min.max(default_size / 4)
        } else if pressure < 0.5 {
            // Low memory pressure: increase batch size
            self.batch_size_max.min(default_size * 2)
        } else {
            // Normal pressure: use default
            default_size
        }
    }

    /// Check if we should switch to streaming mode
    #[must_use]
    pub fn should_stream(&self) -> bool {
        self.memory_pressure() > self.pressure_threshold
    }
}

impl Default for AdaptiveMemory {
    fn default() -> Self {
        Self::new()
    }
}

/// Complete adaptive optimization strategy
pub struct AdaptiveStrategy {
    pub parallel: AdaptiveParallel,
    pub cache: AdaptiveCache,
    pub simd: AdaptiveSIMD,
    pub io: AdaptiveIO,
    pub memory: AdaptiveMemory,
}

impl AdaptiveStrategy {
    #[must_use]
    pub fn new() -> Self {
        Self {
            parallel: AdaptiveParallel::new(),
            cache: AdaptiveCache::new(),
            simd: AdaptiveSIMD::new(),
            io: AdaptiveIO::new(),
            memory: AdaptiveMemory::new(),
        }
    }

    /// Analyze workload and return optimization recommendations
    #[must_use]
    pub fn analyze(&self, stats: &WorkloadStats) -> OptimizationPlan {
        OptimizationPlan {
            use_parallel: self.parallel.should_parallelize(stats),
            batch_size: self.memory.adaptive_batch_size(self.parallel.optimal_batch_size(stats)),
            use_mmap: stats.large_files > 0,
            use_streaming: self.memory.should_stream(),
        }
    }
}

impl Default for AdaptiveStrategy {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct OptimizationPlan {
    pub use_parallel: bool,
    pub batch_size: usize,
    pub use_mmap: bool,
    pub use_streaming: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workload_detection() {
        let detector = WorkloadDetector::new();
        detector.record_file(500);
        detector.record_file(2000);
        detector.record_file(15_000_000);

        let stats = detector.get_stats();
        assert_eq!(stats.file_count, 3);
        assert_eq!(stats.small_files, 1);
        assert_eq!(stats.large_files, 1);
    }

    #[test]
    fn test_adaptive_parallel_threshold() {
        let strategy = AdaptiveParallel::new();

        let small_workload = WorkloadStats {
            file_count: 5,
            total_size: 5000,
            small_files: 5,
            large_files: 0,
            avg_file_size: 1000,
        };
        assert!(!strategy.should_parallelize(&small_workload));

        let large_workload = WorkloadStats {
            file_count: 100,
            total_size: 100_000,
            small_files: 50,
            large_files: 10,
            avg_file_size: 1000,
        };
        assert!(strategy.should_parallelize(&large_workload));
    }

    #[test]
    fn test_adaptive_cache() {
        let strategy = AdaptiveCache::new();

        // Small file: don't cache
        assert!(!strategy.should_cache(500, 1));

        // Large file: cache
        assert!(strategy.should_cache(10_000, 1));

        // Small but frequently accessed: cache
        assert!(strategy.should_cache(500, 5));
    }

    #[test]
    fn test_adaptive_simd() {
        let strategy = AdaptiveSIMD::new();

        // Short string: don't use SIMD
        assert!(!strategy.should_use_simd(32, 5));

        // Long string: use SIMD
        assert!(strategy.should_use_simd(200, 5));

        // Medium string with complex pattern: use SIMD
        assert!(strategy.should_use_simd(80, 15));
    }

    #[test]
    fn test_adaptive_io_threshold() {
        let strategy = AdaptiveIO::new();

        // Small file: standard I/O
        assert!(!strategy.should_use_mmap(1_000_000));

        #[cfg(target_os = "windows")]
        {
            // Large file on Windows: use mmap
            assert!(strategy.should_use_mmap(20_000_000));
        }
    }

    #[test]
    fn test_optimization_plan() {
        let strategy = AdaptiveStrategy::new();

        let stats = WorkloadStats {
            file_count: 50,
            total_size: 500_000,
            small_files: 30,
            large_files: 5,
            avg_file_size: 10_000,
        };

        let plan = strategy.analyze(&stats);
        assert!(plan.use_parallel);
        assert!(plan.batch_size > 0);
    }
}
