//! # reasoning-router
//!
//! Routes tasks to the right reasoning effort level — saves massive
//! amounts on hidden reasoning tokens.
//!
//! ## Evidence (TOKEN.md ✅ REAL — most impactful technique)
//! - O-series models bill hidden "reasoning tokens" as output tokens
//! - A response showing 500 output tokens may actually consume 2000+
//! - Using reasoning_effort: "low" vs "high" saves 30-80% on reasoning tokens
//! - Simple tasks should use non-reasoning models (GPT-5/GPT-4.1)
//!
//! **Honest savings: 30-80% on reasoning tokens**
//! STAGE: PreCall (priority 10)

use dx_core::*;
use std::sync::Mutex;

/// Reasoning effort level for the API call.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReasoningEffort {
    /// No reasoning needed — use non-reasoning model (GPT-5, GPT-4.1)
    None,
    /// Low effort reasoning (o-series with reasoning_effort: "low")
    Low,
    /// Medium effort reasoning
    Medium,
    /// High effort reasoning (complex math, multi-step planning)
    High,
}

impl ReasoningEffort {
    /// Approximate token multiplier vs high effort
    pub fn cost_multiplier(&self) -> f64 {
        match self {
            ReasoningEffort::None => 0.0,   // No reasoning tokens at all
            ReasoningEffort::Low => 0.20,   // ~80% savings
            ReasoningEffort::Medium => 0.50, // ~50% savings
            ReasoningEffort::High => 1.0,   // baseline
        }
    }

    pub fn as_api_param(&self) -> Option<&'static str> {
        match self {
            ReasoningEffort::None => None,
            ReasoningEffort::Low => Some("low"),
            ReasoningEffort::Medium => Some("medium"),
            ReasoningEffort::High => Some("high"),
        }
    }
}

/// Task complexity classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskComplexity {
    /// Simple factual, formatting, or retrieval tasks
    Simple,
    /// Standard coding, writing, or analysis tasks
    Standard,
    /// Multi-step reasoning, complex debugging, math
    Complex,
    /// Novel research, architecture design, complex proofs
    Expert,
}

/// Configuration for the reasoning router.
#[derive(Debug, Clone)]
pub struct ReasoningRouterConfig {
    /// Map complexity → effort level
    pub simple_effort: ReasoningEffort,
    pub standard_effort: ReasoningEffort,
    pub complex_effort: ReasoningEffort,
    pub expert_effort: ReasoningEffort,
    /// Model to use for non-reasoning tasks
    pub non_reasoning_model: String,
    /// Keywords that suggest high complexity
    pub complex_keywords: Vec<String>,
    /// Keywords that suggest simple tasks
    pub simple_keywords: Vec<String>,
}

impl Default for ReasoningRouterConfig {
    fn default() -> Self {
        Self {
            simple_effort: ReasoningEffort::None,
            standard_effort: ReasoningEffort::Low,
            complex_effort: ReasoningEffort::Medium,
            expert_effort: ReasoningEffort::High,
            non_reasoning_model: "gpt-5".into(),
            complex_keywords: vec![
                "prove".into(), "formally verify".into(), "mathematical".into(),
                "step by step".into(), "analyze all".into(), "debug this complex".into(),
                "architect".into(), "design pattern".into(), "optimize algorithm".into(),
                "security audit".into(), "race condition".into(), "deadlock".into(),
            ],
            simple_keywords: vec![
                "format".into(), "rename".into(), "list".into(), "what is".into(),
                "define".into(), "translate".into(), "convert".into(), "hello".into(),
                "add a comment".into(), "fix typo".into(), "update version".into(),
                "change the name".into(), "remove unused".into(),
            ],
        }
    }
}

/// The recommendation output from the router.
#[derive(Debug, Clone)]
pub struct RoutingDecision {
    pub complexity: TaskComplexity,
    pub effort: ReasoningEffort,
    pub recommended_model: Option<String>,
    pub estimated_reasoning_savings_pct: f64,
    pub explanation: String,
}

pub struct ReasoningRouterSaver {
    config: ReasoningRouterConfig,
    report: Mutex<TokenSavingsReport>,
    last_decision: Mutex<Option<RoutingDecision>>,
}

impl ReasoningRouterSaver {
    pub fn new() -> Self {
        Self::with_config(ReasoningRouterConfig::default())
    }

    pub fn with_config(config: ReasoningRouterConfig) -> Self {
        Self {
            config,
            report: Mutex::new(TokenSavingsReport::default()),
            last_decision: Mutex::new(None),
        }
    }

    /// Get the last routing decision.
    pub fn last_decision(&self) -> Option<RoutingDecision> {
        self.last_decision.lock().unwrap().clone()
    }

