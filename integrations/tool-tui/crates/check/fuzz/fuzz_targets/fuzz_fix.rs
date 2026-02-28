//! Fuzz test for fix application
//!
//! **Validates: Requirement 10.7 - Fuzz tests for parser edge cases**

#![no_main]

use arbitrary::Arbitrary;
use libfuzzer_sys::fuzz_target;

/// Arbitrary input for fix fuzzing
#[derive(Debug, Arbitrary)]
struct FixInput {
    /// Source content
    source: Vec<u8>,
    /// Fix type
    fix_type: FixType,
    /// Start position
    start: u32,
    /// End position
    end: u32,
    /// Replacement text (for replace/insert)
    replacement: String,
}

#[derive(Debug, Arbitrary)]
enum FixType {
    Delete,
    Replace,
    Insert,
}

fuzz_target!(|input: FixInput| {
    use dx_check::diagnostics::{Fix, Span};
    use dx_check::fix::FixEngine;

    let engine = FixEngine::new();

    // Normalize positions
    let start = input.start.min(input.source.len() as u32);
    let end = input.end.min(input.source.len() as u32);
    let (start, end) = if start <= end { (start, end) } else { (end, start) };

    let fix = match input.fix_type {
        FixType::Delete => Fix::delete("Fuzz delete", Span::new(start, end)),
        FixType::Replace => Fix::replace("Fuzz replace", Span::new(start, end), &input.replacement),
        FixType::Insert => Fix::insert("Fuzz insert", start, &input.replacement),
    };

    // The fix engine should never panic on any input
    let _ = engine.apply_fix(&input.source, &fix);
});
