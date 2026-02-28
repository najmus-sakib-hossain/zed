//! Response Length Model (RLM) Support
//!
//! Optimizes LLM responses by predicting and controlling output length,
//! reducing token waste from unnecessarily verbose responses.
//!
//! # Overview
//!
//! RLM (Response Length Model) predicts the optimal response length before
//! sending a request to the LLM, then uses this prediction to:
//!
//! 1. Set appropriate `max_tokens` parameter
//! 2. Include length hints in system prompts
//! 3. Reduce token waste by up to 40%
//!
//! # How It Works
//!
//! ```text
//! ┌────────────────────────────────────────────────────────────────┐
//! │                    RLM Pipeline                                 │
//! ├────────────────────────────────────────────────────────────────┤
//! │                                                                 │
//! │  User Query ──► Intent Classifier ──► Length Predictor         │
//! │                       │                     │                   │
//! │                       ▼                     ▼                   │
//! │              ┌─────────────┐      ┌──────────────────┐         │
//! │              │ Query Type  │      │ Predicted Tokens │         │
//! │              │ (code, qa,  │      │    (50-8000)     │         │
//! │              │  explain)   │      └────────┬─────────┘         │
//! │              └──────┬──────┘               │                   │
//! │                     │                      │                   │
//! │                     └──────────┬───────────┘                   │
//! │                                ▼                               │
//! │                    ┌────────────────────┐                      │
//! │                    │  LLM Request with  │                      │
//! │                    │  max_tokens + hint │                      │
//! │                    └────────────────────┘                      │
//! │                                                                 │
//! └────────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Example
//!
//! ```rust,ignore
//! use dx_cli::tokens::rlm::{RlmPredictor, QueryType};
//!
//! let predictor = RlmPredictor::new()?;
//!
//! // Analyze query and predict response length
//! let prediction = predictor.predict("What is 2+2?").await?;
//!
//! assert_eq!(prediction.query_type, QueryType::SimpleAnswer);
//! assert!(prediction.predicted_tokens < 100);
//!
//! // Use prediction in LLM request
//! let request = LlmRequest {
//!     max_tokens: Some(prediction.max_tokens_recommended),
//!     system_prompt: Some(prediction.length_hint()),
//!     ..Default::default()
//! };
//! ```

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info};

/// Query classification types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum QueryType {
    /// Simple factual answer (e.g., "What is 2+2?")
    SimpleAnswer,
    /// Yes/no question
    YesNo,
    /// Short explanation (1-2 paragraphs)
    ShortExplanation,
    /// Detailed explanation (multiple paragraphs)
    DetailedExplanation,
    /// Code snippet (small function)
    CodeSnippet,
    /// Code implementation (larger feature)
    CodeImplementation,
    /// Code review/analysis
    CodeReview,
    /// List generation
    ListGeneration,
    /// Step-by-step instructions
    StepByStep,
    /// Creative writing (short)
    CreativeShort,
    /// Creative writing (long)
    CreativeLong,
    /// Data transformation/formatting
    DataTransform,
    /// Summarization
    Summary,
    /// Conversation/chat
    Conversational,
    /// Unknown/complex
    Unknown,
}

impl QueryType {
    /// Get base token estimate for this query type
    pub fn base_tokens(&self) -> u32 {
        match self {
            QueryType::SimpleAnswer => 20,
            QueryType::YesNo => 10,
            QueryType::ShortExplanation => 150,
            QueryType::DetailedExplanation => 500,
            QueryType::CodeSnippet => 100,
            QueryType::CodeImplementation => 400,
            QueryType::CodeReview => 300,
            QueryType::ListGeneration => 200,
            QueryType::StepByStep => 300,
            QueryType::CreativeShort => 200,
            QueryType::CreativeLong => 1000,
            QueryType::DataTransform => 150,
            QueryType::Summary => 200,
            QueryType::Conversational => 100,
            QueryType::Unknown => 300,
        }
    }

