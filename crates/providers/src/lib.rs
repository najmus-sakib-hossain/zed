pub mod mlvoca;
pub mod opencode;
pub mod pollinations;

/// Total number of free models across all providers.
///
/// - Pollinations: 1 model (openai-fast)
/// - mlvoca: 2 models (tinyllama, deepseek-r1:1.5b)
pub const TOTAL_FREE_MODELS: usize = 3;