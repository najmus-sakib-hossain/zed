//! JIT Integration for the Interpreter
//!
//! This module wires the interpreter to the JIT compiler, enabling
//! tiered compilation based on execution profiles.

use dx_py_jit::{CompilationTier, FunctionId, OsrManager, TieredJit};
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

/// Maps function names to FunctionIds for consistent ID assignment
struct FunctionIdMapper {
    /// Maps function names to their assigned FunctionId
    name_to_id: RwLock<HashMap<String, FunctionId>>,
    /// Counter for generating unique IDs
    next_id: AtomicU64,
}

impl FunctionIdMapper {
    /// Create a new FunctionIdMapper
    fn new() -> Self {
        Self {
            name_to_id: RwLock::new(HashMap::new()),
            next_id: AtomicU64::new(1), // Start from 1, 0 could be reserved
        }
    }

    /// Get or create a FunctionId for the given function name
    fn get_or_create(&self, name: &str) -> FunctionId {
        // Check existing first (read lock)
        {
            let map = self.name_to_id.read();
            if let Some(id) = map.get(name) {
                return *id;
            }
        }

        // Create new (write lock)
        let mut map = self.name_to_id.write();
        // Double-check after acquiring write lock
        if let Some(id) = map.get(name) {
            return *id;
        }

        let id = FunctionId(self.next_id.fetch_add(1, Ordering::Relaxed));
        map.insert(name.to_string(), id);
        id
    }

    /// Get the FunctionId for a name if it exists
    fn get(&self, name: &str) -> Option<FunctionId> {
        self.name_to_id.read().get(name).copied()
    }

    /// Clear all mappings
    fn clear(&self) {
        self.name_to_id.write().clear();
    }
}

/// JIT integration for the interpreter
pub struct JitIntegration {
    /// The tiered JIT compiler
    jit: Arc<TieredJit>,
    /// Local tier tracking (FunctionProfile doesn't store tier state)
    tiers: RwLock<HashMap<FunctionId, CompilationTier>>,
    /// Function name to ID mapping
    func_ids: FunctionIdMapper,
    /// OSR manager for on-stack replacement
    osr: Arc<OsrManager>,
    /// Whether JIT is enabled
    enabled: bool,
    /// Tier thresholds
    tier1_threshold: u64,
    tier2_threshold: u64,
    tier3_threshold: u64,
}

impl JitIntegration {
    /// Create a new JIT integration
    #[allow(clippy::arc_with_non_send_sync)]
    pub fn new() -> Self {
        Self {
            jit: Arc::new(TieredJit::new()),
            tiers: RwLock::new(HashMap::new()),
            func_ids: FunctionIdMapper::new(),
            osr: Arc::new(OsrManager::new()),
            enabled: true,
            tier1_threshold: 100,
            tier2_threshold: 1000,
            tier3_threshold: 10000,
        }
    }

    /// Create with custom thresholds
    #[allow(clippy::arc_with_non_send_sync)]
    pub fn with_thresholds(tier1: u64, tier2: u64, tier3: u64) -> Self {
        Self {
            jit: Arc::new(TieredJit::new()),
            tiers: RwLock::new(HashMap::new()),
            func_ids: FunctionIdMapper::new(),
            osr: Arc::new(OsrManager::new()),
            enabled: true,
            tier1_threshold: tier1,
            tier2_threshold: tier2,
            tier3_threshold: tier3,
        }
    }

    /// Enable or disable JIT
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// Check if JIT is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Record a function call and check for tier promotion
    pub fn record_call(&self, func_name: &str, bytecode: &[u8]) -> Option<CompilationTier> {
        if !self.enabled {
            return None;
        }

        let func_id = self.func_ids.get_or_create(func_name);
        let profile = self.jit.get_profile(func_id, bytecode.len(), 0);

        profile.record_call();
        let call_count = profile.get_call_count();

        // Get current tier from local tracking
        let current_tier =
            self.tiers.read().get(&func_id).copied().unwrap_or(CompilationTier::Interpreter);

        // Determine if promotion is needed
        let new_tier = if call_count >= self.tier3_threshold {
            CompilationTier::AotOptimized
        } else if call_count >= self.tier2_threshold {
            CompilationTier::OptimizingJit
        } else if call_count >= self.tier1_threshold {
            CompilationTier::BaselineJit
        } else {
            CompilationTier::Interpreter
        };

        if new_tier > current_tier {
            self.tiers.write().insert(func_id, new_tier);
            Some(new_tier)
        } else {
            None
        }
    }

    /// Get the current tier for a function
    pub fn get_tier(&self, func_name: &str) -> CompilationTier {
        if let Some(func_id) = self.func_ids.get(func_name) {
            self.tiers.read().get(&func_id).copied().unwrap_or(CompilationTier::Interpreter)
        } else {
            CompilationTier::Interpreter
        }
    }

    /// Get the call count for a function
    pub fn get_call_count(&self, func_name: &str) -> u64 {
        if let Some(func_id) = self.func_ids.get(func_name) {
            self.jit.get_profile(func_id, 0, 0).get_call_count()
        } else {
            0
        }
    }

