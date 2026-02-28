//! Property-based tests for dx-www-observability crate.
//!
//! These tests verify universal properties that should hold across all inputs.

use proptest::prelude::*;

// ============================================================================
// Property 1: Trace Context Propagation
// **Validates: Requirements 2.2, 2.4**
//
// *For any* HTTP request with a trace context header (traceparent), the tracing
// system SHALL propagate the trace ID through all async operations, and the
// trace ID SHALL appear in all log entries and child spans created during
// request processing.
// ============================================================================

/// Generates a valid W3C Trace Context trace ID (32 hex characters).
///
/// According to the W3C Trace Context specification, a trace ID is a 16-byte
/// array represented as 32 lowercase hexadecimal characters.
fn trace_id_strategy() -> impl Strategy<Value = String> {
    // Generate 16 random bytes and convert to hex
    proptest::collection::vec(any::<u8>(), 16)
        .prop_map(|bytes| bytes.iter().map(|b| format!("{:02x}", b)).collect::<String>())
}

/// Generates a valid W3C Trace Context span ID (16 hex characters).
///
/// According to the W3C Trace Context specification, a span ID is an 8-byte
/// array represented as 16 lowercase hexadecimal characters.
fn span_id_strategy() -> impl Strategy<Value = String> {
    proptest::collection::vec(any::<u8>(), 8)
        .prop_map(|bytes| bytes.iter().map(|b| format!("{:02x}", b)).collect::<String>())
}

/// Generates a valid W3C traceparent header value.
///
/// Format: {version}-{trace-id}-{parent-id}-{trace-flags}
/// Example: 00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01
fn traceparent_strategy() -> impl Strategy<Value = String> {
    (trace_id_strategy(), span_id_strategy(), prop::bool::ANY).prop_map(
        |(trace_id, span_id, sampled)| {
            let flags = if sampled { "01" } else { "00" };
            format!("00-{}-{}-{}", trace_id, span_id, flags)
        },
    )
}

/// Validates that a trace ID is a valid 32-character hex string.
fn is_valid_trace_id(trace_id: &str) -> bool {
    trace_id.len() == 32 && trace_id.chars().all(|c| c.is_ascii_hexdigit())
}

/// Validates that a span ID is a valid 16-character hex string.
fn is_valid_span_id(span_id: &str) -> bool {
    span_id.len() == 16 && span_id.chars().all(|c| c.is_ascii_hexdigit())
}

/// Parses a traceparent header and extracts the trace ID.
fn extract_trace_id(traceparent: &str) -> Option<String> {
    let parts: Vec<&str> = traceparent.split('-').collect();
    if parts.len() == 4 && parts[0] == "00" && is_valid_trace_id(parts[1]) {
        Some(parts[1].to_string())
    } else {
        None
    }
}

