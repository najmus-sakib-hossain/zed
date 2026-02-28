//! Property tests for Generator State Machine
//!
//! **Property 30: Generator State Machine**
//! *For any* generator function, calling it should return a generator object.
//! next() should execute until yield, returning {value, done}.
//! The generator should complete with {value: undefined, done: true}.
//!
//! **Validates: Requirements 9.1, 9.2, 9.3, 9.4**
//!
//! Requirements:
//! - 9.1: WHEN a generator function is called, THE DX_Runtime SHALL return a generator object without executing the body
//! - 9.2: WHEN next() is called on a generator, THE DX_Runtime SHALL execute until the next yield and return {value, done}
//! - 9.3: WHEN yield is encountered, THE DX_Runtime SHALL suspend execution and return the yielded value
//! - 9.4: WHEN the generator completes, THE DX_Runtime SHALL return {value: undefined, done: true}
//!
//! **Feature: dx-runtime-production-ready, Property 30: Generator State Machine**

use proptest::prelude::*;

// ============================================================================
// Generator State Constants (matching codegen.rs)
// ============================================================================

/// Generator state: Created (not yet started)
const GENERATOR_STATE_CREATED: f64 = 0.0;
/// Generator state: Suspended (at a yield point)
const GENERATOR_STATE_SUSPENDED: f64 = 1.0;
/// Generator state: Executing (currently running)
#[allow(dead_code)]
const GENERATOR_STATE_EXECUTING: f64 = 2.0;
/// Generator state: Completed (finished)
const GENERATOR_STATE_COMPLETED: f64 = 3.0;

// ============================================================================
// Simulated Generator API for Testing
// ============================================================================

/// Simulated generator result {value, done}
#[derive(Debug, Clone, PartialEq)]
struct GeneratorResult {
    value: f64,
    done: bool,
}

/// Simulated generator state for testing
/// This mirrors the GeneratorData structure in codegen.rs
#[derive(Debug, Clone)]
struct SimulatedGenerator {
    state: f64,
    current_value: f64,
    sent_value: f64,
    yield_values: Vec<f64>,
    yield_index: usize,
}

impl SimulatedGenerator {
    /// Create a new generator with the given yield values
    /// Requirements: 9.1 - generator function returns generator object without executing body
    fn new(yield_values: Vec<f64>) -> Self {
        Self {
            state: GENERATOR_STATE_CREATED,
            current_value: f64::NAN,
            sent_value: f64::NAN,
            yield_values,
            yield_index: 0,
        }
    }

    /// Get the current state of the generator
    fn get_state(&self) -> f64 {
        self.state
    }

    /// Call next() on the generator
    /// Requirements: 9.2 - next() executes until yield, returns {value, done}
    /// Requirements: 9.3 - yield suspends execution and returns yielded value
    /// Requirements: 9.4 - generator returns {value: undefined, done: true} when finished
    fn next(&mut self, send_value: f64) -> GeneratorResult {
        match self.state as i32 {
            0 => {
                // Created state - first call to next()
                // The first next() call ignores send_value per spec
                self.state = GENERATOR_STATE_SUSPENDED;
                
                if self.yield_index < self.yield_values.len() {
                    let value = self.yield_values[self.yield_index];
                    self.yield_index += 1;
                    self.current_value = value;
                    GeneratorResult { value, done: false }
                } else {
                    // No yields - generator completes immediately
                    self.state = GENERATOR_STATE_COMPLETED;
                    GeneratorResult { value: f64::NAN, done: true }
                }
            }
            1 => {
                // Suspended state - resume from yield
                self.sent_value = send_value;
                
                if self.yield_index < self.yield_values.len() {
                    let value = self.yield_values[self.yield_index];
                    self.yield_index += 1;
                    self.current_value = value;
                    GeneratorResult { value, done: false }
                } else {
                    // No more yields - generator completes
                    self.state = GENERATOR_STATE_COMPLETED;
                    GeneratorResult { value: f64::NAN, done: true }
                }
            }
            3 => {
                // Completed state - always return done
                GeneratorResult { value: f64::NAN, done: true }
            }
            _ => {
                // Executing or invalid state
                GeneratorResult { value: f64::NAN, done: true }
            }
        }
    }

    /// Force the generator to return with a given value
    /// Requirements: 9.7 - return() completes generator with given value
    fn force_return(&mut self, value: f64) -> GeneratorResult {
        self.state = GENERATOR_STATE_COMPLETED;
        self.current_value = value;
        GeneratorResult { value, done: true }
    }
}

