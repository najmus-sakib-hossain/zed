//! SIMD integration tests

use dx_py_simd::{get_engine, SimdStringEngine, SimdDispatcher};

#[test]
fn test_engine_selection() {
    let dispatcher = SimdDispatcher::new();
    let engine = dispatcher.get_engine();
    
    // Should always get a valid engine
    let name = engine.name();
    assert!(
        name == "AVX2" || name == "AVX-512" || name == "NEON" || name == "Scalar",
        "Unknown engine: {}",
        name
    );
}

#[test]
fn test_find_correctness() {
    let engine = get_engine();
    
    // Basic find
    assert_eq!(engine.find("hello world", "world"), Some(6));
    assert_eq!(engine.find("hello world", "hello"), Some(0));
    assert_eq!(engine.find("hello world", "xyz"), None);
    
    // Empty needle
    assert_eq!(engine.find("hello", ""), Some(0));
    
    // Empty haystack
    assert_eq!(engine.find("", "hello"), None);
    
    // Needle longer than haystack
    assert_eq!(engine.find("hi", "hello"), None);
    
    // Same string
    assert_eq!(engine.find("hello", "hello"), Some(0));
}

#[test]
fn test_count_correctness() {
    let engine = get_engine();
    
    assert_eq!(engine.count("aaa", "a"), 3);
    assert_eq!(engine.count("abcabc", "abc"), 2);
    assert_eq!(engine.count("hello", "x"), 0);
    assert_eq!(engine.count("", "a"), 0);
}

#[test]
fn test_eq_correctness() {
    let engine = get_engine();
    
    assert!(engine.eq("hello", "hello"));
    assert!(!engine.eq("hello", "world"));
    assert!(!engine.eq("hello", "hell"));
    assert!(engine.eq("", ""));
}

#[test]
fn test_case_conversion() {
    let engine = get_engine();
    
    // Lowercase
    assert_eq!(engine.to_lowercase("HELLO"), "hello");
    assert_eq!(engine.to_lowercase("Hello World"), "hello world");
    assert_eq!(engine.to_lowercase("123ABC"), "123abc");
    
    // Uppercase
    assert_eq!(engine.to_uppercase("hello"), "HELLO");
    assert_eq!(engine.to_uppercase("Hello World"), "HELLO WORLD");
    assert_eq!(engine.to_uppercase("123abc"), "123ABC");
}

#[test]
fn test_split_correctness() {
    let engine = get_engine();
    
    assert_eq!(engine.split("a,b,c", ","), vec!["a", "b", "c"]);
    assert_eq!(engine.split("hello", ","), vec!["hello"]);
    assert_eq!(engine.split("a::b::c", "::"), vec!["a", "b", "c"]);
    assert_eq!(engine.split("", ","), vec![""]);
}

#[test]
fn test_join_correctness() {
    let engine = get_engine();
    
    assert_eq!(engine.join(&["a", "b", "c"], ","), "a,b,c");
    assert_eq!(engine.join(&["hello"], ","), "hello");
    assert_eq!(engine.join(&[], ","), "");
    assert_eq!(engine.join(&["a", "b"], "::"), "a::b");
}

#[test]
fn test_replace_correctness() {
    let engine = get_engine();
    
    assert_eq!(engine.replace("hello world", "world", "rust"), "hello rust");
    assert_eq!(engine.replace("aaa", "a", "b"), "bbb");
    assert_eq!(engine.replace("hello", "x", "y"), "hello");
    assert_eq!(engine.replace("abab", "ab", "cd"), "cdcd");
}

#[test]
fn test_long_strings() {
    let engine = get_engine();
    
    // Test with strings longer than SIMD register width
    let long_str = "a".repeat(1000);
    let needle = "aaa";
    
    assert_eq!(engine.find(&long_str, needle), Some(0));
    assert_eq!(engine.count(&long_str, "a"), 1000);
    
    let upper = "A".repeat(1000);
    assert_eq!(engine.to_lowercase(&upper), long_str);
    assert_eq!(engine.to_uppercase(&long_str), upper);
}

#[test]
fn test_unicode_preservation() {
    let engine = get_engine();
    
    // Non-ASCII characters should be preserved
    let s = "héllo wörld";
    assert_eq!(engine.find(s, "wörld"), Some(7));
    
    // Case conversion only affects ASCII
    let mixed = "Héllo Wörld";
    let lower = engine.to_lowercase(mixed);
    assert!(lower.contains("é")); // Non-ASCII preserved
}

/// Property test: SIMD results should match scalar results
#[test]
fn test_simd_scalar_equivalence() {
    use dx_py_simd::scalar::ScalarStringEngine;
    
    let simd_engine = get_engine();
    let scalar_engine = ScalarStringEngine::new();
    
    let test_cases = vec![
        ("hello world", "world"),
        ("abcdefghijklmnopqrstuvwxyz", "xyz"),
        ("aaaaaaaaaaaaaaaa", "aaa"),
        ("", "test"),
        ("test", ""),
        ("a".repeat(100).as_str(), "aaa"),
    ];
    
    for (haystack, needle) in test_cases {
        let haystack = haystack.to_string();
        
        // find
        assert_eq!(
            simd_engine.find(&haystack, needle),
            scalar_engine.find(&haystack, needle),
            "find mismatch for haystack='{}', needle='{}'",
            haystack,
            needle
        );
        
        // count
        assert_eq!(
            simd_engine.count(&haystack, needle),
            scalar_engine.count(&haystack, needle),
            "count mismatch for haystack='{}', needle='{}'",
            haystack,
            needle
        );
    }
    
    // eq
    let eq_cases = vec![
        ("hello", "hello"),
        ("hello", "world"),
        ("", ""),
        ("a".repeat(100).as_str(), "a".repeat(100).as_str()),
    ];
    
    for (a, b) in eq_cases {
        let a = a.to_string();
        let b = b.to_string();
        
        assert_eq!(
            simd_engine.eq(&a, &b),
            scalar_engine.eq(&a, &b),
            "eq mismatch for a='{}', b='{}'",
            a,
            b
        );
    }
    
    // case conversion
    let case_cases = vec![
        "HELLO",
        "hello",
        "Hello World",
        "123ABC",
        &"A".repeat(100),
    ];
    
    for s in case_cases {
        let s = s.to_string();
        
        assert_eq!(
            simd_engine.to_lowercase(&s),
            scalar_engine.to_lowercase(&s),
            "to_lowercase mismatch for s='{}'",
            s
        );
        
        assert_eq!(
            simd_engine.to_uppercase(&s),
            scalar_engine.to_uppercase(&s),
            "to_uppercase mismatch for s='{}'",
            s
        );
    }
}
