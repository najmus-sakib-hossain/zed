//! Grammar diagnostic types — unified representation for all three tiers.

use serde::{Deserialize, Serialize};

/// Byte range within a document.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    pub fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }

    pub fn len(&self) -> usize {
        self.end.saturating_sub(self.start)
    }

    pub fn is_empty(&self) -> bool {
        self.start >= self.end
    }

    /// True if this span overlaps with another.
    pub fn overlaps(&self, other: &Span) -> bool {
        self.start < other.end && other.start < self.end
    }
}

/// Severity of a grammar issue — determines squiggly color.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum GrammarSeverity {
    /// Spelling error (red squiggly).
    Spelling,
    /// Grammar error (yellow squiggly).
    Grammar,
    /// Style suggestion (blue squiggly).
    Style,
    /// Clarity / readability (purple squiggly).
    Clarity,
}

impl GrammarSeverity {
    /// CSS-like color hint for the squiggly.
    pub fn color_hint(&self) -> &'static str {
        match self {
            Self::Spelling => "#ef4444",  // red
            Self::Grammar => "#eab308",   // yellow
            Self::Style => "#3b82f6",     // blue
            Self::Clarity => "#a855f7",   // purple
        }
    }
}

/// Which tier produced this diagnostic.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DiagnosticSource {
    Harper,
    Nlprule,
    Llm,
}

/// A single grammar diagnostic.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrammarDiagnostic {
    pub span: Span,
    pub severity: GrammarSeverity,
    pub source: DiagnosticSource,
    /// Human-readable message explaining the issue.
    pub message: String,
    /// Suggested replacement text (may be empty).
    pub suggestion: Option<String>,
    /// Rule ID for deduplication.
    pub rule_id: Option<String>,
}

impl GrammarDiagnostic {
    /// Deduplicate overlapping diagnostics: keep the one from the higher-priority source.
    pub fn deduplicate(mut diagnostics: Vec<GrammarDiagnostic>) -> Vec<GrammarDiagnostic> {
        if diagnostics.len() <= 1 {
            return diagnostics;
        }

        // Sort by span start, then by source priority (Harper < Nlprule < LLM)
        diagnostics.sort_by(|a, b| {
            a.span
                .start
                .cmp(&b.span.start)
                .then_with(|| source_priority(a.source).cmp(&source_priority(b.source)))
        });

        let mut result: Vec<GrammarDiagnostic> = Vec::new();
        for diag in diagnostics {
            let dominated = result.iter().any(|existing| {
                existing.span.overlaps(&diag.span)
                    && source_priority(existing.source) >= source_priority(diag.source)
            });
            if !dominated {
                // Remove any existing that this new one dominates
                result.retain(|existing| {
                    !(existing.span.overlaps(&diag.span)
                        && source_priority(existing.source) < source_priority(diag.source))
                });
                result.push(diag);
            }
        }
        result
    }
}

fn source_priority(source: DiagnosticSource) -> u8 {
    match source {
        DiagnosticSource::Harper => 0,
        DiagnosticSource::Nlprule => 1,
        DiagnosticSource::Llm => 2,
    }
}
