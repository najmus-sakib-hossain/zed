//! Token counting for LLM serializer output
//!
//! Provides token counting and measurement for various LLM models:
//! - **GPT-4o / o1**: Uses `o200k_base` tokenizer via tiktoken-rs
//! - **Gemini 3**: Uses SentencePiece tokenizer via tokenizers crate
//! - **Claude Opus 4.5**: Uses approximate BPE tokenizer
//!
//! ## Usage
//!
//! ```rust
//! use serializer::llm::tokens::{TokenCounter, ModelType, TokenInfo};
//!
//! let counter = TokenCounter::new();
//! let text = "Hello, world!";
//!
//! // Count tokens for GPT-4o
//! let info = counter.count(text, ModelType::Gpt4o);
//! assert!(info.count > 0);
//! println!("Token count: {}", info.count);
//! ```

use std::collections::HashMap;

/// Supported LLM model types for token counting
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ModelType {
    /// OpenAI GPT-4o (uses o200k_base tokenizer)
    Gpt4o,
    /// OpenAI o1 model (uses o200k_base tokenizer)
    O1,
    /// OpenAI GPT-4 (uses cl100k_base tokenizer)
    Gpt4,
    /// Google Gemini 3 (uses SentencePiece tokenizer)
    Gemini3,
    /// Anthropic Claude Opus 4.5 (uses BPE tokenizer)
    ClaudeOpus45,
    /// Anthropic Claude Sonnet 4 (uses BPE tokenizer)
    ClaudeSonnet4,
    /// Generic "Other" model (uses average tokenization)
    Other,
}

impl std::fmt::Display for ModelType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ModelType::Gpt4o => write!(f, "GPT-4o"),
            ModelType::O1 => write!(f, "o1"),
            ModelType::Gpt4 => write!(f, "GPT-4"),
            ModelType::Gemini3 => write!(f, "Gemini 3"),
            ModelType::ClaudeOpus45 => write!(f, "Claude Opus 4.5"),
            ModelType::ClaudeSonnet4 => write!(f, "Claude Sonnet 4"),
            ModelType::Other => write!(f, "Other"),
        }
    }
}

/// Token information returned from counting
#[derive(Debug, Clone)]
pub struct TokenInfo {
    /// Total number of tokens
    pub count: usize,
    /// Token IDs (if available)
    pub ids: Vec<u32>,
    /// Token strings (decoded tokens)
    pub tokens: Vec<String>,
    /// Model used for counting
    pub model: ModelType,
}

impl TokenInfo {
    /// Create a new TokenInfo
    pub fn new(count: usize, ids: Vec<u32>, tokens: Vec<String>, model: ModelType) -> Self {
        Self {
            count,
            ids,
            tokens,
            model,
        }
    }

    /// Create TokenInfo with just count (for models without ID access)
    pub fn count_only(count: usize, model: ModelType) -> Self {
        Self {
            count,
            ids: Vec::new(),
            tokens: Vec::new(),
            model,
        }
    }
}

/// Token counter for multiple LLM models
///
/// Provides unified interface for counting tokens across different models.
/// Uses model-specific tokenizers internally.
pub struct TokenCounter {
    // Note: Caching was considered but removed as token counting is fast enough
    // that the overhead of cache management outweighs the benefits for typical use cases.
    // If profiling shows token counting as a bottleneck, caching can be re-added.
}

impl TokenCounter {
    /// Create a new token counter
    pub fn new() -> Self {
        Self {}
    }

    /// Count tokens for the given text and model
    ///
    /// # Arguments
    /// * `text` - The text to tokenize
    /// * `model` - The model type to use for tokenization
    ///
    /// # Returns
    /// TokenInfo containing count, IDs, and decoded tokens
    pub fn count(&self, text: &str, model: ModelType) -> TokenInfo {
        match model {
            ModelType::Gpt4o | ModelType::O1 => self.count_openai_o200k(text, model),
            ModelType::Gpt4 => self.count_openai_cl100k(text, model),
            ModelType::Gemini3 => self.count_gemini(text, model),
            ModelType::ClaudeOpus45 | ModelType::ClaudeSonnet4 => self.count_claude(text, model),
            ModelType::Other => self.count_other(text, model),
        }
    }