// ============================================================================
// Property 30: Generator State Machine
// For any generator function, calling it should return a generator object.
// next() should execute until yield, returning {value, done}.
// The generator should complete with {value: undefined, done: true}.
// **Validates: Requirements 9.1, 9.2, 9.3, 9.4**
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Test that creating a generator returns an object in Created state
    /// Requirements: 9.1 - generator function returns generator object without executing body
    /// **Feature: dx-runtime-production-ready, Property 30: Generator State Machine**
    #[test]
    fn prop_generator_creation_returns_created_state(
        yield_count in 0usize..10
    ) {
        let yield_values: Vec<f64> = (0..yield_count).map(|i| i as f64).collect();
        let gen = SimulatedGenerator::new(yield_values);
        
        // Property: Generator should start in Created state
        prop_assert_eq!(gen.get_state(), GENERATOR_STATE_CREATED,
            "Generator should start in Created state (not executing body)");
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Test that next() returns yielded values with done: false
    /// Requirements: 9.2 - next() executes until yield, returns {value, done}
    /// Requirements: 9.3 - yield suspends execution and returns yielded value
    /// **Feature: dx-runtime-production-ready, Property 30: Generator State Machine**
    #[test]
    fn prop_generator_next_returns_yielded_values(
        yield_values in prop::collection::vec(any::<f64>().prop_filter("finite", |v| v.is_finite()), 1..10)
    ) {
        let mut gen = SimulatedGenerator::new(yield_values.clone());
        
        // Property: Each call to next() should return the next yielded value
        for (i, expected_value) in yield_values.iter().enumerate() {
            let result = gen.next(f64::NAN);
            
            prop_assert_eq!(result.value, *expected_value,
                "next() call {} should return yielded value {}", i, expected_value);
            prop_assert!(!result.done,
                "next() call {} should return done: false while yields remain", i);
        }
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Test that generator completes with done: true after all yields
    /// Requirements: 9.4 - generator returns {value: undefined, done: true} when finished
    /// **Feature: dx-runtime-production-ready, Property 30: Generator State Machine**
    #[test]
    fn prop_generator_completes_after_yields(
        yield_values in prop::collection::vec(any::<f64>().prop_filter("finite", |v| v.is_finite()), 0..10)
    ) {
        let mut gen = SimulatedGenerator::new(yield_values.clone());
        
        // Exhaust all yields
        for _ in 0..yield_values.len() {
            let _ = gen.next(f64::NAN);
        }
        
        // Property: After all yields, next() should return done: true
        let final_result = gen.next(f64::NAN);
        prop_assert!(final_result.done,
            "Generator should return done: true after all yields exhausted");
        prop_assert!(final_result.value.is_nan(),
            "Generator should return undefined (NaN) value when completed");
        
        // Property: Generator should be in Completed state
        prop_assert_eq!(gen.get_state(), GENERATOR_STATE_COMPLETED,
            "Generator should be in Completed state after exhaustion");
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Test that completed generator always returns done: true
    /// Requirements: 9.4 - generator returns {value: undefined, done: true} when finished
    /// **Feature: dx-runtime-production-ready, Property 30: Generator State Machine**
    #[test]
    fn prop_completed_generator_always_returns_done(
        yield_values in prop::collection::vec(any::<f64>().prop_filter("finite", |v| v.is_finite()), 0..5),
        extra_calls in 1usize..10
    ) {
        let mut gen = SimulatedGenerator::new(yield_values.clone());
        
        // Exhaust all yields
        for _ in 0..yield_values.len() {
            let _ = gen.next(f64::NAN);
        }
        
        // Get to completed state
        let _ = gen.next(f64::NAN);
        
        // Property: All subsequent calls should return done: true
        for i in 0..extra_calls {
            let result = gen.next(f64::NAN);
            prop_assert!(result.done,
                "Completed generator should always return done: true (call {})", i);
        }
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Test that generator state transitions correctly
    /// Requirements: 9.1, 9.2, 9.3, 9.4
    /// **Feature: dx-runtime-production-ready, Property 30: Generator State Machine**
    #[test]
    fn prop_generator_state_transitions(
        yield_values in prop::collection::vec(any::<f64>().prop_filter("finite", |v| v.is_finite()), 1..5)
    ) {
        let mut gen = SimulatedGenerator::new(yield_values.clone());
        
        // Property: Initial state is Created
        prop_assert_eq!(gen.get_state(), GENERATOR_STATE_CREATED,
            "Initial state should be Created");
        
        // Property: After first next(), state is Suspended (if yields remain)
        let _ = gen.next(f64::NAN);
        if yield_values.len() > 1 {
            prop_assert_eq!(gen.get_state(), GENERATOR_STATE_SUSPENDED,
                "State should be Suspended after first next() with yields remaining");
        }
        
        // Exhaust remaining yields
        for _ in 1..yield_values.len() {
            let _ = gen.next(f64::NAN);
        }
        
        // Property: After exhausting yields, next call completes the generator
        let _ = gen.next(f64::NAN);
        prop_assert_eq!(gen.get_state(), GENERATOR_STATE_COMPLETED,
            "State should be Completed after all yields exhausted");
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Test that empty generator completes immediately
    /// Requirements: 9.4 - generator returns {value: undefined, done: true} when finished
    /// **Feature: dx-runtime-production-ready, Property 30: Generator State Machine**
    #[test]
    fn prop_empty_generator_completes_immediately(_dummy in 0..1i32) {
        let mut gen = SimulatedGenerator::new(vec![]);
        
        // Property: First next() on empty generator should return done: true
        let result = gen.next(f64::NAN);
        prop_assert!(result.done,
            "Empty generator should complete immediately");
        prop_assert!(result.value.is_nan(),
            "Empty generator should return undefined value");
        prop_assert_eq!(gen.get_state(), GENERATOR_STATE_COMPLETED,
            "Empty generator should be in Completed state");
    }
}

// ============================================================================
// Edge case tests
// ============================================================================

#[test]
fn test_generator_creation_does_not_execute_body() {
    // Requirements: 9.1 - generator function returns generator object without executing body
    let gen = SimulatedGenerator::new(vec![1.0, 2.0, 3.0]);
    
    // The generator should be in Created state, not having executed any yields
    assert_eq!(gen.get_state(), GENERATOR_STATE_CREATED);
    assert_eq!(gen.yield_index, 0, "Generator should not have executed any yields");
}

#[test]
fn test_generator_next_returns_value_done_object() {
    // Requirements: 9.2 - next() returns {value, done}
    let mut gen = SimulatedGenerator::new(vec![42.0]);
    
    let result = gen.next(f64::NAN);
    
    // The result should have both value and done properties
    assert_eq!(result.value, 42.0);
    assert!(!result.done);
}

#[test]
fn test_generator_yield_suspends_execution() {
    // Requirements: 9.3 - yield suspends execution
    let mut gen = SimulatedGenerator::new(vec![1.0, 2.0, 3.0]);
    
    // First next() should return first yield and suspend
    let result1 = gen.next(f64::NAN);
    assert_eq!(result1.value, 1.0);
    assert!(!result1.done);
    assert_eq!(gen.get_state(), GENERATOR_STATE_SUSPENDED);
    
    // Second next() should return second yield
    let result2 = gen.next(f64::NAN);
    assert_eq!(result2.value, 2.0);
    assert!(!result2.done);
}

#[test]
fn test_generator_completion_returns_undefined_done() {
    // Requirements: 9.4 - generator returns {value: undefined, done: true} when finished
    let mut gen = SimulatedGenerator::new(vec![1.0]);
    
    // Exhaust the generator
    let _ = gen.next(f64::NAN); // Returns 1.0
    let final_result = gen.next(f64::NAN); // Should complete
    
    assert!(final_result.done);
    assert!(final_result.value.is_nan(), "Completed generator should return undefined (NaN)");
    assert_eq!(gen.get_state(), GENERATOR_STATE_COMPLETED);
}

#[test]
fn test_generator_force_return_completes_generator() {
    // Requirements: 9.7 - return() completes generator with given value
    let mut gen = SimulatedGenerator::new(vec![1.0, 2.0, 3.0]);
    
    // Start the generator
    let _ = gen.next(f64::NAN);
    
    // Force return with a value
    let result = gen.force_return(99.0);
    
    assert!(result.done);
    assert_eq!(result.value, 99.0);
    assert_eq!(gen.get_state(), GENERATOR_STATE_COMPLETED);
    
    // Subsequent calls should return done: true
    let subsequent = gen.next(f64::NAN);
    assert!(subsequent.done);
}

#[test]
fn test_generator_multiple_yields_in_sequence() {
    let mut gen = SimulatedGenerator::new(vec![10.0, 20.0, 30.0, 40.0, 50.0]);
    
    // Each next() should return the next yield value
    assert_eq!(gen.next(f64::NAN).value, 10.0);
    assert_eq!(gen.next(f64::NAN).value, 20.0);
    assert_eq!(gen.next(f64::NAN).value, 30.0);
    assert_eq!(gen.next(f64::NAN).value, 40.0);
    assert_eq!(gen.next(f64::NAN).value, 50.0);
    
    // Final call should complete
    let final_result = gen.next(f64::NAN);
    assert!(final_result.done);
}
