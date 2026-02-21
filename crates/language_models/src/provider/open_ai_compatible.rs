use anyhow::Result;
use convert_case::{Case, Casing};
use futures::{AsyncReadExt, FutureExt, StreamExt, future::BoxFuture, stream};
use gpui::{AnyView, App, AsyncApp, Context, Entity, SharedString, Task, Window};
use http_client::HttpClient;
use http_client::{AsyncBody, Method, Request as HttpRequest};
use language_model::{
    ApiKeyState, AuthenticateError, EnvVar, IconOrSvg, LanguageModel, LanguageModelCompletionError,
    LanguageModelCompletionEvent, LanguageModelId, LanguageModelName, LanguageModelProvider,
    LanguageModelProviderId, LanguageModelProviderName, LanguageModelProviderState,
    LanguageModelRequest, LanguageModelToolChoice, LanguageModelToolSchemaFormat, RateLimiter,
};
use menu;
use open_ai::{
    ResponseStreamEvent,
    responses::{Request as ResponseRequest, StreamEvent as ResponsesStreamEvent, stream_response},
    stream_completion,
};
use settings::{Settings, SettingsStore};
use serde::Deserialize;
use std::sync::Arc;
use ui::{ElevationIndex, Tooltip, prelude::*};
use ui_input::InputField;
use util::ResultExt;

use crate::provider::open_ai::{
    OpenAiEventMapper, OpenAiResponseEventMapper, into_open_ai, into_open_ai_response,
};
pub use settings::OpenAiCompatibleAvailableModel as AvailableModel;
pub use settings::OpenAiCompatibleModelCapabilities as ModelCapabilities;

#[derive(Default, Clone, Debug, PartialEq)]
pub struct OpenAiCompatibleSettings {
    pub api_url: String,
    pub available_models: Vec<AvailableModel>,
}

pub struct OpenAiCompatibleLanguageModelProvider {
    id: LanguageModelProviderId,
    name: LanguageModelProviderName,
    http_client: Arc<dyn HttpClient>,
    state: Entity<State>,
}

pub struct State {
    id: Arc<str>,
    api_key_state: ApiKeyState,
    settings: OpenAiCompatibleSettings,
}

impl State {
    fn requires_api_key(&self) -> bool {
        !matches!(self.id.as_ref(), "opencode" | "pollinations" | "mlvoca")
    }

    fn is_authenticated(&self) -> bool {
        !self.requires_api_key() || self.api_key_state.has_key()
    }

    fn set_api_key(&mut self, api_key: Option<String>, cx: &mut Context<Self>) -> Task<Result<()>> {
        let api_url = SharedString::new(self.settings.api_url.as_str());
        self.api_key_state
            .store(api_url, api_key, |this| &mut this.api_key_state, cx)
    }

    fn authenticate(&mut self, cx: &mut Context<Self>) -> Task<Result<(), AuthenticateError>> {
        if !self.requires_api_key() {
            return Task::ready(Ok(()));
        }

        let api_url = SharedString::new(self.settings.api_url.clone());
        self.api_key_state
            .load_if_needed(api_url, |this| &mut this.api_key_state, cx)
    }
}

impl OpenAiCompatibleLanguageModelProvider {
    pub fn new(id: Arc<str>, http_client: Arc<dyn HttpClient>, cx: &mut App) -> Self {
        fn resolve_settings<'a>(id: &'a str, cx: &'a App) -> Option<&'a OpenAiCompatibleSettings> {
            crate::AllLanguageModelSettings::get_global(cx)
                .openai_compatible
                .get(id)
        }

        let api_key_env_var_name = format!("{}_API_KEY", id).to_case(Case::UpperSnake).into();
        let state = cx.new(|cx| {
            cx.observe_global::<SettingsStore>(|this: &mut State, cx| {
                let Some(settings) = resolve_settings(&this.id, cx).cloned() else {
                    return;
                };
                if &this.settings != &settings {
                    let api_url = SharedString::new(settings.api_url.as_str());
                    this.api_key_state.handle_url_change(
                        api_url,
                        |this| &mut this.api_key_state,
                        cx,
                    );
                    this.settings = settings;
                    cx.notify();
                }
            })
            .detach();
            let settings = resolve_settings(&id, cx).cloned().unwrap_or_default();
            State {
                id: id.clone(),
                api_key_state: ApiKeyState::new(
                    SharedString::new(settings.api_url.as_str()),
                    EnvVar::new(api_key_env_var_name),
                ),
                settings,
            }
        });

        Self {
            id: id.clone().into(),
            name: if matches!(id.as_ref(), "opencode" | "pollinations" | "mlvoca") {
                LanguageModelProviderName::from("Free".to_string())
            } else {
                id.into()
            },
            http_client,
            state,
        }
    }

