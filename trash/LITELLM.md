# DX: The Complete Strategy for 100+ LLM Providers in Rust

Here's your comprehensive battle plan for building DX's LLM provider layer — combining everything from OpenCode's approach, LiteLLM's feature set, the Models.dev registry, Zed's codebase, and the real-world provider incompatibilities you'll need to handle.

---

## Part 1: How OpenCode Achieves 75+ Providers (And How You Beat It)

OpenCode uses the AI SDK and Models.dev to support 75+ LLM providers and it supports running local models. Standard providers pull these from Models.dev automatically.

The key insight: OpenCode doesn't manually implement 75 provider adapters. It uses two multipliers:

1. **Models.dev** — Fetches from models.dev API: Retrieves comprehensive model metadata from models.dev, which aggregates LLM documentation across providers. It provides details about model capabilities, pricing, context windows, and more.

2. **OpenAI-compatible catch-all** — For example, use @ai-sdk/cerebras for Cerebras. And for all other OpenAI-compatible providers, use @ai-sdk/openai-compatible.

### Your 100+ Provider Strategy (5 Tiers)

| Tier | Providers | Implementation | Count |
|------|-----------|----------------|-------|
| **Tier 1: Native** | OpenAI, Anthropic, Google (Gemini + Vertex), AWS Bedrock, Azure OpenAI | Full native SDK-level adapters, every edge case handled | ~6 |
| **Tier 2: Named** | Mistral, Cohere, DeepSeek, xAI, Groq, Fireworks, Together, Ollama, HuggingFace, NVIDIA NIM, Replicate, Sagemaker | Named adapters with provider-specific quirks | ~15 |
| **Tier 3: OpenAI-Compatible** | Cerebras, Perplexity, Venice AI, Baseten, Deep Infra, IO.NET, Moonshot, MiniMax, Nebius, OVHcloud, Scaleway, SiliconFlow, Inference.net, LM Studio, vLLM, etc. | Generic OpenAI-compatible adapter with `base_url` override | ~40+ |
| **Tier 4: Aggregators** | OpenRouter, Cloudflare AI Gateway, Vercel AI Gateway, Helicone, Cortecs, ZenMux, 302.AI | Each one multiplies your model count by 100x | ~7 |
| **Tier 5: Local** | Ollama, LM Studio, llama.cpp, GPUStack, llamafile | OpenAI-compatible with localhost URLs | ~5 |

**Total: 100+ providers, but you only build ~25 real adapters + 1 generic OpenAI-compatible adapter.**

Additional providers include: 302.AI, Baseten, Cerebras, Cloudflare AI Gateway, Cortecs, Deep Infra, Firmware, Fireworks AI, Hugging Face, Helicone, IO.NET, Moonshot AI, MiniMax, Nebius Token Factory, OVHcloud AI Endpoints, SAP AI Core, Scaleway, Together AI, Venice AI, Vercel AI Gateway, xAI, Z.AI, ZenMux.

---

## Part 2: What Zed Already Gives You

Since you're building from Zed's codebase, you already have a foundation. Written from scratch in Rust to efficiently leverage multiple CPU cores and your GPU.

To use AI in Zed, you need to have at least one large language model provider set up. Once configured, providers are available in the Agent Panel, Inline Assistant, and Text Threads.

Zed currently supports a limited set natively: Zed supports 12+ LLM providers including local models via Ollama. Also supports OpenAI, Google Gemini, Ollama, DeepSeek, Mistral, GitHub Copilot, and others through custom API keys.

Zed also supports custom OpenAI-compatible providers: OpenRouter provides access to multiple AI models through a single API. It supports tool use for compatible models.

**Your gap**: You need to go from Zed's ~12 providers to 100+ while adding all the LiteLLM operational features Zed doesn't have.

---

## Part 3: The Models.dev Integration (Your Secret Weapon)

OpenCode uses Models.dev catalog to discover 75+ providers and 1000+ models.

Build a Rust module that fetches and caches the Models.dev catalog at startup:

