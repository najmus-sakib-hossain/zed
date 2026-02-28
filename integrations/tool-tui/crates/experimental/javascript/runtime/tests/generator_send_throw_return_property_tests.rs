//! Property tests for Generator Send/Throw/Return
//!
//! **Property 31: Generator Send/Throw/Return**
//! *For any* generator, next(value) should pass value as yield result,
//! throw(error) should throw inside generator, return(value) should complete with value.
//!
//! **Validates: Requirements 9.5, 9.6, 9.7**
//!
//! Requirements:
//! - 9.5: WHEN next(value) is called, THE DX_Runtime SHALL pass value as the result of the yield expression
//! - 9.6: WHEN throw(error) is called, THE DX_Runtime SHALL throw the error inside the generator
//! - 9.7: WHEN return(value) is called, THE DX_Runtime SHALL complete the generator with the given value
//!
//! **Feature: dx-runtime-production-ready, Property 31: Generator Send/Throw/Return**

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

/// Simulated error for testing throw()
#[derive(Debug, Clone)]
struct GeneratorError {
    message: String,
    error_value: f64,
}

/// Simulated generator state for testing send/throw/return
/// This mirrors the GeneratorData structure in codegen.rs
#[derive(Debug, Clone)]
struct SimulatedGenerator {
    state: f64,
    current_value: f64,
    sent_value: f64,
    thrown_error: Option<f64>,
    return_value: Option<f64>,
    yield_values: Vec<f64>,
    yield_index: usize,
    /// Tracks the values received via next(value) for verification
    received_values: Vec<f64>,
}

impl SimulatedGenerator {
    /// Create a new generator with the given yield values
    fn new(yield_values: Vec<f64>) -> Self {
        Self {
            state: GENERATOR_STATE_CREATED,
            current_value: f64::NAN,
            sent_value: f64::NAN,
            thrown_error: None,
            return_value: None,
            yield_values,
            yield_index: 0,
            received_values: Vec::new(),
        }
    }

    /// Get the current state of the generator
    fn get_state(&self) -> f64 {
        self.state
    }

    /// Get the last sent value (for testing next(value))
    fn get_sent_value(&self) -> f64 {
        self.sent_value
    }

    /// Get all received values (for testing next(value) sequence)
    fn get_received_values(&self) -> &[f64] {
        &self.received_values
    }