    /// Request compilation at a specific tier
    pub fn compile(
        &self,
        func_name: &str,
        tier: CompilationTier,
        bytecode: &[u8],
    ) -> Result<(), JitError> {
        if !self.enabled {
            return Err(JitError::Disabled);
        }

        let func_id = self.func_ids.get_or_create(func_name);

        // compile returns Option<*const u8>, not Result
        match self.jit.compile(func_id, tier, bytecode) {
            Some(_ptr) => {
                // Update local tier tracking on successful compilation
                self.tiers.write().insert(func_id, tier);
                Ok(())
            }
            None => Err(JitError::CompilationFailed("Compilation returned None".to_string())),
        }
    }

    /// Check if a function has compiled code
    pub fn has_compiled(&self, func_name: &str) -> bool {
        if let Some(func_id) = self.func_ids.get(func_name) {
            self.jit.get_compiled(func_id).is_some()
        } else {
            false
        }
    }

    /// Trigger deoptimization for a function
    pub fn deoptimize(&self, func_name: &str) -> Result<(), JitError> {
        if let Some(func_id) = self.func_ids.get(func_name) {
            self.jit.invalidate(func_id);
            // Reset tier to Interpreter in local tracking
            self.tiers.write().insert(func_id, CompilationTier::Interpreter);
            // Also remove OSR entries for this function
            self.osr.remove_function(func_id);
            Ok(())
        } else {
            // Function not found, nothing to deoptimize
            Ok(())
        }
    }

    /// Check if OSR is available at the given bytecode offset
    pub fn can_osr(&self, func_name: &str, bytecode_offset: usize) -> bool {
        if let Some(func_id) = self.func_ids.get(func_name) {
            self.osr.get_entry(func_id, bytecode_offset).is_some()
        } else {
            false
        }
    }

    /// Perform OSR transition
    pub fn do_osr(&self, func_name: &str, bytecode_offset: usize) -> Result<(), JitError> {
        let func_id = self
            .func_ids
            .get(func_name)
            .ok_or_else(|| JitError::OsrFailed("Function not found".to_string()))?;

        match self.osr.get_entry(func_id, bytecode_offset) {
            Some(_entry) => {
                // OSR entry exists, transition is handled by the caller using the entry
                Ok(())
            }
            None => Err(JitError::OsrFailed("No OSR entry available".to_string())),
        }
    }

    /// Get JIT statistics
    pub fn stats(&self) -> JitStats {
        let tiers = self.tiers.read();
        let mut stats = JitStats::default();

        for (func_id, tier) in tiers.iter() {
            let profile = self.jit.get_profile(*func_id, 0, 0);
            stats.total_calls += profile.get_call_count();

            match tier {
                CompilationTier::Interpreter => stats.tier0_functions += 1,
                CompilationTier::BaselineJit => stats.tier1_functions += 1,
                CompilationTier::OptimizingJit => stats.tier2_functions += 1,
                CompilationTier::AotOptimized => stats.tier3_functions += 1,
            }
        }

        stats.total_functions = tiers.len();
        stats
    }

    /// Reset all profiles and tier tracking
    pub fn reset(&self) {
        self.tiers.write().clear();
        self.func_ids.clear();
    }
}

impl Default for JitIntegration {
    fn default() -> Self {
        Self::new()
    }
}

/// JIT integration errors
#[derive(Debug, thiserror::Error)]
pub enum JitError {
    #[error("JIT is disabled")]
    Disabled,

    #[error("Compilation failed: {0}")]
    CompilationFailed(String),

    #[error("Deoptimization failed: {0}")]
    DeoptFailed(String),

    #[error("OSR failed: {0}")]
    OsrFailed(String),
}

/// JIT statistics
#[derive(Debug, Default, Clone)]
pub struct JitStats {
    pub total_functions: usize,
    pub total_calls: u64,
    pub tier0_functions: usize,
    pub tier1_functions: usize,
    pub tier2_functions: usize,
    pub tier3_functions: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jit_integration_creation() {
        let jit = JitIntegration::new();
        assert!(jit.is_enabled());
    }

    #[test]
    fn test_tier_promotion() {
        let jit = JitIntegration::with_thresholds(10, 100, 1000);
        let bytecode = vec![0u8; 100]; // Dummy bytecode

        // Initial tier is Interpreter
        assert_eq!(jit.get_tier("test_func"), CompilationTier::Interpreter);

        // Record calls until BaselineJit
        for _ in 0..9 {
            assert!(jit.record_call("test_func", &bytecode).is_none());
        }

        // 10th call should trigger BaselineJit
        assert_eq!(jit.record_call("test_func", &bytecode), Some(CompilationTier::BaselineJit));
        assert_eq!(jit.get_tier("test_func"), CompilationTier::BaselineJit);
    }

