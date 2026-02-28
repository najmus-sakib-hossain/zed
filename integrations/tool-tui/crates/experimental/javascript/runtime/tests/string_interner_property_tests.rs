//! Property tests for Thread-Safe String Interner
//!
//! Feature: dx-runtime-production-ready
//! Property 2: Thread-Safe String Interner
//!
//! These tests verify that the string interner:
//! - Returns consistent IDs for identical strings across threads
//! - Does not have data races during concurrent access
//! - Maintains correctness under concurrent interning
//!
//! **Validates: Requirements 1.2**

use dx_js_runtime::value::string::{intern, get_interned, is_interned, get_id, interned_count};
use proptest::prelude::*;
use std::collections::HashSet;
use std::sync::Arc;
use std::thread;

// ============================================================================
// Property 2: Thread-Safe String Interner
// For any sequence of concurrent string intern operations from multiple threads,
// the interner should return consistent IDs for identical strings without data races.
// **Validates: Requirements 1.2**
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Property 2: Same string always returns same ID (single thread)
    /// Feature: dx-runtime-production-ready, Property 2: Thread-Safe String Interner
    #[test]
    fn prop_same_string_same_id(s in "[a-zA-Z0-9_]{1,50}") {
        let id1 = intern(&s);
        let id2 = intern(&s);
        
        // Property: Interning the same string twice should return the same ID
        prop_assert_eq!(
            id1, id2,
            "Same string '{}' should always return same ID, got {} and {}",
            s, id1, id2
        );
    }

    /// Property 2: Different strings get different IDs
    /// Feature: dx-runtime-production-ready, Property 2: Thread-Safe String Interner
    #[test]
    fn prop_different_strings_different_ids(
        s1 in "[a-z]{5,20}",
        s2 in "[A-Z]{5,20}"
    ) {
        prop_assume!(s1 != s2);
        
        let id1 = intern(&s1);
        let id2 = intern(&s2);
        
        // Property: Different strings should get different IDs
        prop_assert_ne!(
            id1, id2,
            "Different strings '{}' and '{}' should have different IDs",
            s1, s2
        );
    }

    /// Property 2: Interned string can be retrieved by ID
    /// Feature: dx-runtime-production-ready, Property 2: Thread-Safe String Interner
    #[test]
    fn prop_round_trip_intern_get(s in "[a-zA-Z0-9 !@#$%^&*()]{1,100}") {
        let id = intern(&s);
        let retrieved = get_interned(id);
        
        // Property: Retrieved string should match original
        prop_assert_eq!(
            retrieved.as_deref(),
            Some(s.as_str()),
            "Retrieved string should match original"
        );
    }

    /// Property 2: is_interned returns true after interning
    /// Feature: dx-runtime-production-ready, Property 2: Thread-Safe String Interner
    #[test]
    fn prop_is_interned_after_intern(s in "[a-zA-Z0-9]{1,50}") {
        let _ = intern(&s);
        
        // Property: String should be marked as interned after interning
        prop_assert!(
            is_interned(&s),
            "String '{}' should be marked as interned after interning",
            s
        );
    }

    /// Property 2: get_id returns correct ID after interning
    /// Feature: dx-runtime-production-ready, Property 2: Thread-Safe String Interner
    #[test]
    fn prop_get_id_matches_intern(s in "[a-zA-Z0-9]{1,50}") {
        let id = intern(&s);
        let retrieved_id = get_id(&s);
        
        // Property: get_id should return the same ID as intern
        prop_assert_eq!(
            retrieved_id,
            Some(id),
            "get_id should return the same ID as intern"
        );
    }

    /// Property 2: Multiple strings maintain unique IDs
    /// Feature: dx-runtime-production-ready, Property 2: Thread-Safe String Interner
    #[test]
    fn prop_multiple_strings_unique_ids(
        strings in prop::collection::vec("[a-zA-Z0-9]{3,20}", 2..20)
    ) {
        // Deduplicate input strings
        let unique_strings: Vec<_> = strings.iter()
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();
        
        // Intern all strings
        let ids: Vec<_> = unique_strings.iter()
            .map(|s| intern(s))
            .collect();
        
        // Property: All unique strings should have unique IDs
        let unique_ids: HashSet<_> = ids.iter().collect();
        prop_assert_eq!(
            unique_ids.len(),
            unique_strings.len(),
            "All unique strings should have unique IDs"
        );
    }
}

// ============================================================================
// Concurrent Access Tests
// These tests verify thread safety under concurrent access
// ============================================================================

/// Property 2: Concurrent interning of same string returns same ID
/// Feature: dx-runtime-production-ready, Property 2: Thread-Safe String Interner
#[test]
fn test_concurrent_same_string_same_id() {
    let test_string = "concurrent_test_string_12345";
    let num_threads = 8;
    let iterations = 100;
    
    let results: Vec<_> = (0..num_threads)
        .map(|_| {
            let s = test_string.to_string();
            thread::spawn(move || {
                let mut ids = Vec::with_capacity(iterations);
                for _ in 0..iterations {
                    ids.push(intern(&s));
                }
                ids
            })
        })
        .collect();
    
    // Collect all IDs from all threads
    let all_ids: Vec<u32> = results
        .into_iter()
        .flat_map(|handle| handle.join().unwrap())
        .collect();
    
    // Property: All IDs should be the same
    let first_id = all_ids[0];
    for (i, &id) in all_ids.iter().enumerate() {
        assert_eq!(
            id, first_id,
            "Thread iteration {} returned different ID {} (expected {})",
            i, id, first_id
        );
    }
}