    /// Count tokens using OpenAI o200k_base tokenizer (GPT-4o, o1)
    fn count_openai_o200k(&self, text: &str, model: ModelType) -> TokenInfo {
        // Use tiktoken-rs o200k_base tokenizer
        #[cfg(feature = "tiktoken")]
        {
            use tiktoken_rs::o200k_base;
            if let Ok(bpe) = o200k_base() {
                let tokens = bpe.encode_with_special_tokens(text);
                let decoded: Vec<String> =
                    tokens.iter().filter_map(|&id| bpe.decode(vec![id]).ok()).collect();
                return TokenInfo::new(tokens.len(), tokens, decoded, model);
            }
        }

        // Fallback: approximate token count (4 chars per token average)
        self.approximate_token_count(text, model, 4.0)
    }

    /// Count tokens using OpenAI cl100k_base tokenizer (GPT-4)
    fn count_openai_cl100k(&self, text: &str, model: ModelType) -> TokenInfo {
        #[cfg(feature = "tiktoken")]
        {
            use tiktoken_rs::cl100k_base;
            if let Ok(bpe) = cl100k_base() {
                let tokens = bpe.encode_with_special_tokens(text);
                let decoded: Vec<String> =
                    tokens.iter().filter_map(|&id| bpe.decode(vec![id]).ok()).collect();
                return TokenInfo::new(tokens.len(), tokens, decoded, model);
            }
        }

        // Fallback: approximate token count
        self.approximate_token_count(text, model, 4.0)
    }

    /// Count tokens using Gemini/Gemma tokenizer
    fn count_gemini(&self, text: &str, model: ModelType) -> TokenInfo {
        #[cfg(feature = "tokenizers")]
        {
            use tokenizers::Tokenizer;
            // Try to load Gemma tokenizer from known paths
            let paths = [
                "tokenizers/gemma-tokenizer.json",
                "~/.cache/huggingface/tokenizers/google/gemma-3/tokenizer.json",
            ];

            for path in &paths {
                if let Ok(tokenizer) = Tokenizer::from_file(path) {
                    if let Ok(encoding) = tokenizer.encode(text, false) {
                        let ids: Vec<u32> = encoding.get_ids().to_vec();
                        let tokens: Vec<String> =
                            encoding.get_tokens().iter().map(|s| s.to_string()).collect();
                        return TokenInfo::new(ids.len(), ids, tokens, model);
                    }
                }
            }
        }

        // Fallback: Gemini uses ~3.5 chars per token
        self.approximate_token_count(text, model, 3.5)
    }

    /// Count tokens using Claude tokenizer
    fn count_claude(&self, text: &str, model: ModelType) -> TokenInfo {
        // Claude tokenizer is approximated since no official local tokenizer exists
        // Use HuggingFace tokenizers crate with community Claude tokenizer if available
        #[cfg(feature = "tokenizers-hf")]
        {
            use tokenizers::Tokenizer;
            // Try to load Claude tokenizer from known paths
            let paths = [
                "tokenizers/claude-tokenizer.json",
                "~/.cache/huggingface/tokenizers/anthropic/claude/tokenizer.json",
            ];

            for path in &paths {
                if let Ok(tokenizer) = Tokenizer::from_file(path) {
                    if let Ok(encoding) = tokenizer.encode(text, false) {
                        let ids: Vec<u32> = encoding.get_ids().to_vec();
                        let tokens: Vec<String> =
                            encoding.get_tokens().iter().map(|s| s.to_string()).collect();
                        return TokenInfo::new(ids.len(), ids, tokens, model);
                    }
                }
            }
        }

        // Fallback: Claude uses ~3.8 chars per token
        self.approximate_token_count(text, model, 3.8)
    }

    /// Count tokens using generic "Other" model (average tokenization)
    fn count_other(&self, text: &str, model: ModelType) -> TokenInfo {
        // Use an average of ~3.7 chars per token (between Claude and OpenAI)
        self.approximate_token_count(text, model, 3.7)
    }

