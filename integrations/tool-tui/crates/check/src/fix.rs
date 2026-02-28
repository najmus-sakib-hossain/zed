//! Fix Engine
//!
//! Predictive fix engine with pre-compiled fix templates.
//! Applies fixes in microseconds via pattern matching.

use crate::diagnostics::{Diagnostic, Edit, Fix};
use std::collections::HashMap;

/// Pre-compiled fix template
#[derive(Clone)]
pub struct FixTemplate {
    /// Rule ID this fix applies to
    pub rule_id: String,
    /// Pattern to match
    pub pattern: FixPattern,
    /// Replacement template
    pub replacement: ReplacementTemplate,
}

/// Pattern for matching code to fix
#[derive(Clone)]
pub enum FixPattern {
    /// Exact string match
    Exact(Vec<u8>),
    /// Simple pattern with wildcards
    Wildcard(String),
}

/// Template for replacement text
#[derive(Clone)]
pub struct ReplacementTemplate {
    /// Segments of the replacement
    pub segments: Vec<Segment>,
}

#[derive(Clone)]
pub enum Segment {
    /// Literal text
    Literal(Vec<u8>),
    /// Captured group reference (1-indexed)
    Capture(u8),
}

impl ReplacementTemplate {
    /// Create a simple literal replacement
    #[must_use]
    pub fn literal(text: &str) -> Self {
        Self {
            segments: vec![Segment::Literal(text.as_bytes().to_vec())],
        }
    }

    /// Apply template with captures
    #[must_use]
    pub fn apply(&self, captures: &[&[u8]]) -> Vec<u8> {
        let mut result = Vec::new();
        for segment in &self.segments {
            match segment {
                Segment::Literal(lit) => result.extend(lit),
                Segment::Capture(n) => {
                    if let Some(&capture) = captures.get(*n as usize) {
                        result.extend(capture);
                    }
                }
            }
        }
        result
    }
}

/// Fix engine with pre-compiled templates
pub struct FixEngine {
    /// Fix templates by rule ID
    templates: HashMap<String, Vec<FixTemplate>>,
}

impl FixEngine {
    /// Create a new fix engine with built-in templates
    #[must_use]
    pub fn new() -> Self {
        let mut engine = Self {
            templates: HashMap::new(),
        };
        engine.register_builtin_fixes();
        engine
    }

    /// Register built-in fix templates
    fn register_builtin_fixes(&mut self) {
        // eqeqeq: == to ===
        self.register(FixTemplate {
            rule_id: "eqeqeq".to_string(),
            pattern: FixPattern::Exact(b"==".to_vec()),
            replacement: ReplacementTemplate::literal("==="),
        });

        // eqeqeq: != to !==
        self.register(FixTemplate {
            rule_id: "eqeqeq".to_string(),
            pattern: FixPattern::Exact(b"!=".to_vec()),
            replacement: ReplacementTemplate::literal("!=="),
        });

        // no-var: var to let
        self.register(FixTemplate {
            rule_id: "no-var".to_string(),
            pattern: FixPattern::Exact(b"var ".to_vec()),
            replacement: ReplacementTemplate::literal("let "),
        });
    }

    /// Register a fix template
    pub fn register(&mut self, template: FixTemplate) {
        self.templates.entry(template.rule_id.clone()).or_default().push(template);
    }

    /// Apply a single fix to source
    #[must_use]
    pub fn apply_fix(&self, source: &[u8], fix: &Fix) -> Vec<u8> {
        // Sort edits by position (reverse order for safe application)
        let mut edits = fix.edits.clone();
        edits.sort_by_key(|e| std::cmp::Reverse(e.span.start));

        let mut current_source = source.to_vec();
        for edit in &edits {
            let start = edit.span.start as usize;
            let end = edit.span.end as usize;

            // Bounds check - skip invalid edits
            if start > current_source.len() {
                tracing::warn!(
                    "Edit start {} beyond source length {}",
                    start,
                    current_source.len()
                );
                continue;
            }

            let safe_end = end.min(current_source.len());
            if start > safe_end {
                tracing::warn!("Edit start {} after end {}", start, safe_end);
                continue;
            }

            let mut new_source = Vec::with_capacity(current_source.len());
            new_source.extend(&current_source[..start]);
            new_source.extend(edit.new_text.as_bytes());
            new_source.extend(&current_source[safe_end..]);

            current_source = new_source;
        }

        current_source
    }