/// Property 2: Concurrent interning of different strings maintains uniqueness
/// Feature: dx-runtime-production-ready, Property 2: Thread-Safe String Interner
#[test]
fn test_concurrent_different_strings_unique_ids() {
    let num_threads = 4;
    let strings_per_thread = 50;
    
    let handles: Vec<_> = (0..num_threads)
        .map(|thread_id| {
            thread::spawn(move || {
                let mut results = Vec::with_capacity(strings_per_thread);
                for i in 0..strings_per_thread {
                    let s = format!("thread_{}_string_{}", thread_id, i);
                    let id = intern(&s);
                    results.push((s, id));
                }
                results
            })
        })
        .collect();
    
    // Collect all results
    let all_results: Vec<(String, u32)> = handles
        .into_iter()
        .flat_map(|handle| handle.join().unwrap())
        .collect();
    
    // Verify each string maps to its ID correctly
    for (s, id) in &all_results {
        let retrieved = get_interned(*id);
        assert_eq!(
            retrieved.as_deref(),
            Some(s.as_str()),
            "String '{}' with ID {} should be retrievable",
            s, id
        );
    }
    
    // Verify all unique strings have unique IDs
    let unique_strings: HashSet<_> = all_results.iter().map(|(s, _)| s).collect();
    let unique_ids: HashSet<_> = all_results.iter().map(|(_, id)| id).collect();
    assert_eq!(
        unique_strings.len(),
        unique_ids.len(),
        "All unique strings should have unique IDs"
    );
}

/// Property 2: Concurrent read and write operations don't cause data races
/// Feature: dx-runtime-production-ready, Property 2: Thread-Safe String Interner
#[test]
fn test_concurrent_read_write() {
    let shared_strings = Arc::new(vec![
        "shared_1".to_string(),
        "shared_2".to_string(),
        "shared_3".to_string(),
        "shared_4".to_string(),
        "shared_5".to_string(),
    ]);
    
    // First, intern all shared strings
    for s in shared_strings.iter() {
        intern(s);
    }
    
    let num_threads = 8;
    let iterations = 100;
    
    let handles: Vec<_> = (0..num_threads)
        .map(|thread_id| {
            let strings = Arc::clone(&shared_strings);
            thread::spawn(move || {
                for i in 0..iterations {
                    // Mix of read and write operations
                    if i % 2 == 0 {
                        // Read operation: check if string is interned
                        let idx = i % strings.len();
                        assert!(
                            is_interned(&strings[idx]),
                            "Thread {}: String '{}' should be interned",
                            thread_id, strings[idx]
                        );
                    } else {
                        // Write operation: intern a new string
                        let new_string = format!("thread_{}_iter_{}", thread_id, i);
                        let id = intern(&new_string);
                        
                        // Verify it was interned correctly
                        let retrieved = get_interned(id);
                        assert_eq!(
                            retrieved.as_deref(),
                            Some(new_string.as_str()),
                            "Thread {}: Newly interned string should be retrievable",
                            thread_id
                        );
                    }
                }
            })
        })
        .collect();
    
    // Wait for all threads to complete
    for handle in handles {
        handle.join().expect("Thread should not panic");
    }
}

/// Property 2: Interned count increases correctly under concurrent access
/// Feature: dx-runtime-production-ready, Property 2: Thread-Safe String Interner
#[test]
fn test_concurrent_count_consistency() {
    let initial_count = interned_count();
    
    let num_threads = 4;
    let unique_strings_per_thread = 10;
    
    let handles: Vec<_> = (0..num_threads)
        .map(|thread_id| {
            thread::spawn(move || {
                for i in 0..unique_strings_per_thread {
                    let s = format!("count_test_thread_{}_string_{}", thread_id, i);
                    intern(&s);
                }
            })
        })
        .collect();
    
    // Wait for all threads
    for handle in handles {
        handle.join().expect("Thread should not panic");
    }
    
    let final_count = interned_count();
    let expected_new_strings = num_threads * unique_strings_per_thread;
    
    // Property: Count should have increased by the number of unique strings
    assert!(
        final_count >= initial_count + expected_new_strings,
        "Interned count should have increased by at least {} (was {}, now {})",
        expected_new_strings, initial_count, final_count
    );
}

// ============================================================================
// Unit Tests for Edge Cases
// ============================================================================

#[test]
fn test_empty_string_interning() {
    let id = intern("");
    let retrieved = get_interned(id);
    
    assert_eq!(retrieved.as_deref(), Some(""), "Empty string should be internable");
    assert!(is_interned(""), "Empty string should be marked as interned");
}

#[test]
fn test_unicode_string_interning() {
    let unicode_strings = [
        "Hello, 世界!",
        "🎉🎊🎈",
        "Привет мир",
        "مرحبا بالعالم",
        "שלום עולם",
        "日本語テスト",
    ];
    
    for s in &unicode_strings {
        let id = intern(s);
        let retrieved = get_interned(id);
        
        assert_eq!(
            retrieved.as_deref(),
            Some(*s),
            "Unicode string '{}' should be correctly interned",
            s
        );
    }
}

#[test]
fn test_whitespace_string_interning() {
    let whitespace_strings = [
        " ",
        "  ",
        "\t",
        "\n",
        "\r\n",
        "  \t  \n  ",
    ];
    
    for s in &whitespace_strings {
        let id = intern(s);
        let retrieved = get_interned(id);
        
        assert_eq!(
            retrieved.as_deref(),
            Some(*s),
            "Whitespace string should be correctly interned"
        );
    }
}

#[test]
fn test_invalid_id_returns_none() {
    // Use a very large ID that's unlikely to exist
    let result = get_interned(u32::MAX);
    assert_eq!(result, None, "Invalid ID should return None");
}
