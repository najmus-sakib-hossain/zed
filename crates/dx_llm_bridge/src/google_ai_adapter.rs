//! Google AI (Gemini) adapter — bridges to dx_core LlmProvider trait.

use anyhow::Result;
use dx_core::cost::{MicroCost, TokenPricing};
use dx_core::llm_provider::*;
use futures::stream::BoxStream;
use futures::StreamExt;
use http_client::HttpClient;
use serde_json::json;
use std::sync::Arc;

pub struct GoogleAiAdapter {
    id: LlmProviderId,
    api_key: String,
    http_client: Arc<dyn HttpClient>,
    available: parking_lot::RwLock<bool>,
}

impl GoogleAiAdapter {
    pub fn new(api_key: String, http_client: Arc<dyn HttpClient>) -> Self {
        Self {
            id: LlmProviderId::new("google-ai"),
            api_key,
            http_client,
            available: parking_lot::RwLock::new(true),
        }
    }

    fn build_contents(&self, messages: &[LlmMessage]) -> (Option<serde_json::Value>, Vec<serde_json::Value>) {
        let mut system_instruction = None;
        let mut contents = Vec::new();

        for m in messages {
            match m.role {
                LlmRole::System => {
                    system_instruction = Some(json!({
                        "parts": [{"text": m.content}]
                    }));
                }
                _ => {
                    let role = match m.role {
                        LlmRole::User | LlmRole::Tool => "user",
                        LlmRole::Assistant => "model",
                        LlmRole::System => unreachable!(),
                    };
                    let mut parts = vec![json!({"text": m.content})];
                    for img in &m.images {
                        parts.push(json!({
                            "inline_data": {
                                "mime_type": "image/png",
                                "data": img
                            }
                        }));
                    }
                    contents.push(json!({
                        "role": role,
                        "parts": parts
                    }));
                }
            }
        }

        (system_instruction, contents)
    }
}

#[async_trait::async_trait]
impl LlmProvider for GoogleAiAdapter {
    fn id(&self) -> &LlmProviderId {
        &self.id
    }

    fn name(&self) -> &str {
        "Google AI (Gemini)"
    }

    fn tier(&self) -> LlmProviderTier {
        LlmProviderTier::Native
    }

    fn is_available(&self) -> bool {
        *self.available.read()
    }

    async fn list_models(&self) -> Result<Vec<LlmModelInfo>> {
        Ok(vec![
            LlmModelInfo {
                id: "gemini-2.0-flash".into(),
                name: "Gemini 2.0 Flash".into(),
                provider_id: self.id.clone(),
                context_window: 1_048_576,
                max_output_tokens: Some(8_192),
                pricing: Some(TokenPricing {
                    input_per_million: MicroCost::from_dollars(0.075),
                    output_per_million: MicroCost::from_dollars(0.30),
                    cached_input_per_million: Some(MicroCost::from_dollars(0.01875)),
                }),
                supports_streaming: true,
                supports_tools: true,
                supports_vision: true,
            },
            LlmModelInfo {
                id: "gemini-2.5-pro".into(),
                name: "Gemini 2.5 Pro".into(),
                provider_id: self.id.clone(),
                context_window: 1_048_576,
                max_output_tokens: Some(65_536),
                pricing: Some(TokenPricing {
                    input_per_million: MicroCost::from_dollars(1.25),
                    output_per_million: MicroCost::from_dollars(10.00),
                    cached_input_per_million: Some(MicroCost::from_dollars(0.3125)),
                }),
                supports_streaming: true,
                supports_tools: true,
                supports_vision: true,
            },
            LlmModelInfo {
                id: "gemini-2.5-flash".into(),
                name: "Gemini 2.5 Flash".into(),
                provider_id: self.id.clone(),
                context_window: 1_048_576,
                max_output_tokens: Some(65_536),
                pricing: Some(TokenPricing {
                    input_per_million: MicroCost::from_dollars(0.15),
                    output_per_million: MicroCost::from_dollars(0.60),
                    cached_input_per_million: Some(MicroCost::from_dollars(0.0375)),
                }),
                supports_streaming: true,
                supports_tools: true,
                supports_vision: true,
            },
        ])
    }

    async fn complete(&self, request: &LlmRequest) -> Result<LlmResponse> {
        let (system_instruction, contents) = self.build_contents(&request.messages);
        let mut body = json!({
            "contents": contents,
        });

        if let Some(si) = system_instruction {
            body["systemInstruction"] = si;
        }

        let mut gen_config = json!({});
        if let Some(max_tokens) = request.max_tokens {
            gen_config["maxOutputTokens"] = json!(max_tokens);
        }
        if let Some(temp) = request.temperature {
            gen_config["temperature"] = json!(temp);
        }
        if let Some(top_p) = request.top_p {
            gen_config["topP"] = json!(top_p);
        }
        if !request.stop_sequences.is_empty() {
            gen_config["stopSequences"] = json!(request.stop_sequences);
        }
        body["generationConfig"] = gen_config;

        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
            request.model, self.api_key
        );

        let http_request = http_client::Request::builder()
            .method(http_client::Method::POST)
            .uri(&url)
            .header("Content-Type", "application/json")
            .body(http_client::Body::from(serde_json::to_vec(&body)?))?;