    /// Apply all fixes from diagnostics
    #[must_use]
    pub fn apply_all_fixes(&self, source: &[u8], diagnostics: &[Diagnostic]) -> Vec<u8> {
        // Collect all fixes
        let mut all_edits: Vec<Edit> = diagnostics
            .iter()
            .filter_map(|d| d.fix.as_ref())
            .flat_map(|f| f.edits.iter().cloned())
            .collect();

        // Sort by position (reverse order)
        all_edits.sort_by_key(|e| std::cmp::Reverse(e.span.start));

        // Check for overlapping edits and remove them
        let mut filtered_edits: Vec<Edit> = Vec::new();
        let mut last_start = usize::MAX;

        for edit in all_edits {
            let end = edit.span.end as usize;
            if end <= last_start {
                last_start = edit.span.start as usize;
                filtered_edits.push(edit);
            }
            // Skip overlapping edits
        }

        // Apply edits
        let mut result = source.to_vec();
        for edit in filtered_edits {
            let start = edit.span.start as usize;
            let end = edit.span.end as usize;

            // Bounds check to prevent panic
            if start > result.len() || end > result.len() || start > end {
                tracing::warn!(
                    "Invalid edit span: start={}, end={}, result_len={}",
                    start,
                    end,
                    result.len()
                );
                continue;
            }

            let mut new_result = Vec::with_capacity(result.len());
            new_result.extend(&result[..start]);
            new_result.extend(edit.new_text.as_bytes());
            new_result.extend(&result[end..]);

            result = new_result;
        }

        result
    }

    /// Get fix template for a rule
    #[must_use]
    pub fn get_templates(&self, rule_id: &str) -> Option<&Vec<FixTemplate>> {
        self.templates.get(rule_id)
    }
}

impl Default for FixEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// XOR differential patch for efficient fix transmission
#[derive(Debug, Clone)]
pub struct XorPatch {
    /// Hash of original content
    pub base_hash: [u8; 32],
    /// XOR chunks
    pub chunks: Vec<XorChunk>,
}

#[derive(Debug, Clone)]
pub struct XorChunk {
    /// Offset in source
    pub offset: u32,
    /// XOR data
    pub xor_data: Vec<u8>,
}

impl XorPatch {
    /// Compute XOR patch between original and fixed content
    #[must_use]
    pub fn compute(original: &[u8], fixed: &[u8]) -> Self {
        let mut chunks = Vec::new();
        let max_len = original.len().max(fixed.len());

        let mut i = 0;
        while i < max_len {
            let orig = original.get(i).copied().unwrap_or(0);
            let fix = fixed.get(i).copied().unwrap_or(0);

            if orig == fix {
                i += 1;
            } else {
                // Start a new chunk
                let offset = i as u32;
                let mut xor_data = Vec::new();

                while i < max_len {
                    let orig = original.get(i).copied().unwrap_or(0);
                    let fix = fixed.get(i).copied().unwrap_or(0);

                    if orig == fix {
                        break;
                    }

                    xor_data.push(orig ^ fix);
                    i += 1;
                }

                chunks.push(XorChunk { offset, xor_data });
            }
        }

        Self {
            base_hash: *blake3::hash(original).as_bytes(),
            chunks,
        }
    }

    /// Apply XOR patch to original content
    #[must_use]
    pub fn apply(&self, original: &[u8]) -> Vec<u8> {
        let mut result = original.to_vec();

        for chunk in &self.chunks {
            for (i, &xor_byte) in chunk.xor_data.iter().enumerate() {
                let pos = chunk.offset as usize + i;
                if pos < result.len() {
                    result[pos] ^= xor_byte;
                } else {
                    // Handle length changes
                    while result.len() < pos {
                        result.push(0);
                    }
                    result.push(xor_byte);
                }
            }
        }

        result
    }

    /// Get total patch size in bytes
    #[must_use]
    pub fn size(&self) -> usize {
        32 + // base_hash
        self.chunks.iter().map(|c| 4 + c.xor_data.len()).sum::<usize>()
    }

    /// Serialize to bytes
    #[must_use]
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(self.size());
        bytes.extend(&self.base_hash);
        for chunk in &self.chunks {
            bytes.extend(&chunk.offset.to_le_bytes());
            bytes.extend(&(chunk.xor_data.len() as u16).to_le_bytes());
            bytes.extend(&chunk.xor_data);
        }
        bytes
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostics::Span;

