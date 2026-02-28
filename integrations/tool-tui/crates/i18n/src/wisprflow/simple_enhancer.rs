//! Simple rule-based text enhancement (no LLM)

use crate::error::Result;
use lazy_static::lazy_static;
use regex::Regex;

lazy_static! {
    static ref FILLER_WORDS: Regex = Regex::new(
        r"(?i)\b(um|uh|like|you know|so|well|actually|basically|literally|kind of|sort of|i mean|right)\b"
    ).unwrap();

    static ref MULTIPLE_SPACES: Regex = Regex::new(r"\s+").unwrap();
}

/// Simple text enhancer using rules
pub struct SimpleEnhancer;

impl SimpleEnhancer {
    pub fn new() -> Self {
        Self
    }

    /// Enhance text with basic rules
    pub fn enhance(&self, text: &str) -> Result<String> {
        let mut result = text.to_string();

        // Remove filler words
        result = FILLER_WORDS.replace_all(&result, "").to_string();

        // Remove repeated words manually (regex doesn't support backreferences)
        let words: Vec<&str> = result.split_whitespace().collect();
        let mut deduped = Vec::new();
        let mut prev = "";
        for word in words {
            if word != prev {
                deduped.push(word);
            }
            prev = word;
        }
        result = deduped.join(" ");

        // Normalize spaces
        result = MULTIPLE_SPACES.replace_all(&result, " ").to_string();

        // Trim
        result = result.trim().to_string();

        // Capitalize first letter
        if !result.is_empty() {
            let mut chars: Vec<char> = result.chars().collect();
            chars[0] = chars[0].to_uppercase().next().unwrap();
            result = chars.into_iter().collect();
        }

        // Add period if missing
        if !result.is_empty()
            && !result.ends_with('.')
            && !result.ends_with('?')
            && !result.ends_with('!')
        {
            result.push('.');
        }

        Ok(result)
    }
}