```rust
// dx_models_registry/src/lib.rs

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ModelInfo {
    pub id: String,
    pub provider: String,
    pub name: String,
    pub family: Option<String>,
    pub capabilities: ModelCapabilities,
    pub limits: ModelLimits,
    pub cost: ModelCost,
    pub deprecated: bool,
    pub aliases: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ModelCapabilities {
    pub chat: bool,
    pub tools: ToolSupport,
    pub json_mode: JsonSupport,
    pub streaming: StreamingSupport,
    pub vision: bool,
    pub audio: bool,
    pub reasoning: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ToolSupport {
    pub enabled: bool,
    pub streaming: bool, // can stream tool calls?
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ModelLimits {
    pub context: u64,       // max input tokens
    pub output: u64,        // max output tokens
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ModelCost {
    pub input_per_million: f64,   // USD per 1M input tokens
    pub output_per_million: f64,  // USD per 1M output tokens
    pub cache_creation_per_million: Option<f64>,
    pub cache_read_per_million: Option<f64>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ProviderInfo {
    pub id: String,
    pub name: String,
    pub base_url: String,
    pub env_keys: Vec<String>,  // e.g. ["OPENAI_API_KEY"]
    pub api_type: ApiType,      // Native, OpenAiCompatible, Custom
    pub doc_url: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum ApiType {
    OpenAi,              // full OpenAI API
    OpenAiCompatible,    // OpenAI-compatible subset
    Anthropic,           // Anthropic Messages API
    GoogleGenerativeAi,  // Google Gemini API
    AwsBedrock,          // AWS Bedrock (SigV4 auth)
    AzureOpenAi,         // Azure OpenAI (custom auth + versioning)
    Custom(String),      // Fully custom
}

/// The central registry — loaded from Models.dev + LiteLLM cost map + local overrides
pub struct DxModelRegistry {
    providers: HashMap<String, ProviderInfo>,
    models: HashMap<String, ModelInfo>,  // key: "provider/model_id"
    last_refreshed: std::time::Instant,
}

impl DxModelRegistry {
    /// Boot sequence:
    /// 1. Load bundled snapshot (no network required)
    /// 2. Attempt refresh from Models.dev API
    /// 3. Merge with LiteLLM cost map for pricing
    /// 4. Apply user overrides from dx_config.toml
    pub async fn initialize() -> Result<Self, RegistryError> {
        let bundled = Self::load_bundled_snapshot()?;
        
        match Self::fetch_models_dev().await {
            Ok(remote) => Ok(bundled.merge(remote)),
            Err(_) => Ok(bundled), // Offline? Use snapshot.
        }
    }

    pub fn resolve_model(&self, spec: &str) -> Result<(&ProviderInfo, &ModelInfo), RegistryError> {
        // Supports "openai/gpt-4o", "gpt-4o@openai", or just "gpt-4o" (inferred)
        // Also resolves aliases: "claude-haiku-4.5" -> "claude-haiku-4-5-20251001"
        todo!()
    }
    
    pub fn select_by_capability(
        &self,
        require: &[Capability],
        prefer_providers: &[&str],
    ) -> Option<(&ProviderInfo, &ModelInfo)> {
        todo!()
    }
}
```

Also integrate **LiteLLM's model cost map** directly — LiteLLM already has pricing for 100+ models in our model cost map. Pulls the cost + context window + provider route for known models from their JSON file. The latest version can be found at https://github.com/BerriAI/litellm/blob/main/model_prices_and_context_window.json.

The cost map schema includes: `deprecation_date`, `file_search_cost_per_1k_calls`, `input_cost_per_audio_token`, `input_cost_per_token`, `litellm_provider`, `max_input_tokens`, `max_output_tokens`, `max_tokens` and `mode`: "one of: chat, embedding, completion, image_generation, audio_transcription, audio_speech, image_generation, moderation, rerank, search".

```rust
// Fetch and merge LiteLLM's cost map
const LITELLM_COST_MAP_URL: &str = 
    "https://raw.githubusercontent.com/BerriAI/litellm/main/model_prices_and_context_window.json";

#[derive(Debug, Deserialize)]
pub struct LiteLlmModelEntry {
    pub max_tokens: Option<u64>,
    pub max_input_tokens: Option<u64>,
    pub max_output_tokens: Option<u64>,
    pub input_cost_per_token: Option<f64>,
    pub output_cost_per_token: Option<f64>,
    pub input_cost_per_audio_token: Option<f64>,
    pub output_cost_per_reasoning_token: Option<f64>,
    pub cache_creation_input_token_cost: Option<f64>,
    pub cache_read_input_token_cost: Option<f64>,
    pub litellm_provider: Option<String>,
    pub mode: Option<String>,  // chat, embedding, completion, etc.
    pub deprecation_date: Option<String>,
    // Tiered pricing
    pub input_cost_per_token_above_200k_tokens: Option<f64>,
    pub output_cost_per_token_above_200k_tokens: Option<f64>,
}
```

---

## Part 4: The Real-World Provider Incompatibilities You MUST Handle

This is where most projects fail. Our LLM client module has accumulated hundreds of lines of provider-specific logic, and new patches keep coming.

If you've ever tried to swap one LLM provider for another, you've probably noticed that things aren't as interchangeable as the docs suggest. On paper, the APIs are converging: they all accept messages, they all support tool calling, they all accept JSON Schemas for structured output. In practice, each provider has opinions about what "valid" means, and those opinions don't always agree with each other or with the relevant specs.

### Critical Provider Quirks to Handle:

**Temperature restrictions:**
- Anthropic requires temperature to be set to 1 when extended thinking is enabled.
- OpenAI's o3 and GPT-5 families don't support temperature at all.
- Google still supports temperature for Gemini 3 models, but it strongly discourages setting it to anything other than 1.

**Caching differences:**
- Anthropic is the only major provider where prompt caching requires explicit markers in your request. You add `cache_control: {"type": "ephemeral"}` to specific messages that mark cache breakpoints.
- OpenAI and Google handle caching automatically on their end. You don't need to do anything.

