use anyhow::Result;
use futures::{FutureExt, StreamExt, TryFutureExt, future::BoxFuture, io::BufReader, AsyncBufReadExt, AsyncReadExt};
use gpui::{AnyView, App, AsyncApp, Context, Entity, FontWeight, Task, Window};
use http_client::{AsyncBody, HttpClient, Method, Request as HttpRequest};
use language_model::{
    AuthenticateError, IconOrSvg, LanguageModel, LanguageModelCompletionError,
    LanguageModelCompletionEvent, LanguageModelId, LanguageModelName, LanguageModelProvider,
    LanguageModelProviderId, LanguageModelProviderName, LanguageModelProviderState,
    LanguageModelRequest, LanguageModelToolChoice, RateLimiter, Role, StopReason,
};
use open_ai::ResponseStreamEvent;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use ui::prelude::*;

use crate::provider::open_ai::{OpenAiEventMapper, into_open_ai};

const PROVIDER_ID: LanguageModelProviderId = LanguageModelProviderId::new("free");
const PROVIDER_NAME: LanguageModelProviderName = LanguageModelProviderName::new("Free");

/// Pollinations API URL (OpenAI-compatible, no key required).
const POLLINATIONS_API_URL: &str = "https://text.pollinations.ai/openai";

/// mlvoca API base URL (Ollama-compatible `/api/generate`, no key required).
const MLVOCA_API_URL: &str = "https://mlvoca.com";

/// Descriptors for all 3 free models.
static FREE_MODEL_DESCRIPTORS: &[FreeModelDescriptor] = &[
    // Pollinations model (1) — OpenAI-compatible
    FreeModelDescriptor {
        id: "openai-fast",
        display_name: "OpenAI Fast (Pollinations)",
        api_url: POLLINATIONS_API_URL,
        api_kind: ApiKind::OpenAi,
        max_tokens: 131_000,
        max_output_tokens: 32_768,
        supports_tools: true,
    },
    // mlvoca models (2) — Ollama /api/generate format
    FreeModelDescriptor {
        id: "tinyllama",
        display_name: "TinyLlama (mlvoca)",
        api_url: MLVOCA_API_URL,
        api_kind: ApiKind::OllamaGenerate,
        max_tokens: 2_048,
        max_output_tokens: 2_048,
        supports_tools: false,
    },
    FreeModelDescriptor {
        id: "deepseek-r1:1.5b",
        display_name: "DeepSeek R1 1.5B (mlvoca)",
        api_url: MLVOCA_API_URL,
        api_kind: ApiKind::OllamaGenerate,
        max_tokens: 16_384,
        max_output_tokens: 8_192,
        supports_tools: false,
    },
];

#[derive(Debug, Clone, Copy)]
enum ApiKind {
    OpenAi,
    OllamaGenerate,
}

/// Request body for Ollama `/api/generate` endpoint.
#[derive(Serialize)]
struct OllamaGenerateRequest {
    model: String,
    prompt: String,
    stream: bool,
}

/// A single streaming chunk from Ollama `/api/generate`.
#[derive(Deserialize)]
struct OllamaGenerateResponse {
    response: String,
    done: bool,
}

#[derive(Debug, Clone)]
struct FreeModelDescriptor {
    id: &'static str,
    display_name: &'static str,
    api_url: &'static str,
    api_kind: ApiKind,
    max_tokens: u64,
    max_output_tokens: u64,
    supports_tools: bool,
}

// ---------------------------------------------------------------------------
// Provider
// ---------------------------------------------------------------------------

pub struct FreeLanguageModelProvider {
    http_client: Arc<dyn HttpClient>,
    state: Entity<FreeProviderState>,
}

pub struct FreeProviderState;

impl FreeLanguageModelProvider {
    pub fn new(http_client: Arc<dyn HttpClient>, cx: &mut App) -> Self {
        let state = cx.new(|_cx| FreeProviderState);
        Self { http_client, state }
    }

    fn create_model(&self, descriptor: &'static FreeModelDescriptor) -> Arc<dyn LanguageModel> {
        Arc::new(FreeLanguageModel {
            id: LanguageModelId::from(format!("free/{}", descriptor.id)),
            descriptor,
            http_client: self.http_client.clone(),
            request_limiter: RateLimiter::new(4),
        })
    }
}

impl LanguageModelProviderState for FreeLanguageModelProvider {
    type ObservableEntity = FreeProviderState;

    fn observable_entity(&self) -> Option<Entity<Self::ObservableEntity>> {
        Some(self.state.clone())
    }
}

impl LanguageModelProvider for FreeLanguageModelProvider {
    fn id(&self) -> LanguageModelProviderId {
        PROVIDER_ID
    }

    fn name(&self) -> LanguageModelProviderName {
        PROVIDER_NAME
    }

    fn icon(&self) -> IconOrSvg {
        IconOrSvg::Icon(IconName::Sparkle)
    }