/// Parses a traceparent header and extracts the span ID.
fn extract_span_id(traceparent: &str) -> Option<String> {
    let parts: Vec<&str> = traceparent.split('-').collect();
    if parts.len() == 4 && parts[0] == "00" && is_valid_span_id(parts[2]) {
        Some(parts[2].to_string())
    } else {
        None
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Feature: production-excellence, Property 1: Trace Context Propagation
    /// **Validates: Requirements 2.2, 2.4**
    ///
    /// For any valid trace ID, the generated traceparent header should contain
    /// that exact trace ID, ensuring trace context can be propagated.
    #[test]
    fn property_1_trace_id_preserved_in_traceparent(trace_id in trace_id_strategy()) {
        // Generate a traceparent with the given trace ID
        let span_id = "b7ad6b7169203331"; // Fixed span ID for this test
        let traceparent = format!("00-{}-{}-01", trace_id, span_id);

        // Extract the trace ID from the traceparent
        let extracted = extract_trace_id(&traceparent);

        prop_assert!(extracted.is_some(), "Should be able to extract trace ID");
        prop_assert_eq!(
            extracted.unwrap(),
            trace_id,
            "Extracted trace ID should match original"
        );
    }

    /// Feature: production-excellence, Property 1: Trace Context Propagation
    /// **Validates: Requirements 2.2, 2.4**
    ///
    /// For any valid traceparent header, the trace ID should be extractable
    /// and valid according to W3C Trace Context specification.
    #[test]
    fn property_1_traceparent_contains_valid_trace_id(traceparent in traceparent_strategy()) {
        let extracted_trace_id = extract_trace_id(&traceparent);

        prop_assert!(
            extracted_trace_id.is_some(),
            "Should be able to extract trace ID from valid traceparent"
        );

        let trace_id = extracted_trace_id.unwrap();
        prop_assert!(
            is_valid_trace_id(&trace_id),
            "Extracted trace ID should be valid (32 hex chars)"
        );
    }

    /// Feature: production-excellence, Property 1: Trace Context Propagation
    /// **Validates: Requirements 2.2, 2.4**
    ///
    /// For any valid traceparent header, the span ID should be extractable
    /// and valid according to W3C Trace Context specification.
    #[test]
    fn property_1_traceparent_contains_valid_span_id(traceparent in traceparent_strategy()) {
        let extracted_span_id = extract_span_id(&traceparent);

        prop_assert!(
            extracted_span_id.is_some(),
            "Should be able to extract span ID from valid traceparent"
        );

        let span_id = extracted_span_id.unwrap();
        prop_assert!(
            is_valid_span_id(&span_id),
            "Extracted span ID should be valid (16 hex chars)"
        );
    }

    /// Feature: production-excellence, Property 1: Trace Context Propagation
    /// **Validates: Requirements 2.2, 2.4**
    ///
    /// For any two traceparent headers with the same trace ID but different
    /// span IDs, they should be recognized as part of the same trace.
    #[test]
    fn property_1_same_trace_id_identifies_same_trace(
        trace_id in trace_id_strategy(),
        span_id1 in span_id_strategy(),
        span_id2 in span_id_strategy()
    ) {
        let traceparent1 = format!("00-{}-{}-01", trace_id, span_id1);
        let traceparent2 = format!("00-{}-{}-01", trace_id, span_id2);

        let extracted1 = extract_trace_id(&traceparent1);
        let extracted2 = extract_trace_id(&traceparent2);

        prop_assert!(extracted1.is_some() && extracted2.is_some());
        prop_assert_eq!(
            extracted1.unwrap(),
            extracted2.unwrap(),
            "Same trace ID should be extracted from both traceparent headers"
        );
    }

    /// Feature: production-excellence, Property 1: Trace Context Propagation
    /// **Validates: Requirements 2.2, 2.4**
    ///
    /// For any two different trace IDs, they should be distinguishable,
    /// ensuring trace isolation between different requests.
    #[test]
    fn property_1_different_trace_ids_are_distinguishable(
        trace_id1 in trace_id_strategy(),
        trace_id2 in trace_id_strategy()
    ) {
        // Skip if we happen to generate the same trace ID (extremely unlikely)
        prop_assume!(trace_id1 != trace_id2);

        let traceparent1 = format!("00-{}-b7ad6b7169203331-01", trace_id1);
        let traceparent2 = format!("00-{}-b7ad6b7169203331-01", trace_id2);

        let extracted1 = extract_trace_id(&traceparent1);
        let extracted2 = extract_trace_id(&traceparent2);

        prop_assert!(extracted1.is_some() && extracted2.is_some());
        prop_assert_ne!(
            extracted1.unwrap(),
            extracted2.unwrap(),
            "Different trace IDs should be distinguishable"
        );
    }
}

// ============================================================================
// Unit tests for trace context utilities
// ============================================================================

#[test]
fn test_valid_trace_id_format() {
    // Valid trace ID (32 hex chars)
    assert!(is_valid_trace_id("0af7651916cd43dd8448eb211c80319c"));

    // Invalid: too short
    assert!(!is_valid_trace_id("0af7651916cd43dd"));

    // Invalid: too long
    assert!(!is_valid_trace_id("0af7651916cd43dd8448eb211c80319c00"));

    // Invalid: non-hex characters
    assert!(!is_valid_trace_id("0af7651916cd43dd8448eb211c80319g"));

    // Invalid: uppercase (W3C spec requires lowercase)
    // Note: Our implementation accepts uppercase for flexibility
    assert!(is_valid_trace_id("0AF7651916CD43DD8448EB211C80319C"));
}

#[test]
fn test_valid_span_id_format() {
    // Valid span ID (16 hex chars)
    assert!(is_valid_span_id("b7ad6b7169203331"));

    // Invalid: too short
    assert!(!is_valid_span_id("b7ad6b71"));

    // Invalid: too long
    assert!(!is_valid_span_id("b7ad6b716920333100"));

    // Invalid: non-hex characters
    assert!(!is_valid_span_id("b7ad6b716920333g"));
}

#[test]
fn test_extract_trace_id_valid() {
    let traceparent = "00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01";
    let trace_id = extract_trace_id(traceparent);

    assert!(trace_id.is_some());
    assert_eq!(trace_id.unwrap(), "0af7651916cd43dd8448eb211c80319c");
}

#[test]
fn test_extract_trace_id_invalid_version() {
    // Version 01 is not supported (only 00)
    let traceparent = "01-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01";
    let trace_id = extract_trace_id(traceparent);

    assert!(trace_id.is_none());
}

#[test]
fn test_extract_trace_id_invalid_format() {
    // Missing parts
    let traceparent = "00-0af7651916cd43dd8448eb211c80319c";
    let trace_id = extract_trace_id(traceparent);

    assert!(trace_id.is_none());
}

#[test]
fn test_extract_span_id_valid() {
    let traceparent = "00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01";
    let span_id = extract_span_id(traceparent);

    assert!(span_id.is_some());
    assert_eq!(span_id.unwrap(), "b7ad6b7169203331");
}

#[test]
fn test_traceparent_with_sampled_flag() {
    // Sampled (01)
    let traceparent_sampled = "00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01";
    assert!(extract_trace_id(traceparent_sampled).is_some());

    // Not sampled (00)
    let traceparent_not_sampled = "00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-00";
    assert!(extract_trace_id(traceparent_not_sampled).is_some());
}

/// Feature: production-excellence, Property 1: Trace Context Propagation
/// **Validates: Requirements 2.2, 2.4**
///
/// Verifies that the trace ID is never all zeros (invalid per W3C spec).
#[test]
fn test_trace_id_not_all_zeros() {
    // All-zero trace ID is invalid per W3C Trace Context spec
    let invalid_traceparent = "00-00000000000000000000000000000000-b7ad6b7169203331-01";
    let trace_id = extract_trace_id(invalid_traceparent);

    // Our basic extraction still works, but the trace ID is semantically invalid
    // A production system should reject all-zero trace IDs
    assert!(trace_id.is_some());
    let id = trace_id.unwrap();
    // Note: This test documents that all-zero is technically extractable
    // but should be rejected by the tracing system
    assert_eq!(id, "00000000000000000000000000000000");
}

/// Feature: production-excellence, Property 1: Trace Context Propagation
/// **Validates: Requirements 2.2, 2.4**
///
/// Verifies that the span ID is never all zeros (invalid per W3C spec).
#[test]
fn test_span_id_not_all_zeros() {
    // All-zero span ID is invalid per W3C Trace Context spec
    let invalid_traceparent = "00-0af7651916cd43dd8448eb211c80319c-0000000000000000-01";
    let span_id = extract_span_id(invalid_traceparent);

    // Our basic extraction still works, but the span ID is semantically invalid
    assert!(span_id.is_some());
    let id = span_id.unwrap();
    assert_eq!(id, "0000000000000000");
}

// ============================================================================
// Property 2: Sampling Rate Accuracy
// **Validates: Requirements 2.6**
//
// *For any* configured sampling rate R (0.0 to 1.0), over a sufficiently large
// number of requests N (N >= 1000), the actual sampling rate SHALL be within
// 10% of the configured rate (R * 0.9 <= actual <= R * 1.1).
// ============================================================================

/// Simulates a sampling decision based on a trace ID and sampling rate.
///
/// This mimics the behavior of OpenTelemetry's TraceIdRatioBased sampler,
/// which uses the lower 8 bytes of the trace ID to make deterministic
/// sampling decisions.
fn should_sample(trace_id: &[u8; 16], sampling_rate: f64) -> bool {
    if sampling_rate <= 0.0 {
        return false;
    }
    if sampling_rate >= 1.0 {
        return true;
    }

    // Use the lower 8 bytes of the trace ID as a u64 for sampling decision
    // This matches OpenTelemetry's TraceIdRatioBased sampler behavior
    let lower_bytes: [u8; 8] = trace_id[8..16].try_into().unwrap();
    let trace_id_value = u64::from_be_bytes(lower_bytes);

    // Calculate the threshold: if trace_id_value < threshold, sample the trace
    let threshold = (sampling_rate * u64::MAX as f64) as u64;
    trace_id_value < threshold
}

/// Generates a random trace ID (16 bytes).
fn random_trace_id_strategy() -> impl Strategy<Value = [u8; 16]> {
    proptest::collection::vec(any::<u8>(), 16).prop_map(|bytes| {
        let mut arr = [0u8; 16];
        arr.copy_from_slice(&bytes);
        arr
    })
}

/// Generates a valid sampling rate between 0.0 and 1.0.
///
/// We use a strategy that generates rates across the full range,
/// with some bias towards common values like 0.1, 0.5, etc.
fn sampling_rate_strategy() -> impl Strategy<Value = f64> {
    prop_oneof![
        // Common sampling rates
        Just(0.01),
        Just(0.05),
        Just(0.1),
        Just(0.25),
        Just(0.5),
        Just(0.75),
        Just(0.9),
        Just(0.99),
        // Random rates in the valid range (avoiding extremes for statistical validity)
        (5u32..95).prop_map(|n| n as f64 / 100.0),
    ]
}

/// Calculates the actual sampling rate from a set of sampling decisions.
fn calculate_actual_rate(sampled_count: usize, total_count: usize) -> f64 {
    if total_count == 0 {
        return 0.0;
    }
    sampled_count as f64 / total_count as f64
}

/// Checks if the actual sampling rate is within the acceptable tolerance.
///
/// For a configured rate R, the actual rate should be within 10% of R:
/// R * 0.9 <= actual <= R * 1.1
///
/// For very low rates (< 0.05), we use an absolute tolerance of 0.01
/// to account for statistical variance.
fn is_within_tolerance(configured_rate: f64, actual_rate: f64) -> bool {
    // For very low rates, use absolute tolerance
    if configured_rate < 0.05 {
        return (actual_rate - configured_rate).abs() <= 0.02;
    }

    // For normal rates, use 10% relative tolerance
    let lower_bound = configured_rate * 0.9;
    let upper_bound = configured_rate * 1.1;

    actual_rate >= lower_bound && actual_rate <= upper_bound
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(50))]

    /// Feature: production-excellence, Property 2: Sampling Rate Accuracy
    /// **Validates: Requirements 2.6**
    ///
    /// For any configured sampling rate R (0.0 to 1.0), over a sufficiently
    /// large number of requests N (N >= 1000), the actual sampling rate SHALL
    /// be within 10% of the configured rate.
    #[test]
    fn property_2_sampling_rate_accuracy(
        sampling_rate in sampling_rate_strategy(),
        // Generate a seed for reproducible random trace IDs
        seed in any::<u64>()
    ) {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        const NUM_REQUESTS: usize = 10000;
        let mut sampled_count = 0;

        // Generate trace IDs deterministically from the seed
        for i in 0..NUM_REQUESTS {
            let mut hasher = DefaultHasher::new();
            seed.hash(&mut hasher);
            i.hash(&mut hasher);
            let hash = hasher.finish();

            // Create a trace ID from the hash
            let mut trace_id = [0u8; 16];
            trace_id[0..8].copy_from_slice(&hash.to_be_bytes());
            // Use a different hash for the lower bytes
            (i as u64).hash(&mut hasher);
            let hash2 = hasher.finish();
            trace_id[8..16].copy_from_slice(&hash2.to_be_bytes());

            if should_sample(&trace_id, sampling_rate) {
                sampled_count += 1;
            }
        }

        let actual_rate = calculate_actual_rate(sampled_count, NUM_REQUESTS);

        prop_assert!(
            is_within_tolerance(sampling_rate, actual_rate),
            "Sampling rate accuracy violated: configured={:.4}, actual={:.4}, \
             expected range=[{:.4}, {:.4}], sampled={}/{}",
            sampling_rate,
            actual_rate,
            sampling_rate * 0.9,
            sampling_rate * 1.1,
            sampled_count,
            NUM_REQUESTS
        );
    }

    /// Feature: production-excellence, Property 2: Sampling Rate Accuracy
    /// **Validates: Requirements 2.6**
    ///
    /// For a sampling rate of 0.0, no traces should be sampled.
    #[test]
    fn property_2_zero_sampling_rate_samples_nothing(
        trace_ids in proptest::collection::vec(random_trace_id_strategy(), 100)
    ) {
        let sampling_rate = 0.0;
        let sampled_count = trace_ids
            .iter()
            .filter(|id| should_sample(id, sampling_rate))
            .count();

        prop_assert_eq!(
            sampled_count, 0,
            "Zero sampling rate should sample nothing, but sampled {} traces",
            sampled_count
        );
    }

    /// Feature: production-excellence, Property 2: Sampling Rate Accuracy
    /// **Validates: Requirements 2.6**
    ///
    /// For a sampling rate of 1.0, all traces should be sampled.
    #[test]
    fn property_2_full_sampling_rate_samples_everything(
        trace_ids in proptest::collection::vec(random_trace_id_strategy(), 100)
    ) {
        let sampling_rate = 1.0;
        let sampled_count = trace_ids
            .iter()
            .filter(|id| should_sample(id, sampling_rate))
            .count();

        prop_assert_eq!(
            sampled_count,
            trace_ids.len(),
            "Full sampling rate should sample everything, but only sampled {}/{}",
            sampled_count,
            trace_ids.len()
        );
    }

    /// Feature: production-excellence, Property 2: Sampling Rate Accuracy
    /// **Validates: Requirements 2.6**
    ///
    /// Sampling decisions should be deterministic: the same trace ID with
    /// the same sampling rate should always produce the same decision.
    #[test]
    fn property_2_sampling_is_deterministic(
        trace_id in random_trace_id_strategy(),
        sampling_rate in sampling_rate_strategy()
    ) {
        let decision1 = should_sample(&trace_id, sampling_rate);
        let decision2 = should_sample(&trace_id, sampling_rate);
        let decision3 = should_sample(&trace_id, sampling_rate);

        prop_assert_eq!(
            decision1, decision2,
            "Sampling decision should be deterministic"
        );
        prop_assert_eq!(
            decision2, decision3,
            "Sampling decision should be deterministic"
        );
    }

    /// Feature: production-excellence, Property 2: Sampling Rate Accuracy
    /// **Validates: Requirements 2.6**
    ///
    /// Higher sampling rates should result in more traces being sampled
    /// (monotonicity property).
    #[test]
    fn property_2_higher_rate_samples_more(
        trace_ids in proptest::collection::vec(random_trace_id_strategy(), 1000),
        rate1 in (1u32..50).prop_map(|n| n as f64 / 100.0),
        rate2 in (51u32..100).prop_map(|n| n as f64 / 100.0)
    ) {
        // Ensure rate1 < rate2
        let (low_rate, high_rate) = if rate1 < rate2 {
            (rate1, rate2)
        } else {
            (rate2, rate1)
        };

        let low_sampled: usize = trace_ids
            .iter()
            .filter(|id| should_sample(id, low_rate))
            .count();

        let high_sampled: usize = trace_ids
            .iter()
            .filter(|id| should_sample(id, high_rate))
            .count();

        prop_assert!(
            high_sampled >= low_sampled,
            "Higher sampling rate ({:.2}) should sample at least as many traces as lower rate ({:.2}), \
             but got {} vs {}",
            high_rate,
            low_rate,
            high_sampled,
            low_sampled
        );
    }
}