```rust
/// Provider-specific request normalization
pub trait ProviderNormalizer: Send + Sync {
    /// Adjust request parameters to comply with provider constraints
    fn normalize_request(&self, req: &mut ChatRequest, model: &ModelInfo) {
        // Default: no-op. Providers override as needed.
    }
    
    /// Map provider error responses to unified DxError
    fn map_error(&self, status: u16, body: &str) -> DxError;
    
    /// Provider-specific auth (SigV4 for Bedrock, OAuth for GitLab, etc.)
    fn authenticate(&self, req: &mut reqwest::RequestBuilder, creds: &Credentials) 
        -> Result<(), AuthError>;
}

struct AnthropicNormalizer;
impl ProviderNormalizer for AnthropicNormalizer {
    fn normalize_request(&self, req: &mut ChatRequest, model: &ModelInfo) {
        // If extended thinking is on, force temperature = 1
        if req.thinking.is_some() {
            req.temperature = Some(1.0);
        }
        // Inject cache_control markers for prompt caching
        if req.enable_caching {
            self.inject_cache_breakpoints(&mut req.messages);
        }
        // Anthropic requires alternating user/assistant messages
        self.ensure_alternating_roles(&mut req.messages);
    }
}

struct OpenAiNormalizer;
impl ProviderNormalizer for OpenAiNormalizer {
    fn normalize_request(&self, req: &mut ChatRequest, model: &ModelInfo) {
        // o3 and GPT-5 don't support temperature — strip it
        if model.family_is("o3") || model.family_is("gpt-5") {
            req.temperature = None;
            req.top_p = None;
        }
        // Responses API vs Chat Completions API routing
        if model.requires_responses_api() {
            req.api_mode = ApiMode::Responses;
        }
    }
}
```

If a provider exposes models that only work with the Responses API, set `chat_completions` to false for those entries. Zed uses the Responses endpoint for these models.

---

## Part 5: Complete LiteLLM Feature Implementation in Rust

### Feature 1: Unified Completion API

Consistent output - same response format regardless of which provider you use.

```rust
use dx_llm::{completion, ChatMessage, Role};

// Identical API regardless of provider
let response = completion(
    "anthropic/claude-sonnet-4",
    &[ChatMessage { role: Role::User, content: "Hello!".into() }],
    CompletionOptions::default(),
).await?;

// Always returns the same unified type
println!("{}", response.choices[0].message.content);
println!("Tokens: {} in / {} out", response.usage.prompt_tokens, response.usage.completion_tokens);
println!("Cost: ${:.6}", response.cost_usd);
```

### Feature 2: Cost Tracking & Spend Management

LiteLLM automatically tracks spend for all known models.

LiteLLM's cost tracking is granular — there are keys for `input_cost_per_token`, `output_cost_per_token`, `cache_creation_input_token_cost`, `cache_read_input_token_cost`, and even tiered pricing like `input_cost_per_token_above_200k_tokens`.

LiteLLM provides functions for this: `cost_per_token` returns the cost (in USD) for prompt and completion tokens. Uses the live list from api.litellm.ai. `completion_cost` returns the overall cost (in USD) for a given LLM API Call. It combines token_counter and cost_per_token to return the cost for that query.

```rust
pub struct CostEngine {
    registry: Arc<DxModelRegistry>,
}

impl CostEngine {
    pub fn cost_per_token(&self, model: &str) -> Result<TokenCost, CostError> {
        let info = self.registry.resolve_model(model)?;
        Ok(TokenCost {
            input_per_token: info.1.cost.input_per_million / 1_000_000.0,
            output_per_token: info.1.cost.output_per_million / 1_000_000.0,
            cache_creation_per_token: info.1.cost.cache_creation_per_million
                .map(|c| c / 1_000_000.0),
            cache_read_per_token: info.1.cost.cache_read_per_million
                .map(|c| c / 1_000_000.0),
        })
    }
    
    pub fn completion_cost(&self, response: &ChatResponse) -> f64 {
        let costs = self.cost_per_token(&response.model).unwrap_or_default();
        let input_cost = response.usage.prompt_tokens as f64 * costs.input_per_token;
        let output_cost = response.usage.completion_tokens as f64 * costs.output_per_token;
        let reasoning_cost = response.usage.reasoning_tokens
            .map(|t| t as f64 * costs.output_per_token) // reasoning usually at output price
            .unwrap_or(0.0);
        let cache_cost = response.usage.cache_creation_tokens
            .map(|t| t as f64 * costs.cache_creation_per_token.unwrap_or(0.0))
            .unwrap_or(0.0)
            + response.usage.cache_read_tokens
                .map(|t| t as f64 * costs.cache_read_per_token.unwrap_or(0.0))
                .unwrap_or(0.0);
        
        input_cost + output_cost + reasoning_cost + cache_cost
    }
}

/// Budget management per key/user/team
pub struct BudgetManager {
    db: sqlx::PgPool,
}

impl BudgetManager {
    pub async fn check_and_record(
        &self, 
        key_id: &str, 
        cost: f64,
    ) -> Result<(), BudgetError> {
        let key = self.get_key(key_id).await?;
        
        // Check budget duration reset
        if let Some(duration) = &key.budget_duration {
            if key.budget_reset_at < Utc::now() {
                self.reset_budget(key_id, duration).await?;
            }
        }
        
        if let Some(max) = key.max_budget {
            if key.spend + cost > max {
                return Err(BudgetError::Exceeded { 
                    spent: key.spend, 
                    budget: max,
                    overage: (key.spend + cost) - max,
                });
            }
        }
        
        // Atomically increment
        sqlx::query!(
            "UPDATE virtual_keys SET spend = spend + $1 WHERE id = $2",
            cost, key_id
        ).execute(&self.db).await?;
        
        Ok(())
    }
}
```

LiteLLM also offers a pricing calculator: The calculator automatically updates as you change values. View the cost breakdown including: Per-Request Cost, Daily Costs, Monthly Costs.

