//! LLM Model Definitions with Pricing
//!
//! Comprehensive list of LLM models with their pricing per 1M tokens.
//! Data sourced from official provider pricing pages (January 2026).
//!
//! # Stability
//!
//! This module is **experimental** and not part of the stable API.
//! It may use `unwrap()` and `expect()` for convenience as it's not production code.

// Allow unwrap/expect in experimental LLM models code
#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]

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
    pub name: &'static str,
    pub provider: Provider,
    pub context_window: &'static str,
    pub input_per_1m: f64,
    pub input_cached_per_1m: f64,
    pub output_per_1m: f64,
    pub chars_per_token: f64,
}

/// All supported LLM models (January 2026)
pub static LLM_MODELS: &[LlmModel] = &[
    // OpenAI/Azure
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
    // Anthropic
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
    // Google
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
    pub fn estimate_tokens(&self, text: &str) -> usize {
        ((text.chars().count() as f64) / self.chars_per_token).ceil() as usize
    }

    pub fn calculate_input_cost(&self, tokens: usize) -> f64 {
        (tokens as f64 / 1_000_000.0) * self.input_per_1m
    }

    pub fn calculate_cached_cost(&self, tokens: usize) -> f64 {
        (tokens as f64 / 1_000_000.0) * self.input_cached_per_1m
    }

    pub fn calculate_output_cost(&self, tokens: usize) -> f64 {
        (tokens as f64 / 1_000_000.0) * self.output_per_1m
    }
}

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

pub fn analyze_all_models(text: &str) -> Vec<TokenAnalysis> {
    LLM_MODELS
        .iter()
        .map(|m| {
            let tokens = m.estimate_tokens(text);
            TokenAnalysis {
                model_name: m.name,
                provider: m.provider,
                context_window: m.context_window,
                tokens,
                input_cost: m.calculate_input_cost(tokens),
                cached_cost: m.calculate_cached_cost(tokens),
                output_cost: m.calculate_output_cost(tokens),
            }
        })
        .collect()
}

pub fn format_cost(cost: f64) -> String {
    if cost == 0.0 {
        "$0.0000".into()
    } else if cost < 0.0001 {
        "<$0.0001".into()
    } else if cost < 1.0 {
        format!("${:.4}", cost)
    } else {
        format!("${:.2}", cost)
    }
}

pub fn format_tokens(tokens: usize) -> String {
    if tokens >= 1_000_000 {
        format!("{:.2}M", tokens as f64 / 1_000_000.0)
    } else if tokens >= 1_000 {
        format!("{:.1}K", tokens as f64 / 1_000.0)
    } else {
        tokens.to_string()
    }
}