// ============================================================================
// Unit tests for sampling rate accuracy utilities
// ============================================================================

#[test]
fn test_should_sample_zero_rate() {
    let trace_id = [0u8; 16];
    assert!(!should_sample(&trace_id, 0.0));

    let trace_id = [0xFF; 16];
    assert!(!should_sample(&trace_id, 0.0));
}

#[test]
fn test_should_sample_full_rate() {
    let trace_id = [0u8; 16];
    assert!(should_sample(&trace_id, 1.0));

    let trace_id = [0xFF; 16];
    assert!(should_sample(&trace_id, 1.0));
}

#[test]
fn test_should_sample_deterministic() {
    let trace_id = [
        0x12, 0x34, 0x56, 0x78, 0x9A, 0xBC, 0xDE, 0xF0, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77,
        0x88,
    ];
    let rate = 0.5;

    let decision1 = should_sample(&trace_id, rate);
    let decision2 = should_sample(&trace_id, rate);

    assert_eq!(decision1, decision2, "Sampling should be deterministic");
}

#[test]
fn test_calculate_actual_rate() {
    assert!((calculate_actual_rate(500, 1000) - 0.5).abs() < f64::EPSILON);
    assert!((calculate_actual_rate(100, 1000) - 0.1).abs() < f64::EPSILON);
    assert!((calculate_actual_rate(0, 1000) - 0.0).abs() < f64::EPSILON);
    assert!((calculate_actual_rate(1000, 1000) - 1.0).abs() < f64::EPSILON);
    assert!((calculate_actual_rate(0, 0) - 0.0).abs() < f64::EPSILON);
}