### Feature 3: Zero-Cost Budget Bypass for Local Models

Use Case: You have on-premises or free models that should be accessible even when users exceed their budget limits. Solution: Set both `input_cost_per_token` and `output_cost_per_token` to 0 (explicitly) to bypass all budget checks for that model.

When a model is configured with zero cost, LiteLLM will automatically skip ALL budget checks (user, team, team member, end-user, organization, and global proxy budget) for requests to that model.

```rust
impl BudgetManager {
    pub fn should_skip_budget_check(&self, model: &ModelInfo) -> bool {
        // If both costs are explicitly zero, bypass all budget checks
        model.cost.input_per_million == 0.0 && model.cost.output_per_million == 0.0
    }
}
```

### Feature 4: Tokenization

LiteLLM provides default tokenizer support for OpenAI, Cohere, Anthropic, Llama2, and Llama3 models. If you are using a different model, you can create a custom tokenizer.

```rust
use tiktoken_rs as tiktoken;

pub enum Tokenizer {
    Tiktoken(tiktoken::CoreBPE),          // OpenAI models
    SentencePiece(sentencepiece::Model),   // Llama, Mistral
    HuggingFace(tokenizers::Tokenizer),    // Anything on HF
}

pub struct TokenCounter {
    tokenizers: HashMap<String, Tokenizer>,  // family -> tokenizer
    default: Tokenizer,                       // tiktoken fallback
}

impl TokenCounter {
    pub fn count(&self, model: &str, messages: &[ChatMessage]) -> u64 {
        let tokenizer = self.get_tokenizer_for_model(model);
        let mut total = 0;
        for msg in messages {
            total += tokenizer.encode(&msg.content).len() as u64;
            total += 4; // OpenAI's per-message overhead
        }
        total + 3 // reply priming
    }
}
```

### Feature 5: Router — Retry, Fallback, Load Balancing

Retry/fallback logic across multiple deployments (e.g. Azure/OpenAI) - Router. Track spend & set budgets per project.

```rust
pub struct DxRouter {
    deployments: Vec<Deployment>,
    strategy: LoadBalancingStrategy,
    retry_policy: RetryPolicy,
    fallback_chain: HashMap<String, Vec<String>>,   // model -> fallback models
    context_window_fallbacks: HashMap<String, String>, // gpt-3.5 -> gpt-4-32k
    cost_engine: Arc<CostEngine>,
    budget_manager: Arc<BudgetManager>,
    rate_limiter: Arc<RateLimitManager>,
}

pub enum LoadBalancingStrategy {
    RoundRobin,
    LowestLatencyP50,
    LowestCost,
    LeastBusy,
    WeightedRandom(Vec<f32>),
}

impl DxRouter {
    pub async fn completion(&self, req: ChatRequest) -> Result<ChatResponse, DxError> {
        let mut last_error = None;
        
        // 1. Try primary deployment with retries
        for attempt in 0..=self.retry_policy.max_retries {
            let deployment = self.select_deployment(&self.strategy);
            
            match deployment.execute(&req).await {
                Ok(resp) => {
                    let cost = self.cost_engine.completion_cost(&resp);
                    self.budget_manager.check_and_record(&req.key_id, cost).await?;
                    return Ok(resp);
                }
                Err(DxError::ContextWindowExceeded { .. }) => {
                    // Try context window fallback immediately
                    if let Some(fallback_model) = self.context_window_fallbacks.get(&req.model) {
                        let mut fallback_req = req.clone();
                        fallback_req.model = fallback_model.clone();
                        return self.execute_single(&fallback_req).await;
                    }
                    last_error = Some(DxError::ContextWindowExceeded { .. });
                    break;
                }
                Err(DxError::RateLimited { retry_after }) => {
                    let backoff = retry_after.unwrap_or(
                        self.retry_policy.exponential_backoff(attempt)
                    );
                    tokio::time::sleep(backoff).await;
                    last_error = Some(DxError::RateLimited { retry_after });
                    continue;
                }
                Err(e) => {
                    last_error = Some(e);
                    break;
                }
            }
        }
        
        // 2. Try fallback models
        if let Some(fallbacks) = self.fallback_chain.get(&req.model) {
            for fallback in fallbacks {
                let mut fallback_req = req.clone();
                fallback_req.model = fallback.clone();
                if let Ok(resp) = self.execute_single(&fallback_req).await {
                    return Ok(resp);
                }
            }
        }
        
        Err(last_error.unwrap_or(DxError::AllDeploymentsFailed))
    }
}
```

### Feature 6: Rate Limiting (RPM/TPM)

