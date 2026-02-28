//! Compilation tier definitions

/// Compilation tiers for the JIT
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
#[derive(Default)]
pub enum CompilationTier {
    /// Tier 0: Interpreter with profiling (all code starts here)
    #[default]
    Interpreter = 0,
    /// Tier 1: Baseline JIT - fast compile, moderate speedup (100 calls)
    BaselineJit = 1,
    /// Tier 2: Optimizing JIT - type-specialized (1000 calls)
    OptimizingJit = 2,
    /// Tier 3: AOT with PGO - persistent across runs (10000 calls)
    AotOptimized = 3,
}

impl CompilationTier {
    /// Get the call count threshold for promotion to this tier
    pub fn threshold(&self) -> u64 {
        match self {
            Self::Interpreter => 0,
            Self::BaselineJit => 100,
            Self::OptimizingJit => 1000,
            Self::AotOptimized => 10000,
        }
    }

    /// Get the next tier (if any)
    pub fn next(&self) -> Option<Self> {
        match self {
            Self::Interpreter => Some(Self::BaselineJit),
            Self::BaselineJit => Some(Self::OptimizingJit),
            Self::OptimizingJit => Some(Self::AotOptimized),
            Self::AotOptimized => None,
        }
    }

    /// Get the previous tier (if any)
    pub fn previous(&self) -> Option<Self> {
        match self {
            Self::Interpreter => None,
            Self::BaselineJit => Some(Self::Interpreter),
            Self::OptimizingJit => Some(Self::BaselineJit),
            Self::AotOptimized => Some(Self::OptimizingJit),
        }
    }

    /// Check if this tier uses JIT compilation
    pub fn is_jit(&self) -> bool {
        matches!(self, Self::BaselineJit | Self::OptimizingJit | Self::AotOptimized)
    }

    /// Get a human-readable name
    pub fn name(&self) -> &'static str {
        match self {
            Self::Interpreter => "Interpreter",
            Self::BaselineJit => "Baseline JIT",
            Self::OptimizingJit => "Optimizing JIT",
            Self::AotOptimized => "AOT Optimized",
        }
    }
}

impl std::fmt::Display for CompilationTier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tier_thresholds() {
        assert_eq!(CompilationTier::Interpreter.threshold(), 0);
        assert_eq!(CompilationTier::BaselineJit.threshold(), 100);
        assert_eq!(CompilationTier::OptimizingJit.threshold(), 1000);
        assert_eq!(CompilationTier::AotOptimized.threshold(), 10000);
    }

    #[test]
    fn test_tier_progression() {
        let mut tier = CompilationTier::Interpreter;

        tier = tier.next().unwrap();
        assert_eq!(tier, CompilationTier::BaselineJit);

        tier = tier.next().unwrap();
        assert_eq!(tier, CompilationTier::OptimizingJit);

        tier = tier.next().unwrap();
        assert_eq!(tier, CompilationTier::AotOptimized);

        assert!(tier.next().is_none());
    }

    #[test]
    fn test_tier_ordering() {
        assert!(CompilationTier::Interpreter < CompilationTier::BaselineJit);
        assert!(CompilationTier::BaselineJit < CompilationTier::OptimizingJit);
        assert!(CompilationTier::OptimizingJit < CompilationTier::AotOptimized);
    }
}