    #[test]
    fn test_replacement_template() {
        let template = ReplacementTemplate {
            segments: vec![
                Segment::Literal(b"const ".to_vec()),
                Segment::Capture(0),
                Segment::Literal(b" = ".to_vec()),
                Segment::Capture(1),
            ],
        };

        let result = template.apply(&[b"x", b"42"]);
        assert_eq!(result, b"const x = 42");
    }

    #[test]
    fn test_apply_fix() {
        let engine = FixEngine::new();
        let source = b"if (x == y) {}";

        let fix = Fix {
            description: "Use ===".to_string(),
            edits: vec![Edit {
                span: Span::new(6, 8),
                new_text: "===".to_string(),
            }],
        };

        let result = engine.apply_fix(source, &fix);
        assert_eq!(result, b"if (x === y) {}");
    }

    #[test]
    fn test_xor_patch() {
        let original = b"const x = foo();";
        let fixed = b"const x = bar();";

        let patch = XorPatch::compute(original, fixed);
        let applied = patch.apply(original);

        assert_eq!(applied, fixed);
        // XOR patch captures differences, size depends on diff spread
        assert!(patch.size() > 0);
    }

    #[test]
    fn test_apply_multiple_non_overlapping_fixes() {
        let engine = FixEngine::new();
        let source = b"var x = 1; var y = 2;";

        let diagnostics = vec![
            Diagnostic {
                file: std::path::PathBuf::from("test.js"),
                span: Span::new(0, 3),
                severity: crate::diagnostics::DiagnosticSeverity::Warning,
                rule_id: "no-var".to_string(),
                message: "Use let instead of var".to_string(),
                suggestion: None,
                related: Vec::new(),
                fix: Some(Fix {
                    description: "Replace var with let".to_string(),
                    edits: vec![Edit {
                        span: Span::new(0, 3),
                        new_text: "let".to_string(),
                    }],
                }),
            },
            Diagnostic {
                file: std::path::PathBuf::from("test.js"),
                span: Span::new(11, 14),
                severity: crate::diagnostics::DiagnosticSeverity::Warning,
                rule_id: "no-var".to_string(),
                message: "Use let instead of var".to_string(),
                suggestion: None,
                related: Vec::new(),
                fix: Some(Fix {
                    description: "Replace var with let".to_string(),
                    edits: vec![Edit {
                        span: Span::new(11, 14),
                        new_text: "let".to_string(),
                    }],
                }),
            },
        ];

        let result = engine.apply_all_fixes(source, &diagnostics);
        assert_eq!(result, b"let x = 1; let y = 2;");
    }

    #[test]
    fn test_overlapping_fixes_are_filtered() {
        let engine = FixEngine::new();
        let source = b"x == y";

        // Two overlapping fixes for the same span
        let diagnostics = vec![
            Diagnostic {
                file: std::path::PathBuf::from("test.js"),
                span: Span::new(2, 4),
                severity: crate::diagnostics::DiagnosticSeverity::Warning,
                rule_id: "eqeqeq".to_string(),
                message: "Use ===".to_string(),
                suggestion: None,
                related: Vec::new(),
                fix: Some(Fix {
                    description: "Replace == with ===".to_string(),
                    edits: vec![Edit {
                        span: Span::new(2, 4),
                        new_text: "===".to_string(),
                    }],
                }),
            },
            Diagnostic {
                file: std::path::PathBuf::from("test.js"),
                span: Span::new(2, 4),
                severity: crate::diagnostics::DiagnosticSeverity::Warning,
                rule_id: "eqeqeq".to_string(),
                message: "Use ===".to_string(),
                suggestion: None,
                related: Vec::new(),
                fix: Some(Fix {
                    description: "Replace == with ===".to_string(),
                    edits: vec![Edit {
                        span: Span::new(2, 4),
                        new_text: "===".to_string(),
                    }],
                }),
            },
        ];

        // Should only apply one fix, not corrupt the output
        let result = engine.apply_all_fixes(source, &diagnostics);
        assert_eq!(result, b"x === y");
    }
}

#[cfg(test)]
mod property_tests {
    use super::*;
    use crate::diagnostics::{Diagnostic, DiagnosticSeverity, Span};
    use proptest::prelude::*;

    // Generator for valid spans within a source length
    fn arb_span_within(source_len: usize) -> impl Strategy<Value = Span> {
        if source_len == 0 {
            Just(Span::new(0, 0)).boxed()
        } else {
            (0..source_len, 0..source_len)
                .prop_map(|(a, b)| {
                    let (start, end) = if a <= b { (a, b) } else { (b, a) };
                    Span::new(start as u32, end as u32)
                })
                .boxed()
        }
    }