        let mut response = self.http_client.send(http_request).await?;
        let body_str = http_client::read_body_to_string(&mut response).await?;

        if !response.status().is_success() {
            *self.available.write() = false;
            anyhow::bail!("Google AI error {}: {}", response.status(), body_str);
        }

        let resp: serde_json::Value = serde_json::from_str(&body_str)?;

        let content = resp["candidates"][0]["content"]["parts"][0]["text"]
            .as_str()
            .unwrap_or("")
            .to_string();

        let input_tokens = resp["usageMetadata"]["promptTokenCount"]
            .as_u64()
            .unwrap_or(0) as usize;
        let output_tokens = resp["usageMetadata"]["candidatesTokenCount"]
            .as_u64()
            .unwrap_or(0) as usize;
        let finish_reason = resp["candidates"][0]["finishReason"]
            .as_str()
            .map(String::from);

        let cost = self
            .pricing(&request.model)
            .map(|p| {
                let ic = MicroCost((p.input_per_million.0 as f64 * input_tokens as f64 / 1_000_000.0) as u64);
                let oc = MicroCost((p.output_per_million.0 as f64 * output_tokens as f64 / 1_000_000.0) as u64);
                ic + oc
            })
            .unwrap_or(MicroCost::ZERO);

        Ok(LlmResponse {
            content,
            model: request.model.clone(),
            input_tokens,
            output_tokens,
            cost,
            finish_reason,
        })
    }

    async fn stream(
        &self,
        request: &LlmRequest,
    ) -> Result<BoxStream<'static, Result<LlmStreamChunk>>> {
        let (system_instruction, contents) = self.build_contents(&request.messages);
        let mut body = json!({ "contents": contents });
        if let Some(si) = system_instruction {
            body["systemInstruction"] = si;
        }

        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:streamGenerateContent?alt=sse&key={}",
            request.model, self.api_key
        );

        let http_request = http_client::Request::builder()
            .method(http_client::Method::POST)
            .uri(&url)
            .header("Content-Type", "application/json")
            .body(http_client::Body::from(serde_json::to_vec(&body)?))?;

        let mut response = self.http_client.send(http_request).await?;
        let body_str = http_client::read_body_to_string(&mut response).await?;

        let chunks: Vec<Result<LlmStreamChunk>> = body_str
            .lines()
            .filter(|line| line.starts_with("data: "))
            .filter_map(|line| {
                let json_str = &line["data: ".len()..];
                serde_json::from_str::<serde_json::Value>(json_str).ok()
            })
            .map(|v| {
                let delta = v["candidates"][0]["content"]["parts"][0]["text"]
                    .as_str()
                    .unwrap_or("")
                    .to_string();
                let finish = v["candidates"][0]["finishReason"]
                    .as_str()
                    .map(String::from);
                Ok(LlmStreamChunk {
                    delta,
                    finish_reason: finish,
                })
            })
            .collect();

        Ok(futures::stream::iter(chunks).boxed())
    }

    async fn embed(&self, request: &EmbeddingRequest) -> Result<EmbeddingResponse> {
        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:embedContent?key={}",
            request.model, self.api_key
        );

        let mut all_embeddings = Vec::new();
        let mut total_tokens = 0;

        for input in &request.inputs {
            let body = json!({
                "content": { "parts": [{"text": input}] }
            });

            let http_request = http_client::Request::builder()
                .method(http_client::Method::POST)
                .uri(&url)
                .header("Content-Type", "application/json")
                .body(http_client::Body::from(serde_json::to_vec(&body)?))?;

            let mut response = self.http_client.send(http_request).await?;
            let body_str = http_client::read_body_to_string(&mut response).await?;
            let resp: serde_json::Value = serde_json::from_str(&body_str)?;

            let embedding: Vec<f32> = resp["embedding"]["values"]
                .as_array()
                .unwrap_or(&vec![])
                .iter()
                .filter_map(|v| v.as_f64().map(|f| f as f32))
                .collect();
            all_embeddings.push(embedding);
            total_tokens += input.len() / 4; // rough estimate
        }

        Ok(EmbeddingResponse {
            embeddings: all_embeddings,
            model: request.model.clone(),
            input_tokens: total_tokens,
            cost: MicroCost::ZERO,
        })
    }

    fn pricing(&self, model: &str) -> Option<TokenPricing> {
        match model {
            m if m.contains("2.5-pro") => Some(TokenPricing {
                input_per_million: MicroCost::from_dollars(1.25),
                output_per_million: MicroCost::from_dollars(10.00),
                cached_input_per_million: Some(MicroCost::from_dollars(0.3125)),
            }),
            m if m.contains("2.5-flash") => Some(TokenPricing {
                input_per_million: MicroCost::from_dollars(0.15),
                output_per_million: MicroCost::from_dollars(0.60),
                cached_input_per_million: Some(MicroCost::from_dollars(0.0375)),
            }),
            m if m.contains("2.0-flash") => Some(TokenPricing {
                input_per_million: MicroCost::from_dollars(0.075),
                output_per_million: MicroCost::from_dollars(0.30),
                cached_input_per_million: Some(MicroCost::from_dollars(0.01875)),
            }),
            _ => None,
        }
    }
}