    fn create_language_model(&self, model: AvailableModel) -> Arc<dyn LanguageModel> {
        Arc::new(OpenAiCompatibleLanguageModel {
            id: LanguageModelId::from(model.name.clone()),
            provider_id: self.id.clone(),
            provider_name: self.name.clone(),
            model,
            state: self.state.clone(),
            http_client: self.http_client.clone(),
            request_limiter: RateLimiter::new(4),
        })
    }
}

impl LanguageModelProviderState for OpenAiCompatibleLanguageModelProvider {
    type ObservableEntity = State;

    fn observable_entity(&self) -> Option<Entity<Self::ObservableEntity>> {
        Some(self.state.clone())
    }
}

impl LanguageModelProvider for OpenAiCompatibleLanguageModelProvider {
    fn id(&self) -> LanguageModelProviderId {
        self.id.clone()
    }

    fn name(&self) -> LanguageModelProviderName {
        self.name.clone()
    }

    fn icon(&self) -> IconOrSvg {
        IconOrSvg::Icon(IconName::AiOpenAiCompat)
    }

    fn default_model(&self, cx: &App) -> Option<Arc<dyn LanguageModel>> {
        self.state
            .read(cx)
            .settings
            .available_models
            .first()
            .map(|model| self.create_language_model(model.clone()))
    }

    fn default_fast_model(&self, _cx: &App) -> Option<Arc<dyn LanguageModel>> {
        None
    }

    fn provided_models(&self, cx: &App) -> Vec<Arc<dyn LanguageModel>> {
        self.state
            .read(cx)
            .settings
            .available_models
            .iter()
            .map(|model| self.create_language_model(model.clone()))
            .collect()
    }

    fn is_authenticated(&self, cx: &App) -> bool {
        self.state.read(cx).is_authenticated()
    }

    fn authenticate(&self, cx: &mut App) -> Task<Result<(), AuthenticateError>> {
        self.state.update(cx, |state, cx| state.authenticate(cx))
    }

    fn configuration_view(
        &self,
        _target_agent: language_model::ConfigurationViewTargetAgent,
        window: &mut Window,
        cx: &mut App,
    ) -> AnyView {
        cx.new(|cx| ConfigurationView::new(self.state.clone(), window, cx))
            .into()
    }

    fn reset_credentials(&self, cx: &mut App) -> Task<Result<()>> {
        self.state
            .update(cx, |state, cx| state.set_api_key(None, cx))
    }
}

pub struct OpenAiCompatibleLanguageModel {
    id: LanguageModelId,
    provider_id: LanguageModelProviderId,
    provider_name: LanguageModelProviderName,
    model: AvailableModel,
    state: Entity<State>,
    http_client: Arc<dyn HttpClient>,
    request_limiter: RateLimiter,
}

impl OpenAiCompatibleLanguageModel {
    fn effective_max_output_tokens(&self) -> Option<u64> {
        let configured_max_output_tokens = self.max_output_tokens();
        if self.provider_name.0.as_ref() == "Free" {
            Some(configured_max_output_tokens.unwrap_or(4096).min(4096))
        } else {
            configured_max_output_tokens
        }
    }

    fn mlvoca_prompt_from_request(request: &open_ai::Request) -> String {
        fn content_to_text(content: &open_ai::MessageContent) -> String {
            match content {
                open_ai::MessageContent::Plain(text) => text.clone(),
                open_ai::MessageContent::Multipart(parts) => parts
                    .iter()
                    .filter_map(|part| match part {
                        open_ai::MessagePart::Text { text } => Some(text.as_str()),
                        open_ai::MessagePart::Image { .. } => None,
                    })
                    .collect::<Vec<_>>()
                    .join("\n"),
            }
        }

        for message in request.messages.iter().rev() {
            if let open_ai::RequestMessage::User { content } = message {
                let prompt = content_to_text(content);
                if !prompt.trim().is_empty() {
                    return prompt;
                }
            }
        }

        for message in request.messages.iter().rev() {
            let prompt = match message {
                open_ai::RequestMessage::Assistant { content, .. } => {
                    content.as_ref().map(content_to_text)
                }
                open_ai::RequestMessage::System { content }
                | open_ai::RequestMessage::Tool { content, .. } => Some(content_to_text(content)),
                open_ai::RequestMessage::User { content } => Some(content_to_text(content)),
            };

            if let Some(prompt) = prompt
                && !prompt.trim().is_empty()
            {
                return prompt;
            }
        }

        String::new()
    }

