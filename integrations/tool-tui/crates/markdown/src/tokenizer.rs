//! Tokenizer wrapper for accurate token counting.
//!
//! This module provides a wrapper around tiktoken-rs for accurate
//! token counting across different LLM tokenizers.

use crate::error::CompileError;
use crate::types::TokenizerType;
use tiktoken_rs::{CoreBPE, cl100k_base, o200k_base, p50k_base};

/// Tokenizer wrapper for accurate token counting.
///
/// Wraps tiktoken-rs to provide consistent token counting
/// across different LLM tokenizers (GPT-4, GPT-4o, GPT-3.5).
pub struct Tokenizer {
    /// The underlying BPE tokenizer
    bpe: CoreBPE,
    /// Tokenizer type
    tokenizer_type: TokenizerType,
}

impl Tokenizer {
    /// Create a new tokenizer with the specified type.
    ///
    /// # Arguments
    /// * `tokenizer_type` - The type of tokenizer to use
    ///
    /// # Returns
    /// A new Tokenizer instance, or an error if initialization fails.
    pub fn new(tokenizer_type: TokenizerType) -> Result<Self, CompileError> {
        let bpe = match tokenizer_type {
            TokenizerType::Cl100k => cl100k_base().map_err(|e| {
                CompileError::tokenizer(format!("failed to load cl100k_base: {}", e))
            })?,
            TokenizerType::O200k => o200k_base().map_err(|e| {
                CompileError::tokenizer(format!("failed to load o200k_base: {}", e))
            })?,
            TokenizerType::P50k => p50k_base()
                .map_err(|e| CompileError::tokenizer(format!("failed to load p50k_base: {}", e)))?,
        };

        Ok(Self {
            bpe,
            tokenizer_type,
        })
    }

    /// Create a tokenizer with the default type (cl100k_base).
    pub fn default_tokenizer() -> Result<Self, CompileError> {
        Self::new(TokenizerType::Cl100k)
    }

    /// Count tokens in the given text.
    ///
    /// # Arguments
    /// * `text` - The text to tokenize
    ///
    /// # Returns
    /// The number of tokens in the text.
    pub fn count(&self, text: &str) -> usize {
        self.bpe.encode_ordinary(text).len()
    }

    /// Count tokens for a phrase (for dictionary decisions).
    ///
    /// This is the same as `count` but named differently for clarity
    /// when used in dictionary building.
    pub fn count_phrase(&self, phrase: &str) -> usize {
        self.count(phrase)
    }

    /// Encode text to token IDs.
    ///
    /// # Arguments
    /// * `text` - The text to encode
    ///
    /// # Returns
    /// A vector of token IDs.
    pub fn encode(&self, text: &str) -> Vec<usize> {
        self.bpe.encode_ordinary(text)
    }

    /// Decode token IDs back to text.
    ///
    /// # Arguments
    /// * `tokens` - The token IDs to decode
    ///
    /// # Returns
    /// The decoded text, or an error if decoding fails.
    pub fn decode(&self, tokens: &[usize]) -> Result<String, CompileError> {
        self.bpe
            .decode(tokens.to_vec())
            .map_err(|e| CompileError::tokenizer(format!("decode error: {}", e)))
    }

    /// Get the tokenizer type.
    pub fn tokenizer_type(&self) -> TokenizerType {
        self.tokenizer_type
    }

    /// Calculate token savings between two texts.
    ///
    /// # Arguments
    /// * `before` - The original text
    /// * `after` - The optimized text
    ///
    /// # Returns
    /// A tuple of (tokens_before, tokens_after, tokens_saved).
    pub fn calculate_savings(&self, before: &str, after: &str) -> (usize, usize, usize) {
        let tokens_before = self.count(before);
        let tokens_after = self.count(after);
        let tokens_saved = tokens_before.saturating_sub(tokens_after);
        (tokens_before, tokens_after, tokens_saved)
    }

    /// Check if replacing a phrase with a variable saves tokens.
    ///
    /// For dictionary deduplication, we need to check if:
    /// `occurrences * phrase_tokens > definition_tokens + occurrences * var_tokens`
    ///
    /// # Arguments
    /// * `phrase` - The phrase to potentially replace
    /// * `occurrences` - Number of times the phrase appears
    /// * `var_name` - The variable name (e.g., "$A")
    ///
    /// # Returns
    /// True if replacement saves tokens, false otherwise.
    pub fn should_replace(&self, phrase: &str, occurrences: usize, var_name: &str) -> bool {
        if occurrences < 2 {
            return false;
        }

        let phrase_tokens = self.count(phrase);
        let var_tokens = self.count(var_name);

        // Minimum phrase length to consider (avoid replacing short words)
        if phrase.len() < 4 {
            return false;
        }

        // Cost of definition: $A="phrase"\n
        // Approximate: var_tokens + tokens for =""\n
        let definition_overhead = 3; // Approximate tokens for ="" and newline
        let definition_cost = var_tokens + phrase_tokens + definition_overhead;

        // Original cost: phrase * occurrences
        let original_cost = phrase_tokens * occurrences;

        // New cost: definition + var * occurrences
        let new_cost = definition_cost + var_tokens * occurrences;

        new_cost < original_cost
    }
}