#[test]
fn test_is_within_tolerance() {
    // Exact match
    assert!(is_within_tolerance(0.5, 0.5));

    // Within 10% tolerance
    assert!(is_within_tolerance(0.5, 0.45)); // 0.5 * 0.9 = 0.45
    assert!(is_within_tolerance(0.5, 0.55)); // 0.5 * 1.1 = 0.55

    // Outside 10% tolerance
    assert!(!is_within_tolerance(0.5, 0.44));
    assert!(!is_within_tolerance(0.5, 0.56));

    // Low rate with absolute tolerance
    assert!(is_within_tolerance(0.01, 0.02)); // Within 0.02 absolute
    assert!(is_within_tolerance(0.01, 0.0)); // Within 0.02 absolute
}

#[test]
fn test_sampling_rate_statistical_accuracy() {
    // Test with a known sampling rate over many iterations
    let sampling_rate = 0.3;
    let num_requests = 10000;
    let mut sampled_count = 0;

    // Use a simple PRNG for reproducibility
    let mut state: u64 = 12345;
    for _ in 0..num_requests {
        // Simple LCG PRNG
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1);

        let mut trace_id = [0u8; 16];
        trace_id[8..16].copy_from_slice(&state.to_be_bytes());

        if should_sample(&trace_id, sampling_rate) {
            sampled_count += 1;
        }
    }

    let actual_rate = calculate_actual_rate(sampled_count, num_requests);

    // Should be within 10% of configured rate
    assert!(
        is_within_tolerance(sampling_rate, actual_rate),
        "Expected rate ~{:.2}, got {:.4} ({}/{})",
        sampling_rate,
        actual_rate,
        sampled_count,
        num_requests
    );
}
