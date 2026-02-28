//! # Token Optimization Module
//!
//! Optimizes LLM token usage through context compression, format conversion,
//! and efficient serialization.
//!
//! ## RLM (Response Length Model) Support
//!
//! The [`rlm`] module provides intelligent response length prediction
//! to reduce token waste from unnecessarily verbose responses.

pub mod compressor;
pub mod converter;
pub mod dashboard;
pub mod metrics;
pub mod rlm;

use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::RwLock;

/// Token optimization errors
#[derive(Debug, Error)]
pub enum TokenError {
    #[error("Compression failed: {0}")]
    CompressionFailed(String),
    #[error("Conversion failed: {0}")]
    ConversionFailed(String),
    #[error("Token limit exceeded: {current} > {limit}")]
    LimitExceeded { current: usize, limit: usize },
    #[error("Invalid format: {0}")]
    InvalidFormat(String),
}

/// Token usage statistics
#[derive(Debug, Clone, Default)]
pub struct TokenUsage {
    /// Total tokens used
    pub total_tokens: u64,
    /// Tokens saved by compression
    pub tokens_saved: u64,
    /// Compression ratio
    pub compression_ratio: f32,
    /// Tokens by category
    pub by_category: HashMap<String, u64>,
    /// Historical usage
    pub history: Vec<TokenSnapshot>,
}

/// Token usage snapshot
#[derive(Debug, Clone)]
pub struct TokenSnapshot {
    /// Timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Tokens used
    pub tokens: u64,
    /// Tokens saved
    pub saved: u64,
    /// Operation type
    pub operation: String,
}

/// Token optimizer configuration
#[derive(Debug, Clone)]
pub struct TokenConfig {
    /// Target compression ratio (e.g., 0.52 for 52% savings)
    pub target_compression: f32,
    /// Maximum context window size
    pub max_context_tokens: usize,
    /// Enable JSON to DX format conversion
    pub enable_dx_conversion: bool,
    /// Enable context summarization
    pub enable_summarization: bool,
    /// Summarization threshold (characters)
    pub summarization_threshold: usize,
    /// Enable token counting
    pub enable_metrics: bool,
}

impl Default for TokenConfig {
    fn default() -> Self {
        Self {
            target_compression: 0.52,
            max_context_tokens: 128_000,
            enable_dx_conversion: true,
            enable_summarization: true,
            summarization_threshold: 10_000,
            enable_metrics: true,
        }
    }
}

/// Token optimizer
pub struct TokenOptimizer {
    /// Configuration
    config: TokenConfig,
    /// Usage statistics
    usage: Arc<RwLock<TokenUsage>>,
    /// Compressor
    compressor: compressor::ContextCompressor,
    /// Converter
    converter: converter::FormatConverter,
}

impl TokenOptimizer {
    /// Create a new token optimizer
    pub fn new(config: TokenConfig) -> Self {
        Self {
            compressor: compressor::ContextCompressor::new(config.clone()),
            converter: converter::FormatConverter::new(config.enable_dx_conversion),
            config,
            usage: Arc::new(RwLock::new(TokenUsage::default())),
        }
    }

    /// Optimize content for LLM communication
    pub async fn optimize(&self, content: &str, category: &str) -> Result<String, TokenError> {
        let original_tokens = self.estimate_tokens(content);

        // Step 1: Convert JSON to DX format if applicable
        let converted = if self.config.enable_dx_conversion {
            self.converter.convert_to_dx(content)?
        } else {
            content.to_string()
        };

        // Step 2: Compress context if above threshold
        let compressed = if self.config.enable_summarization
            && converted.len() > self.config.summarization_threshold
        {
            self.compressor.compress(&converted).await?
        } else {
            converted
        };

        // Track metrics
        let optimized_tokens = self.estimate_tokens(&compressed);
        let tokens_saved = original_tokens.saturating_sub(optimized_tokens);

        if self.config.enable_metrics {
            self.record_usage(category, original_tokens, tokens_saved).await;
        }

        Ok(compressed)
    }

