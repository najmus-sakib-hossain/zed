//! Wispr Flow processor - fast text enhancement

use crate::error::Result;
use std::time::Instant;

/// Wispr Flow processor
pub struct WisprFlow;

/// Processing result with timing information
#[derive(Debug)]
pub struct WisprFlowResult {
    pub raw_transcript: String,
    pub enhanced_text: String,
    pub grammar_issues: usize,
    pub style_score: f64,
    pub enhancement_time_ms: u128,
    pub total_time_ms: u128,
}

impl WisprFlow {
    /// Create a new Wispr Flow processor
    pub fn new() -> Result<Self> {
        Ok(Self)
    }

    /// Process raw text transcript with fast auto-fix
    pub fn process_text(&self, raw_text: &str) -> Result<WisprFlowResult> {
        let start_total = Instant::now();
        let start_enhance = Instant::now();

        let enhanced_text = Self::quick_fix(raw_text);

        let enhancement_time_ms = start_enhance.elapsed().as_millis();
        let total_time_ms = start_total.elapsed().as_millis();

        Ok(WisprFlowResult {
            raw_transcript: raw_text.to_string(),
            enhanced_text,
            grammar_issues: 0,
            style_score: 100.0,
            enhancement_time_ms,
            total_time_ms,
        })
    }

    /// Quick text fixes
    fn quick_fix(text: &str) -> String {
        let mut result = text.to_string();

        // Fix common errors
        result = result.replace(" i ", " I ");
        result = result.replace(" im ", " I'm ");
        result = result.replace(" dont ", " don't ");
        result = result.replace(" cant ", " can't ");
        result = result.replace(" wont ", " won't ");
        result = result.replace(" didnt ", " didn't ");
        result = result.replace(" isnt ", " isn't ");
        result = result.replace(" arent ", " aren't ");
        result = result.replace(" wasnt ", " wasn't ");
        result = result.replace(" werent ", " weren't ");

        // Capitalize first letter
        if let Some(first) = result.chars().next() {
            result = first.to_uppercase().collect::<String>() + &result[first.len_utf8()..];
        }

        // Add period
        let trimmed = result.trim();
        if !trimmed.is_empty()
            && !trimmed.ends_with('.')
            && !trimmed.ends_with('?')
            && !trimmed.ends_with('!')
        {
            result = format!("{}.", trimmed);
        }

        result
    }
}
