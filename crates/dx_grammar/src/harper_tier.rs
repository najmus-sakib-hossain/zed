//! Harper tier — fast local grammar checking (< 10ms).
//!
//! Uses the harper-core crate for basic English spell/grammar check.

use anyhow::Result;

use crate::diagnostic::{DiagnosticSource, GrammarDiagnostic, GrammarSeverity, Span};

/// Harper local grammar checker.
pub struct HarperTier {
    enabled: bool,
}

impl HarperTier {
    pub fn new() -> Self {
        Self { enabled: true }
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Run Harper on the given text, returning diagnostics.
    /// Placeholder until harper-core is wired in.
    pub fn check(&self, text: &str) -> Result<Vec<GrammarDiagnostic>> {
        if !self.enabled || text.is_empty() {
            return Ok(Vec::new());
        }

        let mut diagnostics = Vec::new();

        // Simple heuristic checks (placeholder for real Harper integration)
        self.check_double_spaces(text, &mut diagnostics);
        self.check_common_misspellings(text, &mut diagnostics);

        Ok(diagnostics)
    }

    fn check_double_spaces(&self, text: &str, out: &mut Vec<GrammarDiagnostic>) {
        let bytes = text.as_bytes();
        let mut i = 0;
        while i + 1 < bytes.len() {
            if bytes[i] == b' ' && bytes[i + 1] == b' ' {
                let start = i;
                let mut end = i + 2;
                while end < bytes.len() && bytes[end] == b' ' {
                    end += 1;
                }
                out.push(GrammarDiagnostic {
                    span: Span::new(start, end),
                    severity: GrammarSeverity::Style,
                    source: DiagnosticSource::Harper,
                    message: "Multiple consecutive spaces".into(),
                    suggestion: Some(" ".into()),
                    rule_id: Some("double_space".into()),
                });
                i = end;
            } else {
                i += 1;
            }
        }
    }

    fn check_common_misspellings(&self, text: &str, out: &mut Vec<GrammarDiagnostic>) {
        static MISSPELLINGS: &[(&str, &str)] = &[
            ("teh", "the"),
            ("recieve", "receive"),
            ("seperate", "separate"),
            ("occurence", "occurrence"),
            ("definately", "definitely"),
            ("accomodate", "accommodate"),
        ];

        let lower = text.to_lowercase();
        for &(wrong, correct) in MISSPELLINGS {
            let mut search_from = 0;
            while let Some(pos) = lower[search_from..].find(wrong) {
                let abs_pos = search_from + pos;
                // Only match whole words
                let before_ok = abs_pos == 0
                    || !text.as_bytes()[abs_pos - 1].is_ascii_alphanumeric();
                let after_pos = abs_pos + wrong.len();
                let after_ok = after_pos >= text.len()
                    || !text.as_bytes()[after_pos].is_ascii_alphanumeric();

                if before_ok && after_ok {
                    out.push(GrammarDiagnostic {
                        span: Span::new(abs_pos, after_pos),
                        severity: GrammarSeverity::Spelling,
                        source: DiagnosticSource::Harper,
                        message: format!("Possible misspelling: did you mean \"{}\"?", correct),
                        suggestion: Some(correct.into()),
                        rule_id: Some(format!("misspelling_{}", wrong)),
                    });
                }
                search_from = abs_pos + wrong.len();
            }
        }
    }
}

impl Default for HarperTier {
    fn default() -> Self {
        Self::new()
    }
}
