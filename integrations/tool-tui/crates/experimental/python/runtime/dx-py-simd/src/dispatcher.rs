//! SIMD Dispatcher - Runtime CPU detection and engine selection

use crate::engine::SimdStringEngine;
use crate::scalar::ScalarStringEngine;

#[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
use crate::avx2::Avx2StringEngine;

#[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
use crate::avx512::Avx512StringEngine;

#[cfg(target_arch = "aarch64")]
use crate::neon::NeonStringEngine;

/// Runtime SIMD detection and dispatch
pub struct SimdDispatcher {
    has_avx2: bool,
    has_avx512: bool,
    has_neon: bool,
}

impl SimdDispatcher {
    /// Create a new dispatcher with runtime CPU detection
    pub fn new() -> Self {
        Self {
            has_avx2: Self::detect_avx2(),
            has_avx512: Self::detect_avx512(),
            has_neon: Self::detect_neon(),
        }
    }

    /// Get the best available SIMD engine for the current CPU
    pub fn get_engine(&self) -> Box<dyn SimdStringEngine> {
        // Prefer AVX-512 > AVX2 > NEON > Scalar
        #[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
        {
            if self.has_avx512 {
                return Box::new(unsafe { Avx512StringEngine::new() });
            }

            if self.has_avx2 {
                return Box::new(unsafe { Avx2StringEngine::new() });
            }
        }

        #[cfg(target_arch = "aarch64")]
        {
            if self.has_neon {
                return Box::new(NeonStringEngine::new());
            }
        }

        // Use NEON on ARM64 even if not detected (it's always available)
        #[cfg(target_arch = "aarch64")]
        {
            return Box::new(NeonStringEngine::new());
        }

        #[cfg(not(target_arch = "aarch64"))]
        Box::new(ScalarStringEngine::new())
    }

    /// Check if AVX2 is available
    pub fn has_avx2(&self) -> bool {
        self.has_avx2
    }

    /// Check if AVX-512 is available
    pub fn has_avx512(&self) -> bool {
        self.has_avx512
    }

    /// Check if NEON is available
    pub fn has_neon(&self) -> bool {
        self.has_neon
    }

    /// Get the name of the best available engine
    pub fn best_engine_name(&self) -> &'static str {
        if self.has_avx512 {
            "AVX-512"
        } else if self.has_avx2 {
            "AVX2"
        } else if self.has_neon {
            "NEON"
        } else {
            "Scalar"
        }
    }

    #[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
    fn detect_avx2() -> bool {
        is_x86_feature_detected!("avx2")
    }

    #[cfg(not(any(target_arch = "x86_64", target_arch = "x86")))]
    fn detect_avx2() -> bool {
        false
    }

    #[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
    fn detect_avx512() -> bool {
        is_x86_feature_detected!("avx512f")
    }

    #[cfg(not(any(target_arch = "x86_64", target_arch = "x86")))]
    fn detect_avx512() -> bool {
        false
    }

    #[cfg(target_arch = "aarch64")]
    fn detect_neon() -> bool {
        // NEON is always available on AArch64
        true
    }

    #[cfg(not(target_arch = "aarch64"))]
    fn detect_neon() -> bool {
        false
    }
}

impl Default for SimdDispatcher {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dispatcher_creation() {
        let dispatcher = SimdDispatcher::new();

        // Should always be able to get an engine
        let engine = dispatcher.get_engine();
        assert!(!engine.name().is_empty());
    }

    #[test]
    fn test_engine_consistency() {
        let dispatcher = SimdDispatcher::new();
        let engine = dispatcher.get_engine();

        // All engines should produce the same results
        assert_eq!(engine.find("hello world", "world"), Some(6));
        assert_eq!(engine.count("aaa", "a"), 3);
        assert!(engine.eq("hello", "hello"));
        assert_eq!(engine.to_lowercase("HELLO"), "hello");
        assert_eq!(engine.to_uppercase("hello"), "HELLO");
    }

    #[test]
    fn test_best_engine_name() {
        let dispatcher = SimdDispatcher::new();
        let name = dispatcher.best_engine_name();

        // Should be one of the known engine names
        assert!(
            name == "AVX-512" || name == "AVX2" || name == "NEON" || name == "Scalar",
            "Unknown engine name: {}",
            name
        );
    }
}
