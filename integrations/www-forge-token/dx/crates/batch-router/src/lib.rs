//! # batch-router
//!
//! Routes non-urgent tasks to the Batch API for 50% cost savings.
//!
//! ## Evidence (TOKEN.md ✅ REAL — hard fact from OpenAI pricing)
//! - OpenAI Batch API: 50% cost discount on inputs and outputs
//! - Completes within 24 hours (often faster)
//! - **Honest savings: 50% cost** (confirmed by OpenAI pricing page)
//! - Caveat: keyword matching for batch eligibility is crude
//!
//! STAGE: PreCall (priority 15)

use dx_core::*;
use std::sync::Mutex;

/// Whether a task should be batched.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BatchDecision {
    /// Send via real-time API (user is waiting)
    RealTime,
    /// Send via Batch API (50% cost savings, ≤24h completion)
    Batch,
}

/// Configuration for batch routing.
#[derive(Debug, Clone)]
pub struct BatchRouterConfig {
    /// Keywords suggesting the task is batch-eligible (non-urgent)
    pub batch_keywords: Vec<String>,
    /// Keywords suggesting real-time is needed
    pub realtime_keywords: Vec<String>,
    /// Minimum message count to consider batching (short conversations are interactive)
    pub min_messages_for_batch: usize,
    /// Whether to batch by default if no signals either way
    pub default_to_batch: bool,
}

impl Default for BatchRouterConfig {
    fn default() -> Self {
        Self {
            batch_keywords: vec![
                "analyze".into(), "report".into(), "summarize all".into(),
                "bulk".into(), "batch".into(), "background".into(),
                "when you have time".into(), "no rush".into(),
                "overnight".into(), "generate docs".into(),
                "run tests".into(), "lint all".into(),
                "process all".into(), "migrate".into(),
            ],
            realtime_keywords: vec![
                "urgent".into(), "now".into(), "immediately".into(),
                "asap".into(), "quick".into(), "help me".into(),
                "fix this".into(), "error".into(), "broken".into(),
                "debug".into(), "why".into(), "how".into(),
            ],
            min_messages_for_batch: 1,
            default_to_batch: false,
        }
    }
}

pub struct BatchRouterSaver {
    config: BatchRouterConfig,
    last_decision: Mutex<Option<BatchDecision>>,
    report: Mutex<TokenSavingsReport>,
}

impl BatchRouterSaver {
    pub fn new() -> Self {
        Self::with_config(BatchRouterConfig::default())
    }

    pub fn with_config(config: BatchRouterConfig) -> Self {
        Self {
            config,
            last_decision: Mutex::new(None),
            report: Mutex::new(TokenSavingsReport::default()),
        }
    }

    /// Get the last routing decision.
    pub fn last_decision(&self) -> Option<BatchDecision> {
        *self.last_decision.lock().unwrap()
    }

    /// Classify whether the current task is batch-eligible.
    fn classify(&self, messages: &[Message]) -> BatchDecision {
        let user_text = messages.iter().rev()
            .find(|m| m.role == "user")
            .map(|m| m.content.to_lowercase())
            .unwrap_or_default();

        let batch_signals: usize = self.config.batch_keywords.iter()
            .filter(|kw| user_text.contains(kw.as_str()))
            .count();

        let realtime_signals: usize = self.config.realtime_keywords.iter()
            .filter(|kw| user_text.contains(kw.as_str()))
            .count();

        if realtime_signals > batch_signals {
            BatchDecision::RealTime
        } else if batch_signals > 0 && batch_signals > realtime_signals {
            BatchDecision::Batch
        } else if self.config.default_to_batch {
            BatchDecision::Batch
        } else {
            BatchDecision::RealTime
        }
    }
}

#[async_trait::async_trait]
impl TokenSaver for BatchRouterSaver {
    fn name(&self) -> &str { "batch-router" }
    fn stage(&self) -> SaverStage { SaverStage::PreCall }
    fn priority(&self) -> u32 { 15 }

    async fn process(
        &self,
        input: SaverInput,
        _ctx: &SaverContext,
    ) -> Result<SaverOutput, SaverError> {
        let tokens = input.messages.iter().map(|m| m.token_count).sum::<usize>()
            + input.tools.iter().map(|t| t.token_count).sum::<usize>();

        let decision = self.classify(&input.messages);
        *self.last_decision.lock().unwrap() = Some(decision);

        // Batch API gives 50% off on both input and output
        let tokens_saved = if decision == BatchDecision::Batch {
            tokens / 2 // 50% cost savings
        } else {
            0
        };

        let report = TokenSavingsReport {
            technique: "batch-router".into(),
            tokens_before: tokens,
            tokens_after: tokens, // tokens don't change, cost does
            tokens_saved,
            description: match decision {
                BatchDecision::Batch => format!(
                    "Routed to Batch API: 50% cost savings (~{} tokens equivalent). \
                     Task appears non-urgent. Completes within 24h.",
                    tokens_saved
                ),
                BatchDecision::RealTime => "Real-time API: task requires immediate response.".into(),
            },
        };
        *self.report.lock().unwrap() = report;

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
        Message { role: "user".into(), content: content.into(), images: vec![], tool_call_id: None, token_count: content.len() / 4 }
    }

    #[tokio::test]
    async fn test_batch_eligible() {
        let saver = BatchRouterSaver::new();
        let ctx = SaverContext::default();
        let input = SaverInput {
            messages: vec![user_msg("please generate docs for all modules in the background when you have time")],
            tools: vec![],
            images: vec![],
            turn_number: 1,
        };
        let _ = saver.process(input, &ctx).await.unwrap();
        assert_eq!(saver.last_decision(), Some(BatchDecision::Batch));
    }

    #[tokio::test]
    async fn test_realtime_urgent() {
        let saver = BatchRouterSaver::new();
        let ctx = SaverContext::default();
        let input = SaverInput {
            messages: vec![user_msg("help me fix this error now, it's broken")],
            tools: vec![],
            images: vec![],
            turn_number: 1,
        };
        let _ = saver.process(input, &ctx).await.unwrap();
        assert_eq!(saver.last_decision(), Some(BatchDecision::RealTime));
    }
}