    /// Estimate token count (approximate using cl100k_base rules)
    pub fn estimate_tokens(&self, content: &str) -> usize {
        // Rough estimation: ~4 characters per token for English text
        // This is a simplified estimation; real implementation would use tiktoken
        let char_count = content.len();
        let word_count = content.split_whitespace().count();

        // Use a weighted average
        (char_count / 4).max(word_count * 4 / 3)
    }

    /// Record usage statistics
    async fn record_usage(&self, category: &str, tokens: usize, saved: usize) {
        let mut usage = self.usage.write().await;

        usage.total_tokens += tokens as u64;
        usage.tokens_saved += saved as u64;

        if usage.total_tokens > 0 {
            usage.compression_ratio =
                usage.tokens_saved as f32 / (usage.total_tokens + usage.tokens_saved) as f32;
        }

        *usage.by_category.entry(category.to_string()).or_insert(0) += tokens as u64;

        usage.history.push(TokenSnapshot {
            timestamp: chrono::Utc::now(),
            tokens: tokens as u64,
            saved: saved as u64,
            operation: category.to_string(),
        });

        // Keep history bounded
        if usage.history.len() > 1000 {
            usage.history.remove(0);
        }
    }

    /// Get current usage statistics
    pub async fn get_usage(&self) -> TokenUsage {
        self.usage.read().await.clone()
    }

    /// Reset usage statistics
    pub async fn reset_usage(&self) {
        let mut usage = self.usage.write().await;
        *usage = TokenUsage::default();
    }

    /// Check if content exceeds token limit
    pub fn check_limit(&self, content: &str) -> Result<(), TokenError> {
        let tokens = self.estimate_tokens(content);
        if tokens > self.config.max_context_tokens {
            return Err(TokenError::LimitExceeded {
                current: tokens,
                limit: self.config.max_context_tokens,
            });
        }
        Ok(())
    }

    /// Truncate content to fit within token limit
    pub fn truncate_to_limit(&self, content: &str) -> String {
        let tokens = self.estimate_tokens(content);
        if tokens <= self.config.max_context_tokens {
            return content.to_string();
        }

        // Simple truncation - real implementation would be smarter
        let target_chars = self.config.max_context_tokens * 4;
        if content.len() <= target_chars {
            return content.to_string();
        }

        let truncated = &content[..target_chars.min(content.len())];
        format!("{}...[truncated]", truncated)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_estimation() {
        let optimizer = TokenOptimizer::new(TokenConfig::default());

        let short_text = "Hello, world!";
        let tokens = optimizer.estimate_tokens(short_text);
        assert!(tokens > 0);
        assert!(tokens < 10);

        let long_text = "This is a longer text that should result in more tokens being estimated.";
        let long_tokens = optimizer.estimate_tokens(long_text);
        assert!(long_tokens > tokens);
    }

    #[test]
    fn test_check_limit() {
        let config = TokenConfig {
            max_context_tokens: 10,
            ..Default::default()
        };
        let optimizer = TokenOptimizer::new(config);

        assert!(optimizer.check_limit("short").is_ok());
        assert!(optimizer.check_limit(&"word ".repeat(1000)).is_err());
    }

    #[test]
    fn test_truncate() {
        let config = TokenConfig {
            max_context_tokens: 10,
            ..Default::default()
        };
        let optimizer = TokenOptimizer::new(config);

        let short = "short";
        assert_eq!(optimizer.truncate_to_limit(short), short);

        let long = "word ".repeat(1000);
        let truncated = optimizer.truncate_to_limit(&long);
        assert!(truncated.ends_with("...[truncated]"));
    }

    #[tokio::test]
    async fn test_usage_tracking() {
        let config = TokenConfig {
            enable_metrics: true,
            ..Default::default()
        };
        let optimizer = TokenOptimizer::new(config);

        let _ = optimizer.optimize("Test content", "test").await;

        let usage = optimizer.get_usage().await;
        assert!(usage.total_tokens > 0);
        assert!(usage.by_category.contains_key("test"));
    }
}
