//! Fuzz test for the SIMD pattern scanner
//!
//! **Validates: Requirement 10.7 - Fuzz tests for parser edge cases**

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    use dx_check::scanner::PatternScanner;

    let scanner = PatternScanner::new();

    // The scanner should never panic on any input
    let _ = scanner.scan(data);
    let _ = scanner.has_any_match(data);
});
