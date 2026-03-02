//! Writing profile — user preferences for grammar strictness and style.

use serde::{Deserialize, Serialize};

/// User's writing profile, affects which grammar rules are active.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WritingProfile {
    pub name: String,
    /// How strict grammar checking should be.
    pub strictness: Strictness,
    /// Type of writing context.
    pub context: WritingContext,
    /// Custom words to ignore (technical terms, names, etc.).
    pub custom_dictionary: Vec<String>,
    /// Disabled rule IDs.
    pub disabled_rules: Vec<String>,
}

/// Grammar strictness level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Strictness {
    /// Only flag clear errors (spelling, major grammar).
    Relaxed,
    /// Standard checking (spelling, grammar, some style).
    Standard,
    /// Strict checking (all rules enabled including clarity).
    Strict,
    /// Academic/formal writing (strictest).
    Academic,
}

/// Writing context affects which rules apply.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WritingContext {
    /// Casual writing (chat, messages).
    Casual,
    /// Professional emails and documents.
    Professional,
    /// Technical documentation and code comments.
    Technical,
    /// Academic papers and formal writing.
    Academic,
    /// Creative writing (fiction, poetry).
    Creative,
}

impl Default for WritingProfile {
    fn default() -> Self {
        Self {
            name: "Default".into(),
            strictness: Strictness::Standard,
            context: WritingContext::Professional,
            custom_dictionary: Vec::new(),
            disabled_rules: Vec::new(),
        }
    }
}

impl WritingProfile {
    pub fn casual() -> Self {
        Self {
            name: "Casual".into(),
            strictness: Strictness::Relaxed,
            context: WritingContext::Casual,
            ..Default::default()
        }
    }

    pub fn technical() -> Self {
        Self {
            name: "Technical".into(),
            strictness: Strictness::Standard,
            context: WritingContext::Technical,
            ..Default::default()
        }
    }

    pub fn academic() -> Self {
        Self {
            name: "Academic".into(),
            strictness: Strictness::Academic,
            context: WritingContext::Academic,
            ..Default::default()
        }
    }

    /// Whether a given rule ID should be checked.
    pub fn is_rule_enabled(&self, rule_id: &str) -> bool {
        !self.disabled_rules.iter().any(|r| r == rule_id)
    }

    /// Whether passive voice should be flagged.
    pub fn check_passive_voice(&self) -> bool {
        matches!(self.strictness, Strictness::Strict | Strictness::Academic)
    }

    /// Whether sentence length should be flagged.
    pub fn check_sentence_length(&self) -> bool {
        !matches!(self.strictness, Strictness::Relaxed)
    }

    /// Create a writing profile from an auto-detected app profile.
    pub fn from_app_profile(app_profile: &crate::app_detection::AppWritingProfile) -> Self {
        let strictness = if app_profile.grammar_strictness >= 0.8 {
            Strictness::Academic
        } else if app_profile.grammar_strictness >= 0.6 {
            Strictness::Strict
        } else if app_profile.grammar_strictness >= 0.3 {
            Strictness::Standard
        } else {
            Strictness::Relaxed
        };

        let context = match app_profile.tone {
            crate::app_detection::Tone::Formal | crate::app_detection::Tone::Professional => WritingContext::Professional,
            crate::app_detection::Tone::Casual => WritingContext::Casual,
            crate::app_detection::Tone::Technical => WritingContext::Technical,
            crate::app_detection::Tone::Creative => WritingContext::Creative,
        };

        Self {
            name: format!("Auto ({})", app_profile.tone.as_str()),
            strictness,
            context,
            custom_dictionary: Vec::new(),
            disabled_rules: Vec::new(),
        }
    }
}