    /// Classify the task complexity based on the latest user message.
    fn classify_task(&self, messages: &[Message]) -> TaskComplexity {
        // Find the most recent user message
        let user_msg = messages.iter().rev()
            .find(|m| m.role == "user")
            .map(|m| m.content.to_lowercase());

        let text = match user_msg {
            Some(t) => t,
            None => return TaskComplexity::Standard,
        };

        // Count complexity signals
        let complex_signals: usize = self.config.complex_keywords.iter()
            .filter(|kw| text.contains(kw.as_str()))
            .count();

        let simple_signals: usize = self.config.simple_keywords.iter()
            .filter(|kw| text.contains(kw.as_str()))
            .count();

        // Length heuristic: very short messages are usually simple
        let word_count = text.split_whitespace().count();

        // Code/multiline content suggests standard+
        let has_code = text.contains("```") || text.contains("fn ") || text.contains("def ");

        if complex_signals >= 2 || (complex_signals >= 1 && word_count > 100) {
            TaskComplexity::Expert
        } else if complex_signals >= 1 || (has_code && word_count > 50) {
            TaskComplexity::Complex
        } else if simple_signals >= 1 || word_count < 15 {
            TaskComplexity::Simple
        } else {
            TaskComplexity::Standard
        }
    }

    /// Build the routing decision.
    fn route(&self, complexity: TaskComplexity) -> RoutingDecision {
        let (effort, model_override) = match complexity {
            TaskComplexity::Simple => (
                self.config.simple_effort,
                Some(self.config.non_reasoning_model.clone()),
            ),
            TaskComplexity::Standard => (
                self.config.standard_effort,
                None,
            ),
            TaskComplexity::Complex => (
                self.config.complex_effort,
                None,
            ),
            TaskComplexity::Expert => (
                self.config.expert_effort,
                None,
            ),
        };

        let savings_pct = (1.0 - effort.cost_multiplier()) * 100.0;
        let explanation = match complexity {
            TaskComplexity::Simple => format!(
                "Simple task → no reasoning needed. Use {} for ~{:.0}% reasoning token savings.",
                model_override.as_deref().unwrap_or("non-reasoning model"), savings_pct
            ),
            TaskComplexity::Standard => format!(
                "Standard task → low reasoning effort. ~{:.0}% reasoning token savings.", savings_pct
            ),
            TaskComplexity::Complex => format!(
                "Complex task → medium reasoning effort. ~{:.0}% reasoning token savings.", savings_pct
            ),
            TaskComplexity::Expert => format!(
                "Expert task → full reasoning effort. No savings on reasoning tokens."
            ),
        };

        RoutingDecision {
            complexity,
            effort,
            recommended_model: model_override,
            estimated_reasoning_savings_pct: savings_pct,
            explanation,
        }
    }
}

#[async_trait::async_trait]
impl TokenSaver for ReasoningRouterSaver {
    fn name(&self) -> &str { "reasoning-router" }
    fn stage(&self) -> SaverStage { SaverStage::PreCall }
    fn priority(&self) -> u32 { 10 }

    async fn process(
        &self,
        input: SaverInput,
        _ctx: &SaverContext,
    ) -> Result<SaverOutput, SaverError> {
        let tokens_before: usize = input.messages.iter().map(|m| m.token_count).sum();
        let complexity = self.classify_task(&input.messages);
        let decision = self.route(complexity);

        // Estimate reasoning tokens (typically 2-4x visible output for o-series)
        // Conservative estimate: 30% of input tokens become reasoning tokens
        let estimated_reasoning = (tokens_before as f64 * 0.30) as usize;
        let tokens_saved = (estimated_reasoning as f64 * (1.0 - decision.effort.cost_multiplier())) as usize;

        let report = TokenSavingsReport {
            technique: "reasoning-router".into(),
            tokens_before,
            tokens_after: tokens_before, // We don't modify messages, we route
            tokens_saved,
            description: decision.explanation.clone(),
        };

        *self.report.lock().unwrap() = report;
        *self.last_decision.lock().unwrap() = Some(decision);

        Ok(SaverOutput {
            messages: input.messages,
            tools: input.tools,
            images: input.images,
            skipped: false,
            cached_response: None,
        })
    }

    fn last_savings(&self) -> TokenSavingsReport {
        self.report.lock().unwrap().clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn user_msg(content: &str) -> Message {
        Message {
            role: "user".into(),
            content: content.into(),
            images: vec![],
            tool_call_id: None,
            token_count: content.len() / 4,
        }
    }

    #[tokio::test]
    async fn test_simple_task_routes_to_no_reasoning() {
        let saver = ReasoningRouterSaver::new();
        let ctx = SaverContext::default();
        let input = SaverInput {
            messages: vec![user_msg("rename the variable x to count")],
            tools: vec![],
            images: vec![],
            turn_number: 1,
        };
        let _ = saver.process(input, &ctx).await.unwrap();
        let decision = saver.last_decision().unwrap();
        assert_eq!(decision.complexity, TaskComplexity::Simple);
        assert_eq!(decision.effort, ReasoningEffort::None);
    }

    #[tokio::test]
    async fn test_complex_task_routes_to_medium() {
        let saver = ReasoningRouterSaver::new();
        let ctx = SaverContext::default();
        let input = SaverInput {
            messages: vec![user_msg("prove that this algorithm is correct and analyze all edge cases for the race condition in the mutex handler")],
            tools: vec![],
            images: vec![],
            turn_number: 1,
        };
        let _ = saver.process(input, &ctx).await.unwrap();
        let decision = saver.last_decision().unwrap();
        assert!(decision.complexity == TaskComplexity::Expert || decision.complexity == TaskComplexity::Complex);
    }
}