```rust
use governor::{Quota, RateLimiter, clock::DefaultClock, state::keyed::DashMapStateStore};
use std::num::NonZeroU32;

pub struct RateLimitManager {
    // RPM limiters keyed by (key_id, model)
    rpm: DashMap<String, Arc<RateLimiter<String, DashMapStateStore<String>, DefaultClock>>>,
    // TPM tracking keyed by (key_id, model)  
    tpm: DashMap<String, Arc<AtomicU64>>,
    tpm_window_start: DashMap<String, Instant>,
}

impl RateLimitManager {
    pub fn check_rpm(&self, key_id: &str, model: &str, limit: u32) -> Result<(), DxError> {
        let k = format!("{key_id}:{model}");
        let limiter = self.rpm.entry(k.clone()).or_insert_with(|| {
            Arc::new(RateLimiter::keyed(
                Quota::per_minute(NonZeroU32::new(limit).unwrap())
            ))
        });
        limiter.check_key(&k).map_err(|_| DxError::RateLimited {
            retry_after: Some(Duration::from_secs(60 / limit as u64)),
        })
    }
    
    pub fn check_tpm(
        &self, key_id: &str, model: &str, estimated_tokens: u64, limit: u64,
    ) -> Result<(), DxError> {
        let k = format!("{key_id}:{model}");
        let current = self.tpm.entry(k.clone())
            .or_insert(Arc::new(AtomicU64::new(0)));
        
        // Reset if window expired
        let window_start = self.tpm_window_start.entry(k.clone())
            .or_insert(Instant::now());
        if window_start.elapsed() > Duration::from_secs(60) {
            current.store(0, Ordering::Relaxed);
            *window_start = Instant::now();
        }
        
        let new_total = current.fetch_add(estimated_tokens, Ordering::Relaxed) + estimated_tokens;
        if new_total > limit {
            current.fetch_sub(estimated_tokens, Ordering::Relaxed);
            return Err(DxError::TokenRateLimited { limit, used: new_total });
        }
        Ok(())
    }
}
```

### Feature 7: Unified Error Mapping

LiteLLM maps exceptions across all supported providers to the OpenAI exceptions. All our exceptions inherit from OpenAI's exception types, so any error-handling you have for that, should work out of the box with LiteLLM.

```rust
#[derive(Debug, thiserror::Error)]
pub enum DxError {
    #[error("Authentication failed: {message}")]
    AuthenticationError { message: String, provider: String },
    
    #[error("Rate limited (retry after {retry_after:?})")]
    RateLimited { retry_after: Option<Duration> },
    
    #[error("Token rate limited: {used}/{limit} TPM")]
    TokenRateLimited { limit: u64, used: u64 },
    
    #[error("Context window exceeded: {tokens} tokens > {max} max")]
    ContextWindowExceeded { tokens: u64, max: u64 },
    
    #[error("Budget exceeded: ${spent:.4} / ${budget:.4}")]
    BudgetExceeded { spent: f64, budget: f64 },
    
    #[error("Content filtered by provider")]
    ContentFiltered { provider: String, reason: Option<String> },
    
    #[error("Model not found: {model}")]
    ModelNotFound { model: String },
    
    #[error("Provider error ({status}): {message}")]
    ProviderError { status: u16, message: String, provider: String },
    
    #[error("All deployments failed")]
    AllDeploymentsFailed,
    
    #[error("Guardrail violation: {rule}")]
    GuardrailViolation { rule: String, detail: Option<String> },
    
    #[error("Timeout after {elapsed:?}")]
    Timeout { elapsed: Duration },
}

// Map from each provider's error format
impl DxError {
    pub fn from_anthropic(status: u16, body: &serde_json::Value) -> Self { /* ... */ }
    pub fn from_openai(status: u16, body: &serde_json::Value) -> Self { /* ... */ }
    pub fn from_bedrock(status: u16, body: &serde_json::Value) -> Self { /* ... */ }
    // ...
    
    /// OpenAI-compatible error code (for proxy compatibility)
    pub fn openai_error_code(&self) -> &str {
        match self {
            Self::AuthenticationError { .. } => "invalid_api_key",
            Self::RateLimited { .. } => "rate_limit_exceeded",
            Self::ContextWindowExceeded { .. } => "context_length_exceeded",
            Self::ContentFiltered { .. } => "content_filter",
            _ => "server_error",
        }
    }
    
    pub fn http_status(&self) -> u16 {
        match self {
            Self::AuthenticationError { .. } => 401,
            Self::RateLimited { .. } | Self::TokenRateLimited { .. } => 429,
            Self::BudgetExceeded { .. } => 402,
            Self::ModelNotFound { .. } => 404,
            Self::ContentFiltered { .. } => 400,
            _ => 500,
        }
    }
}
```

### Feature 8: Guardrails (Zero-Cost, <0.1ms)

```rust
#[async_trait]
pub trait Guardrail: Send + Sync {
    fn name(&self) -> &str;
    async fn pre_call(&self, req: &ChatRequest) -> Result<(), GuardrailViolation>;
    async fn post_call(&self, resp: &ChatResponse) -> Result<(), GuardrailViolation>;
}

/// Zero-cost: runs in-process with Aho-Corasick, <0.1ms
pub struct CompetitorBlocker {
    patterns: aho_corasick::AhoCorasick,
    competitor_names: Vec<String>,
}

pub struct TopicBlocker {
    blocked_topics: Vec<String>,
    patterns: aho_corasick::AhoCorasick,
}

pub struct InsultFilter {
    patterns: aho_corasick::AhoCorasick,
}

pub struct PiiFilter {
    regexes: Vec<regex::Regex>, // SSN, email, phone, credit card patterns
}

pub struct GuardrailChain {
    pre: Vec<Box<dyn Guardrail>>,
    post: Vec<Box<dyn Guardrail>>,
}

impl GuardrailChain {
    pub async fn run_pre(&self, req: &ChatRequest) -> Result<(), DxError> {
        for g in &self.pre {
            g.pre_call(req).await.map_err(|v| DxError::GuardrailViolation {
                rule: g.name().to_string(),
                detail: Some(v.to_string()),
            })?;
        }
        Ok(())
    }
}
```