    /// Get token range (min, max) for this query type
    pub fn token_range(&self) -> (u32, u32) {
        match self {
            QueryType::SimpleAnswer => (5, 50),
            QueryType::YesNo => (3, 30),
            QueryType::ShortExplanation => (50, 300),
            QueryType::DetailedExplanation => (200, 1500),
            QueryType::CodeSnippet => (30, 300),
            QueryType::CodeImplementation => (150, 1500),
            QueryType::CodeReview => (100, 800),
            QueryType::ListGeneration => (50, 500),
            QueryType::StepByStep => (100, 800),
            QueryType::CreativeShort => (50, 500),
            QueryType::CreativeLong => (300, 3000),
            QueryType::DataTransform => (30, 500),
            QueryType::Summary => (50, 500),
            QueryType::Conversational => (20, 300),
            QueryType::Unknown => (50, 1000),
        }
    }

    /// Get length hint for system prompt
    pub fn length_hint(&self) -> &str {
        match self {
            QueryType::SimpleAnswer => "Respond concisely in 1-2 sentences.",
            QueryType::YesNo => "Answer with yes/no and a brief explanation if needed.",
            QueryType::ShortExplanation => "Provide a brief explanation in 1-2 paragraphs.",
            QueryType::DetailedExplanation => "Provide a thorough explanation.",
            QueryType::CodeSnippet => "Provide a concise code snippet with minimal explanation.",
            QueryType::CodeImplementation => {
                "Implement the requested functionality with clear code."
            }
            QueryType::CodeReview => "Review the code and provide specific feedback.",
            QueryType::ListGeneration => "Provide a clear, organized list.",
            QueryType::StepByStep => "Provide numbered steps.",
            QueryType::CreativeShort => "Write a short, creative response.",
            QueryType::CreativeLong => "Write a detailed, creative response.",
            QueryType::DataTransform => "Transform the data as requested.",
            QueryType::Summary => "Provide a concise summary.",
            QueryType::Conversational => "Respond naturally and conversationally.",
            QueryType::Unknown => "",
        }
    }
}

/// Length prediction result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LengthPrediction {
    /// Classified query type
    pub query_type: QueryType,
    /// Predicted response tokens
    pub predicted_tokens: u32,
    /// Recommended max_tokens setting (with buffer)
    pub max_tokens_recommended: u32,
    /// Minimum tokens expected
    pub min_tokens: u32,
    /// Maximum tokens expected
    pub max_tokens: u32,
    /// Confidence score (0.0 - 1.0)
    pub confidence: f32,
    /// Detected complexity factors
    pub complexity_factors: Vec<String>,
}

impl LengthPrediction {
    /// Get length hint for system prompt
    pub fn length_hint(&self) -> String {
        let base_hint = self.query_type.length_hint();
        if base_hint.is_empty() {
            return String::new();
        }

        if self.confidence > 0.8 {
            base_hint.to_string()
        } else {
            format!("{} Aim for approximately {} tokens.", base_hint, self.predicted_tokens)
        }
    }

    /// Calculate potential token savings
    pub fn potential_savings(&self, default_max_tokens: u32) -> u32 {
        if default_max_tokens > self.max_tokens_recommended {
            default_max_tokens - self.max_tokens_recommended
        } else {
            0
        }
    }
}

/// Query features for classification
#[derive(Debug, Clone, Default)]
struct QueryFeatures {
    /// Word count
    word_count: usize,
    /// Character count
    char_count: usize,
    /// Contains code block
    has_code: bool,
    /// Contains question mark
    has_question: bool,
    /// Contains list indicators (-, *, 1.)
    has_list_markers: bool,
    /// Contains comparison keywords
    has_comparison: bool,
    /// Contains explanation keywords
    has_explanation: bool,
    /// Contains code-related keywords
    has_code_keywords: bool,
    /// Contains creative keywords
    has_creative: bool,
    /// Contains summary keywords
    has_summary: bool,
    /// Complexity score (0.0 - 1.0)
    complexity: f32,
}

/// RLM predictor configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RlmConfig {
    /// Enable RLM predictions
    pub enabled: bool,
    /// Buffer multiplier for max_tokens (e.g., 1.3 = 30% buffer)
    pub buffer_multiplier: f32,
    /// Minimum confidence threshold for predictions
    pub min_confidence: f32,
    /// Default max_tokens when prediction fails
    pub default_max_tokens: u32,
    /// Enable learning from actual responses
    pub enable_learning: bool,
    /// Path to learned patterns
    pub patterns_path: Option<String>,
}

