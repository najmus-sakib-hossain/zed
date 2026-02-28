//! LLM Model Definitions with Pricing
//!
//! Comprehensive list of LLM models with their pricing per 1M tokens.
//! Data sourced from official provider pricing pages (January 2026).

use std::fmt;

/// LLM Provider
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Provider {
    OpenAI,
    Anthropic,
    Google,
}

impl fmt::Display for Provider {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Provider::OpenAI => write!(f, "OpenAI/Azure"),
            Provider::Anthropic => write!(f, "Anthropic"),
            Provider::Google => write!(f, "Google"),
        }
    }
}

/// LLM Model definition with pricing
#[derive(Debug, Clone)]
pub struct LlmModel {
    /// Model name (e.g., "GPT-5.2", "Claude Sonnet 4")
    pub name: &'static str,
    /// Provider
    pub provider: Provider,
    /// Context window size (e.g., "400K", "200K", "1M", "2M")
    pub context_window: &'static str,
    /// Price per 1M input tokens in USD
    pub input_per_1m: f64,
    /// Price per 1M cached input tokens in USD
    pub input_cached_per_1m: f64,
    /// Price per 1M output tokens in USD
    pub output_per_1m: f64,
    /// Characters per token ratio (for estimation)
    pub chars_per_token: f64,
}

/// All supported LLM models with current pricing (January 2026)
pub static LLM_MODELS: &[LlmModel] = &[
    // OpenAI/Azure Models
    LlmModel {
        name: "GPT-5.2",
        provider: Provider::OpenAI,
        context_window: "400K",
        input_per_1m: 1.75,
        input_cached_per_1m: 0.175,
        output_per_1m: 14.0,
        chars_per_token: 4.0,
    },
    LlmModel {
        name: "GPT-5.2 pro",
        provider: Provider::OpenAI,
        context_window: "400K",
        input_per_1m: 21.0,
        input_cached_per_1m: 0.0,
        output_per_1m: 168.0,
        chars_per_token: 4.0,
    },
    LlmModel {
        name: "GPT-5 mini",
        provider: Provider::OpenAI,
        context_window: "400K",
        input_per_1m: 0.25,
        input_cached_per_1m: 0.025,
        output_per_1m: 2.0,
        chars_per_token: 4.0,
    },
    // Anthropic Models
    LlmModel {
        name: "Claude Opus 4.5",
        provider: Provider::Anthropic,
        context_window: "200K",
        input_per_1m: 5.0,
        input_cached_per_1m: 0.5,
        output_per_1m: 25.0,
        chars_per_token: 3.8,
    },
    LlmModel {
        name: "Claude Opus 4",
        provider: Provider::Anthropic,
        context_window: "200K",
        input_per_1m: 15.0,
        input_cached_per_1m: 1.5,
        output_per_1m: 75.0,
        chars_per_token: 3.8,
    },
    LlmModel {
        name: "Claude Sonnet 4.5",
        provider: Provider::Anthropic,
        context_window: "200K",
        input_per_1m: 3.0,
        input_cached_per_1m: 0.3,
        output_per_1m: 15.0,
        chars_per_token: 3.8,
    },
    LlmModel {
        name: "Claude Sonnet 4",
        provider: Provider::Anthropic,
        context_window: "200K",
        input_per_1m: 3.0,
        input_cached_per_1m: 0.3,
        output_per_1m: 15.0,
        chars_per_token: 3.8,
    },
    LlmModel {
        name: "Claude Haiku 4.5",
        provider: Provider::Anthropic,
        context_window: "200K",
        input_per_1m: 1.0,
        input_cached_per_1m: 0.1,
        output_per_1m: 5.0,
        chars_per_token: 3.8,
    },
    LlmModel {
        name: "Claude Haiku 3.5",
        provider: Provider::Anthropic,
        context_window: "200K",
        input_per_1m: 0.8,
        input_cached_per_1m: 0.08,
        output_per_1m: 4.0,
        chars_per_token: 3.8,
    },
    // Google Models
    LlmModel {
        name: "Gemini 3 Pro (Preview)",
        provider: Provider::Google,
        context_window: "1M",
        input_per_1m: 2.0,
        input_cached_per_1m: 0.2,
        output_per_1m: 12.0,
        chars_per_token: 4.2,
    },
    LlmModel {
        name: "Gemini 3 Flash (Preview)",
        provider: Provider::Google,
        context_window: "1M",
        input_per_1m: 0.5,
        input_cached_per_1m: 0.05,
        output_per_1m: 3.0,
        chars_per_token: 4.2,
    },
    LlmModel {
        name: "Gemini 2.5 Pro",
        provider: Provider::Google,
        context_window: "2M",
        input_per_1m: 1.25,
        input_cached_per_1m: 0.125,
        output_per_1m: 10.0,
        chars_per_token: 4.2,
    },
    LlmModel {
        name: "Gemini 2.5 Flash",
        provider: Provider::Google,
        context_window: "1M",
        input_per_1m: 0.3,
        input_cached_per_1m: 0.03,
        output_per_1m: 2.5,
        chars_per_token: 4.2,
    },
    LlmModel {
        name: "Gemini 2.5 Flash-Lite",
        provider: Provider::Google,
        context_window: "1M",
        input_per_1m: 0.1,
        input_cached_per_1m: 0.01,
        output_per_1m: 0.4,
        chars_per_token: 4.2,
    },
    LlmModel {
        name: "Gemini 2.0 Flash",
        provider: Provider::Google,
        context_window: "1M",
        input_per_1m: 0.1,
        input_cached_per_1m: 0.025,
        output_per_1m: 0.4,
        chars_per_token: 4.2,
    },
    LlmModel {
        name: "Gemini 2.0 Flash-Lite",
        provider: Provider::Google,
        context_window: "1M",
        input_per_1m: 0.075,
        input_cached_per_1m: 0.0,
        output_per_1m: 0.3,
        chars_per_token: 4.2,
    },
];

