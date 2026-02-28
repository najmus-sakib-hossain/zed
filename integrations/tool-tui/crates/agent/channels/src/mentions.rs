//! @mention detection and stripping utilities.
//!
//! Detects agent mentions in incoming messages and strips
//! them so the agent sees clean text without its own name.

use serde::{Deserialize, Serialize};

/// A pattern used to detect mentions of a specific agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MentionPattern {
    /// The text pattern to search for (e.g. `"@dx_bot"`).
    pub pattern: String,
    /// The agent ID this mention maps to.
    pub agent_id: String,
}

/// Detect all agent mentions in `text`.
///
/// Returns a list of matching agent IDs (deduplicated).
pub fn detect_mentions(text: &str, patterns: &[MentionPattern]) -> Vec<String> {
    let lower = text.to_lowercase();
    let mut found = Vec::new();
    for p in patterns {
        let needle = p.pattern.to_lowercase();
        if lower.contains(&needle) && !found.contains(&p.agent_id) {
            found.push(p.agent_id.clone());
        }
    }
    found
}

/// Strip all known mention patterns from `text`.
///
/// Replaces each pattern with an empty string and collapses
/// extra whitespace.
pub fn strip_mentions(text: &str, patterns: &[MentionPattern]) -> String {
    let mut result = text.to_string();
    for p in patterns {
        // Case-insensitive removal — rebuild on each pattern
        loop {
            let lower_result = result.to_lowercase();
            let needle = p.pattern.to_lowercase();
            if let Some(pos) = lower_result.find(&needle) {
                result = format!("{}{}", &result[..pos], &result[pos + p.pattern.len()..]);
            } else {
                break;
            }
        }
    }
    // Collapse multiple spaces
    collapse_whitespace(&result)
}

/// Extract the first `@agent_id` token from text.
///
/// Looks for patterns like `@some_id` and returns the id
/// portion (without the `@`).
pub fn extract_agent_id(text: &str) -> Option<String> {
    for word in text.split_whitespace() {
        if let Some(stripped) = word.strip_prefix('@') {
            let id: String =
                stripped.chars().take_while(|c| c.is_alphanumeric() || *c == '_').collect();
            if !id.is_empty() {
                return Some(id);
            }
        }
    }
    None
}

/// Check if the text contains any mention from the patterns.
pub fn has_mention(text: &str, patterns: &[MentionPattern]) -> bool {
    let lower = text.to_lowercase();
    patterns.iter().any(|p| lower.contains(&p.pattern.to_lowercase()))
}

/// Collapse runs of whitespace into single spaces and trim.
fn collapse_whitespace(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut prev_space = false;
    for ch in s.chars() {
        if ch.is_whitespace() {
            if !prev_space && !result.is_empty() {
                result.push(' ');
            }
            prev_space = true;
        } else {
            result.push(ch);
            prev_space = false;
        }
    }
    result.trim_end().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_patterns() -> Vec<MentionPattern> {
        vec![
            MentionPattern {
                pattern: "@dx_bot".into(),
                agent_id: "dx".into(),
            },
            MentionPattern {
                pattern: "@helper".into(),
                agent_id: "helper".into(),
            },
        ]
    }

    #[test]
    fn test_detect_mentions() {
        let patterns = test_patterns();

        let found = detect_mentions("Hey @dx_bot, help me!", &patterns);
        assert_eq!(found, vec!["dx"]);

        let found = detect_mentions("@dx_bot and @helper both here", &patterns);
        assert_eq!(found.len(), 2);
    }

    #[test]
    fn test_detect_mentions_case_insensitive() {
        let patterns = test_patterns();
        let found = detect_mentions("Hey @DX_BOT!", &patterns);
        assert_eq!(found, vec!["dx"]);
    }

    #[test]
    fn test_detect_mentions_none() {
        let patterns = test_patterns();
        let found = detect_mentions("No mention here", &patterns);
        assert!(found.is_empty());
    }

    #[test]
    fn test_strip_mentions() {
        let patterns = test_patterns();
        let result = strip_mentions("Hey @dx_bot do this", &patterns);
        assert_eq!(result, "Hey do this");
    }

    #[test]
    fn test_strip_multiple_mentions() {
        let patterns = test_patterns();
        let result = strip_mentions("@dx_bot @helper please help", &patterns);
        // Both should be stripped
        assert!(!result.contains("@dx_bot"));
        assert!(!result.contains("@helper"));
    }

    #[test]
    fn test_extract_agent_id() {
        assert_eq!(extract_agent_id("Hello @dx_bot world"), Some("dx_bot".into()));
        assert_eq!(extract_agent_id("@user123 hi"), Some("user123".into()));
        assert_eq!(extract_agent_id("no mention"), None);
    }

    #[test]
    fn test_has_mention() {
        let patterns = test_patterns();
        assert!(has_mention("Hi @dx_bot", &patterns));
        assert!(!has_mention("Hi everyone", &patterns));
    }

    #[test]
    fn test_collapse_whitespace() {
        assert_eq!(collapse_whitespace("  a   b  c  "), "a b c");
        assert_eq!(collapse_whitespace("single"), "single");
    }
}