    #[test]
    fn test_disabled_jit() {
        let mut jit = JitIntegration::new();
        jit.set_enabled(false);
        let bytecode = vec![0u8; 100];

        assert!(!jit.is_enabled());
        assert!(jit.record_call("test_func", &bytecode).is_none());
    }

    #[test]
    fn test_stats() {
        let jit = JitIntegration::with_thresholds(5, 50, 500);
        let bytecode = vec![0u8; 100];

        for _ in 0..10 {
            jit.record_call("func1", &bytecode);
        }
        for _ in 0..3 {
            jit.record_call("func2", &bytecode);
        }

        let stats = jit.stats();
        // Only func1 is in the tiers map (it was promoted to BaselineJit)
        // func2 never got promoted, so it's not tracked in tiers
        assert_eq!(stats.total_functions, 1);
        assert_eq!(stats.total_calls, 10); // Only func1's calls are counted
        assert_eq!(stats.tier1_functions, 1); // func1 at BaselineJit
        assert_eq!(stats.tier0_functions, 0); // No functions explicitly at Interpreter in tiers map
    }

    #[test]
    fn test_reset() {
        let jit = JitIntegration::new();
        let bytecode = vec![0u8; 100];
        jit.record_call("test_func", &bytecode);

        assert_eq!(jit.get_call_count("test_func"), 1);

        jit.reset();
        assert_eq!(jit.get_call_count("test_func"), 0);
    }
}

#[cfg(test)]
mod property_tests {
    use super::*;
    use proptest::prelude::*;

    // Feature: dx-py-runtime-compilation-fix, Property 1: Function ID Mapping Consistency
    // *For any* function name, calling `get_or_create` multiple times SHALL always return the same FunctionId.
    // **Validates: Requirements 5.2**
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_function_id_mapping_consistency(name in "[a-zA-Z_][a-zA-Z0-9_]{0,50}") {
            let mapper = FunctionIdMapper::new();

            // First call creates the ID
            let id1 = mapper.get_or_create(&name);

            // Subsequent calls should return the same ID
            let id2 = mapper.get_or_create(&name);
            let id3 = mapper.get_or_create(&name);

            prop_assert_eq!(id1, id2, "Second call returned different ID");
            prop_assert_eq!(id2, id3, "Third call returned different ID");

            // get() should also return the same ID
            let id4 = mapper.get(&name);
            prop_assert_eq!(Some(id1), id4, "get() returned different ID");
        }
    }

    // Feature: dx-py-runtime-compilation-fix, Property 2: Tier Tracking Consistency
    // *For any* function that has been promoted to a tier, calling `get_tier` SHALL return that tier until the function is deoptimized.
    // **Validates: Requirements 4.1, 4.2, 4.3**
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_tier_tracking_consistency(
            func_name in "[a-zA-Z_][a-zA-Z0-9_]{0,30}",
            call_count in 1u64..200u64
        ) {
            let jit = JitIntegration::with_thresholds(10, 100, 1000);
            let bytecode = vec![0u8; 100];

            // Record calls
            for _ in 0..call_count {
                jit.record_call(&func_name, &bytecode);
            }

            // Get the tier after recording
            let tier_after_calls = jit.get_tier(&func_name);

            // Verify tier is consistent with call count
            let expected_tier = if call_count >= 1000 {
                CompilationTier::AotOptimized
            } else if call_count >= 100 {
                CompilationTier::OptimizingJit
            } else if call_count >= 10 {
                CompilationTier::BaselineJit
            } else {
                CompilationTier::Interpreter
            };

            prop_assert_eq!(tier_after_calls, expected_tier,
                "Tier mismatch for {} calls: expected {:?}, got {:?}",
                call_count, expected_tier, tier_after_calls);

            // Multiple get_tier calls should return the same value
            let tier_again = jit.get_tier(&func_name);
            prop_assert_eq!(tier_after_calls, tier_again,
                "get_tier returned different values on consecutive calls");
        }
    }

    // Feature: dx-py-runtime-compilation-fix, Property 3: Tier Promotion Monotonicity
    // *For any* function, the tier SHALL only increase (never decrease) unless explicitly deoptimized via `deoptimize()`.
    // **Validates: Requirements 4.3**
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_tier_promotion_monotonicity(
            func_name in "[a-zA-Z_][a-zA-Z0-9_]{0,30}",
            call_counts in prop::collection::vec(1u64..50u64, 1..10)
        ) {
            let jit = JitIntegration::with_thresholds(10, 100, 1000);
            let bytecode = vec![0u8; 100];

            let mut previous_tier = CompilationTier::Interpreter;

            for batch_size in call_counts {
                // Record a batch of calls
                for _ in 0..batch_size {
                    jit.record_call(&func_name, &bytecode);
                }

                let current_tier = jit.get_tier(&func_name);

                // Tier should never decrease without deoptimization
                prop_assert!(
                    current_tier >= previous_tier,
                    "Tier decreased from {:?} to {:?} without deoptimization",
                    previous_tier, current_tier
                );

                previous_tier = current_tier;
            }
        }
    }
}