### Feature 9: Caching

```rust
pub enum CacheBackend {
    InMemory(DashMap<String, CachedEntry>),
    Redis(redis::aio::MultiplexedConnection),
}

pub struct DxCache {
    backend: CacheBackend,
    namespace_fn: Option<Box<dyn Fn(&ChatRequest) -> String + Send + Sync>>,
}

impl DxCache {
    pub async fn get(&self, req: &ChatRequest) -> Option<ChatResponse> {
        let key = self.compute_key(req);
        match &self.backend {
            CacheBackend::InMemory(map) => map.get(&key).map(|e| e.response.clone()),
            CacheBackend::Redis(conn) => {
                let data: Option<Vec<u8>> = redis::cmd("GET").arg(&key)
                    .query_async(&mut conn.clone()).await.ok()?;
                data.and_then(|d| bincode::deserialize(&d).ok())
            }
        }
    }
    
    fn compute_key(&self, req: &ChatRequest) -> String {
        let mut hasher = sha2::Sha256::new();
        hasher.update(req.model.as_bytes());
        for msg in &req.messages {
            hasher.update(msg.role.as_str().as_bytes());
            hasher.update(msg.content.as_bytes());
        }
        if let Some(ns_fn) = &self.namespace_fn {
            hasher.update(ns_fn(req).as_bytes());
        }
        format!("dx:cache:{}", hex::encode(hasher.finalize()))
    }
}
```

### Feature 10: Observability & Logging

LiteLLM exposes pre defined callbacks to send data to Lunary, MLflow, Langfuse, Helicone, Promptlayer, Traceloop, Slack.

```rust
#[async_trait]
pub trait DxCallback: Send + Sync {
    async fn on_request(&self, event: &RequestEvent);
    async fn on_response(&self, event: &ResponseEvent);
    async fn on_error(&self, event: &ErrorEvent);
}

pub struct ResponseEvent {
    pub request_id: String,
    pub model: String,
    pub provider: String,
    pub latency: Duration,
    pub usage: TokenUsage,
    pub cost_usd: f64,
    pub user_id: Option<String>,
    pub team_id: Option<String>,
    pub tags: Vec<String>,
    pub cache_hit: bool,
}

// Built-in callback implementations
pub struct LangfuseCallback { /* ... */ }
pub struct PrometheusCallback { metrics: PrometheusMetrics }
pub struct OpenTelemetryCallback { tracer: BoxedTracer }
pub struct SlackAlertCallback { webhook_url: String, cooldown: Duration }

pub struct CallbackChain(Vec<Box<dyn DxCallback>>);
```

### Feature 11: Virtual Key Management

```rust
#[derive(Debug, sqlx::FromRow)]
pub struct VirtualKey {
    pub id: String,
    pub key_hash: String,           // argon2 hash
    pub key_prefix: String,         // "dx-..." for display
    pub team_id: Option<String>,
    pub user_id: Option<String>,
    pub max_budget: Option<f64>,
    pub budget_duration: Option<String>, // "30d", "1h", etc.
    pub budget_reset_at: Option<DateTime<Utc>>,
    pub spend: f64,
    pub rpm_limit: Option<u32>,
    pub tpm_limit: Option<u64>,
    pub allowed_models: Option<Vec<String>>,
    pub blocked_models: Option<Vec<String>>,
    pub expires_at: Option<DateTime<Utc>>,
    pub metadata: serde_json::Value,
    pub tags: Vec<String>,
    pub created_at: DateTime<Utc>,
}
```

### Feature 12: Proxy Server (AI Gateway)

```rust
use axum::{Router, Json, extract::State, middleware};

pub fn build_proxy_router(state: AppState) -> Router {
    Router::new()
        // OpenAI-compatible endpoints
        .route("/v1/chat/completions", post(chat_completions))
        .route("/v1/completions", post(completions))
        .route("/v1/embeddings", post(embeddings))
        .route("/v1/models", get(list_models))
        .route("/v1/images/generations", post(image_generations))
        .route("/v1/audio/speech", post(audio_speech))
        .route("/v1/audio/transcriptions", post(audio_transcriptions))
        // DX management endpoints
        .route("/key/generate", post(generate_key))
        .route("/key/info", get(key_info))
        .route("/key/update", post(update_key))
        .route("/key/delete", post(delete_key))
        .route("/team/new", post(create_team))
        .route("/user/daily/activity", get(daily_activity))
        .route("/cost/estimate", post(cost_estimate))
        .route("/model/info", get(model_info))
        .route("/health", get(health_check))
        // Middleware pipeline
        .layer(middleware::from_fn_with_state(state.clone(), auth_middleware))
        .layer(middleware::from_fn_with_state(state.clone(), rate_limit_middleware))
        .layer(middleware::from_fn_with_state(state.clone(), budget_middleware))
        .with_state(state)
}

async fn chat_completions(
    State(app): State<AppState>,
    DxAuth(key): DxAuth,
    Json(req): Json<ChatRequest>,
) -> Result<impl IntoResponse, DxError> {
    // 1. Check allowed models
    key.check_model_access(&req.model)?;
    // 2. Rate limit
    app.rate_limiter.check_rpm(&key.id, &req.model, key.rpm_limit.unwrap_or(600))?;
    // 3. Budget check (skip if zero-cost model)
    if !app.budget_manager.should_skip_budget_check(&req.model) {
        app.budget_manager.check_budget(&key.id).await?;
    }
    // 4. Cache check
    if let Some(cached) = app.cache.get(&req).await {
        return Ok(Json(cached));
    }
    // 5. Guardrails (pre-call)
    app.guardrails.run_pre(&req).await?;
    // 6. Normalize request for provider
    let provider = app.registry.resolve_provider(&req.model)?;
    provider.normalizer().normalize_request(&mut req, &model_info);
    // 7. Route to LLM (with retry/fallback)
    let response = if req.stream {
        return Ok(app.router.stream_completion(req).await?.into_response());
    } else {
        app.router.completion(req).await?
    };
    // 8. Guardrails (post-call)
    app.guardrails.run_post(&response).await?;
    // 9. Cost tracking
    let cost = app.cost_engine.completion_cost(&response);
    app.budget_manager.check_and_record(&key.id, cost).await?;
    // 10. Callbacks (async, non-blocking)
    tokio::spawn(app.callbacks.on_response(ResponseEvent { /* ... */ }));
    // 11. Cache set
    app.cache.set(&req, &response, Duration::from_secs(3600)).await;
    
    Ok(Json(response))
}
```