    fn default_model(&self, _cx: &App) -> Option<Arc<dyn LanguageModel>> {
        FREE_MODEL_DESCRIPTORS
            .first()
            .map(|d| self.create_model(d))
    }

    fn default_fast_model(&self, _cx: &App) -> Option<Arc<dyn LanguageModel>> {
        // Pollinations openai-fast is the fastest free model.
        FREE_MODEL_DESCRIPTORS
            .iter()
            .find(|d| d.id == "openai-fast")
            .map(|d| self.create_model(d))
    }

    fn provided_models(&self, _cx: &App) -> Vec<Arc<dyn LanguageModel>> {
        FREE_MODEL_DESCRIPTORS
            .iter()
            .map(|d| self.create_model(d))
            .collect()
    }

    fn is_authenticated(&self, _cx: &App) -> bool {
        // Free models never require authentication.
        true
    }

    fn authenticate(&self, _cx: &mut App) -> Task<Result<(), AuthenticateError>> {
        Task::ready(Ok(()))
    }

    fn configuration_view(
        &self,
        _target_agent: language_model::ConfigurationViewTargetAgent,
        _window: &mut Window,
        cx: &mut App,
    ) -> AnyView {
        cx.new(|_cx| FreeConfigurationView).into()
    }

    fn reset_credentials(&self, _cx: &mut App) -> Task<Result<()>> {
        Task::ready(Ok(()))
    }
}

// ---------------------------------------------------------------------------
// Model
// ---------------------------------------------------------------------------

struct FreeLanguageModel {
    id: LanguageModelId,
    descriptor: &'static FreeModelDescriptor,
    http_client: Arc<dyn HttpClient>,
    request_limiter: RateLimiter,
}

impl FreeLanguageModel {
    /// Stream an OpenAI-compatible completion (used for Pollinations).
    fn stream_openai(
        &self,
        request: open_ai::Request,
    ) -> BoxFuture<
        'static,
        Result<futures::stream::BoxStream<'static, Result<ResponseStreamEvent>>, LanguageModelCompletionError>,
    > {
        let http_client = self.http_client.clone();
        let api_url = self.descriptor.api_url.to_string();

        let future = self.request_limiter.stream(async move {
            let response = open_ai::stream_completion(
                http_client.as_ref(),
                PROVIDER_NAME.0.as_str(),
                &api_url,
                "free",
                request,
            )
            .await?;
            Ok(response)
        });

        async move { Ok(future.await?.boxed()) }.boxed()
    }

    /// Stream from mlvoca's Ollama-compatible `/api/generate` endpoint.
    fn stream_ollama_generate(
        &self,
        request: LanguageModelRequest,
    ) -> BoxFuture<
        'static,
        Result<
            futures::stream::BoxStream<
                'static,
                Result<LanguageModelCompletionEvent, LanguageModelCompletionError>,
            >,
            LanguageModelCompletionError,
        >,
    > {
        let http_client = self.http_client.clone();
        let api_url = self.descriptor.api_url.to_string();
        let model_id = self.descriptor.id.to_string();

        // Flatten all messages into a single prompt string.
        let prompt = request
            .messages
            .iter()
            .map(|msg| {
                let role_prefix = match msg.role {
                    Role::User => "User",
                    Role::Assistant => "Assistant",
                    Role::System => "System",
                };
                format!("{}: {}", role_prefix, msg.string_contents())
            })
            .collect::<Vec<_>>()
            .join("\n\n");

        let future = self.request_limiter.stream(async move {
            let body = OllamaGenerateRequest {
                model: model_id,
                prompt,
                stream: true,
            };

            let uri = format!("{api_url}/api/generate");
            let body_json = serde_json::to_string(&body).map_err(|error| {
                LanguageModelCompletionError::from(anyhow::Error::new(error))
            })?;
            let http_request = HttpRequest::builder()
                .method(Method::POST)
                .uri(uri)
                .header("Content-Type", "application/json")
                .body(AsyncBody::from(body_json))
                .map_err(|error| {
                    LanguageModelCompletionError::from(anyhow::Error::new(error))
                })?;

            let mut response = http_client
                .send(http_request)
                .await
                .map_err(LanguageModelCompletionError::from)?;
            if !response.status().is_success() {
                let mut body_text = String::new();
                response
                    .body_mut()
                    .read_to_string(&mut body_text)
                    .await
                    .map_err(|error| {
                        LanguageModelCompletionError::from(anyhow::Error::new(error))
                    })?;
                return Err(LanguageModelCompletionError::from(anyhow::anyhow!(
                    "mlvoca API error: {body_text}"
                )));
            }

            let reader = BufReader::new(response.into_body());
            let stream = reader
                .lines()
                .filter_map(|line| async move {
                    match line {
                        Ok(line) if line.is_empty() => None,
                        Ok(line) => {
                            match serde_json::from_str::<OllamaGenerateResponse>(&line) {
                                Ok(resp) => {
                                    if resp.done {
                                        Some(Ok(LanguageModelCompletionEvent::Stop(
                                            StopReason::EndTurn,
                                        )))
                                    } else if !resp.response.is_empty() {
                                        Some(Ok(LanguageModelCompletionEvent::Text(
                                            resp.response,
                                        )))
                                    } else {
                                        None
                                    }
                                }
                                Err(e) => Some(Err(LanguageModelCompletionError::from(
                                    anyhow::anyhow!("Failed to parse mlvoca response: {e}"),
                                ))),
                            }
                        }
                        Err(e) => Some(Err(LanguageModelCompletionError::from(
                            anyhow::anyhow!("mlvoca stream error: {e}"),
                        ))),
                    }
                });

            Ok(stream.boxed())
        });

        future.map_ok(|f| f.boxed()).boxed()
    }
}