    fn stream_mlvoca_completion(
        &self,
        request: open_ai::Request,
        api_url: String,
        api_key: String,
    ) -> BoxFuture<
        'static,
        Result<
            futures::stream::BoxStream<'static, Result<ResponseStreamEvent>>,
            LanguageModelCompletionError,
        >,
    > {
        #[derive(Deserialize)]
        struct MlvocaResponse {
            response: String,
        }

        let http_client = self.http_client.clone();
        let model_name = request.model.clone();
        let prompt = Self::mlvoca_prompt_from_request(&request);

        async move {
            let uri = format!("{api_url}/api/generate");
            let mut request_builder = HttpRequest::builder()
                .method(Method::POST)
                .uri(uri)
                .header("Content-Type", "application/json");

            if !api_key.trim().is_empty() {
                request_builder =
                    request_builder.header("Authorization", format!("Bearer {}", api_key.trim()));
            }

            let body = serde_json::json!({
                "model": model_name,
                "prompt": prompt,
                "stream": false,
            });

            let request = request_builder
                .body(AsyncBody::from(body.to_string()))
                .map_err(|error| LanguageModelCompletionError::Other(error.into()))?;

            let mut response = http_client
                .send(request)
                .await
                .map_err(LanguageModelCompletionError::from)?;

            let mut response_body = String::new();
            response
                .body_mut()
                .read_to_string(&mut response_body)
                .await
                .map_err(|error| LanguageModelCompletionError::Other(error.into()))?;

            if !response.status().is_success() {
                return Err(LanguageModelCompletionError::Other(anyhow::anyhow!(
                    "mlvoca API error ({}): {}",
                    response.status(),
                    response_body
                )));
            }

            let parsed: MlvocaResponse =
                serde_json::from_str(&response_body)
                    .map_err(|error| LanguageModelCompletionError::Other(error.into()))?;

            let events = vec![
                Ok(ResponseStreamEvent {
                    choices: vec![open_ai::ChoiceDelta {
                        index: 0,
                        delta: Some(open_ai::ResponseMessageDelta {
                            role: Some(open_ai::Role::Assistant),
                            content: Some(parsed.response),
                            tool_calls: None,
                            reasoning_content: None,
                        }),
                        finish_reason: None,
                    }],
                    usage: None,
                }),
                Ok(ResponseStreamEvent {
                    choices: vec![open_ai::ChoiceDelta {
                        index: 0,
                        delta: None,
                        finish_reason: Some("stop".to_string()),
                    }],
                    usage: None,
                }),
            ];

            Ok(stream::iter(events).boxed())
        }
        .boxed()
    }

    fn stream_completion(
        &self,
        request: open_ai::Request,
        cx: &AsyncApp,
    ) -> BoxFuture<
        'static,
        Result<
            futures::stream::BoxStream<'static, Result<ResponseStreamEvent>>,
            LanguageModelCompletionError,
        >,
    > {
        let http_client = self.http_client.clone();

        let (api_key, api_url, requires_api_key, provider_id) = self.state.read_with(cx, |state, _cx| {
            let api_url = &state.settings.api_url;
            (
                state.api_key_state.key(api_url),
                state.settings.api_url.clone(),
                state.requires_api_key(),
                state.id.clone(),
            )
        });

        if provider_id.as_ref() == "mlvoca" {
            let api_key = api_key.unwrap_or_default().to_string();
            return self.stream_mlvoca_completion(request, api_url, api_key);
        }

        let provider = self.provider_name.clone();
        let future = self.request_limiter.stream(async move {
            let api_key = match api_key {
                Some(api_key) => api_key,
                None if !requires_api_key => "".into(),
                None => {
                    return Err(LanguageModelCompletionError::NoApiKey { provider });
                }
            };
            let request = stream_completion(
                http_client.as_ref(),
                provider.0.as_str(),
                &api_url,
                &api_key,
                request,
            );
            let response = request.await?;
            Ok(response)
        });

        async move { Ok(future.await?.boxed()) }.boxed()
    }