    /// Approximate token count when tokenizer is not available
    fn approximate_token_count(
        &self,
        text: &str,
        model: ModelType,
        chars_per_token: f64,
    ) -> TokenInfo {
        let count = (text.len() as f64 / chars_per_token).ceil() as usize;
        TokenInfo::count_only(count.max(1), model)
    }

    /// Count tokens for all supported models
    pub fn count_all(&self, text: &str) -> HashMap<ModelType, TokenInfo> {
        let models = [
            ModelType::Gpt4o,
            ModelType::O1,
            ModelType::Gpt4,
            ModelType::Gemini3,
            ModelType::ClaudeOpus45,
            ModelType::ClaudeSonnet4,
            ModelType::Other,
        ];

        models.iter().map(|&model| (model, self.count(text, model))).collect()
    }

    /// Count tokens for the 4 primary models (OpenAI, Claude, Gemini, Other)
    /// as required by the token efficiency display feature.
    ///
    /// Returns counts for:
    /// - OpenAI (GPT-4o)
    /// - Claude (Sonnet 4)
    /// - Gemini (Gemini 3)
    /// - Other (generic model)
    pub fn count_primary_models(&self, text: &str) -> HashMap<ModelType, TokenInfo> {
        let models = [
            ModelType::Gpt4o,         // OpenAI representative
            ModelType::ClaudeSonnet4, // Claude representative
            ModelType::Gemini3,       // Gemini representative
            ModelType::Other,         // Generic model
        ];

        models.iter().map(|&model| (model, self.count(text, model))).collect()
    }

    /// Get a summary of token counts for all models
    pub fn summary(&self, text: &str) -> String {
        let counts = self.count_all(text);
        let mut lines = vec![format!("Token counts for {} chars:", text.len())];

        for model in [
            ModelType::Gpt4o,
            ModelType::Gemini3,
            ModelType::ClaudeOpus45,
        ] {
            if let Some(info) = counts.get(&model) {
                lines.push(format!("  {}: {} tokens", model, info.count));
            }
        }

        lines.join("\n")
    }
}

impl Default for TokenCounter {
    fn default() -> Self {
        Self::new()
    }
}

/// Measure token efficiency of dx format vs other formats
pub struct TokenEfficiencyMeasurement {
    /// Original text (e.g., JSON)
    pub original: TokenInfo,
    /// DX format text
    pub dx_format: TokenInfo,
    /// Savings percentage
    pub savings_percent: f64,
}

impl TokenEfficiencyMeasurement {
    /// Calculate token savings
    pub fn calculate(original: TokenInfo, dx_format: TokenInfo) -> Self {
        let savings = if original.count > 0 {
            ((original.count as f64 - dx_format.count as f64) / original.count as f64) * 100.0
        } else {
            0.0
        };

        Self {
            original,
            dx_format,
            savings_percent: savings,
        }
    }
}

/// Extension trait for adding token counting to dx format serializer
pub trait TokenCountExt {
    /// Get token count for the serialized output
    fn token_count(&self, model: ModelType) -> TokenInfo;

    /// Get token counts for all models
    fn token_counts(&self) -> HashMap<ModelType, TokenInfo>;
}

impl TokenCountExt for String {
    fn token_count(&self, model: ModelType) -> TokenInfo {
        let counter = TokenCounter::new();
        counter.count(self, model)
    }

    fn token_counts(&self) -> HashMap<ModelType, TokenInfo> {
        let counter = TokenCounter::new();
        counter.count_all(self)
    }
}

impl TokenCountExt for str {
    fn token_count(&self, model: ModelType) -> TokenInfo {
        let counter = TokenCounter::new();
        counter.count(self, model)
    }

