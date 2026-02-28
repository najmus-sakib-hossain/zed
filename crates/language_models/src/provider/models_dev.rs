use anyhow::{anyhow, Result};
use collections::HashMap;
use futures::{future::BoxFuture, AsyncReadExt, FutureExt};
use gpui::{App, AsyncApp, Entity, Task, Window, AnyView};
use http_client::{HttpClient, Method};
use language_model::{
    AuthenticateError, IconOrSvg, LanguageModel, LanguageModelCompletionError,
    LanguageModelCompletionEvent, LanguageModelId, LanguageModelName, LanguageModelProvider,
    LanguageModelProviderId, LanguageModelProviderName, LanguageModelProviderState,
    LanguageModelRequest, LanguageModelToolChoice,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use ui::{IconName, prelude::*};

const MODELS_DEV_API_URL: &str = "https://models.dev/api.json";

/// Provider data from models.dev
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ModelsDevProvider {
    pub id: String,
    pub name: String,
    pub api: Option<String>,
    pub env: Vec<String>,
    pub npm: Option<String>,
    pub models: HashMap<String, ModelsDevModel>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ModelsDevModel {
    pub id: String,
    pub name: String,
    pub family: Option<String>,
    pub release_date: String,
    pub attachment: bool,
    pub reasoning: bool,
    pub tool_call: bool,
    pub temperature: bool,
    pub cost: Option<ModelCost>,
    pub limit: ModelLimit,
    pub modalities: Option<Modalities>,
    pub open_weights: Option<bool>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ModelCost {
    pub input: f64,
    pub output: f64,
    pub cache_read: Option<f64>,
    pub cache_write: Option<f64>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ModelLimit {
    pub context: u64,
    pub output: u64,
    pub input: Option<u64>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Modalities {
    pub input: Vec<String>,
    pub output: Vec<String>,
}

/// Fetches and caches provider data from models.dev
pub struct ModelsDevRegistry {
    http_client: Arc<dyn HttpClient>,
    providers: Arc<std::sync::RwLock<HashMap<String, ModelsDevProvider>>>,
}

impl ModelsDevRegistry {
    pub fn new(http_client: Arc<dyn HttpClient>) -> Self {
        Self {
            http_client,
            providers: Arc::new(std::sync::RwLock::new(HashMap::default())),
        }
    }

    /// Fetch providers from models.dev API
    pub async fn fetch_providers(&self) -> Result<HashMap<String, ModelsDevProvider>> {
        let request = http_client::Request::builder()
            .method(Method::GET)
            .uri(MODELS_DEV_API_URL)
            .body(Default::default())?;

        let mut response = self
            .http_client
            .send(request)
            .await
            .map_err(|e| anyhow!("Failed to fetch models.dev API: {}", e))?;

        if !response.status().is_success() {
            return Err(anyhow!(
                "models.dev API returned status: {}",
                response.status()
            ));
        }

        let mut body = String::new();
        response.body_mut().read_to_string(&mut body).await?;
        let providers: HashMap<String, ModelsDevProvider> = serde_json::from_str(&body)?;

        // Cache the providers
        *self.providers.write().unwrap() = providers.clone();

        Ok(providers)
    }

    /// Get cached providers
    pub fn get_providers(&self) -> HashMap<String, ModelsDevProvider> {
        self.providers.read().unwrap().clone()
    }

    /// Get a specific provider by ID
    pub fn get_provider(&self, id: &str) -> Option<ModelsDevProvider> {
        self.providers.read().unwrap().get(id).cloned()
    }
}

/// Dynamic language model provider that uses models.dev data
pub struct ModelsDevLanguageModelProvider {
    provider_data: ModelsDevProvider,
    http_client: Arc<dyn HttpClient>,
    state: Entity<ModelsDevProviderState>,
}

pub struct ModelsDevProviderState {
    api_key: Option<String>,
}

impl ModelsDevLanguageModelProvider {
    pub fn new(
        provider_data: ModelsDevProvider,
        http_client: Arc<dyn HttpClient>,
        cx: &mut App,
    ) -> Self {
        let state = cx.new(|_cx| ModelsDevProviderState { api_key: None });

        Self {
            provider_data,
            http_client,
            state,
        }
    }

    fn get_api_key(&self, cx: &App) -> Option<String> {
        self.state.read(cx).api_key.clone().or_else(|| {
            // Try to get from environment variable
            self.provider_data
                .env
                .first()
                .and_then(|env_var| std::env::var(env_var).ok())
        })
    }
}

impl LanguageModelProviderState for ModelsDevLanguageModelProvider {
    type ObservableEntity = ModelsDevProviderState;

    fn observable_entity(&self) -> Option<Entity<Self::ObservableEntity>> {
        Some(self.state.clone())
    }
}

impl LanguageModelProvider for ModelsDevLanguageModelProvider {
    fn id(&self) -> LanguageModelProviderId {
        LanguageModelProviderId::from(self.provider_data.id.clone())
    }

    fn name(&self) -> LanguageModelProviderName {
        LanguageModelProviderName::from(self.provider_data.name.clone())
    }

    fn icon(&self) -> IconOrSvg {
        IconOrSvg::Icon(IconName::AiOpenAiCompat)
    }

    fn default_model(&self, _cx: &App) -> Option<Arc<dyn LanguageModel>> {
        None
    }

    fn default_fast_model(&self, _cx: &App) -> Option<Arc<dyn LanguageModel>> {
        None
    }

    fn provided_models(&self, _cx: &App) -> Vec<Arc<dyn LanguageModel>> {
        self.provider_data
            .models
            .values()
            .map(|model| {
                Arc::new(ModelsDevLanguageModel {
                    provider_id: self.id(),
                    provider_name: self.name(),
                    model_data: model.clone(),
                    api_url: self.provider_data.api.clone(),
                    http_client: self.http_client.clone(),
                    api_key_provider: Arc::new({
                        let state = self.state.clone();
                        let env_vars = self.provider_data.env.clone();
                        move |cx: &App| {
                            state.read(cx).api_key.clone().or_else(|| {
                                env_vars.first().and_then(|var| std::env::var(var).ok())
                            })
                        }
                    }),
                }) as Arc<dyn LanguageModel>
            })
            .collect()
    }

    fn is_authenticated(&self, cx: &App) -> bool {
        self.get_api_key(cx).is_some()
    }

    fn authenticate(&self, cx: &mut App) -> Task<Result<(), AuthenticateError>> {
        if self.is_authenticated(cx) {
            Task::ready(Ok(()))
        } else {
            Task::ready(Err(AuthenticateError::CredentialsNotFound))
        }
    }

    fn configuration_view(
        &self,
        _target_agent: language_model::ConfigurationViewTargetAgent,
        _window: &mut Window,
        cx: &mut App,
    ) -> AnyView {
        cx.new(|_cx| ModelsDevConfigurationView {
            provider_name: self.name(),
            env_vars: self.provider_data.env.clone(),
        })
        .into()
    }

    fn reset_credentials(&self, _cx: &mut App) -> Task<Result<()>> {
        Task::ready(Ok(()))
    }
}

struct ModelsDevLanguageModel {
    provider_id: LanguageModelProviderId,
    provider_name: LanguageModelProviderName,
    model_data: ModelsDevModel,
    #[allow(dead_code)]
    api_url: Option<String>,
    #[allow(dead_code)]
    http_client: Arc<dyn HttpClient>,
    #[allow(dead_code)]
    api_key_provider: Arc<dyn Fn(&App) -> Option<String> + Send + Sync>,
}

impl LanguageModel for ModelsDevLanguageModel {
    fn id(&self) -> LanguageModelId {
        LanguageModelId::from(format!("{}::{}", self.provider_id.0, self.model_data.id))
    }

    fn name(&self) -> LanguageModelName {
        LanguageModelName::from(self.model_data.name.clone())
    }

    fn provider_id(&self) -> LanguageModelProviderId {
        self.provider_id.clone()
    }

    fn provider_name(&self) -> LanguageModelProviderName {
        self.provider_name.clone()
    }

    fn telemetry_id(&self) -> String {
        format!("models_dev/{}/{}", self.provider_id.0, self.model_data.id)
    }

    fn max_token_count(&self) -> u64 {
        self.model_data.limit.context
    }

    fn max_output_tokens(&self) -> Option<u64> {
        Some(self.model_data.limit.output)
    }

    fn supports_images(&self) -> bool {
        self.model_data
            .modalities
            .as_ref()
            .map(|m| m.input.iter().any(|i| i.contains("image")))
            .unwrap_or(false)
    }

    fn supports_tools(&self) -> bool {
        self.model_data.tool_call
    }

    fn supports_tool_choice(&self, _choice: LanguageModelToolChoice) -> bool {
        self.model_data.tool_call
    }

    fn count_tokens(
        &self,
        request: LanguageModelRequest,
        _cx: &App,
    ) -> BoxFuture<'static, Result<u64>> {
        // Simple token estimation
        let text = request
            .messages
            .iter()
            .map(|msg| msg.content.len())
            .sum::<usize>();
        async move { Ok((text / 4) as u64) }.boxed()
    }

    fn stream_completion(
        &self,
        _request: LanguageModelRequest,
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
        // This is a placeholder - actual implementation would use OpenAI-compatible API
        async move {
            Err(LanguageModelCompletionError::Other(anyhow!(
                "Streaming not yet implemented for models.dev providers"
            )))
        }
        .boxed()
    }
}

struct ModelsDevConfigurationView {
    provider_name: LanguageModelProviderName,
    env_vars: Vec<String>,
}

impl gpui::Render for ModelsDevConfigurationView {
    fn render(&mut self, _window: &mut gpui::Window, _cx: &mut gpui::Context<Self>) -> impl gpui::IntoElement {
        v_flex()
            .gap_2()
            .child(Label::new(format!(
                "Configure {} by setting environment variable:",
                self.provider_name.0
            )))
            .children(self.env_vars.iter().map(|var| {
                Label::new(var.clone()).color(Color::Muted)
            }))
    }
}