impl LanguageModel for FreeLanguageModel {
    fn id(&self) -> LanguageModelId {
        self.id.clone()
    }

    fn name(&self) -> LanguageModelName {
        LanguageModelName::from(self.descriptor.display_name.to_string())
    }

    fn provider_id(&self) -> LanguageModelProviderId {
        PROVIDER_ID
    }

    fn provider_name(&self) -> LanguageModelProviderName {
        PROVIDER_NAME
    }

    fn supports_tools(&self) -> bool {
        self.descriptor.supports_tools
    }

    fn supports_images(&self) -> bool {
        false
    }

    fn supports_tool_choice(&self, choice: LanguageModelToolChoice) -> bool {
        match choice {
            LanguageModelToolChoice::Auto => self.descriptor.supports_tools,
            LanguageModelToolChoice::Any => self.descriptor.supports_tools,
            LanguageModelToolChoice::None => true,
        }
    }

    fn telemetry_id(&self) -> String {
        format!("free/{}", self.descriptor.id)
    }

    fn max_token_count(&self) -> u64 {
        self.descriptor.max_tokens
    }

    fn max_output_tokens(&self) -> Option<u64> {
        Some(self.descriptor.max_output_tokens)
    }

    fn count_tokens(
        &self,
        request: LanguageModelRequest,
        cx: &App,
    ) -> BoxFuture<'static, Result<u64>> {
        match self.descriptor.api_kind {
            ApiKind::OpenAi => {
                cx.background_spawn(async move {
                    let messages = request
                        .messages
                        .into_iter()
                        .map(|message| tiktoken_rs::ChatCompletionRequestMessage {
                            role: match message.role {
                                Role::User => "user".into(),
                                Role::Assistant => "assistant".into(),
                                Role::System => "system".into(),
                            },
                            content: Some(message.string_contents()),
                            name: None,
                            function_call: None,
                        })
                        .collect::<Vec<_>>();

                    tiktoken_rs::num_tokens_from_messages("gpt-4o", &messages)
                        .map(|tokens| tokens as u64)
                })
                .boxed()
            }
            ApiKind::OllamaGenerate => {
                // Approximate: 1 token ≈ 4 chars
                let token_count = request
                    .messages
                    .iter()
                    .map(|msg| msg.string_contents().chars().count())
                    .sum::<usize>()
                    / 4;
                async move { Ok(token_count as u64) }.boxed()
            }
        }
    }

    fn stream_completion(
        &self,
        request: LanguageModelRequest,
        _cx: &AsyncApp,
    ) -> BoxFuture<
        'static,
        Result<
            futures::stream::BoxStream<
                'static,
                Result<LanguageModelCompletionEvent, LanguageModelCompletionError>,
            >,
            LanguageModelCompletionError,
        >,
    > {
        match self.descriptor.api_kind {
            ApiKind::OpenAi => {
                let openai_request = into_open_ai(
                    request,
                    self.descriptor.id,
                    false,  // parallel_tool_calls
                    false,  // prompt_cache_key
                    self.max_output_tokens(),
                    None,   // reasoning_effort
                );
                let completions = self.stream_openai(openai_request);
                async move {
                    let mapper = OpenAiEventMapper::new();
                    Ok(mapper.map_stream(completions.await?).boxed())
                }
                .boxed()
            }
            ApiKind::OllamaGenerate => {
                self.stream_ollama_generate(request)
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Configuration view (minimal — no credentials needed)
// ---------------------------------------------------------------------------

struct FreeConfigurationView;

impl Render for FreeConfigurationView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .gap_2()
            .child(
                Label::new("Free Models")
                    .size(LabelSize::Large)
                    .weight(FontWeight::BOLD),
            )
            .child(Label::new(
                "These 3 models are completely free — no API key or sign-up required.",
            ))
            .child(
                Label::new("Powered by Pollinations.ai and mlvoca.com.")
                    .size(LabelSize::Small)
                    .color(Color::Muted),
            )
    }
}