impl Default for Tokenizer {
    fn default() -> Self {
        // Default to Cl100k tokenizer (GPT-4, Claude)
        // This should always succeed as Cl100k is built-in to tiktoken-rs
        // If it fails, it's a critical library initialization error
        match Self::default_tokenizer() {
            Ok(tokenizer) => tokenizer,
            Err(_) => {
                // This should never happen as Cl100k is built into tiktoken-rs
                // If it does, the library is in an invalid state
                unreachable!(
                    "Failed to create default Cl100k tokenizer - tiktoken-rs library is broken"
                )
            }
        }
    }
}

/// A lazy tokenizer that initializes on first use.
///
/// Useful when you want to avoid initialization cost until needed.
pub struct LazyTokenizer {
    tokenizer_type: TokenizerType,
    tokenizer: Option<Tokenizer>,
}

impl LazyTokenizer {
    /// Create a new lazy tokenizer.
    pub fn new(tokenizer_type: TokenizerType) -> Self {
        Self {
            tokenizer_type,
            tokenizer: None,
        }
    }

    /// Get or initialize the tokenizer.
    pub fn get(&mut self) -> Result<&Tokenizer, CompileError> {
        if self.tokenizer.is_none() {
            self.tokenizer = Some(Tokenizer::new(self.tokenizer_type)?);
        }
        // At this point, tokenizer is guaranteed to be Some because we just initialized it
        self.tokenizer
            .as_ref()
            .ok_or_else(|| CompileError::tokenizer("tokenizer initialization failed unexpectedly"))
    }

    /// Count tokens, initializing the tokenizer if needed.
    pub fn count(&mut self, text: &str) -> Result<usize, CompileError> {
        Ok(self.get()?.count(text))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tokenizer_creation() {
        let tokenizer = Tokenizer::new(TokenizerType::Cl100k);
        assert!(tokenizer.is_ok());
    }

    #[test]
    fn test_tokenizer_count_empty() {
        let tokenizer = Tokenizer::default();
        assert_eq!(tokenizer.count(""), 0);
    }

    #[test]
    fn test_tokenizer_count_simple() {
        let tokenizer = Tokenizer::default();
        let count = tokenizer.count("Hello, world!");
        assert!(count > 0);
        assert!(count < 10); // Should be around 4 tokens
    }

    #[test]
    fn test_tokenizer_count_code() {
        let tokenizer = Tokenizer::default();
        let code = "fn main() { println!(\"Hello\"); }";
        let count = tokenizer.count(code);
        assert!(count > 5);
    }

    #[test]
    fn test_tokenizer_encode_decode() {
        let tokenizer = Tokenizer::default();
        let text = "Hello, world!";
        let tokens = tokenizer.encode(text);
        let decoded = tokenizer.decode(&tokens).unwrap();
        assert_eq!(decoded, text);
    }

    #[test]
    fn test_calculate_savings() {
        let tokenizer = Tokenizer::default();
        let before = "This is a long sentence with many words.";
        let after = "Short.";
        let (tokens_before, tokens_after, saved) = tokenizer.calculate_savings(before, after);
        assert!(tokens_before > tokens_after);
        assert!(saved > 0);
    }

    #[test]
    fn test_should_replace_phrase() {
        let tokenizer = Tokenizer::default();

        // Test with a phrase that has more tokens than the variable
        // "https://example.com/very/long/documentation/url" should have many tokens
        let long_url = "https://example.com/very/long/documentation/url";
        let phrase_tokens = tokenizer.count(long_url);
        let var_tokens = tokenizer.count("$A");

        println!("URL '{}' = {} tokens", long_url, phrase_tokens);
        println!("Var '$A' = {} tokens", var_tokens);

        // This URL should have many more tokens than $A
        assert!(phrase_tokens > var_tokens, "URL should have more tokens than variable");

        // With many occurrences, should save tokens
        if phrase_tokens > var_tokens + 1 {
            // Only test if the phrase actually has more tokens
            assert!(
                tokenizer.should_replace(long_url, 5, "$A"),
                "Expected URL replacement to save tokens"
            );
        }

        // Short phrase should not be replaced (less than 6 chars)
        let short_phrase = "the";
        assert!(!tokenizer.should_replace(short_phrase, 10, "$A"));

        // Single occurrence should not be replaced
        assert!(!tokenizer.should_replace(long_url, 1, "$A"));
    }

    #[test]
    fn test_different_tokenizers() {
        // All tokenizers should work
        let cl100k = Tokenizer::new(TokenizerType::Cl100k).unwrap();
        let o200k = Tokenizer::new(TokenizerType::O200k).unwrap();
        let p50k = Tokenizer::new(TokenizerType::P50k).unwrap();

        let text = "Hello, world!";

        // All should return positive counts
        assert!(cl100k.count(text) > 0);
        assert!(o200k.count(text) > 0);
        assert!(p50k.count(text) > 0);
    }

    #[test]
    fn test_lazy_tokenizer() {
        let mut lazy = LazyTokenizer::new(TokenizerType::Cl100k);

        // First call initializes
        let count1 = lazy.count("Hello").unwrap();
        assert!(count1 > 0);

        // Second call reuses
        let count2 = lazy.count("Hello").unwrap();
        assert_eq!(count1, count2);
    }

    #[test]
    fn test_tokenizer_type() {
        let tokenizer = Tokenizer::new(TokenizerType::O200k).unwrap();
        assert_eq!(tokenizer.tokenizer_type(), TokenizerType::O200k);
    }
}
