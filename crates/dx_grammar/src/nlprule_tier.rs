//! nlprule tier — LanguageTool-quality grammar checking (< 50ms local).

use anyhow::Result;

use crate::diagnostic::{DiagnosticSource, GrammarDiagnostic, GrammarSeverity, Span};

/// nlprule grammar checker.
pub struct NlpruleTier {
    enabled: bool,
}

impl NlpruleTier {
    pub fn new() -> Self {
        Self { enabled: true }
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Run nlprule on the given text.
    /// Placeholder until nlprule crate is wired in.
    pub fn check(&self, text: &str) -> Result<Vec<GrammarDiagnostic>> {
        if !self.enabled || text.is_empty() {
            return Ok(Vec::new());
        }

        let mut diagnostics = Vec::new();

        self.check_passive_voice(text, &mut diagnostics);
        self.check_sentence_length(text, &mut diagnostics);

        Ok(diagnostics)
    }

    /// Flag passive voice constructions.
    fn check_passive_voice(&self, text: &str, out: &mut Vec<GrammarDiagnostic>) {
        // Simple pattern matching for common passive constructions
        static PASSIVE_PATTERNS: &[&str] = &[
            "was done",
            "was made",
            "was given",
            "were done",
            "were made",
            "is being",
            "was being",
            "has been",
            "had been",
            "will be done",
        ];

        let lower = text.to_lowercase();
        for pattern in PASSIVE_PATTERNS {
            let mut search_from = 0;
            while let Some(pos) = lower[search_from..].find(pattern) {
                let abs_pos = search_from + pos;
                let end = abs_pos + pattern.len();
                out.push(GrammarDiagnostic {
                    span: Span::new(abs_pos, end),
                    severity: GrammarSeverity::Style,
                    source: DiagnosticSource::Nlprule,
                    message: "Consider using active voice for clarity.".into(),
                    suggestion: None,
                    rule_id: Some("passive_voice".into()),
                });
                search_from = end;
            }
        }
    }

    /// Flag very long sentences.
    fn check_sentence_length(&self, text: &str, out: &mut Vec<GrammarDiagnostic>) {
        let mut pos = 0;
        for sentence in text.split(|c: char| c == '.' || c == '!' || c == '?') {
            let trimmed = sentence.trim();
            let word_count = trimmed.split_whitespace().count();
            if word_count > 40 {
                out.push(GrammarDiagnostic {
                    span: Span::new(pos, pos + sentence.len()),
                    severity: GrammarSeverity::Clarity,
                    source: DiagnosticSource::Nlprule,
                    message: format!(
                        "Sentence is {} words — consider splitting for readability.",
                        word_count
                    ),
                    suggestion: None,
                    rule_id: Some("long_sentence".into()),
                });
            }
            pos += sentence.len() + 1; // +1 for the delimiter
        }
    }
}

impl Default for NlpruleTier {
    fn default() -> Self {
        Self::new()
    }
}