    fn stream_response(
        &self,
        request: ResponseRequest,
        cx: &AsyncApp,
    ) -> BoxFuture<'static, Result<futures::stream::BoxStream<'static, Result<ResponsesStreamEvent>>>>
    {
        let http_client = self.http_client.clone();

        let (api_key, api_url, requires_api_key) = self.state.read_with(cx, |state, _cx| {
            let api_url = &state.settings.api_url;
            (
                state.api_key_state.key(api_url),
                state.settings.api_url.clone(),
                state.requires_api_key(),
            )
        });

        let provider = self.provider_name.clone();
        let future = self.request_limiter.stream(async move {
            let api_key = match api_key {
                Some(api_key) => api_key,
                None if !requires_api_key => "".into(),
                None => {
                    return Err(LanguageModelCompletionError::NoApiKey { provider });
                }
            };
            let request = stream_response(
                http_client.as_ref(),
                provider.0.as_str(),
                &api_url,
                &api_key,
                request,
            );
            let response = request.await?;
            Ok(response)
        });

        async move { Ok(future.await?.boxed()) }.boxed()
    }
}

impl LanguageModel for OpenAiCompatibleLanguageModel {
    fn id(&self) -> LanguageModelId {
        self.id.clone()
    }

    fn name(&self) -> LanguageModelName {
        LanguageModelName::from(
            self.model
                .display_name
                .clone()
                .unwrap_or_else(|| self.model.name.clone()),
        )
    }

    fn provider_id(&self) -> LanguageModelProviderId {
        self.provider_id.clone()
    }

    fn provider_name(&self) -> LanguageModelProviderName {
        self.provider_name.clone()
    }

    fn supports_tools(&self) -> bool {
        self.model.capabilities.tools
    }

    fn tool_input_format(&self) -> LanguageModelToolSchemaFormat {
        LanguageModelToolSchemaFormat::JsonSchemaSubset
    }

    fn supports_images(&self) -> bool {
        self.model.capabilities.images
    }

    fn supports_tool_choice(&self, choice: LanguageModelToolChoice) -> bool {
        match choice {
            LanguageModelToolChoice::Auto => self.model.capabilities.tools,
            LanguageModelToolChoice::Any => self.model.capabilities.tools,
            LanguageModelToolChoice::None => true,
        }
    }

    fn supports_split_token_display(&self) -> bool {
        true
    }

    fn telemetry_id(&self) -> String {
        format!("openai/{}", self.model.name)
    }

    fn max_token_count(&self) -> u64 {
        self.model.max_tokens
    }

    fn max_output_tokens(&self) -> Option<u64> {
        self.model.max_output_tokens
    }

    fn count_tokens(
        &self,
        request: LanguageModelRequest,
        cx: &App,
    ) -> BoxFuture<'static, Result<u64>> {
        let max_token_count = self.max_token_count();
        cx.background_spawn(async move {
            let messages = super::open_ai::collect_tiktoken_messages(request);
            let model = if max_token_count >= 100_000 {
                // If the max tokens is 100k or more, it is likely the o200k_base tokenizer from gpt4o
                "gpt-4o"
            } else {
                // Otherwise fallback to gpt-4, since only cl100k_base and o200k_base are
                // supported with this tiktoken method
                "gpt-4"
            };
            tiktoken_rs::num_tokens_from_messages(model, &messages).map(|tokens| tokens as u64)
        })
        .boxed()
    }

    fn stream_completion(
        &self,
        request: LanguageModelRequest,
        cx: &AsyncApp,
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
        if self.model.capabilities.chat_completions {
            let max_output_tokens = self.effective_max_output_tokens();
            let request = into_open_ai(
                request,
                &self.model.name,
                self.model.capabilities.parallel_tool_calls,
                self.model.capabilities.prompt_cache_key,
                max_output_tokens,
                None,
            );
            let completions = self.stream_completion(request, cx);
            async move {
                let mapper = OpenAiEventMapper::new();
                Ok(mapper.map_stream(completions.await?).boxed())
            }
            .boxed()
        } else {
            let max_output_tokens = self.effective_max_output_tokens();
            let request = into_open_ai_response(
                request,
                &self.model.name,
                self.model.capabilities.parallel_tool_calls,
                self.model.capabilities.prompt_cache_key,
                max_output_tokens,
                None,
            );
            let completions = self.stream_response(request, cx);
            async move {
                let mapper = OpenAiResponseEventMapper::new();
                Ok(mapper.map_stream(completions.await?).boxed())
            }
            .boxed()
        }
    }
}