    // Generator for non-overlapping spans
    fn arb_non_overlapping_spans(
        source_len: usize,
        count: usize,
    ) -> impl Strategy<Value = Vec<Span>> {
        if source_len < count * 2 || count == 0 {
            Just(Vec::new()).boxed()
        } else {
            // Generate sorted, non-overlapping spans
            let segment_size = source_len / count.max(1);
            (0..count)
                .map(|i| {
                    let base = i * segment_size;
                    let max_end = ((i + 1) * segment_size).min(source_len);
                    if base >= max_end {
                        Just(Span::new(base as u32, base as u32)).boxed()
                    } else {
                        (base..max_end, base..max_end)
                            .prop_map(move |(a, b)| {
                                let (start, end) = if a <= b { (a, b) } else { (b, a) };
                                Span::new(start as u32, end as u32)
                            })
                            .boxed()
                    }
                })
                .collect::<Vec<_>>()
                .prop_map(|spans| spans.into_iter().collect())
                .boxed()
        }
    }

    // Generator for replacement text
    fn arb_replacement() -> impl Strategy<Value = String> {
        "[a-zA-Z0-9 ]{0,20}".prop_map(String::from)
    }

    // Generator for source code
    fn arb_source() -> impl Strategy<Value = Vec<u8>> {
        "[a-zA-Z0-9 ;=(){}]{10,100}".prop_map(|s| s.into_bytes())
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// **Property 2: Fix Application Correctness**
        /// *For any* file with fixable rule violations, applying the generated fixes SHALL result in
        /// a file that no longer triggers those specific violations when re-checked.
        /// **Validates: Requirements 2.3, 3.4**
        ///
        /// This property tests that:
        /// 1. Applying a fix produces valid output (no panics, no corruption)
        /// 2. The fix is applied at the correct position
        /// 3. Non-overlapping fixes can all be applied
        #[test]
        fn prop_fix_application_produces_valid_output(
            source in arb_source(),
        ) {
            let engine = FixEngine::new();
            let source_len = source.len();

            if source_len >= 4 {
                // Create a simple fix that replaces a portion of the source
                let start = source_len / 4;
                let end = source_len / 2;
                let fix = Fix {
                    description: "Test fix".to_string(),
                    edits: vec![Edit {
                        span: Span::new(start as u32, end as u32),
                        new_text: "REPLACED".to_string(),
                    }],
                };

                let result = engine.apply_fix(&source, &fix);

                // Result should be valid (not empty unless source was empty)
                prop_assert!(!result.is_empty() || source.is_empty());

                // Result should contain the replacement text
                let result_str = String::from_utf8_lossy(&result);
                prop_assert!(result_str.contains("REPLACED"), "Result should contain replacement text");

                // Result length should be: original - removed + added
                let expected_len = source_len - (end - start) + "REPLACED".len();
                prop_assert_eq!(result.len(), expected_len, "Result length should match expected");
            }
        }

        /// **Property 2 (continued): Multiple Non-Overlapping Fixes**
        /// Applying multiple non-overlapping fixes should apply all of them correctly.
        #[test]
        fn prop_multiple_non_overlapping_fixes_all_applied(
            source in "[a-zA-Z0-9 ]{50,100}".prop_map(|s| s.into_bytes()),
        ) {
            let engine = FixEngine::new();
            let source_len = source.len();

            // Create two non-overlapping fixes
            if source_len >= 20 {
                let fix1_start = 0;
                let fix1_end = 5;
                let fix2_start = source_len - 5;
                let fix2_end = source_len;

                let diagnostics = vec![
                    Diagnostic {
                        file: std::path::PathBuf::from("test.js"),
                        span: Span::new(fix1_start as u32, fix1_end as u32),
                        severity: DiagnosticSeverity::Warning,
                        rule_id: "test".to_string(),
                        message: "Test".to_string(),
                        suggestion: None,
                        related: Vec::new(),
                        fix: Some(Fix {
                            description: "Fix 1".to_string(),
                            edits: vec![Edit {
                                span: Span::new(fix1_start as u32, fix1_end as u32),
                                new_text: "AAA".to_string(),
                            }],
                        }),
                    },
                    Diagnostic {
                        file: std::path::PathBuf::from("test.js"),
                        span: Span::new(fix2_start as u32, fix2_end as u32),
                        severity: DiagnosticSeverity::Warning,
                        rule_id: "test".to_string(),
                        message: "Test".to_string(),
                        suggestion: None,
                        related: Vec::new(),
                        fix: Some(Fix {
                            description: "Fix 2".to_string(),
                            edits: vec![Edit {
                                span: Span::new(fix2_start as u32, fix2_end as u32),
                                new_text: "BBB".to_string(),
                            }],
                        }),
                    },
                ];

                let result = engine.apply_all_fixes(&source, &diagnostics);
                let result_str = String::from_utf8_lossy(&result);

                // Both fixes should be applied
                prop_assert!(result_str.starts_with("AAA"), "First fix should be applied at start");
                prop_assert!(result_str.ends_with("BBB"), "Second fix should be applied at end");
            }
        }

        /// **Property 2 (continued): Overlapping Fixes Are Handled Safely**
        /// Overlapping fixes should not corrupt the output.
        #[test]
        fn prop_overlapping_fixes_handled_safely(
            source in "[a-zA-Z0-9 ]{20,50}".prop_map(|s| s.into_bytes()),
        ) {
            let engine = FixEngine::new();
            let source_len = source.len();

            if source_len >= 10 {
                // Create two overlapping fixes
                let diagnostics = vec![
                    Diagnostic {
                        file: std::path::PathBuf::from("test.js"),
                        span: Span::new(5, 15),
                        severity: DiagnosticSeverity::Warning,
                        rule_id: "test".to_string(),
                        message: "Test".to_string(),
                        suggestion: None,
                        related: Vec::new(),
                        fix: Some(Fix {
                            description: "Fix 1".to_string(),
                            edits: vec![Edit {
                                span: Span::new(5, 15),
                                new_text: "XXX".to_string(),
                            }],
                        }),
                    },
                    Diagnostic {
                        file: std::path::PathBuf::from("test.js"),
                        span: Span::new(10, 20.min(source_len as u32)),
                        severity: DiagnosticSeverity::Warning,
                        rule_id: "test".to_string(),
                        message: "Test".to_string(),
                        suggestion: None,
                        related: Vec::new(),
                        fix: Some(Fix {
                            description: "Fix 2".to_string(),
                            edits: vec![Edit {
                                span: Span::new(10, 20.min(source_len as u32)),
                                new_text: "YYY".to_string(),
                            }],
                        }),
                    },
                ];

                let result = engine.apply_all_fixes(&source, &diagnostics);

                // Result should be valid UTF-8 (or at least not corrupted)
                prop_assert!(!result.is_empty(), "Result should not be empty");

                // At least one fix should be applied (the non-overlapping one)
                let result_str = String::from_utf8_lossy(&result);
                let has_fix = result_str.contains("XXX") || result_str.contains("YYY");
                prop_assert!(has_fix, "At least one fix should be applied");
            }
        }

        /// **Property 2 (continued): Empty Fix Does Not Change Source**
        /// A fix with no edits should not change the source.
        #[test]
        fn prop_empty_fix_preserves_source(source in arb_source()) {
            let engine = FixEngine::new();

            let fix = Fix {
                description: "Empty fix".to_string(),
                edits: Vec::new(),
            };

            let result = engine.apply_fix(&source, &fix);
            prop_assert_eq!(result, source, "Empty fix should not change source");
        }

        /// **Property 2 (continued): Deletion Fix Removes Content**
        /// A fix that replaces content with empty string should delete that content.
        #[test]
        fn prop_deletion_fix_removes_content(
            source in "[a-zA-Z0-9]{20,50}".prop_map(|s| s.into_bytes()),
        ) {
            let engine = FixEngine::new();
            let source_len = source.len();

            if source_len >= 10 {
                let start = 5;
                let end = 10;
                let deleted_len = end - start;

                let fix = Fix {
                    description: "Delete content".to_string(),
                    edits: vec![Edit {
                        span: Span::new(start as u32, end as u32),
                        new_text: String::new(),
                    }],
                };

                let result = engine.apply_fix(&source, &fix);

                // Result should be shorter by the deleted amount
                prop_assert_eq!(result.len(), source_len - deleted_len, "Result should be shorter");

                // Content before and after deletion should be preserved
                prop_assert_eq!(&result[..start], &source[..start], "Content before deletion preserved");
                prop_assert_eq!(&result[start..], &source[end..], "Content after deletion preserved");
            }
        }
    }
}