### Feature 13: Config via TOML/YAML

```rust
#[derive(Deserialize)]
pub struct DxConfig {
    pub model_list: Vec<ModelDeployment>,
    pub settings: DxSettings,
    pub guardrails: Option<Vec<GuardrailConfig>>,
    pub callbacks: Option<Vec<CallbackConfig>>,
    pub cache: Option<CacheConfig>,
}

#[derive(Deserialize)]
pub struct ModelDeployment {
    pub model_name: String,          // Routing name
    pub provider: String,            // "openai", "anthropic", etc.
    pub model: String,               // Provider-specific model ID
    pub api_key: Option<String>,     // "env:OPENAI_API_KEY"
    pub api_base: Option<String>,
    pub rpm: Option<u32>,
    pub tpm: Option<u64>,
    pub model_info: Option<ModelInfoOverride>,
}

// Load: supports both TOML and YAML
let config: DxConfig = match path.extension() {
    Some("toml") => toml::from_str(&std::fs::read_to_string(path)?)?,
    Some("yaml" | "yml") => serde_yaml::from_reader(std::fs::File::open(path)?)?,
    _ => return Err(ConfigError::UnsupportedFormat),
};
```

---

## Part 6: DX-Specific Advantages Over LiteLLM

Here's where your Rust implementation + DX vision leapfrogs LiteLLM:

### 6a. RLM Integration at the Provider Layer

Since you have RLM (80-90% token savings on large files), integrate it directly into the request pipeline:

```rust
impl DxRouter {
    async fn prepare_request(&self, mut req: ChatRequest) -> ChatRequest {
        // Apply RLM compression to any file content in messages
        for msg in &mut req.messages {
            if let Some(file_content) = msg.extract_file_content() {
                if file_content.len() > RLM_THRESHOLD {
                    msg.content = self.rlm_engine.compress(file_content);
                    // This is where Rust's speed makes RLM viable
                }
            }
        }
        req
    }
}
```

### 6b. DX Serializer for Tool Calls

Replace JSON tool call serialization with your compact format:

```rust
pub trait ToolSerializer: Send + Sync {
    fn serialize_tool_call(&self, call: &ToolCall) -> Vec<u8>;
    fn deserialize_tool_result(&self, data: &[u8]) -> ToolResult;
    fn token_estimate(&self, data: &[u8]) -> u64;
}

pub struct DxSerializer;   // 70-90% smaller than JSON
pub struct JsonSerializer; // Standard fallback

impl DxRouter {
    fn select_serializer(&self, provider: &ProviderInfo) -> Box<dyn ToolSerializer> {
        if provider.supports_dx_format() {
            Box::new(DxSerializer)
        } else {
            Box::new(JsonSerializer) // Fall back for external providers
        }
    }
}
```

### 6c. Offline-First with Local Model Fallback

```rust
pub struct HybridRouter {
    cloud_router: DxRouter,
    local_router: DxRouter,  // Ollama, llama.cpp, etc.
}

impl HybridRouter {
    pub async fn completion(&self, req: ChatRequest) -> Result<ChatResponse, DxError> {
        // Try cloud first
        match self.cloud_router.completion(req.clone()).await {
            Ok(resp) => Ok(resp),
            Err(DxError::Timeout { .. }) | Err(DxError::AllDeploymentsFailed) => {
                // Seamless fallback to local model
                let local_model = self.select_best_local_model(&req)?;
                let mut local_req = req;
                local_req.model = local_model;
                self.local_router.completion(local_req).await
            }
            Err(e) => Err(e),
        }
    }
}
```

---

## Part 7: Recommended Rust Crate Stack

