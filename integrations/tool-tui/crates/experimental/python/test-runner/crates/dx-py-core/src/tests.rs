//! Tests for dx-py-core types

use super::*;
use proptest::prelude::*;
use std::time::Duration;

// Generators for property tests
fn arb_test_id() -> impl Strategy<Value = TestId> {
    any::<u64>().prop_map(TestId)
}

fn arb_fixture_id() -> impl Strategy<Value = FixtureId> {
    any::<u64>().prop_map(FixtureId)
}

fn arb_marker() -> impl Strategy<Value = Marker> {
    ("[a-z_][a-z0-9_]{0,20}", prop::collection::vec("[a-zA-Z0-9_]+", 0..5))
        .prop_map(|(name, args)| Marker::with_args(name, args))
}

fn arb_test_case() -> impl Strategy<Value = TestCase> {
    (
        "test_[a-z_]{1,20}",
        "[a-z_/]{1,30}\\.py",
        1u32..10000u32,
        prop::option::of("Test[A-Z][a-zA-Z]{0,20}"),
        prop::collection::vec(arb_marker(), 0..3),
        prop::collection::vec(arb_fixture_id(), 0..3),
    )
        .prop_map(|(name, path, line, class, markers, fixtures)| {
            let mut tc = TestCase::new(&name, &path, line);
            if let Some(c) = class {
                tc = tc.with_class(c);
            }
            for m in markers {
                tc = tc.with_marker(m);
            }
            for f in fixtures {
                tc = tc.with_fixture(f);
            }
            tc
        })
}

fn arb_assertion_stats() -> impl Strategy<Value = AssertionStats> {
    (any::<u32>(), any::<u32>()).prop_map(|(p, f)| AssertionStats::new(p, f))
}

fn arb_test_status() -> impl Strategy<Value = TestStatus> {
    prop_oneof![
        Just(TestStatus::Pass),
        Just(TestStatus::Fail),
        "[a-zA-Z ]{0,50}".prop_map(|r| TestStatus::Skip { reason: r }),
        "[a-zA-Z ]{0,50}".prop_map(|m| TestStatus::Error { message: m }),
    ]
}

fn arb_test_result() -> impl Strategy<Value = TestResult> {
    (
        arb_test_id(),
        arb_test_status(),
        0u64..1_000_000u64,
        "[a-zA-Z0-9 \n]{0,100}",
        "[a-zA-Z0-9 \n]{0,100}",
        prop::option::of("[a-zA-Z0-9 \n]{0,200}"),
        arb_assertion_stats(),
    )
        .prop_map(|(id, status, dur_us, stdout, stderr, tb, assertions)| TestResult {
            test_id: id,
            status,
            duration: Duration::from_micros(dur_us),
            stdout,
            stderr,
            traceback: tb,
            assertions,
            assertion_failure: None,
        })
}

// Property 5: Protocol Message Round-Trip
// For any valid TestCase, serializing and deserializing produces equivalent data
proptest! {
    /// Feature: dx-py-test-runner, Property 5: Protocol Message Round-Trip
    /// Validates: Requirements 3.4, 3.5
    #[test]
    fn prop_test_case_roundtrip(tc in arb_test_case()) {
        let serialized = bincode::serialize(&tc).expect("serialize");
        let deserialized: TestCase = bincode::deserialize(&serialized).expect("deserialize");
        prop_assert_eq!(tc, deserialized);
    }

    /// Feature: dx-py-test-runner, Property 5: Protocol Message Round-Trip
    /// Validates: Requirements 3.4, 3.5
    #[test]
    fn prop_test_result_roundtrip(result in arb_test_result()) {
        let serialized = bincode::serialize(&result).expect("serialize");
        let deserialized: TestResult = bincode::deserialize(&serialized).expect("deserialize");
        prop_assert_eq!(result, deserialized);
    }

    /// Feature: dx-py-test-runner, Property 5: Protocol Message Round-Trip
    /// Validates: Requirements 3.4, 3.5
    #[test]
    fn prop_marker_roundtrip(marker in arb_marker()) {
        let serialized = bincode::serialize(&marker).expect("serialize");
        let deserialized: Marker = bincode::deserialize(&serialized).expect("deserialize");
        prop_assert_eq!(marker, deserialized);
    }

    /// Feature: dx-py-test-runner, Property 5: Protocol Message Round-Trip
    /// Validates: Requirements 3.4, 3.5
    #[test]
    fn prop_test_id_roundtrip(id in arb_test_id()) {
        let serialized = bincode::serialize(&id).expect("serialize");
        let deserialized: TestId = bincode::deserialize(&serialized).expect("deserialize");
        prop_assert_eq!(id, deserialized);
    }
}

// Unit tests
#[test]
fn test_case_full_name_without_class() {
    let tc = TestCase::new("test_example", "test_file.py", 10);
    assert_eq!(tc.full_name(), "test_example");
}

#[test]
fn test_case_full_name_with_class() {
    let tc = TestCase::new("test_example", "test_file.py", 10).with_class("TestClass");
    assert_eq!(tc.full_name(), "TestClass::test_example");
}

#[test]
fn test_status_is_success() {
    assert!(TestStatus::Pass.is_success());
    assert!(TestStatus::Skip {
        reason: "skipped".into()
    }
    .is_success());
    assert!(!TestStatus::Fail.is_success());
    assert!(!TestStatus::Error {
        message: "error".into()
    }
    .is_success());
}

#[test]
fn test_status_is_failure() {
    assert!(!TestStatus::Pass.is_failure());
    assert!(!TestStatus::Skip {
        reason: "skipped".into()
    }
    .is_failure());
    assert!(TestStatus::Fail.is_failure());
    assert!(TestStatus::Error {
        message: "error".into()
    }
    .is_failure());
}

#[test]
fn test_summary_from_results() {
    let results = vec![
        TestResult::pass(TestId(1), Duration::from_millis(100)),
        TestResult::pass(TestId(2), Duration::from_millis(50)),
        TestResult::fail(TestId(3), Duration::from_millis(200), "failed"),
        TestResult::skip(TestId(4), "skipped"),
        TestResult::error(TestId(5), "error"),
    ];

    let summary = TestSummary::from_results(&results);
    assert_eq!(summary.total, 5);
    assert_eq!(summary.passed, 2);
    assert_eq!(summary.failed, 1);
    assert_eq!(summary.skipped, 1);
    assert_eq!(summary.errors, 1);
    assert!(!summary.is_success());
}

#[test]
fn test_summary_success() {
    let results = vec![
        TestResult::pass(TestId(1), Duration::from_millis(100)),
        TestResult::skip(TestId(2), "skipped"),
    ];

    let summary = TestSummary::from_results(&results);
    assert!(summary.is_success());
}

#[test]
fn test_assertion_stats_total() {
    let stats = AssertionStats::new(5, 2);
    assert_eq!(stats.total(), 7);
}