struct ConfigurationView {
    api_key_editor: Entity<InputField>,
    state: Entity<State>,
    load_credentials_task: Option<Task<()>>,
}

impl ConfigurationView {
    fn new(state: Entity<State>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let api_key_editor = cx.new(|cx| {
            InputField::new(
                window,
                cx,
                "000000000000000000000000000000000000000000000000000",
            )
        });

        cx.observe(&state, |_, _, cx| {
            cx.notify();
        })
        .detach();

        let load_credentials_task = Some(cx.spawn_in(window, {
            let state = state.clone();
            async move |this, cx| {
                if let Some(task) = Some(state.update(cx, |state, cx| state.authenticate(cx))) {
                    // We don't log an error, because "not signed in" is also an error.
                    let _ = task.await;
                }
                this.update(cx, |this, cx| {
                    this.load_credentials_task = None;
                    cx.notify();
                })
                .log_err();
            }
        }));

        Self {
            api_key_editor,
            state,
            load_credentials_task,
        }
    }

    fn save_api_key(&mut self, _: &menu::Confirm, window: &mut Window, cx: &mut Context<Self>) {
        let api_key = self.api_key_editor.read(cx).text(cx).trim().to_string();
        if api_key.is_empty() {
            return;
        }

        // url changes can cause the editor to be displayed again
        self.api_key_editor
            .update(cx, |input, cx| input.set_text("", window, cx));

        let state = self.state.clone();
        cx.spawn_in(window, async move |_, cx| {
            state
                .update(cx, |state, cx| state.set_api_key(Some(api_key), cx))
                .await
        })
        .detach_and_log_err(cx);
    }

    fn reset_api_key(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.api_key_editor
            .update(cx, |input, cx| input.set_text("", window, cx));

        let state = self.state.clone();
        cx.spawn_in(window, async move |_, cx| {
            state
                .update(cx, |state, cx| state.set_api_key(None, cx))
                .await
        })
        .detach_and_log_err(cx);
    }

    fn should_render_editor(&self, cx: &Context<Self>) -> bool {
        !self.state.read(cx).is_authenticated()
    }
}

impl Render for ConfigurationView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let state = self.state.read(cx);
        let env_var_set = state.api_key_state.is_from_env_var();
        let env_var_name = state.api_key_state.env_var_name();

        let api_key_section = if self.should_render_editor(cx) {
            v_flex()
                .on_action(cx.listener(Self::save_api_key))
                .child(Label::new("To use Zed's agent with an OpenAI-compatible provider, you need to add an API key."))
                .child(
                    div()
                        .pt(DynamicSpacing::Base04.rems(cx))
                        .child(self.api_key_editor.clone())
                )
                .child(
                    Label::new(
                        format!("You can also set the {env_var_name} environment variable and restart Zed."),
                    )
                    .size(LabelSize::Small).color(Color::Muted),
                )
                .into_any()
        } else {
            h_flex()
                .mt_1()
                .p_1()
                .justify_between()
                .rounded_md()
                .border_1()
                .border_color(cx.theme().colors().border)
                .bg(cx.theme().colors().background)
                .child(
                    h_flex()
                        .flex_1()
                        .min_w_0()
                        .gap_1()
                        .child(Icon::new(IconName::Check).color(Color::Success))
                        .child(
                            div()
                                .w_full()
                                .overflow_x_hidden()
                                .text_ellipsis()
                                .child(Label::new(
                                    if env_var_set {
                                        format!("API key set in {env_var_name} environment variable")
                                    } else {
                                        format!("API key configured for {}", &state.settings.api_url)
                                    }
                                ))
                        ),
                )
                .child(
                    h_flex()
                        .flex_shrink_0()
                        .child(
                            Button::new("reset-api-key", "Reset API Key")
                                .label_size(LabelSize::Small)
                                .icon(IconName::Undo)
                                .icon_size(IconSize::Small)
                                .icon_position(IconPosition::Start)
                                .layer(ElevationIndex::ModalSurface)
                                .when(env_var_set, |this| {
                                    this.tooltip(Tooltip::text(format!("To reset your API key, unset the {env_var_name} environment variable.")))
                                })
                                .on_click(cx.listener(|this, _, window, cx| this.reset_api_key(window, cx))),
                        ),
                )
                .into_any()
        };

        if self.load_credentials_task.is_some() {
            div().child(Label::new("Loading credentials…")).into_any()
        } else {
            v_flex().size_full().child(api_key_section).into_any()
        }
    }
}