impl LlmModel {
    /// Estimate token count for given text
    pub fn estimate_tokens(&self, text: &str) -> usize {
        let char_count = text.chars().count();
        ((char_count as f64) / self.chars_per_token).ceil() as usize
    }

    /// Calculate input cost for given token count
    pub fn calculate_input_cost(&self, tokens: usize) -> f64 {
        (tokens as f64 / 1_000_000.0) * self.input_per_1m
    }

    /// Calculate cached input cost for given token count
    pub fn calculate_cached_cost(&self, tokens: usize) -> f64 {
        (tokens as f64 / 1_000_000.0) * self.input_cached_per_1m
    }

    /// Calculate output cost for given token count
    pub fn calculate_output_cost(&self, tokens: usize) -> f64 {
        (tokens as f64 / 1_000_000.0) * self.output_per_1m
    }
}

/// Token analysis result for a single model
#[derive(Debug, Clone)]
pub struct TokenAnalysis {
    pub model_name: &'static str,
    pub provider: Provider,
    pub context_window: &'static str,
    pub tokens: usize,
    pub input_cost: f64,
    pub cached_cost: f64,
    pub output_cost: f64,
}

/// Analyze text for all LLM models
pub fn analyze_all_models(text: &str) -> Vec<TokenAnalysis> {
    LLM_MODELS
        .iter()
        .map(|model| {
            let tokens = model.estimate_tokens(text);
            TokenAnalysis {
                model_name: model.name,
                provider: model.provider,
                context_window: model.context_window,
                tokens,
                input_cost: model.calculate_input_cost(tokens),
                cached_cost: model.calculate_cached_cost(tokens),
                output_cost: model.calculate_output_cost(tokens),
            }
        })
        .collect()
}

/// Format cost as string
pub fn format_cost(cost: f64) -> String {
    if cost == 0.0 {
        "$0.0000".to_string()
    } else if cost < 0.0001 {
        "<$0.0001".to_string()
    } else if cost < 1.0 {
        // Use 4 decimal places for costs under $1
        format!("${:.4}", cost)
    } else {
        format!("${:.2}", cost)
    }
}

/// Format token count
pub fn format_tokens(tokens: usize) -> String {
    if tokens >= 1_000_000 {
        format!("{:.2}M", tokens as f64 / 1_000_000.0)
    } else if tokens >= 1_000 {
        format!("{:.1}K", tokens as f64 / 1_000.0)
    } else {
        tokens.to_string()
    }
}

/// Get models grouped by provider
pub fn models_by_provider() -> Vec<(Provider, Vec<&'static LlmModel>)> {
    let mut openai = Vec::new();
    let mut anthropic = Vec::new();
    let mut google = Vec::new();

    for model in LLM_MODELS {
        match model.provider {
            Provider::OpenAI => openai.push(model),
            Provider::Anthropic => anthropic.push(model),
            Provider::Google => google.push(model),
        }
    }

    vec![
        (Provider::OpenAI, openai),
        (Provider::Anthropic, anthropic),
        (Provider::Google, google),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_estimation() {
        let text = "Hello, world!"; // 13 chars
        let gpt5 = &LLM_MODELS[0]; // GPT-5.2, 4.0 chars/token
        assert_eq!(gpt5.estimate_tokens(text), 4); // ceil(13/4.0) = 4
    }

    #[test]
    fn test_cost_calculation() {
        let gpt5_mini = &LLM_MODELS[2]; // GPT-5 mini
        let tokens = 1_000_000;
        assert!((gpt5_mini.calculate_input_cost(tokens) - 0.25).abs() < 0.001);
    }

    #[test]
    fn test_format_cost() {
        assert_eq!(format_cost(0.0), "$0.0000");
        assert_eq!(format_cost(0.00001), "<$0.0001");
        assert_eq!(format_cost(0.0012), "$0.0012");
        assert_eq!(format_cost(1.5), "$1.50");
    }

    #[test]
    fn test_format_tokens() {
        assert_eq!(format_tokens(500), "500");
        assert_eq!(format_tokens(1500), "1.5K");
        assert_eq!(format_tokens(1_500_000), "1.50M");
    }
}