| Feature | Crate(s) |
|---------|----------|
| HTTP client | `reqwest` |
| Async runtime | `tokio` |
| Web framework (proxy) | `axum` + `tower` |
| Serialization | `serde`, `serde_json`, `serde_yaml`, `toml`, `bincode` |
| Database | `sqlx` (PostgreSQL) |
| Cache | `redis`, `dashmap` |
| Rate limiting | `governor` |
| Retry/backoff | `backoff` or `again` |
| SSE streaming | `eventsource-stream`, `reqwest` streaming, `async-stream` |
| Error handling | `thiserror`, `anyhow` |
| Observability | `tracing`, `opentelemetry`, `prometheus` |
| Pattern matching (guardrails) | `aho-corasick`, `regex` |
| Key hashing | `argon2`, `sha2` |
| Tokenization | `tiktoken-rs`, `tokenizers` (HF) |
| AWS auth (Bedrock) | `aws-sigv4`, `aws-credential-types` |
| GUI (desktop) | GPUI (Zed's framework) |

---

## Part 8: File/Module Layout

```
dx/
├── crates/
│   ├── dx_llm/                    # Core unified LLM client
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── completion.rs      # Unified completion() API
│   │   │   ├── streaming.rs       # SSE stream handling
│   │   │   ├── types.rs           # ChatRequest, ChatResponse, etc.
│   │   │   ├── error.rs           # DxError unified error type
│   │   │   ├── providers/
│   │   │   │   ├── mod.rs
│   │   │   │   ├── openai.rs      # OpenAI + Responses API
│   │   │   │   ├── anthropic.rs   # Anthropic Messages API
│   │   │   │   ├── google.rs      # Gemini + Vertex
│   │   │   │   ├── bedrock.rs     # AWS Bedrock (SigV4)
│   │   │   │   ├── azure.rs       # Azure OpenAI
│   │   │   │   ├── ollama.rs      # Local Ollama
│   │   │   │   ├── openai_compat.rs # Generic OpenAI-compatible (40+ providers)
│   │   │   │   └── ...
│   │   │   └── normalizers/       # Provider-specific request tweaks
│   │   │       ├── mod.rs
│   │   │       ├── anthropic.rs   # Cache markers, alternating roles, temp=1
│   │   │       ├── openai.rs      # Strip temp for o3/GPT-5, Responses API routing
│   │   │       └── google.rs      # Vertex auth, location routing
│   │   
│   ├── dx_registry/               # Models.dev + LiteLLM cost map integration
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── models_dev.rs      # Fetch from Models.dev API
│   │   │   ├── litellm_costs.rs   # Parse LiteLLM cost JSON
│   │   │   ├── registry.rs        # Merged registry with capability queries
│   │   │   └── snapshot.json      # Bundled offline snapshot
│   │   
│   ├── dx_router/                 # Retry, fallback, load balancing
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── router.rs
│   │   │   ├── retry.rs
│   │   │   ├── fallback.rs
│   │   │   └── load_balance.rs
│   │   
│   ├── dx_budget/                 # Cost tracking, budgets, virtual keys
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── cost_engine.rs
│   │   │   ├── budget_manager.rs
│   │   │   ├── virtual_keys.rs
│   │   │   └── pricing_calculator.rs
│   │   
│   ├── dx_rate_limit/             # RPM/TPM rate limiting
│   ├── dx_cache/                  # In-memory + Redis caching
│   ├── dx_guardrails/             # Competitor blocker, PII filter, etc.
│   ├── dx_observe/                # Callbacks: Langfuse, Prometheus, OTEL
│   ├── dx_tokenizer/              # Multi-provider tokenization
│   ├── dx_proxy/                  # Axum-based AI Gateway server
│   │   ├── src/
│   │   │   ├── main.rs
│   │   │   ├── routes/
│   │   │   │   ├── chat.rs
│   │   │   │   ├── embeddings.rs
│   │   │   │   ├── models.rs
│   │   │   │   ├── keys.rs
│   │   │   │   └── health.rs
│   │   │   └── middleware/
│   │   │       ├── auth.rs
│   │   │       ├── rate_limit.rs
│   │   │       └── budget.rs
│   │   
│   ├── dx_serializer/             # DX compact serialization (your 70-90% savings)
│   └── dx_rlm/                    # Reference-Length Minimization engine
│
├── config/
│   ├── dx_config.toml             # Default config
│   └── dx_config.example.yaml     # Example YAML config
│
└── data/
    ├── model_costs.json           # Bundled LiteLLM cost map snapshot
    └── models_dev_snapshot.json   # Bundled Models.dev snapshot
```

---

## Summary: Your Path to 100+ Providers

1. **Build ~6 Tier 1 native adapters** (OpenAI, Anthropic, Google, Bedrock, Azure, Ollama)
2. **Build 1 generic OpenAI-compatible adapter** → instantly covers 40+ providers
3. **Integrate Models.dev API** for automatic model discovery, capabilities, and limits
4. **Integrate LiteLLM's `model_prices_and_context_window.json`** for comprehensive cost data
5. **Support aggregators** (OpenRouter, Cloudflare AI Gateway, Vercel AI Gateway) → each multiplies your reach
6. **Handle the real quirks** (temperature stripping, cache markers, Responses API routing, SigV4 auth)
7. **Layer on operational features**: router (retry/fallback/load balance), budget management, rate limiting, guardrails, caching, observability, virtual keys, proxy server
8. **DX-specific innovations**: RLM integration, DX Serializer, offline-first hybrid routing

This gives you **100+ providers with fewer than 30 adapter implementations**, full LiteLLM parity on operational features, and Rust-grade performance that makes RLM and DX Serializer viable where Node.js/Python can't.