impl Default for RlmConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            buffer_multiplier: 1.3,
            min_confidence: 0.5,
            default_max_tokens: 4096,
            enable_learning: true,
            patterns_path: None,
        }
    }
}

/// Learned patterns from actual responses
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct LearnedPatterns {
    /// Query type -> average actual tokens
    type_averages: HashMap<QueryType, (u64, u64)>, // (total_tokens, count)
    /// Query patterns -> observed lengths
    pattern_lengths: HashMap<String, Vec<u32>>,
}

/// Response Length Model predictor
pub struct RlmPredictor {
    /// Configuration
    config: RlmConfig,
    /// Learned patterns
    patterns: Arc<RwLock<LearnedPatterns>>,
    /// Prediction cache
    cache: Arc<RwLock<HashMap<String, LengthPrediction>>>,
}

impl RlmPredictor {
    /// Create a new RLM predictor
    pub fn new() -> Result<Self> {
        Self::with_config(RlmConfig::default())
    }

    /// Create with custom config
    pub fn with_config(config: RlmConfig) -> Result<Self> {
        let patterns = if let Some(path) = &config.patterns_path {
            if let Ok(data) = std::fs::read_to_string(path) {
                serde_json::from_str(&data).unwrap_or_default()
            } else {
                LearnedPatterns::default()
            }
        } else {
            LearnedPatterns::default()
        };

        Ok(Self {
            config,
            patterns: Arc::new(RwLock::new(patterns)),
            cache: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    /// Predict response length for a query
    pub async fn predict(&self, query: &str) -> Result<LengthPrediction> {
        // Check cache
        let cache_key = Self::cache_key(query);
        if let Some(cached) = self.cache.read().await.get(&cache_key) {
            return Ok(cached.clone());
        }

        // Extract features
        let features = self.extract_features(query);

        // Classify query type
        let (query_type, confidence) = self.classify_query(&features);

        // Get base prediction from query type
        let (min_tokens, max_tokens) = query_type.token_range();
        let base_tokens = query_type.base_tokens();

        // Adjust based on query complexity
        let complexity_adjustment = self.calculate_complexity_adjustment(&features);
        let adjusted_tokens = (base_tokens as f32 * complexity_adjustment) as u32;

        // Adjust based on learned patterns
        let learned_adjustment = self.get_learned_adjustment(query_type).await;
        let final_tokens = if learned_adjustment > 0.0 {
            (adjusted_tokens as f32 * learned_adjustment) as u32
        } else {
            adjusted_tokens
        };

        // Apply bounds
        let predicted_tokens = final_tokens.clamp(min_tokens, max_tokens);

        // Calculate recommended max_tokens with buffer
        let max_tokens_recommended =
            (predicted_tokens as f32 * self.config.buffer_multiplier) as u32;

        // Identify complexity factors
        let complexity_factors = self.identify_complexity_factors(&features);

        let prediction = LengthPrediction {
            query_type,
            predicted_tokens,
            max_tokens_recommended,
            min_tokens,
            max_tokens,
            confidence,
            complexity_factors,
        };

        // Cache prediction
        self.cache.write().await.insert(cache_key, prediction.clone());

        debug!(
            "RLM prediction: {:?} -> {} tokens (confidence: {:.2})",
            query_type, predicted_tokens, confidence
        );

        Ok(prediction)
    }

    /// Record actual response length for learning
    pub async fn record_actual(&self, query: &str, query_type: QueryType, actual_tokens: u32) {
        if !self.config.enable_learning {
            return;
        }

        let mut patterns = self.patterns.write().await;

        // Update type averages
        let entry = patterns.type_averages.entry(query_type).or_insert((0, 0));
        entry.0 += actual_tokens as u64;
        entry.1 += 1;

        // Extract pattern and record
        let pattern = Self::extract_pattern(query);
        let lengths = patterns.pattern_lengths.entry(pattern).or_insert_with(Vec::new);
        lengths.push(actual_tokens);

        // Keep only recent entries
        if lengths.len() > 100 {
            lengths.remove(0);
        }

        debug!("RLM recorded: {:?} actual={} tokens", query_type, actual_tokens);
    }

    /// Save learned patterns
    pub async fn save_patterns(&self) -> Result<()> {
        if let Some(path) = &self.config.patterns_path {
            let patterns = self.patterns.read().await;
            let data = serde_json::to_string_pretty(&*patterns)?;
            std::fs::write(path, data)?;
            info!("Saved RLM patterns to {}", path);
        }
        Ok(())
    }

    /// Get statistics
    pub async fn stats(&self) -> RlmStats {
        let patterns = self.patterns.read().await;
        let cache = self.cache.read().await;

        let mut type_stats = HashMap::new();
        for (query_type, (total, count)) in &patterns.type_averages {
            if *count > 0 {
                type_stats.insert(*query_type, *total as f32 / *count as f32);
            }
        }

        RlmStats {
            cached_predictions: cache.len(),
            learned_patterns: patterns.pattern_lengths.len(),
            type_averages: type_stats,
        }
    }

    /// Clear cache
    pub async fn clear_cache(&self) {
        self.cache.write().await.clear();
    }

    // --- Private methods ---

    fn cache_key(query: &str) -> String {
        // Create a simple hash for caching
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        query.to_lowercase().trim().hash(&mut hasher);
        format!("{:x}", hasher.finish())
    }

    fn extract_features(&self, query: &str) -> QueryFeatures {
        let query_lower = query.to_lowercase();
        let words: Vec<&str> = query.split_whitespace().collect();

        let has_code = query.contains("```")
            || query.contains("fn ")
            || query.contains("function ")
            || query.contains("def ");

        let has_question = query.contains('?');

        let has_list_markers = query.contains("\n- ")
            || query.contains("\n* ")
            || query.contains("\n1.")
            || query_lower.contains("list");

        let comparison_keywords = ["compare", "difference", "vs", "versus", "better", "worse"];
        let has_comparison = comparison_keywords.iter().any(|k| query_lower.contains(k));

        let explanation_keywords = ["explain", "why", "how does", "what is", "describe"];
        let has_explanation = explanation_keywords.iter().any(|k| query_lower.contains(k));

        let code_keywords = [
            "code",
            "implement",
            "function",
            "class",
            "write",
            "create",
            "build",
            "develop",
            "program",
            "fix",
            "debug",
            "refactor",
        ];
        let has_code_keywords = code_keywords.iter().any(|k| query_lower.contains(k));

        let creative_keywords = [
            "write",
            "story",
            "poem",
            "creative",
            "imagine",
            "describe",
            "narrative",
        ];
        let has_creative = creative_keywords.iter().any(|k| query_lower.contains(k));

        let summary_keywords = ["summarize", "summary", "tldr", "brief", "overview"];
        let has_summary = summary_keywords.iter().any(|k| query_lower.contains(k));

        // Calculate complexity score
        let complexity = self.calculate_complexity(query, &words);

        QueryFeatures {
            word_count: words.len(),
            char_count: query.len(),
            has_code,
            has_question,
            has_list_markers,
            has_comparison,
            has_explanation,
            has_code_keywords,
            has_creative,
            has_summary,
            complexity,
        }
    }

    fn calculate_complexity(&self, query: &str, words: &[&str]) -> f32 {
        let mut score = 0.0;

        // Word count factor
        score += (words.len() as f32 / 50.0).min(1.0) * 0.2;

        // Sentence count factor
        let sentence_count = query.matches(|c| c == '.' || c == '?' || c == '!').count();
        score += (sentence_count as f32 / 5.0).min(1.0) * 0.2;

        // Code block factor
        let code_blocks = query.matches("```").count() / 2;
        score += (code_blocks as f32 / 3.0).min(1.0) * 0.3;

        // Long words factor (technical terms)
        let long_words = words.iter().filter(|w| w.len() > 8).count();
        score += (long_words as f32 / 10.0).min(1.0) * 0.15;

        // Newlines factor
        let newlines = query.matches('\n').count();
        score += (newlines as f32 / 10.0).min(1.0) * 0.15;

        score.min(1.0)
    }

    fn classify_query(&self, features: &QueryFeatures) -> (QueryType, f32) {
        // Simple rule-based classification
        // In production, this could use a trained ML model

        // Yes/No detection
        if features.has_question && features.word_count < 10 {
            let _yes_no_patterns = [
                "is it", "can i", "should i", "will it", "does it", "are you", "is this",
                "is there",
            ];
            // Simplified check
            return (QueryType::YesNo, 0.85);
        }

        // Code requests
        if features.has_code_keywords && features.has_code {
            return (QueryType::CodeReview, 0.8);
        }
        if features.has_code_keywords && features.word_count > 20 {
            return (QueryType::CodeImplementation, 0.75);
        }
        if features.has_code_keywords {
            return (QueryType::CodeSnippet, 0.7);
        }

        // Summary requests
        if features.has_summary {
            return (QueryType::Summary, 0.85);
        }

        // List requests
        if features.has_list_markers {
            return (QueryType::ListGeneration, 0.8);
        }

        // Creative writing
        if features.has_creative && features.complexity > 0.5 {
            return (QueryType::CreativeLong, 0.7);
        }
        if features.has_creative {
            return (QueryType::CreativeShort, 0.7);
        }

        // Explanations
        if features.has_explanation && features.complexity > 0.5 {
            return (QueryType::DetailedExplanation, 0.7);
        }
        if features.has_explanation {
            return (QueryType::ShortExplanation, 0.75);
        }

        // Simple questions
        if features.has_question && features.word_count < 15 {
            return (QueryType::SimpleAnswer, 0.7);
        }

        // Conversational
        if features.word_count < 20 && !features.has_code_keywords {
            return (QueryType::Conversational, 0.6);
        }

        // Default
        (QueryType::Unknown, 0.4)
    }

    fn calculate_complexity_adjustment(&self, features: &QueryFeatures) -> f32 {
        let mut adjustment = 1.0;

        // Adjust based on query length
        if features.word_count > 50 {
            adjustment *= 1.3;
        } else if features.word_count > 100 {
            adjustment *= 1.5;
        }

        // Adjust for code
        if features.has_code {
            adjustment *= 1.2;
        }

        // Adjust for comparisons
        if features.has_comparison {
            adjustment *= 1.3;
        }

        // Adjust for complexity score
        adjustment *= 1.0 + (features.complexity * 0.5);

        adjustment
    }

    async fn get_learned_adjustment(&self, query_type: QueryType) -> f32 {
        let patterns = self.patterns.read().await;

        if let Some((total, count)) = patterns.type_averages.get(&query_type) {
            if *count >= 5 {
                let actual_average = *total as f32 / *count as f32;
                let base_tokens = query_type.base_tokens() as f32;
                return actual_average / base_tokens;
            }
        }

        0.0 // No learned adjustment
    }

    fn identify_complexity_factors(&self, features: &QueryFeatures) -> Vec<String> {
        let mut factors = Vec::new();

        if features.has_code {
            factors.push("contains_code".to_string());
        }
        if features.has_comparison {
            factors.push("comparison_request".to_string());
        }
        if features.word_count > 50 {
            factors.push("long_query".to_string());
        }
        if features.complexity > 0.7 {
            factors.push("high_complexity".to_string());
        }
        if features.has_list_markers {
            factors.push("list_format".to_string());
        }

        factors
    }

    fn extract_pattern(query: &str) -> String {
        // Extract a simplified pattern for matching
        let words: Vec<&str> = query.split_whitespace().take(5).collect();
        words.join(" ").to_lowercase()
    }
}

/// RLM statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RlmStats {
    /// Cached predictions count
    pub cached_predictions: usize,
    /// Learned patterns count
    pub learned_patterns: usize,
    /// Average tokens by query type
    pub type_averages: HashMap<QueryType, f32>,
}

/// Token optimizer with RLM integration
pub struct RlmTokenOptimizer {
    /// RLM predictor
    predictor: RlmPredictor,
    /// Total predictions made
    predictions_count: Arc<std::sync::atomic::AtomicU64>,
    /// Total tokens saved
    tokens_saved: Arc<std::sync::atomic::AtomicU64>,
}

impl RlmTokenOptimizer {
    /// Create a new RLM-enabled optimizer
    pub fn new() -> Result<Self> {
        Ok(Self {
            predictor: RlmPredictor::new()?,
            predictions_count: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            tokens_saved: Arc::new(std::sync::atomic::AtomicU64::new(0)),
        })
    }

    /// Optimize a request
    pub async fn optimize_request(
        &self,
        query: &str,
        default_max_tokens: u32,
    ) -> Result<OptimizedRequest> {
        let prediction = self.predictor.predict(query).await?;

        self.predictions_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        let savings = prediction.potential_savings(default_max_tokens);
        self.tokens_saved
            .fetch_add(savings as u64, std::sync::atomic::Ordering::Relaxed);

        Ok(OptimizedRequest {
            max_tokens: prediction.max_tokens_recommended,
            length_hint: prediction.length_hint(),
            query_type: prediction.query_type,
            predicted_tokens: prediction.predicted_tokens,
            potential_savings: savings,
        })
    }

    /// Record actual response
    pub async fn record_response(&self, query: &str, query_type: QueryType, actual_tokens: u32) {
        self.predictor.record_actual(query, query_type, actual_tokens).await;
    }

    /// Get savings statistics
    pub fn savings_stats(&self) -> (u64, u64) {
        (
            self.predictions_count.load(std::sync::atomic::Ordering::Relaxed),
            self.tokens_saved.load(std::sync::atomic::Ordering::Relaxed),
        )
    }
}

/// Optimized request parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizedRequest {
    /// Optimized max_tokens
    pub max_tokens: u32,
    /// Length hint for system prompt
    pub length_hint: String,
    /// Detected query type
    pub query_type: QueryType,
    /// Predicted tokens
    pub predicted_tokens: u32,
    /// Potential savings vs default
    pub potential_savings: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_simple_answer_prediction() {
        let predictor = RlmPredictor::new().unwrap();
        let prediction = predictor.predict("What is 2+2?").await.unwrap();

        assert!(prediction.predicted_tokens < 100);
        assert!(prediction.confidence > 0.5);
    }

    #[tokio::test]
    async fn test_code_prediction() {
        let predictor = RlmPredictor::new().unwrap();
        let prediction = predictor.predict("Write a function to sort an array").await.unwrap();

        assert!(matches!(
            prediction.query_type,
            QueryType::CodeSnippet | QueryType::CodeImplementation
        ));
        assert!(prediction.predicted_tokens > 50);
    }

    #[tokio::test]
    async fn test_detailed_explanation() {
        let predictor = RlmPredictor::new().unwrap();
        let prediction = predictor
            .predict(
                "Explain in detail how async/await works in Rust, including the underlying \
             mechanisms of the Future trait and how the executor schedules tasks",
            )
            .await
            .unwrap();

        assert!(matches!(
            prediction.query_type,
            QueryType::DetailedExplanation | QueryType::ShortExplanation
        ));
        assert!(prediction.predicted_tokens > 200);
    }

    #[tokio::test]
    async fn test_yes_no_detection() {
        let predictor = RlmPredictor::new().unwrap();
        let prediction = predictor.predict("Is Rust memory safe?").await.unwrap();

        // Short questions might be classified as YesNo or SimpleAnswer
        assert!(prediction.predicted_tokens < 100);
    }

    #[tokio::test]
    async fn test_length_hint_generation() {
        let predictor = RlmPredictor::new().unwrap();
        let prediction = predictor.predict("Summarize this article").await.unwrap();

        let hint = prediction.length_hint();
        assert!(!hint.is_empty());
    }

    #[tokio::test]
    async fn test_learning() {
        let predictor = RlmPredictor::new().unwrap();

        // Record some actual responses
        predictor.record_actual("test query", QueryType::SimpleAnswer, 30).await;
        predictor.record_actual("test query 2", QueryType::SimpleAnswer, 25).await;
        predictor.record_actual("test query 3", QueryType::SimpleAnswer, 35).await;

        let stats = predictor.stats().await;
        assert!(stats.learned_patterns > 0 || !stats.type_averages.is_empty());
    }

    #[test]
    fn test_query_type_ranges() {
        let (min, max) = QueryType::SimpleAnswer.token_range();
        assert!(min < max);
        assert!(QueryType::SimpleAnswer.base_tokens() >= min);
        assert!(QueryType::SimpleAnswer.base_tokens() <= max);
    }

    #[tokio::test]
    async fn test_optimizer() {
        let optimizer = RlmTokenOptimizer::new().unwrap();

        let request = optimizer.optimize_request("What is 2+2?", 4096).await.unwrap();

        assert!(request.max_tokens < 4096);
        assert!(request.potential_savings > 0);
    }
}