    fn token_counts(&self) -> HashMap<ModelType, TokenInfo> {
        let counter = TokenCounter::new();
        counter.count_all(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_counter_creation() {
        let counter = TokenCounter::new();
        let info = counter.count("Hello, world!", ModelType::Gpt4o);
        assert!(info.count > 0);
    }

    #[test]
    fn test_count_all_models() {
        let counter = TokenCounter::new();
        let counts = counter.count_all("Hello, world!");

        assert!(counts.contains_key(&ModelType::Gpt4o));
        assert!(counts.contains_key(&ModelType::Gemini3));
        assert!(counts.contains_key(&ModelType::ClaudeOpus45));
        assert!(counts.contains_key(&ModelType::Other));
    }

    #[test]
    fn test_count_primary_models() {
        let counter = TokenCounter::new();
        let counts = counter.count_primary_models("Hello, world!");

        // Should have exactly 4 models
        assert_eq!(counts.len(), 4);
        assert!(counts.contains_key(&ModelType::Gpt4o));
        assert!(counts.contains_key(&ModelType::ClaudeSonnet4));
        assert!(counts.contains_key(&ModelType::Gemini3));
        assert!(counts.contains_key(&ModelType::Other));

        // All counts should be non-zero
        for info in counts.values() {
            assert!(info.count > 0, "Token count should be non-zero");
        }
    }

    #[test]
    fn test_other_model() {
        let counter = TokenCounter::new();
        let info = counter.count("Hello, world!", ModelType::Other);
        assert!(info.count > 0);
        assert_eq!(info.model, ModelType::Other);
    }

    #[test]
    fn test_token_efficiency_measurement() {
        let original = TokenInfo::count_only(100, ModelType::Gpt4o);
        let dx = TokenInfo::count_only(73, ModelType::Gpt4o);
        let measurement = TokenEfficiencyMeasurement::calculate(original, dx);

        assert!((measurement.savings_percent - 27.0).abs() < 0.1);
    }

    #[test]
    fn test_empty_string() {
        let counter = TokenCounter::new();
        let info = counter.count("", ModelType::Gpt4o);
        assert_eq!(info.count, 1); // Minimum 1 token
    }

    #[test]
    fn test_model_display() {
        assert_eq!(format!("{}", ModelType::Gpt4o), "GPT-4o");
        assert_eq!(format!("{}", ModelType::Gemini3), "Gemini 3");
        assert_eq!(format!("{}", ModelType::ClaudeOpus45), "Claude Opus 4.5");
    }

    #[test]
    fn test_summary() {
        let counter = TokenCounter::new();
        let summary = counter.summary("Hello, world!");
        assert!(summary.contains("Token counts"));
        assert!(summary.contains("GPT-4o"));
    }

    #[test]
    fn test_token_count_extension() {
        let text = "Hello, world!";
        let info = text.token_count(ModelType::Gpt4o);
        assert!(info.count > 0);
    }

    #[test]
    fn test_dx_format_token_efficiency() {
        // Compare JSON vs DX format for same data
        // Use a larger example to show more significant savings
        let json = r#"{"name":"dx-serializer","version":"0.1.0","description":"Binary-first serialization format for LLMs","workspace":["frontend/www","frontend/mobile","backend/api","backend/workers"],"dependencies":{"serde":"1.0","bincode":"2.0","tokio":"1.0"},"enabled":true,"count":42}"#;
        let dx = "nm=dx-serializer ver=0.1.0 ds=\"Binary-first serialization format for LLMs\" ws:frontend/www,frontend/mobile,backend/api,backend/workers deps.serde=1.0 deps.bincode=2.0 deps.tokio=1.0 en=true ct=42";

        let counter = TokenCounter::new();
        let json_tokens = counter.count(json, ModelType::Gpt4o);
        let dx_tokens = counter.count(dx, ModelType::Gpt4o);

        // Store counts before moving
        let json_count = json_tokens.count;
        let dx_count = dx_tokens.count;

        // DX format should use fewer tokens
        let measurement = TokenEfficiencyMeasurement::calculate(json_tokens, dx_tokens);
        println!(
            "JSON: {} tokens, DX: {} tokens, Savings: {:.1}%",
            measurement.original.count, measurement.dx_format.count, measurement.savings_percent
        );

        // DX should be more efficient (or at least not worse)
        // Small examples may not show significant savings due to tokenizer overhead
        assert!(
            measurement.savings_percent >= 0.0 || dx_count <= json_count,
            "DX format should not be worse than JSON"
        );
    }
}