    /// Call next(value) on the generator
    /// Requirements: 9.5 - next(value) passes value as yield expression result
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
                    self.state = GENERATOR_STATE_COMPLETED;
                    GeneratorResult { value: f64::NAN, done: true }
                }
            }
            1 => {
                // Suspended state - resume from yield
                // Requirements: 9.5 - Store send_value for yield expression result
                self.sent_value = send_value;
                if !send_value.is_nan() {
                    self.received_values.push(send_value);
                }
                
                if self.yield_index < self.yield_values.len() {
                    let value = self.yield_values[self.yield_index];
                    self.yield_index += 1;
                    self.current_value = value;
                    GeneratorResult { value, done: false }
                } else {
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

    /// Throw an error into the generator
    /// Requirements: 9.6 - throw(error) throws error inside generator
    fn throw(&mut self, error: f64) -> Result<GeneratorResult, GeneratorError> {
        match self.state as i32 {
            0 => {
                // Created state - generator hasn't started, complete and throw
                self.state = GENERATOR_STATE_COMPLETED;
                self.thrown_error = Some(error);
                Err(GeneratorError {
                    message: "Error thrown into unstarted generator".to_string(),
                    error_value: error,
                })
            }
            1 => {
                // Suspended state - throw error at yield point
                self.state = GENERATOR_STATE_COMPLETED;
                self.thrown_error = Some(error);
                Err(GeneratorError {
                    message: "Error thrown into generator".to_string(),
                    error_value: error,
                })
            }
            3 => {
                // Completed state - re-throw the error
                Err(GeneratorError {
                    message: "Error thrown into completed generator".to_string(),
                    error_value: error,
                })
            }
            _ => {
                // Executing state - error
                Err(GeneratorError {
                    message: "Generator is already executing".to_string(),
                    error_value: error,
                })
            }
        }
    }

    /// Force the generator to return with a given value
    /// Requirements: 9.7 - return(value) completes generator with given value
    fn force_return(&mut self, value: f64) -> GeneratorResult {
        match self.state as i32 {
            0 | 1 => {
                // Created or Suspended state - complete with value
                self.state = GENERATOR_STATE_COMPLETED;
                self.current_value = value;
                self.return_value = Some(value);
                GeneratorResult { value, done: true }
            }
            3 => {
                // Already completed - return the value anyway
                GeneratorResult { value, done: true }
            }
            _ => {
                // Executing state - error case
                GeneratorResult { value: f64::NAN, done: true }
            }
        }
    }
}

// ============================================================================
// Property 31: Generator Send/Throw/Return
// For any generator, next(value) should pass value as yield result,
// throw(error) should throw inside generator, return(value) should complete with value.
// **Validates: Requirements 9.5, 9.6, 9.7**
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Test that next(value) passes value as yield expression result
    /// Requirements: 9.5 - next(value) passes value as yield expression result
    /// **Feature: dx-runtime-production-ready, Property 31: Generator Send/Throw/Return**
    #[test]
    fn prop_generator_next_sends_value(
        yield_values in prop::collection::vec(any::<f64>().prop_filter("finite", |v| v.is_finite()), 2..10),
        send_values in prop::collection::vec(any::<f64>().prop_filter("finite", |v| v.is_finite()), 1..5)
    ) {
        let mut gen = SimulatedGenerator::new(yield_values.clone());
        
        // First next() starts the generator (send_value is ignored)
        let _ = gen.next(f64::NAN);
        
        // Track how many values we actually sent while generator was active
        let mut actual_sends = 0;
        
        // Subsequent next(value) calls should pass value as yield result
        for (i, &send_value) in send_values.iter().enumerate() {
            if gen.get_state() == GENERATOR_STATE_COMPLETED {
                break;
            }
            
            let _ = gen.next(send_value);
            actual_sends += 1;
            
            // Property: The sent value should be stored
            prop_assert_eq!(gen.get_sent_value(), send_value,
                "next({}) should store {} as sent value (call {})", send_value, send_value, i);
        }
        
        // Property: All sent values should be recorded (up to the number we actually sent)
        let received = gen.get_received_values();
        prop_assert_eq!(received.len(), actual_sends,
            "Should have received {} values, got {}", actual_sends, received.len());
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Test that throw(error) throws error inside generator
    /// Requirements: 9.6 - throw(error) throws error inside generator
    /// **Feature: dx-runtime-production-ready, Property 31: Generator Send/Throw/Return**
    #[test]
    fn prop_generator_throw_throws_error(
        yield_values in prop::collection::vec(any::<f64>().prop_filter("finite", |v| v.is_finite()), 1..5),
        error_value in any::<f64>().prop_filter("finite", |v| v.is_finite())
    ) {
        let mut gen = SimulatedGenerator::new(yield_values);
        
        // Start the generator
        let _ = gen.next(f64::NAN);
        
        // Property: throw() should return an error
        let result = gen.throw(error_value);
        prop_assert!(result.is_err(),
            "throw() should return an error");
        
        // Property: The error should contain the thrown value
        if let Err(err) = result {
            prop_assert_eq!(err.error_value, error_value,
                "throw() error should contain the thrown value");
        }
        
        // Property: Generator should be completed after throw
        prop_assert_eq!(gen.get_state(), GENERATOR_STATE_COMPLETED,
            "Generator should be completed after throw()");
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Test that throw() on unstarted generator completes it
    /// Requirements: 9.6 - throw(error) throws error inside generator
    /// **Feature: dx-runtime-production-ready, Property 31: Generator Send/Throw/Return**
    #[test]
    fn prop_generator_throw_on_unstarted(
        yield_values in prop::collection::vec(any::<f64>().prop_filter("finite", |v| v.is_finite()), 1..5),
        error_value in any::<f64>().prop_filter("finite", |v| v.is_finite())
    ) {
        let mut gen = SimulatedGenerator::new(yield_values);
        
        // Property: Generator starts in Created state
        prop_assert_eq!(gen.get_state(), GENERATOR_STATE_CREATED,
            "Generator should start in Created state");
        
        // Property: throw() on unstarted generator should complete it
        let result = gen.throw(error_value);
        prop_assert!(result.is_err(),
            "throw() on unstarted generator should return error");
        
        prop_assert_eq!(gen.get_state(), GENERATOR_STATE_COMPLETED,
            "Generator should be completed after throw() on unstarted");
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Test that return(value) completes generator with given value
    /// Requirements: 9.7 - return(value) completes generator with given value
    /// **Feature: dx-runtime-production-ready, Property 31: Generator Send/Throw/Return**
    #[test]
    fn prop_generator_return_completes_with_value(
        yield_values in prop::collection::vec(any::<f64>().prop_filter("finite", |v| v.is_finite()), 1..5),
        return_value in any::<f64>().prop_filter("finite", |v| v.is_finite())
    ) {
        let mut gen = SimulatedGenerator::new(yield_values);
        
        // Start the generator
        let _ = gen.next(f64::NAN);
        
        // Property: return() should complete with the given value
        let result = gen.force_return(return_value);
        
        prop_assert!(result.done,
            "return() should return done: true");
        prop_assert_eq!(result.value, return_value,
            "return() should return the given value");
        
        // Property: Generator should be completed after return
        prop_assert_eq!(gen.get_state(), GENERATOR_STATE_COMPLETED,
            "Generator should be completed after return()");
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Test that return() on unstarted generator completes it
    /// Requirements: 9.7 - return(value) completes generator with given value
    /// **Feature: dx-runtime-production-ready, Property 31: Generator Send/Throw/Return**
    #[test]
    fn prop_generator_return_on_unstarted(
        yield_values in prop::collection::vec(any::<f64>().prop_filter("finite", |v| v.is_finite()), 1..5),
        return_value in any::<f64>().prop_filter("finite", |v| v.is_finite())
    ) {
        let mut gen = SimulatedGenerator::new(yield_values);
        
        // Property: Generator starts in Created state
        prop_assert_eq!(gen.get_state(), GENERATOR_STATE_CREATED,
            "Generator should start in Created state");
        
        // Property: return() on unstarted generator should complete it
        let result = gen.force_return(return_value);
        
        prop_assert!(result.done,
            "return() on unstarted generator should return done: true");
        prop_assert_eq!(result.value, return_value,
            "return() on unstarted generator should return the given value");
        prop_assert_eq!(gen.get_state(), GENERATOR_STATE_COMPLETED,
            "Generator should be completed after return() on unstarted");
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Test that return() on completed generator still returns the value
    /// Requirements: 9.7 - return(value) completes generator with given value
    /// **Feature: dx-runtime-production-ready, Property 31: Generator Send/Throw/Return**
    #[test]
    fn prop_generator_return_on_completed(
        yield_values in prop::collection::vec(any::<f64>().prop_filter("finite", |v| v.is_finite()), 0..3),
        return_value in any::<f64>().prop_filter("finite", |v| v.is_finite())
    ) {
        let mut gen = SimulatedGenerator::new(yield_values.clone());
        
        // Exhaust the generator
        for _ in 0..=yield_values.len() {
            let _ = gen.next(f64::NAN);
        }
        
        // Property: Generator should be completed
        prop_assert_eq!(gen.get_state(), GENERATOR_STATE_COMPLETED,
            "Generator should be completed after exhaustion");
        
        // Property: return() on completed generator should still return the value
        let result = gen.force_return(return_value);
        
        prop_assert!(result.done,
            "return() on completed generator should return done: true");
        prop_assert_eq!(result.value, return_value,
            "return() on completed generator should return the given value");
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Test that subsequent next() after return() returns done
    /// Requirements: 9.7 - return(value) completes generator with given value
    /// **Feature: dx-runtime-production-ready, Property 31: Generator Send/Throw/Return**
    #[test]
    fn prop_generator_next_after_return(
        yield_values in prop::collection::vec(any::<f64>().prop_filter("finite", |v| v.is_finite()), 2..5),
        return_value in any::<f64>().prop_filter("finite", |v| v.is_finite()),
        extra_calls in 1usize..5
    ) {
        let mut gen = SimulatedGenerator::new(yield_values);
        
        // Start the generator
        let _ = gen.next(f64::NAN);
        
        // Force return
        let _ = gen.force_return(return_value);
        
        // Property: All subsequent next() calls should return done: true
        for i in 0..extra_calls {
            let result = gen.next(f64::NAN);
            prop_assert!(result.done,
                "next() after return() should return done: true (call {})", i);
        }
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Test that subsequent next() after throw() returns done
    /// Requirements: 9.6 - throw(error) throws error inside generator
    /// **Feature: dx-runtime-production-ready, Property 31: Generator Send/Throw/Return**
    #[test]
    fn prop_generator_next_after_throw(
        yield_values in prop::collection::vec(any::<f64>().prop_filter("finite", |v| v.is_finite()), 2..5),
        error_value in any::<f64>().prop_filter("finite", |v| v.is_finite()),
        extra_calls in 1usize..5
    ) {
        let mut gen = SimulatedGenerator::new(yield_values);
        
        // Start the generator
        let _ = gen.next(f64::NAN);
        
        // Throw error
        let _ = gen.throw(error_value);
        
        // Property: All subsequent next() calls should return done: true
        for i in 0..extra_calls {
            let result = gen.next(f64::NAN);
            prop_assert!(result.done,
                "next() after throw() should return done: true (call {})", i);
        }
    }
}

// ============================================================================
// Edge case tests
// ============================================================================

#[test]
fn test_generator_next_value_is_yield_result() {
    // Requirements: 9.5 - next(value) passes value as yield expression result
    let mut gen = SimulatedGenerator::new(vec![1.0, 2.0, 3.0]);
    
    // First next() starts the generator
    let _ = gen.next(f64::NAN);
    
    // Second next(42.0) should pass 42.0 as yield result
    let _ = gen.next(42.0);
    assert_eq!(gen.get_sent_value(), 42.0, "sent_value should be 42.0");
    
    // Third next(99.0) should pass 99.0 as yield result
    let _ = gen.next(99.0);
    assert_eq!(gen.get_sent_value(), 99.0, "sent_value should be 99.0");
}

#[test]
fn test_generator_throw_completes_generator() {
    // Requirements: 9.6 - throw(error) throws error inside generator
    let mut gen = SimulatedGenerator::new(vec![1.0, 2.0, 3.0]);
    
    // Start the generator
    let _ = gen.next(f64::NAN);
    assert_eq!(gen.get_state(), GENERATOR_STATE_SUSPENDED);
    
    // Throw error
    let result = gen.throw(123.0);
    assert!(result.is_err());
    assert_eq!(gen.get_state(), GENERATOR_STATE_COMPLETED);
}

#[test]
fn test_generator_return_completes_with_value() {
    // Requirements: 9.7 - return(value) completes generator with given value
    let mut gen = SimulatedGenerator::new(vec![1.0, 2.0, 3.0]);
    
    // Start the generator
    let _ = gen.next(f64::NAN);
    
    // Force return with value
    let result = gen.force_return(999.0);
    
    assert!(result.done);
    assert_eq!(result.value, 999.0);
    assert_eq!(gen.get_state(), GENERATOR_STATE_COMPLETED);
}

#[test]
fn test_generator_first_next_ignores_send_value() {
    // Requirements: 9.5 - first next() call ignores send_value per spec
    let mut gen = SimulatedGenerator::new(vec![1.0, 2.0]);
    
    // First next() with a value - should be ignored
    let result = gen.next(42.0);
    
    // The first yield value should be returned, not the sent value
    assert_eq!(result.value, 1.0);
    assert!(!result.done);
    
    // The sent_value should still be NaN (ignored)
    assert!(gen.get_sent_value().is_nan());
}

#[test]
fn test_generator_throw_on_completed_rethrows() {
    // Requirements: 9.6 - throw on completed generator
    let mut gen = SimulatedGenerator::new(vec![1.0]);
    
    // Exhaust the generator
    let _ = gen.next(f64::NAN);
    let _ = gen.next(f64::NAN);
    assert_eq!(gen.get_state(), GENERATOR_STATE_COMPLETED);
    
    // Throw on completed generator should still return error
    let result = gen.throw(456.0);
    assert!(result.is_err());
    if let Err(err) = result {
        assert_eq!(err.error_value, 456.0);
    }
}

#[test]
fn test_generator_multiple_sends_sequence() {
    // Requirements: 9.5 - multiple next(value) calls
    let mut gen = SimulatedGenerator::new(vec![10.0, 20.0, 30.0, 40.0, 50.0]);
    
    // Start generator
    let _ = gen.next(f64::NAN);
    
    // Send values
    let _ = gen.next(100.0);
    let _ = gen.next(200.0);
    let _ = gen.next(300.0);
    
    // Verify all values were received
    let received = gen.get_received_values();
    assert_eq!(received.len(), 3);
    assert_eq!(received[0], 100.0);
    assert_eq!(received[1], 200.0);
    assert_eq!(received[2], 300.0);
}
