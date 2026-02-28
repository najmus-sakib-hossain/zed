//! CLI Prompt Test Suite - Run: cargo run [1-36] or cargo run -- --dx-onboard
#![allow(dead_code)]

mod prompt_suite;
mod prompts;

use anyhow::Result;
use argon2::Argon2;
use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use chrono::Local;
use dx_onboard::llm::types::{ChatMessage, MessageContent};
use dx_onboard::llm::{
    ChatRequest, GenericProvider, LlmProvider, ProviderConfigFile, ProviderProfileEntry,
    ProviderRegistry, openai_compatible_provider_presets, refresh_discovery_catalog,
};
use image::imageops::FilterType;
#[allow(unused_imports)]
use libsql::{Builder, params};
use oauth2::basic::BasicClient;
use oauth2::reqwest::async_http_client;
use oauth2::{
    AuthUrl, AuthorizationCode, ClientId, ClientSecret, CsrfToken, PkceCodeChallenge, RedirectUrl,
    Scope, TokenResponse, TokenUrl,
};
use owo_colors::OwoColorize;
use rand::thread_rng;
use serde::{Deserialize, Serialize};
use serde_json::{self, Value, json};
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::num::NonZeroU32;
use std::path::{Path, PathBuf};
use std::time::Duration;
use textwrap::wrap;
use url::Url;

use prompts::PromptInteraction;

#[derive(Debug, Default, Clone)]
struct OnboardCliArgs {
    shared_account: Option<String>,
    account_email: Option<String>,
}

#[derive(Debug, Clone, Copy)]
enum RuntimeEnvironment {
    RealOs,
    Vps,
    Container,
    Restricted,
}

impl RuntimeEnvironment {
    fn as_str(self) -> &'static str {
        match self {
            RuntimeEnvironment::RealOs => "real_os",
            RuntimeEnvironment::Vps => "vps",
            RuntimeEnvironment::Container => "container",
            RuntimeEnvironment::Restricted => "restricted",
        }
    }

    fn label(self) -> &'static str {
        match self {
            RuntimeEnvironment::RealOs => "Real OS workstation",
            RuntimeEnvironment::Vps => "VPS / Cloud VM",
            RuntimeEnvironment::Container => "Docker / Container",
            RuntimeEnvironment::Restricted => "Restricted / CI runner",
        }
    }

    fn hint(self) -> &'static str {
        match self {
            RuntimeEnvironment::RealOs => "Best for desktop app + extension onboarding",
            RuntimeEnvironment::Vps => "Best for remote gateway + channel bridge",
            RuntimeEnvironment::Container => "Best for ephemeral test/deploy environments",
            RuntimeEnvironment::Restricted => "Best for non-interactive automation",
        }
    }
}

fn parse_onboard_args(raw: &[String]) -> OnboardCliArgs {
    let mut parsed = OnboardCliArgs::default();
    let mut index = 0;

    while index < raw.len() {
        match raw[index].as_str() {
            "--shared-account" => {
                if let Some(value) = raw.get(index + 1) {
                    parsed.shared_account = Some(value.clone());
                    index += 1;
                }
            }
            "--account-email" => {
                if let Some(value) = raw.get(index + 1) {
                    parsed.account_email = Some(value.clone());
                    index += 1;
                }
            }
            _ => {}
        }
        index += 1;
    }

    parsed
}

fn detect_runtime_environment() -> RuntimeEnvironment {
    let ci = env::var("CI")
        .map(|value| {
            let normalized = value.to_ascii_lowercase();
            normalized == "1" || normalized == "true"
        })
        .unwrap_or(false);
    if ci {
        return RuntimeEnvironment::Restricted;
    }

    let container_detected = Path::new("/.dockerenv").exists()
        || env::var("KUBERNETES_SERVICE_HOST").is_ok()
        || env::var("DOCKER_CONTAINER").is_ok()
        || fs::read_to_string("/proc/1/cgroup")
            .map(|content| {
                let lowered = content.to_ascii_lowercase();
                lowered.contains("docker")
                    || lowered.contains("containerd")
                    || lowered.contains("kubepods")
                    || lowered.contains("podman")
            })
            .unwrap_or(false);
    if container_detected {
        return RuntimeEnvironment::Container;
    }

    let cloud_hint = env::var("VERCEL")
        .or_else(|_| env::var("RAILWAY_ENVIRONMENT"))
        .or_else(|_| env::var("FLY_APP_NAME"))
        .or_else(|_| env::var("HEROKU_APP_NAME"))
        .or_else(|_| env::var("DIGITALOCEAN_APP_ID"))
        .or_else(|_| env::var("AWS_EXECUTION_ENV"))
        .or_else(|_| env::var("GCP_PROJECT"))
        .or_else(|_| env::var("AZURE_HTTP_USER_AGENT"))
        .is_ok();
    let virtualization_hint =
        Path::new("/proc/vz").exists() || Path::new("/proc/user_beancounters").exists();

    if cloud_hint || virtualization_hint {
        return RuntimeEnvironment::Vps;
    }

    RuntimeEnvironment::RealOs
}

#[derive(Debug, Clone, Serialize)]
struct AuthResult {
    method: String,
    email: String,
    name: String,
    oauth_subject: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
struct OnboardingPayload {
    runtime_environment: String,
    runtime_label: String,
    install_mode: String,
    components_to_download: Vec<String>,
    account: AuthResult,
    avatar_source: Option<String>,
    avatar_ascii_preview_generated: bool,
    avatar_pixel_preview_generated: bool,
    llm_providers: Vec<String>,
    available_provider_count: usize,
    provider_100_plus_ready: bool,
    provider_connection_status: Vec<ProviderConnectionStatus>,
    provider_models: Vec<ProviderModelListing>,
    provider_model_probes: Vec<ProviderModelProbeResult>,
    provider_pricing_hints: Vec<String>,
    provider_config_path: Option<String>,
    provider_config_status: String,
    smart_default_model: String,
    small_subagent_model: String,
    multi_agent_model: String,
    messaging_channels: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
struct ProviderConnectionStatus {
    provider_id: String,
    status: String,
    detail: String,
}

#[derive(Debug, Clone, Serialize)]
struct ProviderModelListing {
    provider_id: String,
    models: Vec<String>,
    modules: Vec<String>,
    status: String,
    detail: String,
}

#[derive(Debug, Clone, Serialize)]
struct ProviderModelProbeResult {
    provider_id: String,
    api_key_env: Option<String>,
    selected_model: Option<String>,
    status: String,
    detail: String,
    response_preview: Option<String>,
}

fn find_workspace_root() -> PathBuf {
    let cwd = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    for ancestor in cwd.ancestors() {
        let cargo_toml = ancestor.join("Cargo.toml");
        if cargo_toml.exists()
            && let Ok(content) = fs::read_to_string(&cargo_toml)
            && content.contains("[workspace]")
        {
            return ancestor.to_path_buf();
        }
    }
    cwd
}

fn load_workspace_env(workspace_root: &Path) {
    let env_path = workspace_root.join(".env");
    if env_path.exists() {
        let _ = dotenvy::from_path(env_path);
    }

    let root_env_path = workspace_root.join("root.env");
    if root_env_path.exists() {
        let _ = dotenvy::from_path(root_env_path);
    }
}

fn ensure_provider_env_aliases() {
    if env::var("GOOGLE_API_KEY").is_err()
        && let Ok(gemini) = env::var("GEMINI_API_KEY")
        && !gemini.trim().is_empty()
    {
        // SAFETY: Setting environment variable for provider alias compatibility
        unsafe {
            env::set_var("GOOGLE_API_KEY", gemini.clone());
        }
    }

    if env::var("GEMINI_API_KEY").is_err()
        && let Ok(google) = env::var("GOOGLE_API_KEY")
        && !google.trim().is_empty()
    {
        // SAFETY: Setting environment variable for provider alias compatibility
        unsafe {
            env::set_var("GEMINI_API_KEY", google);
        }
    }
}

async fn discover_provider_models(provider_id: &str) -> ProviderModelListing {
    let api_key_var = match provider_id {
        "google" => "GOOGLE_API_KEY",
        "groq" => "GROQ_API_KEY",
        "github_copilot" | "github-copilot" => "GITHUB_COPILOT_TOKEN",
        _ => {
            return ProviderModelListing {
                provider_id: provider_id.to_string(),
                models: Vec::new(),
                modules: Vec::new(),
                status: "skipped".to_string(),
                detail: "model discovery currently enabled for google, groq, and github_copilot"
                    .to_string(),
            };
        }
    };

    let api_key = if provider_id == "github_copilot" || provider_id == "github-copilot" {
        // Use retrieve_copilot_token which checks config files
        match dx_onboard::llm::copilot::retrieve_copilot_token() {
            Some(token) => token,
            None => {
                return ProviderModelListing {
                    provider_id: provider_id.to_string(),
                    models: Vec::new(),
                    modules: Vec::new(),
                    status: "missing_key".to_string(),
                    detail: format!("missing {}", api_key_var),
                };
            }
        }
    } else {
        match env::var(api_key_var) {
            Ok(value) if !value.trim().is_empty() => value,
            _ => {
                return ProviderModelListing {
                    provider_id: provider_id.to_string(),
                    models: Vec::new(),
                    modules: Vec::new(),
                    status: "missing_key".to_string(),
                    detail: format!("missing {}", api_key_var),
                };
            }
        }
    };

    // Set the token in environment so GitHubCopilotProvider::from_env() can find it
    if provider_id == "github_copilot" || provider_id == "github-copilot" {
        // SAFETY: Setting environment variable for GitHub Copilot provider registration
        unsafe {
            env::set_var("GITHUB_COPILOT_TOKEN", &api_key);
        }
    }

    let mut registry = ProviderRegistry::new();
    registry.register_default_genai_providers();
    registry.register_enterprise_custom_providers();

    let Some(provider) = registry.get(provider_id) else {
        return ProviderModelListing {
            provider_id: provider_id.to_string(),
            models: Vec::new(),
            modules: Vec::new(),
            status: "error".to_string(),
            detail: "provider is not registered".to_string(),
        };
    };

    eprintln!("DEBUG: Using provider with base_url: {}", provider.base_url());
    eprintln!("DEBUG: Provider ID: {}", provider.id());

    if provider_id == "google" {
        // SAFETY: Setting environment variables for Google/Gemini provider compatibility
        unsafe {
            env::set_var("GOOGLE_API_KEY", api_key.clone());
            env::set_var("GEMINI_API_KEY", &api_key);
        }
    }

    if provider_id == "github_copilot" || provider_id == "github-copilot" {
        // SAFETY: Setting environment variable for GitHub Copilot provider
        unsafe {
            env::set_var("GITHUB_COPILOT_TOKEN", &api_key);
        }
    }

    match provider.get_models().await {
        Ok(models) => {
            let unique = models
                .into_iter()
                .map(|model| model.id)
                .filter(|id| !id.trim().is_empty())
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect::<Vec<_>>();

            ProviderModelListing {
                provider_id: provider_id.to_string(),
                models: unique,
                modules: Vec::new(),
                status: "ok".to_string(),
                detail: "fetched model list".to_string(),
            }
        }
        Err(err) => ProviderModelListing {
            provider_id: provider_id.to_string(),
            models: Vec::new(),
            modules: Vec::new(),
            status: "error".to_string(),
            detail: err.to_string(),
        },
    }
}

fn build_component_targets(runtime: RuntimeEnvironment) -> Vec<String> {
    match runtime {
        RuntimeEnvironment::RealOs => vec![
            "desktop_app".to_string(),
            "tui".to_string(),
            "ide_extension".to_string(),
            "browser_extension".to_string(),
            "local_website".to_string(),
        ],
        RuntimeEnvironment::Vps
        | RuntimeEnvironment::Container
        | RuntimeEnvironment::Restricted => {
            vec!["tui".to_string(), "local_website".to_string()]
        }
    }
}

fn provider_catalog() -> Vec<(&'static str, &'static str)> {
    provider_catalog_detailed()
        .into_iter()
        .map(|item| (item.id, item.label))
        .collect()
}

#[derive(Clone, Copy)]
struct ProviderCatalogItem {
    id: &'static str,
    label: &'static str,
    category: &'static str,
}

fn provider_catalog_detailed() -> Vec<ProviderCatalogItem> {
    vec![
        // Temporarily disabled per product direction:
        // ProviderCatalogItem {
        //     id: "google_antigravity",
        //     label: "Google AntiGravity Code Editor",
        //     category: "enterprise",
        // },
        ProviderCatalogItem {
            id: "github_copilot",
            label: "GitHub Copilot",
            category: "popular",
        },
        ProviderCatalogItem {
            id: "openai",
            label: "OpenAI",
            category: "popular",
        },
        ProviderCatalogItem {
            id: "anthropic",
            label: "Anthropic",
            category: "popular",
        },
        ProviderCatalogItem {
            id: "google",
            label: "Google Gemini",
            category: "popular",
        },
        ProviderCatalogItem {
            id: "xai",
            label: "xAI",
            category: "popular",
        },
        ProviderCatalogItem {
            id: "mistral",
            label: "Mistral",
            category: "popular",
        },
        ProviderCatalogItem {
            id: "cohere",
            label: "Cohere",
            category: "popular",
        },
        ProviderCatalogItem {
            id: "meta",
            label: "Meta",
            category: "popular",
        },
        ProviderCatalogItem {
            id: "amazon_bedrock",
            label: "Amazon Bedrock",
            category: "enterprise",
        },
        ProviderCatalogItem {
            id: "azure_openai",
            label: "Azure OpenAI",
            category: "enterprise",
        },
        ProviderCatalogItem {
            id: "groq",
            label: "Groq",
            category: "specialized",
        },
        ProviderCatalogItem {
            id: "deepseek",
            label: "DeepSeek",
            category: "specialized",
        },
        ProviderCatalogItem {
            id: "perplexity",
            label: "Perplexity",
            category: "specialized",
        },
        ProviderCatalogItem {
            id: "openrouter",
            label: "OpenRouter",
            category: "specialized",
        },
        ProviderCatalogItem {
            id: "together",
            label: "Together AI",
            category: "specialized",
        },
        ProviderCatalogItem {
            id: "replicate",
            label: "Replicate",
            category: "open_source",
        },
        ProviderCatalogItem {
            id: "fireworks",
            label: "Fireworks",
            category: "specialized",
        },
        ProviderCatalogItem {
            id: "huggingface",
            label: "Hugging Face",
            category: "open_source",
        },
        ProviderCatalogItem {
            id: "ollama",
            label: "Ollama",
            category: "open_source",
        },
        ProviderCatalogItem {
            id: "lmstudio",
            label: "LM Studio",
            category: "open_source",
        },
        ProviderCatalogItem {
            id: "vllm",
            label: "vLLM",
            category: "open_source",
        },
        ProviderCatalogItem {
            id: "litellm",
            label: "LiteLLM",
            category: "specialized",
        },
        ProviderCatalogItem {
            id: "nvidia_nim",
            label: "NVIDIA NIM",
            category: "enterprise",
        },
        ProviderCatalogItem {
            id: "qwen",
            label: "Qwen",
            category: "specialized",
        },
        ProviderCatalogItem {
            id: "zai",
            label: "Z.AI",
            category: "specialized",
        },
        ProviderCatalogItem {
            id: "baidu_qianfan",
            label: "Baidu Qianfan",
            category: "enterprise",
        },
        ProviderCatalogItem {
            id: "moonshot",
            label: "Moonshot",
            category: "specialized",
        },
        ProviderCatalogItem {
            id: "kimi",
            label: "Kimi",
            category: "specialized",
        },
        ProviderCatalogItem {
            id: "minimax",
            label: "MiniMax",
            category: "specialized",
        },
        ProviderCatalogItem {
            id: "byteplus",
            label: "BytePlus",
            category: "enterprise",
        },
        ProviderCatalogItem {
            id: "volcengine",
            label: "Volcengine",
            category: "enterprise",
        },
        ProviderCatalogItem {
            id: "sambanova",
            label: "SambaNova",
            category: "enterprise",
        },
        ProviderCatalogItem {
            id: "cerebras",
            label: "Cerebras",
            category: "specialized",
        },
        ProviderCatalogItem {
            id: "ai21",
            label: "AI21",
            category: "specialized",
        },
        ProviderCatalogItem {
            id: "writer",
            label: "Writer",
            category: "enterprise",
        },
        ProviderCatalogItem {
            id: "you",
            label: "You.com",
            category: "specialized",
        },
        ProviderCatalogItem {
            id: "venice",
            label: "Venice",
            category: "specialized",
        },
        ProviderCatalogItem {
            id: "friendli",
            label: "Friendli",
            category: "specialized",
        },
        ProviderCatalogItem {
            id: "baseten",
            label: "Baseten",
            category: "specialized",
        },
        ProviderCatalogItem {
            id: "lepton",
            label: "Lepton",
            category: "specialized",
        },
        ProviderCatalogItem {
            id: "modal",
            label: "Modal",
            category: "specialized",
        },
        ProviderCatalogItem {
            id: "anyscale",
            label: "Anyscale",
            category: "specialized",
        },
        ProviderCatalogItem {
            id: "runpod",
            label: "RunPod",
            category: "specialized",
        },
        ProviderCatalogItem {
            id: "clarifai",
            label: "Clarifai",
            category: "specialized",
        },
        ProviderCatalogItem {
            id: "databricks",
            label: "Databricks",
            category: "enterprise",
        },
        ProviderCatalogItem {
            id: "snowflake",
            label: "Snowflake Cortex",
            category: "enterprise",
        },
        ProviderCatalogItem {
            id: "cloudflare_workers_ai",
            label: "Cloudflare Workers AI",
            category: "specialized",
        },
        ProviderCatalogItem {
            id: "vercel_ai_gateway",
            label: "Vercel AI Gateway",
            category: "specialized",
        },
        ProviderCatalogItem {
            id: "cloudflare_ai_gateway",
            label: "Cloudflare AI Gateway",
            category: "specialized",
        },
        ProviderCatalogItem {
            id: "fastapi_compatible",
            label: "FastAPI Compatible",
            category: "specialized",
        },
        ProviderCatalogItem {
            id: "openapi_compatible",
            label: "OpenAPI Compatible",
            category: "specialized",
        },
        ProviderCatalogItem {
            id: "local_llama_cpp",
            label: "llama.cpp",
            category: "open_source",
        },
        ProviderCatalogItem {
            id: "jan",
            label: "Jan",
            category: "open_source",
        },
        ProviderCatalogItem {
            id: "anythingllm",
            label: "AnythingLLM",
            category: "open_source",
        },
        ProviderCatalogItem {
            id: "langchain_gateway",
            label: "LangChain Gateway",
            category: "specialized",
        },
        ProviderCatalogItem {
            id: "semantic_kernel",
            label: "Semantic Kernel",
            category: "enterprise",
        },
        ProviderCatalogItem {
            id: "ibm_watsonx",
            label: "IBM watsonx",
            category: "enterprise",
        },
        ProviderCatalogItem {
            id: "oracle_genai",
            label: "Oracle GenAI",
            category: "enterprise",
        },
        ProviderCatalogItem {
            id: "sap_ai_core",
            label: "SAP AI Core",
            category: "enterprise",
        },
        ProviderCatalogItem {
            id: "nebius",
            label: "Nebius",
            category: "specialized",
        },
        ProviderCatalogItem {
            id: "groq_cloud",
            label: "Groq Cloud",
            category: "specialized",
        },
        ProviderCatalogItem {
            id: "groq_proxy",
            label: "Groq Proxy",
            category: "specialized",
        },
        ProviderCatalogItem {
            id: "github_models",
            label: "GitHub Models",
            category: "enterprise",
        },
        ProviderCatalogItem {
            id: "alibaba_cloud",
            label: "Alibaba Cloud",
            category: "enterprise",
        },
        ProviderCatalogItem {
            id: "tencent_hunyuan",
            label: "Tencent Hunyuan",
            category: "enterprise",
        },
        ProviderCatalogItem {
            id: "huawei_pangu",
            label: "Huawei Pangu",
            category: "enterprise",
        },
        ProviderCatalogItem {
            id: "sensecore",
            label: "SenseCore",
            category: "enterprise",
        },
        ProviderCatalogItem {
            id: "inflection",
            label: "Inflection",
            category: "specialized",
        },
        ProviderCatalogItem {
            id: "alefalpha",
            label: "Aleph Alpha",
            category: "specialized",
        },
        ProviderCatalogItem {
            id: "stability",
            label: "Stability AI",
            category: "specialized",
        },
        ProviderCatalogItem {
            id: "deepinfra",
            label: "DeepInfra",
            category: "specialized",
        },
        ProviderCatalogItem {
            id: "novita",
            label: "Novita",
            category: "specialized",
        },
        ProviderCatalogItem {
            id: "siliconflow",
            label: "SiliconFlow",
            category: "specialized",
        },
        ProviderCatalogItem {
            id: "nscale",
            label: "NScale",
            category: "specialized",
        },
        ProviderCatalogItem {
            id: "portkey",
            label: "Portkey",
            category: "specialized",
        },
        ProviderCatalogItem {
            id: "humanloop",
            label: "Humanloop",
            category: "specialized",
        },
        ProviderCatalogItem {
            id: "langdock",
            label: "Langdock",
            category: "specialized",
        },
        ProviderCatalogItem {
            id: "custom",
            label: "Custom Endpoint",
            category: "specialized",
        },
    ]
}

fn provider_category_catalog() -> Vec<(&'static str, &'static str)> {
    vec![
        ("popular", "Popular"),
        ("enterprise", "Enterprise"),
        ("open_source", "Open Source"),
        ("specialized", "Specialized"),
    ]
}

fn provider_category_label(category: &str) -> &'static str {
    match category {
        "popular" => "Popular",
        "enterprise" => "Enterprise",
        "open_source" => "Open Source",
        "specialized" => "Specialized",
        _ => "Other",
    }
}

fn select_providers_grouped() -> Result<Vec<String>> {
    // Auto-select OpenCode (free models) by default
    Ok(vec!["opencode".to_string()])
}

async fn fetch_pricing_hints(provider_ids: &[String]) -> Vec<String> {
    if provider_ids.is_empty() {
        return Vec::new();
    }

    let client = reqwest::Client::new();
    let catalog = match refresh_discovery_catalog(&client).await {
        Ok(value) => value,
        Err(_) => return Vec::new(),
    };

    let mut hints = Vec::new();
    for provider_id in provider_ids {
        let canonical = provider_id.replace('_', "-");
        let discovered = catalog.providers.iter().find(|provider| {
            provider.id == *provider_id
                || provider.id == canonical
                || provider.id.replace('-', "_") == *provider_id
        });

        if let Some(provider) = discovered {
            let input = provider
                .avg_input_price_per_million
                .map(|value| format!("${value:.4}/1M in"))
                .unwrap_or_else(|| "n/a in".to_string());
            let output = provider
                .avg_output_price_per_million
                .map(|value| format!("${value:.4}/1M out"))
                .unwrap_or_else(|| "n/a out".to_string());
            hints.push(format!("{} => {} | {}", provider_id, input, output));
        }
    }

    hints
}

fn hash_password(password: &str) -> Result<String> {
    let salt = SaltString::generate(&mut thread_rng());
    let hash = Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map_err(|err| anyhow::anyhow!("password hashing failed: {}", err))?;
    Ok(hash.to_string())
}

fn verify_password(password: &str, password_hash: &str) -> bool {
    let parsed = match PasswordHash::new(password_hash) {
        Ok(value) => value,
        Err(_) => return false,
    };
    Argon2::default().verify_password(password.as_bytes(), &parsed).is_ok()
}

async fn load_avatar_image(source: &str) -> Result<image::DynamicImage> {
    if source.to_ascii_lowercase().starts_with("http://")
        || source.to_ascii_lowercase().starts_with("https://")
    {
        let bytes = reqwest::get(source).await?.bytes().await?;
        let image = image::load_from_memory(&bytes)?;
        return Ok(image);
    }

    let image = image::open(source)?;
    Ok(image)
}

fn render_ascii_preview(image: &image::DynamicImage, width: u32) -> String {
    let target_size = match NonZeroU32::new(width.max(16)) {
        Some(value) => value,
        None => return String::new(),
    };
    let config = artem::config::ConfigBuilder::new().target_size(target_size).build();
    artem::convert(image.clone(), &config)
}

fn render_pixelated_preview(image: &image::DynamicImage, width: u32, height: u32) -> String {
    let tiny = image.resize_exact(width.max(12), height.max(8), FilterType::Nearest).to_rgb8();
    let mut out = String::new();

    for y in 0..tiny.height() {
        for x in 0..tiny.width() {
            let pixel = tiny.get_pixel(x, y);
            out.push_str(&format!("\x1b[38;2;{};{};{}m██", pixel[0], pixel[1], pixel[2]));
        }
        out.push_str("\x1b[0m\n");
    }

    out
}

async fn ensure_turso_schema(conn: &libsql::Connection) -> Result<()> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS dx_users (
            id TEXT PRIMARY KEY,
            email TEXT NOT NULL UNIQUE,
            display_name TEXT NOT NULL,
            password_hash TEXT,
            auth_provider TEXT NOT NULL,
            oauth_subject TEXT,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        )",
        (),
    )
    .await?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS dx_onboarding_profiles (
            id TEXT PRIMARY KEY,
            user_email TEXT NOT NULL UNIQUE,
            payload_json TEXT NOT NULL,
            updated_at TEXT NOT NULL
        )",
        (),
    )
    .await?;

    Ok(())
}

fn now_id(prefix: &str) -> String {
    let ts = Local::now().timestamp_millis();
    let random = rand::random::<u32>();
    format!("{}-{}-{}", prefix, ts, random)
}

#[derive(Debug, Deserialize)]
struct OAuthUserProfile {
    id: Option<String>,
    sub: Option<String>,
    email: Option<String>,
    name: Option<String>,
    login: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GithubDeviceStart {
    device_code: String,
    user_code: String,
    verification_uri: String,
    verification_uri_complete: Option<String>,
    expires_in: u64,
    interval: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct GithubDeviceToken {
    access_token: Option<String>,
    error: Option<String>,
}

async fn oauth_sign_in(
    provider: &str,
    client_id: &str,
    client_secret: &str,
    auth_url: &str,
    token_url: &str,
    redirect_uri: &str,
    scopes: &[&str],
    user_info_url: &str,
) -> Result<AuthResult> {
    let oauth_client = BasicClient::new(
        ClientId::new(client_id.to_string()),
        Some(ClientSecret::new(client_secret.to_string())),
        AuthUrl::new(auth_url.to_string())?,
        Some(TokenUrl::new(token_url.to_string())?),
    )
    .set_redirect_uri(RedirectUrl::new(redirect_uri.to_string())?);

    let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();
    let mut auth_builder = oauth_client
        .authorize_url(CsrfToken::new_random)
        .set_pkce_challenge(pkce_challenge);
    for scope in scopes {
        auth_builder = auth_builder.add_scope(Scope::new((*scope).to_string()));
    }
    let (auth_uri, csrf_state) = auth_builder.url();

    let _ = webbrowser::open(auth_uri.as_ref());

    prompts::section_with_width("OAuth Browser Step", 78, |lines| {
        lines.push(format!("Provider: {}", provider));
        lines.push("Browser opened automatically. If not, open this URL manually:".to_string());
        push_wrapped(lines, auth_uri.as_ref(), 74);
        lines.push("After sign in, paste the full callback URL below.".to_string());
    })?;

    let callback_url = prompts::input::input("Paste callback URL")
        .placeholder("http://localhost:3000/api/auth/callback/provider?code=...&state=...")
        .interact()?;

    let parsed = Url::parse(&callback_url)?;
    let mut code_value: Option<String> = None;
    let mut state_value: Option<String> = None;
    for (key, value) in parsed.query_pairs() {
        if key == "code" {
            code_value = Some(value.to_string());
        }
        if key == "state" {
            state_value = Some(value.to_string());
        }
    }

    let code = match code_value {
        Some(value) => value,
        None => return Err(anyhow::anyhow!("Missing OAuth code in callback URL")),
    };
    let state = match state_value {
        Some(value) => value,
        None => return Err(anyhow::anyhow!("Missing OAuth state in callback URL")),
    };

    if state != *csrf_state.secret() {
        return Err(anyhow::anyhow!("OAuth state mismatch; authentication rejected"));
    }

    let token = oauth_client
        .exchange_code(AuthorizationCode::new(code))
        .set_pkce_verifier(pkce_verifier)
        .request_async(async_http_client)
        .await?;

    let access_token = token.access_token().secret().clone();
    let http_client = reqwest::Client::new();
    let profile = http_client
        .get(user_info_url)
        .bearer_auth(access_token)
        .send()
        .await?
        .json::<OAuthUserProfile>()
        .await?;

    let email = match profile.email {
        Some(value) if !value.trim().is_empty() => value,
        _ => format!("{}-user@dx.local", provider),
    };
    let name = profile.name.or(profile.login).unwrap_or_else(|| format!("{} user", provider));
    let oauth_subject = profile.id.or(profile.sub);

    Ok(AuthResult {
        method: provider.to_string(),
        email,
        name,
        oauth_subject,
    })
}

fn push_wrapped(lines: &mut Vec<String>, text: &str, width: usize) {
    for line in wrap(text, width) {
        lines.push(line.into_owned());
    }
}

async fn github_device_sign_in(client_id: &str) -> Result<AuthResult> {
    let http_client = reqwest::Client::new();
    let start = http_client
        .post("https://github.com/login/device/code")
        .header("accept", "application/json")
        .form(&[("client_id", client_id), ("scope", "read:user user:email")])
        .send()
        .await?
        .error_for_status()?
        .json::<GithubDeviceStart>()
        .await?;

    let open_url = start
        .verification_uri_complete
        .clone()
        .unwrap_or_else(|| start.verification_uri.clone());
    let _ = webbrowser::open(&open_url);

    prompts::section_with_width("GitHub Device Login", 78, |lines| {
        lines
            .push("GitHub sign in is using device code flow (no callback URL needed).".to_string());
        lines.push(format!("User code: {}", start.user_code));
        lines.push("Open this URL and enter the code:".to_string());
        push_wrapped(lines, &start.verification_uri, 74);
        if let Some(url) = &start.verification_uri_complete {
            lines.push("Or open direct URL:".to_string());
            push_wrapped(lines, url, 74);
        }
    })?;

    let interval = start.interval.unwrap_or(5);
    let deadline = std::time::Instant::now() + Duration::from_secs(start.expires_in);

    loop {
        if std::time::Instant::now() >= deadline {
            return Err(anyhow::anyhow!("GitHub device code expired before authorization"));
        }

        tokio::time::sleep(Duration::from_secs(interval)).await;

        let token = http_client
            .post("https://github.com/login/oauth/access_token")
            .header("accept", "application/json")
            .form(&[
                ("client_id", client_id),
                ("device_code", start.device_code.as_str()),
                ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
            ])
            .send()
            .await?
            .error_for_status()?
            .json::<GithubDeviceToken>()
            .await?;

        if let Some(access_token) = token.access_token {
            let profile = http_client
                .get("https://api.github.com/user")
                .header("User-Agent", "dx-onboard")
                .bearer_auth(&access_token)
                .send()
                .await?
                .error_for_status()?
                .json::<OAuthUserProfile>()
                .await?;

            let email = match profile.email {
                Some(value) if !value.trim().is_empty() => value,
                _ => {
                    let emails = http_client
                        .get("https://api.github.com/user/emails")
                        .header("User-Agent", "dx-onboard")
                        .bearer_auth(&access_token)
                        .send()
                        .await?
                        .error_for_status()?
                        .json::<Vec<serde_json::Value>>()
                        .await
                        .unwrap_or_default();

                    emails
                        .iter()
                        .find_map(|item| {
                            item.get("email").and_then(|x| x.as_str()).map(str::to_string)
                        })
                        .unwrap_or_else(|| "github-user@dx.local".to_string())
                }
            };

            return Ok(AuthResult {
                method: "github".to_string(),
                email,
                name: profile.name.or(profile.login).unwrap_or_else(|| "GitHub User".to_string()),
                oauth_subject: profile.id,
            });
        }

        match token.error.as_deref() {
            Some("authorization_pending") => continue,
            Some("slow_down") => {
                tokio::time::sleep(Duration::from_secs(interval + 2)).await;
                continue;
            }
            Some(other) => return Err(anyhow::anyhow!("GitHub device auth failed: {}", other)),
            None => continue,
        }
    }
}

fn provider_env_var(provider_id: &str) -> Option<&'static str> {
    match provider_id {
        "openai" => Some("OPENAI_API_KEY"),
        "anthropic" => Some("ANTHROPIC_API_KEY"),
        "google" => Some("GOOGLE_API_KEY"),
        "xai" => Some("XAI_API_KEY"),
        "mistral" => Some("MISTRAL_API_KEY"),
        "cohere" => Some("COHERE_API_KEY"),
        "groq" => Some("GROQ_API_KEY"),
        "openrouter" => Some("OPENROUTER_API_KEY"),
        "together" => Some("TOGETHER_API_KEY"),
        "perplexity" => Some("PERPLEXITY_API_KEY"),
        "deepinfra" => Some("DEEPINFRA_API_KEY"),
        "cerebras" => Some("CEREBRAS_API_KEY"),
        "github_copilot" | "github-copilot" => Some("GITHUB_COPILOT_TOKEN"),
        "google_antigravity" | "google-antigravity" => Some("GOOGLE_ANTIGRAVITY_API_KEY"),
        _ => None,
    }
}

fn canonical_provider_id(provider_id: &str) -> String {
    provider_id.replace('_', "-")
}

const ANTIGRAVITY_CLIENT_ID: &str =
    "1071006060591-tmhssin2h21lcre235vtolojh4g403ep.apps.googleusercontent.com";
const ANTIGRAVITY_CLIENT_SECRET: &str = "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";
const ANTIGRAVITY_DEFAULT_PROJECT_ID: &str = "rising-fact-p41fc";
const ANTIGRAVITY_ENDPOINT_DAILY: &str = "https://daily-cloudcode-pa.sandbox.googleapis.com";
const ANTIGRAVITY_ENDPOINT_AUTOPUSH: &str = "https://autopush-cloudcode-pa.sandbox.googleapis.com";
const ANTIGRAVITY_ENDPOINT_PROD: &str = "https://cloudcode-pa.googleapis.com";

#[derive(Debug, Clone)]
struct AntigravityAuthContext {
    access_token: String,
    project_id: Option<String>,
    source: String,
}

fn parse_antigravity_refresh_parts(raw: &str) -> (String, Option<String>) {
    let mut parts = raw.split('|');
    let refresh_token = parts.next().unwrap_or_default().trim().to_string();
    let project_id = parts
        .next()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string);
    (refresh_token, project_id)
}

fn antigravity_endpoints_for_connectivity() -> Vec<String> {
    let mut endpoints = Vec::new();
    if let Ok(base) = env::var("GOOGLE_ANTIGRAVITY_BASE_URL") {
        let trimmed = base.trim().trim_end_matches('/');
        if !trimmed.is_empty() {
            endpoints.push(trimmed.to_string());
        }
    }
    endpoints.extend([
        ANTIGRAVITY_ENDPOINT_PROD.to_string(),
        ANTIGRAVITY_ENDPOINT_DAILY.to_string(),
        ANTIGRAVITY_ENDPOINT_AUTOPUSH.to_string(),
    ]);
    endpoints.sort();
    endpoints.dedup();
    endpoints
}

fn antigravity_endpoints_for_requests() -> Vec<String> {
    let mut endpoints = Vec::new();
    if let Ok(base) = env::var("GOOGLE_ANTIGRAVITY_BASE_URL") {
        let trimmed = base.trim().trim_end_matches('/');
        if !trimmed.is_empty() {
            endpoints.push(trimmed.to_string());
        }
    }
    endpoints.extend([
        ANTIGRAVITY_ENDPOINT_DAILY.to_string(),
        ANTIGRAVITY_ENDPOINT_AUTOPUSH.to_string(),
        ANTIGRAVITY_ENDPOINT_PROD.to_string(),
    ]);
    endpoints.sort();
    endpoints.dedup();
    endpoints
}

fn antigravity_static_models() -> Vec<String> {
    vec![
        "antigravity-gemini-3-pro".to_string(),
        "antigravity-gemini-3-flash".to_string(),
        "antigravity-claude-sonnet-4-5".to_string(),
        "antigravity-claude-sonnet-4-5-thinking".to_string(),
        "antigravity-claude-opus-4-5-thinking".to_string(),
        "antigravity-claude-opus-4-6-thinking".to_string(),
        "gemini-2.5-flash".to_string(),
        "gemini-2.5-pro".to_string(),
        "gemini-3-flash-preview".to_string(),
        "gemini-3-pro-preview".to_string(),
    ]
}

async fn refresh_antigravity_access_token(refresh_token: &str) -> Result<String> {
    let client = reqwest::Client::new();
    let response = client
        .post("https://oauth2.googleapis.com/token")
        .header("Content-Type", "application/x-www-form-urlencoded")
        .form(&[
            ("grant_type", "refresh_token"),
            ("refresh_token", refresh_token),
            ("client_id", ANTIGRAVITY_CLIENT_ID),
            ("client_secret", ANTIGRAVITY_CLIENT_SECRET),
        ])
        .send()
        .await?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(anyhow::anyhow!("token refresh failed ({status}): {body}"));
    }

    let payload: Value = response.json().await?;
    let access_token = payload
        .get("access_token")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| anyhow::anyhow!("token refresh response missing access_token"))?;

    Ok(access_token.to_string())
}

async fn resolve_antigravity_auth(api_key: Option<&str>) -> Result<AntigravityAuthContext> {
    if let Ok(token) = env::var("GOOGLE_ANTIGRAVITY_ACCESS_TOKEN") {
        let token = token.trim();
        if !token.is_empty() {
            return Ok(AntigravityAuthContext {
                access_token: token.to_string(),
                project_id: env::var("GOOGLE_ANTIGRAVITY_PROJECT_ID")
                    .ok()
                    .map(|value| value.trim().to_string())
                    .filter(|value| !value.is_empty()),
                source: "GOOGLE_ANTIGRAVITY_ACCESS_TOKEN".to_string(),
            });
        }
    }

    if let Some(raw) = api_key {
        let trimmed = raw.trim();
        if !trimmed.is_empty() {
            if trimmed.starts_with("ya29.") {
                return Ok(AntigravityAuthContext {
                    access_token: trimmed.to_string(),
                    project_id: env::var("GOOGLE_ANTIGRAVITY_PROJECT_ID")
                        .ok()
                        .map(|value| value.trim().to_string())
                        .filter(|value| !value.is_empty()),
                    source: "GOOGLE_ANTIGRAVITY_API_KEY(access_token)".to_string(),
                });
            }

            let (refresh_token, project_id) = parse_antigravity_refresh_parts(trimmed);
            if !refresh_token.is_empty() {
                let access_token = refresh_antigravity_access_token(&refresh_token).await?;
                return Ok(AntigravityAuthContext {
                    access_token,
                    project_id: project_id.or_else(|| {
                        env::var("GOOGLE_ANTIGRAVITY_PROJECT_ID")
                            .ok()
                            .map(|value| value.trim().to_string())
                            .filter(|value| !value.is_empty())
                    }),
                    source: "GOOGLE_ANTIGRAVITY_API_KEY(refresh_token)".to_string(),
                });
            }
        }
    }

    if let Ok(raw) = env::var("GOOGLE_ANTIGRAVITY_API_KEY") {
        let trimmed = raw.trim();
        if !trimmed.is_empty() {
            if trimmed.starts_with("ya29.") {
                return Ok(AntigravityAuthContext {
                    access_token: trimmed.to_string(),
                    project_id: env::var("GOOGLE_ANTIGRAVITY_PROJECT_ID")
                        .ok()
                        .map(|value| value.trim().to_string())
                        .filter(|value| !value.is_empty()),
                    source: "GOOGLE_ANTIGRAVITY_API_KEY(access_token)".to_string(),
                });
            }

            let (refresh_token, project_id) = parse_antigravity_refresh_parts(trimmed);
            if !refresh_token.is_empty() {
                let access_token = refresh_antigravity_access_token(&refresh_token).await?;
                return Ok(AntigravityAuthContext {
                    access_token,
                    project_id: project_id.or_else(|| {
                        env::var("GOOGLE_ANTIGRAVITY_PROJECT_ID")
                            .ok()
                            .map(|value| value.trim().to_string())
                            .filter(|value| !value.is_empty())
                    }),
                    source: "GOOGLE_ANTIGRAVITY_API_KEY(refresh_token)".to_string(),
                });
            }
        }
    }

    if let Ok(raw) = env::var("GOOGLE_ANTIGRAVITY_REFRESH_TOKEN") {
        let (refresh_token, project_id) = parse_antigravity_refresh_parts(raw.trim());
        if !refresh_token.is_empty() {
            let access_token = refresh_antigravity_access_token(&refresh_token).await?;
            return Ok(AntigravityAuthContext {
                access_token,
                project_id: project_id.or_else(|| {
                    env::var("GOOGLE_ANTIGRAVITY_PROJECT_ID")
                        .ok()
                        .map(|value| value.trim().to_string())
                        .filter(|value| !value.is_empty())
                }),
                source: "GOOGLE_ANTIGRAVITY_REFRESH_TOKEN".to_string(),
            });
        }
    }

    let root = find_workspace_root();
    for file_name in [
        "antigravity_access_token.txt",
        ".antigravity_access_token",
        "google_antigravity_access_token.txt",
    ] {
        let path = root.join(file_name);
        if path.exists()
            && let Ok(content) = fs::read_to_string(&path)
        {
            let token = content.trim();
            if !token.is_empty() {
                return Ok(AntigravityAuthContext {
                    access_token: token.to_string(),
                    project_id: env::var("GOOGLE_ANTIGRAVITY_PROJECT_ID")
                        .ok()
                        .map(|value| value.trim().to_string())
                        .filter(|value| !value.is_empty()),
                    source: format!("{}", path.display()),
                });
            }
        }
    }

    Err(anyhow::anyhow!(
        "missing AntiGravity auth: set GOOGLE_ANTIGRAVITY_ACCESS_TOKEN, or GOOGLE_ANTIGRAVITY_REFRESH_TOKEN, or GOOGLE_ANTIGRAVITY_API_KEY (access token or refresh_token|project_id)"
    ))
}

async fn antigravity_load_code_assist(
    access_token: &str,
    project_id_hint: Option<&str>,
) -> Result<Option<String>> {
    let client = reqwest::Client::builder().timeout(Duration::from_secs(10)).build()?;

    let metadata_platform = if cfg!(target_os = "windows") {
        "WINDOWS"
    } else {
        "MACOS"
    };
    let client_metadata = format!(
        "{{\"ideType\":\"ANTIGRAVITY\",\"platform\":\"{}\",\"pluginType\":\"GEMINI\"}}",
        metadata_platform
    );

    let mut last_error: Option<anyhow::Error> = None;
    for base in antigravity_endpoints_for_connectivity() {
        let mut body = json!({
            "metadata": {
                "ideType": "ANTIGRAVITY",
                "platform": metadata_platform,
                "pluginType": "GEMINI",
            }
        });
        if let Some(project_id) = project_id_hint
            && !project_id.trim().is_empty()
        {
            body["metadata"]["duetProject"] = Value::String(project_id.trim().to_string());
        }

        let resp = client
            .post(format!("{}/v1internal:loadCodeAssist", base))
            .header("Authorization", format!("Bearer {}", access_token))
            .header("Content-Type", "application/json")
            .header("User-Agent", "google-api-nodejs-client/9.15.1")
            .header("Client-Metadata", client_metadata.clone())
            .json(&body)
            .send()
            .await;

        match resp {
            Ok(response) if response.status().is_success() => {
                let payload: Value = response.json().await.unwrap_or_else(|_| json!({}));
                let project = payload
                    .get("cloudaicompanionProject")
                    .and_then(Value::as_str)
                    .map(str::to_string)
                    .or_else(|| {
                        payload
                            .get("cloudaicompanionProject")
                            .and_then(Value::as_object)
                            .and_then(|value| value.get("id"))
                            .and_then(Value::as_str)
                            .map(str::to_string)
                    });
                return Ok(project);
            }
            Ok(response) => {
                let status = response.status();
                let body = response.text().await.unwrap_or_default();
                last_error =
                    Some(anyhow::anyhow!("loadCodeAssist {} on {}: {}", status, base, body));
            }
            Err(err) => {
                last_error = Some(anyhow::anyhow!("loadCodeAssist error on {}: {}", base, err));
            }
        }
    }

    if let Some(err) = last_error {
        return Err(err);
    }

    Ok(None)
}

fn extract_antigravity_probe_text(payload: &Value) -> Option<String> {
    payload
        .get("response")
        .and_then(|value| value.get("candidates"))
        .and_then(Value::as_array)
        .and_then(|candidates| candidates.first())
        .and_then(|candidate| candidate.get("content"))
        .and_then(|content| content.get("parts"))
        .and_then(Value::as_array)
        .and_then(|parts| {
            let mut merged = String::new();
            for part in parts {
                if let Some(text) = part.get("text").and_then(Value::as_str)
                    && !text.trim().is_empty()
                {
                    if !merged.is_empty() {
                        merged.push('\n');
                    }
                    merged.push_str(text);
                }
            }
            if merged.is_empty() {
                None
            } else {
                Some(merged)
            }
        })
}

fn provider_is_local_no_key(provider_id: &str) -> bool {
    matches!(
        provider_id,
        "ollama" | "lmstudio" | "vllm" | "local_llama_cpp" | "jan" | "anythingllm"
    )
}

fn resolve_provider_api_key_env(provider_id: &str) -> Option<String> {
    if provider_is_local_no_key(provider_id) {
        return None;
    }

    if let Some(preset) = openai_compatible_provider_presets()
        .into_iter()
        .find(|preset| preset.id == provider_id || preset.id.replace('-', "_") == provider_id)
        && !preset.api_key_env.trim().is_empty()
    {
        return Some(preset.api_key_env.to_string());
    }

    if let Some(var_name) = provider_env_var(provider_id) {
        return Some(var_name.to_string());
    }

    Some(format!("{}_API_KEY", provider_id.to_ascii_uppercase().replace('-', "_")))
}

fn upsert_provider_env(provider_id: &str, api_key_env: &str, api_key: &str) {
    // SAFETY: Setting environment variable for provider API key
    unsafe {
        env::set_var(api_key_env, api_key);
    }
    if provider_id == "google" {
        // SAFETY: Setting environment variables for Google/Gemini provider compatibility
        unsafe {
            env::set_var("GOOGLE_API_KEY", api_key);
            env::set_var("GEMINI_API_KEY", api_key);
        }
    }
}

fn keyring_service_name() -> &'static str {
    "dx-onboard"
}

fn provider_profile_key(provider_id: &str, profile: &str) -> String {
    format!("{}:{}", provider_id, profile)
}

fn read_keyring_api_key(provider_id: &str, profile: &str) -> Option<String> {
    let entry =
        keyring::Entry::new(keyring_service_name(), &provider_profile_key(provider_id, profile))
            .ok()?;
    let value = entry.get_password().ok()?;
    if value.trim().is_empty() {
        None
    } else {
        Some(value)
    }
}

fn store_keyring_api_key(provider_id: &str, profile: &str, api_key: &str) -> Result<()> {
    let entry =
        keyring::Entry::new(keyring_service_name(), &provider_profile_key(provider_id, profile))
            .map_err(|err| anyhow::anyhow!("failed to initialize keyring entry: {}", err))?;
    entry
        .set_password(api_key)
        .map_err(|err| anyhow::anyhow!("failed to store key in keyring: {}", err))?;
    Ok(())
}

fn prompt_provider_api_key(
    provider_id: &str,
    profile: &str,
    api_key_env: Option<&str>,
) -> Result<Option<String>> {
    let Some(api_key_env) = api_key_env else {
        return Ok(None);
    };

    // Special handling for GitHub Copilot - token is short-lived; use only the freshly
    // bootstrapped `GITHUB_COPILOT_TOKEN` and never reuse a persisted keyring entry.
    if provider_id == "github_copilot" || provider_id == "github-copilot" {
        if let Ok(existing) = env::var(api_key_env)
            && !existing.trim().is_empty()
        {
            return Ok(Some(existing));
        }

        // Never prompt the user for a Copilot token here.
        // Copilot expects a short-lived *service token* (not a PAT), which should come
        // from the HTTPS bootstrap flow. If bootstrap fails, keep going without Copilot.
        return Ok(None);
    }

    // OpenCode free models intentionally work without an API key.
    // If the user has set `OPENCODE_API_KEY`, use it (to unlock paid models), but never prompt.
    if provider_id == "opencode" {
        if let Ok(existing) = env::var(api_key_env)
            && !existing.trim().is_empty()
        {
            return Ok(Some(existing));
        }
        return Ok(None);
    }

    // For all other providers, allow keyring reuse.
    if provider_id != "github_copilot" && provider_id != "github-copilot" {
        if let Some(keyring_value) = read_keyring_api_key(provider_id, profile) {
            let keep_keyring = prompts::confirm(format!(
                "Use secure keyring key for {} profile {}?",
                provider_id, profile
            ))
            .initial_value(true)
            .interact()?;
            if keep_keyring {
                return Ok(Some(keyring_value));
            }
        }
    }

    if let Ok(existing) = env::var(api_key_env)
        && !existing.trim().is_empty()
    {
        let keep_existing =
            prompts::confirm(format!("Use existing {} from {}?", provider_id, api_key_env))
                .initial_value(true)
                .interact()?;
        if keep_existing {
            return Ok(Some(existing));
        }
    }

    let key = prompts::password::password(format!("API key for {} ({})", provider_id, api_key_env))
        .interact()?;
    if key.trim().is_empty() {
        return Ok(None);
    }

    Ok(Some(key.to_string()))
}

fn select_provider_profile(_provider_id: &str) -> Result<String> {
    // Auto-select default profile for all providers
    Ok("default".to_string())

    // let custom =
    //     prompts::confirm(format!("Use a custom profile name for provider {}?", provider_id))
    //         .initial_value(false)
    //         .interact()?;

    // if !custom {
    //     return Ok("default".to_string());
    // }

    // let profile = prompts::input::input(format!("Profile name for {}", provider_id))
    //     .placeholder("default")
    //     .interact()?;
    // let trimmed = profile.trim();
    // if trimmed.is_empty() {
    //     Ok("default".to_string())
    // } else {
    //     Ok(trimmed.to_string())
    // }
}

async fn fetch_models_for_provider(
    provider_id: &str,
    api_key: Option<&str>,
    registry: &ProviderRegistry,
) -> ProviderModelListing {
    let canonical_id = canonical_provider_id(provider_id);
    let is_github_copilot = matches!(provider_id, "github_copilot" | "github-copilot")
        || matches!(canonical_id.as_str(), "github-copilot" | "github_copilot");

    if provider_id == "google_antigravity" || provider_id == "google-antigravity" {
        return fetch_google_antigravity_models(api_key).await;
    }

    if is_github_copilot
        && let Some(provider) =
            registry.get("github_copilot").or_else(|| registry.get("github-copilot"))
    {
        return match provider.get_models().await {
            Ok(models) => {
                let unique = models
                    .into_iter()
                    .map(|model| model.id)
                    .filter(|id| !id.trim().is_empty())
                    .collect::<BTreeSet<_>>()
                    .into_iter()
                    .collect::<Vec<_>>();

                ProviderModelListing {
                    provider_id: provider_id.to_string(),
                    models: unique,
                    modules: Vec::new(),
                    status: "ok".to_string(),
                    detail: "fetched model list".to_string(),
                }
            }
            Err(err) => ProviderModelListing {
                provider_id: provider_id.to_string(),
                models: Vec::new(),
                modules: Vec::new(),
                status: "error".to_string(),
                detail: err.to_string(),
            },
        };
    }

    if let Some(api_key) = api_key
        && let Some(preset) = openai_compatible_provider_presets().into_iter().find(|preset| {
            preset.id == provider_id
                || preset.id == canonical_id
                || preset.id.replace('-', "_") == provider_id
        })
        && !preset.base_url.contains('{')
    {
        let provider = GenericProvider::new(provider_id, preset.base_url, api_key);
        match provider.get_models().await {
            Ok(models) => {
                let unique = models
                    .into_iter()
                    .map(|model| model.id)
                    .filter(|id| !id.trim().is_empty())
                    .collect::<BTreeSet<_>>()
                    .into_iter()
                    .collect::<Vec<_>>();

                return ProviderModelListing {
                    provider_id: provider_id.to_string(),
                    models: unique,
                    modules: Vec::new(),
                    status: "ok".to_string(),
                    detail: "fetched model list".to_string(),
                };
            }
            Err(err) => {
                return ProviderModelListing {
                    provider_id: provider_id.to_string(),
                    models: Vec::new(),
                    modules: Vec::new(),
                    status: "error".to_string(),
                    detail: err.to_string(),
                };
            }
        }
    }

    if let Some(provider) = registry.get(provider_id).or_else(|| registry.get(&canonical_id)) {
        return match provider.get_models().await {
            Ok(models) => {
                let unique = models
                    .into_iter()
                    .map(|model| model.id)
                    .filter(|id| !id.trim().is_empty())
                    .collect::<BTreeSet<_>>()
                    .into_iter()
                    .collect::<Vec<_>>();

                ProviderModelListing {
                    provider_id: provider_id.to_string(),
                    models: unique,
                    modules: Vec::new(),
                    status: "ok".to_string(),
                    detail: "fetched model list".to_string(),
                }
            }
            Err(err) => ProviderModelListing {
                provider_id: provider_id.to_string(),
                models: Vec::new(),
                modules: Vec::new(),
                status: "error".to_string(),
                detail: err.to_string(),
            },
        };
    }

    let Some(api_key) = api_key else {
        return ProviderModelListing {
            provider_id: provider_id.to_string(),
            models: Vec::new(),
            modules: Vec::new(),
            status: "missing_key".to_string(),
            detail: "missing API key for model listing".to_string(),
        };
    };

    let Some(preset) = openai_compatible_provider_presets().into_iter().find(|preset| {
        preset.id == provider_id
            || preset.id == canonical_id
            || preset.id.replace('-', "_") == provider_id
    }) else {
        return ProviderModelListing {
            provider_id: provider_id.to_string(),
            models: Vec::new(),
            modules: Vec::new(),
            status: "unsupported".to_string(),
            detail: "provider model listing not supported yet".to_string(),
        };
    };

    if preset.base_url.contains('{') {
        return ProviderModelListing {
            provider_id: provider_id.to_string(),
            models: Vec::new(),
            modules: Vec::new(),
            status: "unsupported".to_string(),
            detail: "provider requires custom enterprise URL configuration".to_string(),
        };
    }

    let provider = GenericProvider::new(provider_id, preset.base_url, api_key);
    match provider.get_models().await {
        Ok(models) => {
            let unique = models
                .into_iter()
                .map(|model| model.id)
                .filter(|id| !id.trim().is_empty())
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect::<Vec<_>>();

            ProviderModelListing {
                provider_id: provider_id.to_string(),
                models: unique,
                modules: Vec::new(),
                status: "ok".to_string(),
                detail: "fetched model list".to_string(),
            }
        }
        Err(err) => ProviderModelListing {
            provider_id: provider_id.to_string(),
            models: Vec::new(),
            modules: Vec::new(),
            status: "error".to_string(),
            detail: err.to_string(),
        },
    }
}

async fn fetch_google_antigravity_models(api_key: Option<&str>) -> ProviderModelListing {
    let auth = match resolve_antigravity_auth(api_key).await {
        Ok(value) => value,
        Err(err) => {
            return ProviderModelListing {
                provider_id: "google_antigravity".to_string(),
                models: Vec::new(),
                modules: Vec::new(),
                status: "missing_key".to_string(),
                detail: err.to_string(),
            };
        }
    };

    let discovered_project =
        antigravity_load_code_assist(&auth.access_token, auth.project_id.as_deref())
            .await
            .ok()
            .flatten();

    let mut models = antigravity_static_models();
    models.sort();
    models.dedup();

    let mut modules = antigravity_endpoints_for_requests();
    modules.sort();
    modules.dedup();

    let effective_project = discovered_project
        .or(auth.project_id)
        .unwrap_or_else(|| ANTIGRAVITY_DEFAULT_PROJECT_ID.to_string());

    ProviderModelListing {
        provider_id: "google_antigravity".to_string(),
        models,
        modules,
        status: "ok".to_string(),
        detail: format!("resolved auth via {}; project={}", auth.source, effective_project),
    }
}

fn sample_chat_request(model: &str) -> ChatRequest {
    ChatRequest {
        model: model.to_string(),
        messages: vec![ChatMessage {
            role: "user".to_string(),
            content: MessageContent::Text(
                "hello".to_string(),
            ),
            name: None,
        }],
        temperature: None,
        max_tokens: Some(512),
        top_p: None,
        stop: None,
        tools: None,
        tool_choice: None,
        stream: false,
        extra: None,
    }
}

async fn run_hello_probe(
    provider_id: &str,
    selected_model: &str,
    api_key: Option<&str>,
    registry: &ProviderRegistry,
) -> ProviderModelProbeResult {
    let canonical_id = canonical_provider_id(provider_id);

    if provider_id == "google_antigravity" || provider_id == "google-antigravity" {
        let auth = match resolve_antigravity_auth(api_key).await {
            Ok(value) => value,
            Err(err) => {
                return ProviderModelProbeResult {
                    provider_id: provider_id.to_string(),
                    api_key_env: resolve_provider_api_key_env(provider_id),
                    selected_model: Some(selected_model.to_string()),
                    status: "missing_key".to_string(),
                    detail: err.to_string(),
                    response_preview: None,
                };
            }
        };

        let metadata_platform = if cfg!(target_os = "windows") {
            "WINDOWS"
        } else {
            "MACOS"
        };
        let client_metadata = format!(
            "{{\"ideType\":\"ANTIGRAVITY\",\"platform\":\"{}\",\"pluginType\":\"GEMINI\"}}",
            metadata_platform
        );

        let discovered_project =
            antigravity_load_code_assist(&auth.access_token, auth.project_id.as_deref())
                .await
                .ok()
                .flatten();

        let project_id = discovered_project
            .or(auth.project_id.clone())
            .unwrap_or_else(|| ANTIGRAVITY_DEFAULT_PROJECT_ID.to_string());

        let body = json!({
            "project": project_id,
            "model": selected_model,
            "request": {
                "contents": [{
                    "role": "user",
                    "parts": [{
                        "text": "Write one long paragraph explaining why Rust can be better than Node.js for production systems, including memory safety, predictable performance, concurrency, and operational reliability. Include concrete technical reasoning and trade-offs."
                    }]
                }],
                "generationConfig": {
                    "maxOutputTokens": 512
                }
            },
            "requestType": "agent",
            "userAgent": "antigravity",
            "requestId": format!("agent-{}", chrono::Utc::now().timestamp_millis()),
        });

        let use_gemini_cli_headers = selected_model.starts_with("gemini-");

        let mut last_error = String::new();
        let client = reqwest::Client::new();
        for base in antigravity_endpoints_for_requests() {
            let mut request = client
                .post(format!("{}/v1internal:generateContent", base))
                .header("Authorization", format!("Bearer {}", auth.access_token))
                .header("Content-Type", "application/json");

            if use_gemini_cli_headers {
                request = request
                    .header("User-Agent", "google-api-nodejs-client/9.15.1")
                    .header("X-Goog-Api-Client", "gl-node/22.17.0")
                    .header(
                        "Client-Metadata",
                        "ideType=IDE_UNSPECIFIED,platform=PLATFORM_UNSPECIFIED,pluginType=GEMINI",
                    );
            } else {
                request = request
                    .header(
                        "User-Agent",
                        "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Antigravity/1.15.8 Chrome/138.0.7204.235 Electron/37.3.1 Safari/537.36",
                    )
                    .header("X-Goog-Api-Client", "google-cloud-sdk vscode_cloudshelleditor/0.1")
                    .header("Client-Metadata", client_metadata.clone());
            }

            let response = request.json(&body).send().await;
            match response {
                Ok(resp) if resp.status().is_success() => {
                    let payload: Value = resp.json().await.unwrap_or_else(|_| json!({}));
                    let preview = extract_antigravity_probe_text(&payload)
                        .or_else(|| Some(payload.to_string()));
                    return ProviderModelProbeResult {
                        provider_id: provider_id.to_string(),
                        api_key_env: resolve_provider_api_key_env(provider_id),
                        selected_model: Some(selected_model.to_string()),
                        status: "ok".to_string(),
                        detail: format!("probe succeeded via {} ({})", base, auth.source),
                        response_preview: preview,
                    };
                }
                Ok(resp) => {
                    let status = resp.status();
                    let body = resp.text().await.unwrap_or_default();
                    last_error = format!("{} {}", status, body);
                }
                Err(err) => {
                    last_error = err.to_string();
                }
            }
        }

        return ProviderModelProbeResult {
            provider_id: provider_id.to_string(),
            api_key_env: resolve_provider_api_key_env(provider_id),
            selected_model: Some(selected_model.to_string()),
            status: "error".to_string(),
            detail: format!("AntiGravity probe failed: {}", last_error),
            response_preview: None,
        };
    }

    if let Some(provider) = registry.get(provider_id).or_else(|| registry.get(&canonical_id)) {
        return match provider.chat(sample_chat_request(selected_model)).await {
            Ok(response) => ProviderModelProbeResult {
                provider_id: provider_id.to_string(),
                api_key_env: resolve_provider_api_key_env(provider_id),
                selected_model: Some(selected_model.to_string()),
                status: "ok".to_string(),
                detail: "long-form probe succeeded".to_string(),
                response_preview: Some(response.content),
            },
            Err(err) => ProviderModelProbeResult {
                provider_id: provider_id.to_string(),
                api_key_env: resolve_provider_api_key_env(provider_id),
                selected_model: Some(selected_model.to_string()),
                status: "error".to_string(),
                detail: err.to_string(),
                response_preview: None,
            },
        };
    }

    let Some(api_key) = api_key else {
        return ProviderModelProbeResult {
            provider_id: provider_id.to_string(),
            api_key_env: resolve_provider_api_key_env(provider_id),
            selected_model: Some(selected_model.to_string()),
            status: "missing_key".to_string(),
            detail: "missing API key for long-form probe".to_string(),
            response_preview: None,
        };
    };

    let Some(preset) = openai_compatible_provider_presets().into_iter().find(|preset| {
        preset.id == provider_id
            || preset.id == canonical_id
            || preset.id.replace('-', "_") == provider_id
    }) else {
        return ProviderModelProbeResult {
            provider_id: provider_id.to_string(),
            api_key_env: resolve_provider_api_key_env(provider_id),
            selected_model: Some(selected_model.to_string()),
            status: "unsupported".to_string(),
            detail: "provider long-form probe not supported yet".to_string(),
            response_preview: None,
        };
    };

    if preset.base_url.contains('{') {
        return ProviderModelProbeResult {
            provider_id: provider_id.to_string(),
            api_key_env: resolve_provider_api_key_env(provider_id),
            selected_model: Some(selected_model.to_string()),
            status: "unsupported".to_string(),
            detail: "provider requires custom enterprise URL configuration".to_string(),
            response_preview: None,
        };
    }

    let provider = GenericProvider::new(provider_id, preset.base_url, api_key);
    match provider.chat(sample_chat_request(selected_model)).await {
        Ok(response) => ProviderModelProbeResult {
            provider_id: provider_id.to_string(),
            api_key_env: resolve_provider_api_key_env(provider_id),
            selected_model: Some(selected_model.to_string()),
            status: "ok".to_string(),
            detail: "long-form probe succeeded".to_string(),
            response_preview: Some(response.content),
        },
        Err(err) => ProviderModelProbeResult {
            provider_id: provider_id.to_string(),
            api_key_env: resolve_provider_api_key_env(provider_id),
            selected_model: Some(selected_model.to_string()),
            status: "error".to_string(),
            detail: err.to_string(),
            response_preview: None,
        },
    }
}

async fn check_provider_connection(provider_id: &str) -> ProviderConnectionStatus {
    let client = reqwest::Client::builder().timeout(Duration::from_secs(10)).build();

    let client = match client {
        Ok(value) => value,
        Err(err) => {
            return ProviderConnectionStatus {
                provider_id: provider_id.to_string(),
                status: "error".to_string(),
                detail: format!("http client init failed: {}", err),
            };
        }
    };

    if provider_id == "google" {
        let api_key = match env::var("GOOGLE_API_KEY") {
            Ok(v) if !v.trim().is_empty() => v,
            _ => {
                return ProviderConnectionStatus {
                    provider_id: provider_id.to_string(),
                    status: "missing_key".to_string(),
                    detail: "missing GOOGLE_API_KEY".to_string(),
                };
            }
        };

        let url =
            format!("https://generativelanguage.googleapis.com/v1beta/models?key={}", api_key);
        return match client.get(url).send().await {
            Ok(resp) if resp.status().is_success() => ProviderConnectionStatus {
                provider_id: provider_id.to_string(),
                status: "ok".to_string(),
                detail: "reachable".to_string(),
            },
            Ok(resp) => ProviderConnectionStatus {
                provider_id: provider_id.to_string(),
                status: "error".to_string(),
                detail: format!("http {}", resp.status()),
            },
            Err(err) => ProviderConnectionStatus {
                provider_id: provider_id.to_string(),
                status: "error".to_string(),
                detail: err.to_string(),
            },
        };
    }

    if provider_id == "google_antigravity" || provider_id == "google-antigravity" {
        let auth = match resolve_antigravity_auth(None).await {
            Ok(value) => value,
            Err(err) => {
                return ProviderConnectionStatus {
                    provider_id: provider_id.to_string(),
                    status: "missing_key".to_string(),
                    detail: err.to_string(),
                };
            }
        };

        return match antigravity_load_code_assist(&auth.access_token, auth.project_id.as_deref())
            .await
        {
            Ok(project) => ProviderConnectionStatus {
                provider_id: provider_id.to_string(),
                status: "ok".to_string(),
                detail: format!(
                    "loadCodeAssist reachable (project={}) via {}",
                    project.unwrap_or_else(|| ANTIGRAVITY_DEFAULT_PROJECT_ID.to_string()),
                    auth.source
                ),
            },
            Err(err) => ProviderConnectionStatus {
                provider_id: provider_id.to_string(),
                status: "error".to_string(),
                detail: err.to_string(),
            },
        };
    }

    let api_key_var = match provider_env_var(provider_id) {
        Some(v) => v,
        None => {
            return ProviderConnectionStatus {
                provider_id: provider_id.to_string(),
                status: "unverified".to_string(),
                detail: "no built-in probe for this provider yet".to_string(),
            };
        }
    };
    let api_key = match env::var(api_key_var) {
        Ok(v) if !v.trim().is_empty() => v,
        _ => {
            return ProviderConnectionStatus {
                provider_id: provider_id.to_string(),
                status: "missing_key".to_string(),
                detail: format!("missing {}", api_key_var),
            };
        }
    };

    let endpoint = match provider_id {
        "openai" => "https://api.openai.com/v1/models",
        "anthropic" => "https://api.anthropic.com/v1/models",
        "xai" => "https://api.x.ai/v1/models",
        "mistral" => "https://api.mistral.ai/v1/models",
        "cohere" => "https://api.cohere.com/v1/models",
        "groq" => "https://api.groq.com/openai/v1/models",
        "openrouter" => "https://openrouter.ai/api/v1/models",
        "together" => "https://api.together.xyz/v1/models",
        "perplexity" => "https://api.perplexity.ai/models",
        "deepinfra" => "https://api.deepinfra.com/v1/openai/models",
        "cerebras" => "https://api.cerebras.ai/v1/models",
        "github_copilot" | "github-copilot" => "https://api.githubcopilot.com/models",
        _ => "",
    };

    let mut request = client.get(endpoint).bearer_auth(api_key);
    if provider_id == "anthropic" {
        request = request
            .header("x-api-key", env::var("ANTHROPIC_API_KEY").unwrap_or_default())
            .header("anthropic-version", "2023-06-01");
    }
    if provider_id == "github_copilot" || provider_id == "github-copilot" {
        request = request
            .header("editor-version", "DX-CLI/1.0.0")
            .header("copilot-integration-id", "dx-cli")
            .header("accept", "application/json");
    }

    match request.send().await {
        Ok(resp) if resp.status().is_success() => ProviderConnectionStatus {
            provider_id: provider_id.to_string(),
            status: "ok".to_string(),
            detail: "reachable".to_string(),
        },
        Ok(resp) => ProviderConnectionStatus {
            provider_id: provider_id.to_string(),
            status: "error".to_string(),
            detail: format!("http {}", resp.status()),
        },
        Err(err) => ProviderConnectionStatus {
            provider_id: provider_id.to_string(),
            status: "error".to_string(),
            detail: err.to_string(),
        },
    }
}

async fn run_dx_onboarding_flow(_args: OnboardCliArgs) -> Result<()> {
    let workspace_root = find_workspace_root();
    load_workspace_env(&workspace_root);
    ensure_provider_env_aliases();

    let detected_env = detect_runtime_environment();
    let install_mode = match detected_env {
        RuntimeEnvironment::RealOs => "full_local".to_string(),
        RuntimeEnvironment::Vps => "vps_remote".to_string(),
        RuntimeEnvironment::Container => "container_remote".to_string(),
        RuntimeEnvironment::Restricted => "restricted_remote".to_string(),
    };
    let installed_components = build_component_targets(detected_env);

    prompts::section_with_width("DX Onboarding", 96, |lines| {
        lines.push("Environment-aware onboarding + auth + provider/channel setup".to_string());
        lines.push(format!("Detected runtime: {}", detected_env.label()));
        lines.push(format!("Runtime hint: {}", detected_env.hint()));
        lines.push("".to_string());
        lines.push(
            "Component install policy: real OS => 5 components, VPS/Container => 2 components"
                .to_string(),
        );
        lines.push("Auth methods: email/password, GitHub OAuth, Google OAuth".to_string());
        lines.push("Avatar preview: pixelated ANSI + ASCII art".to_string());
    })?;

    // ── Component download spinners (commented out: providers-only mode) ──
    // for component in &installed_components {
    //     let mut spinner = prompts::spinner::spinner(format!("Downloading {}", component));
    //     spinner.start()?;
    //     std::thread::sleep(Duration::from_millis(220));
    //     spinner.stop(format!("{} ready", component))?;
    // }

    // ── Turso DB (commented out: providers-only mode) ──
    let _turso_conn: Option<libsql::Connection> = None;
    // let turso_url = env::var("TURSO_DATABASE_URL").ok();
    // let turso_api_key = env::var("TURSO_API_KEY").ok();
    // ... (turso setup skipped)

    // ── Auth (commented out: providers-only mode) ──
    let auth_result = AuthResult {
        method: "skipped".to_string(),
        email: "dev@localhost".to_string(),
        name: "DX Developer".to_string(),
        oauth_subject: None,
    };

    // ── Avatar (commented out: providers-only mode) ──
    let avatar_source: Option<String> = None;
    let ascii_generated = false;
    let pixel_generated = false;

    // ── Avatar preview (commented out: providers-only mode) ──
    // if avatar_mode != "skip" { ... }

    let providers = select_providers_grouped()?;

    let client = reqwest::Client::new();
    let discovery_catalog = refresh_discovery_catalog(&client).await.ok();
    let available_provider_count = discovery_catalog
        .as_ref()
        .map(|catalog| catalog.providers.len())
        .unwrap_or_else(|| provider_catalog_detailed().len());
    let provider_100_plus_ready = available_provider_count >= 100;

    let provider_pricing_hints = fetch_pricing_hints(&providers).await;
    if !provider_pricing_hints.is_empty() {
        prompts::section_with_width("Provider Pricing (models.dev)", 76, |lines| {
            for hint in &provider_pricing_hints {
                lines.push(hint.to_string());
            }
        })?;
    }

    let mut provider_api_keys: BTreeMap<String, Option<String>> = BTreeMap::new();
    let mut provider_profiles: BTreeMap<String, String> = BTreeMap::new();
    for provider in &providers {
        if provider == "github_copilot" || provider == "github-copilot" {
            prompts::section_with_width("GitHub Copilot Setup", 76, |lines| {
                lines.push("Copilot tokens are short-lived. DX will fetch a fresh Copilot service token every run.".to_string());
                lines.push("DX will NOT store the Copilot token in keyring by default (prevents 'works once then Access denied').".to_string());
            })?;

            match dx_onboard::llm::ensure_github_copilot_ready_interactive().await {
                Ok(result) => {
                    eprintln!(
                        "● Copilot ready (base_url={}, source={})",
                        result.base_url, result.source
                    );
                }
                Err(err) => {
                    eprintln!("● Copilot auto-setup failed: {err}");
                    eprintln!(
                        "│ Continuing; Copilot will be unavailable until DX can fetch a Copilot service token via HTTPS."
                    );
                }
            }
        }

        let profile = select_provider_profile(provider)?;
        let api_key_env = resolve_provider_api_key_env(provider);
        let api_key = prompt_provider_api_key(provider, &profile, api_key_env.as_deref())?;
        if let (Some(env_name), Some(value)) = (api_key_env.as_deref(), api_key.as_deref()) {
            upsert_provider_env(provider, env_name, value);

            // Copilot service tokens are short-lived; never persist them.
            if provider != "github_copilot" && provider != "github-copilot" {
                let _ = store_keyring_api_key(provider, &profile, value);
            }
        }
        provider_api_keys.insert(provider.clone(), api_key);
        provider_profiles.insert(provider.clone(), profile);
    }

    let mut provider_connection_status = Vec::new();
    for provider in &providers {
        provider_connection_status.push(check_provider_connection(provider).await);
    }

    let mut runtime_registry = ProviderRegistry::new();
    runtime_registry.register_default_genai_providers();
    runtime_registry.register_openai_compatible_presets();
    runtime_registry.register_enterprise_custom_providers();

    let mut provider_models = Vec::new();
    for provider in &providers {
        let api_key = provider_api_keys.get(provider).and_then(|value| value.as_deref());
        provider_models.push(fetch_models_for_provider(provider, api_key, &runtime_registry).await);
    }

    prompts::section_with_width("Provider Connectivity", 76, |lines| {
        for status in &provider_connection_status {
            lines.push(format!("{} => {} ({})", status.provider_id, status.status, status.detail));
        }
    })?;

    if !provider_models.is_empty() {
        prompts::section_with_width("Discovered Provider Models", 76, |lines| {
            for provider in &provider_models {
                lines.push(format!(
                    "{} => {} ({})",
                    provider.provider_id, provider.status, provider.detail
                ));
                if provider.models.is_empty() {
                    lines.push("  (no models discovered)".to_string());
                } else {
                    for model in &provider.models {
                        lines.push(format!("  - {}:{}", provider.provider_id, model));
                    }
                }
                if !provider.modules.is_empty() {
                    lines.push("  modules: ".to_string());
                    for module in &provider.modules {
                        lines.push(format!("    * {}", module));
                    }
                }
            }
        })?;
    }

    let mut provider_model_probes = Vec::new();
    for model_listing in &provider_models {
        if model_listing.models.is_empty() {
            provider_model_probes.push(ProviderModelProbeResult {
                provider_id: model_listing.provider_id.clone(),
                api_key_env: resolve_provider_api_key_env(&model_listing.provider_id),
                selected_model: None,
                status: "skipped".to_string(),
                detail: "no models available for selection".to_string(),
                response_preview: None,
            });
            continue;
        }

        let mut model_select =
            prompts::select(format!("Choose model for provider {}", model_listing.provider_id));
        for model in &model_listing.models {
            model_select = model_select.item(model.clone(), model.clone(), "Available model");
        }

        let selected_model = model_select.interact()?;
        let run_probe = prompts::confirm(format!(
            "Run hello prompt probe for {}:{}?",
            model_listing.provider_id, selected_model
        ))
        .initial_value(true)
        .interact()?;

        if !run_probe {
            provider_model_probes.push(ProviderModelProbeResult {
                provider_id: model_listing.provider_id.clone(),
                api_key_env: resolve_provider_api_key_env(&model_listing.provider_id),
                selected_model: Some(selected_model),
                status: "skipped".to_string(),
                detail: "hello probe skipped by user".to_string(),
                response_preview: None,
            });
            continue;
        }

        let api_key = provider_api_keys
            .get(&model_listing.provider_id)
            .and_then(|value| value.as_deref());
        let probe = run_hello_probe(
            &model_listing.provider_id,
            &selected_model,
            api_key,
            &runtime_registry,
        )
        .await;
        provider_model_probes.push(probe);
    }

    prompts::section_with_width("Provider Hello Probe", 76, |lines| {
        for probe in &provider_model_probes {
            lines.push(format!("{} => {} ({})", probe.provider_id, probe.status, probe.detail));
            if let Some(model) = &probe.selected_model {
                lines.push(format!("  model: {}", model));
            }
            if let Some(preview) = &probe.response_preview {
                lines.push(format!("  response: {}", preview));
            }
        }
    })?;

    let discovered_prefixed_models = provider_models
        .iter()
        .flat_map(|provider| {
            provider
                .models
                .iter()
                .map(|model| format!("{}:{}", provider.provider_id, model))
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();

    let default_smart_model = discovered_prefixed_models
        .iter()
        .find(|model| model.as_str() == "opencode:glm-5-free")
        .cloned()
        .or_else(|| {
            discovered_prefixed_models
                .iter()
                .find(|model| model.starts_with("opencode:"))
                .cloned()
        })
        .or_else(|| {
            discovered_prefixed_models
                .iter()
                .find(|model| {
                    model.ends_with(":gpt-5-mini")
                        || model.ends_with(":gpt-5.2")
                        || model.ends_with(":gpt-5.1")
                })
                .cloned()
        })
        .or_else(|| discovered_prefixed_models.first().cloned())
        .unwrap_or_else(|| "opencode:glm-5-free".to_string());
    let default_small_model = discovered_prefixed_models
        .get(1)
        .cloned()
        .unwrap_or_else(|| "opencode:glm-5-free".to_string());
    let default_multi_model = discovered_prefixed_models
        .get(2)
        .cloned()
        .unwrap_or_else(|| "anthropic:claude-3-5-haiku".to_string());

    let smart_default_model = default_smart_model;
    let small_subagent_model = default_small_model;
    let multi_agent_model = default_multi_model;

    let config_default_provider = smart_default_model
        .split(':')
        .next()
        .filter(|provider_id| !provider_id.trim().is_empty())
        .map(|provider_id| provider_id.to_string())
        .or_else(|| providers.first().cloned());

    let mut provider_config_path: Option<String> = None;
    let provider_config_status =
        match ProviderConfigFile::load_with_migration(config_default_provider.clone()) {
            Ok(mut provider_config) => {
                provider_config
                    .apply_provider_selection(&providers, config_default_provider.clone());

                match provider_config.validate() {
                    Ok(_) => match provider_config.save_to_default_path() {
                        Ok(path) => {
                            provider_config_path = Some(path.display().to_string());
                            "saved".to_string()
                        }
                        Err(err) => format!("save_failed: {err}"),
                    },
                    Err(err) => format!("validation_failed: {err}"),
                }
            }
            Err(err) => format!("load_failed: {err}"),
        };

    if provider_config_status == "saved"
        && let Ok(mut provider_config) =
            ProviderConfigFile::load_with_migration(config_default_provider.clone())
    {
        for provider in &providers {
            let profile = provider_profiles
                .get(provider)
                .cloned()
                .unwrap_or_else(|| "default".to_string());
            let api_key_env = resolve_provider_api_key_env(provider);
            let base_url = provider_config
                .providers
                .get(provider)
                .map(|entry| entry.base_url.clone())
                .unwrap_or_else(|| {
                    format!(
                        "https://api.{}.com/v1",
                        provider.replace('_', "-").to_ascii_lowercase()
                    )
                });

            if let Some(entry) = provider_config.providers.get_mut(provider) {
                entry.active_profile = Some(profile.clone());
                entry.profiles.insert(
                    profile,
                    ProviderProfileEntry {
                        enabled: true,
                        base_url,
                        api_key_env,
                        custom_headers: BTreeMap::new(),
                    },
                );
            }
        }
        let _ = provider_config.save_to_default_path();
    }

    // ── Channels (commented out: providers-only mode) ──
    let channels: Vec<String> = Vec::new();
    // let channels = prompts::multiselect("Messaging apps")
    //     .item("telegram".to_string(), "Telegram", "Bot API + groups")
    //     .item("discord".to_string(), "Discord", "Servers + channels")
    //     .item("slack".to_string(), "Slack", "Workspace channels")
    //     .item("whatsapp".to_string(), "WhatsApp", "Bridge integration")
    //     .item("google_chat".to_string(), "Google Chat", "Spaces integration")
    //     .item("teams".to_string(), "Microsoft Teams", "Enterprise chat")
    //     .item("matrix".to_string(), "Matrix", "Federated chat")
    //     .item("mattermost".to_string(), "Mattermost", "Self-hosted chat")
    //     .required(false)
    //     .interact()?;

    let payload = OnboardingPayload {
        runtime_environment: detected_env.as_str().to_string(),
        runtime_label: detected_env.label().to_string(),
        install_mode,
        components_to_download: installed_components.clone(),
        account: auth_result.clone(),
        avatar_source: avatar_source.clone(),
        avatar_ascii_preview_generated: ascii_generated,
        avatar_pixel_preview_generated: pixel_generated,
        llm_providers: providers.clone(),
        available_provider_count,
        provider_100_plus_ready,
        provider_connection_status,
        provider_models,
        provider_model_probes,
        provider_pricing_hints,
        provider_config_path: provider_config_path.clone(),
        provider_config_status: provider_config_status.clone(),
        smart_default_model: smart_default_model.clone(),
        small_subagent_model: small_subagent_model.clone(),
        multi_agent_model: multi_agent_model.clone(),
        messaging_channels: channels.clone(),
    };

    // ── Turso save (commented out: providers-only mode) ──
    // if let Some(conn) = turso_conn.as_ref() { ... }

    prompts::section_with_width("Onboarding Summary", 76, |lines| {
        lines.push(format!("Runtime: {}", detected_env.label()));
        lines.push(format!("Components: {}", installed_components.join(", ")));
        lines.push(format!("Auth: {} ({})", auth_result.method, auth_result.email));
        lines.push(format!("Providers selected: {}", providers.len()));
        lines.push(format!("Provider config: {}", provider_config_status));
        if let Some(path) = &provider_config_path {
            lines.push(format!("Provider config path: {}", path));
        }
        lines.push(format!("Channels selected: {}", channels.len()));
        lines.push("Storage: response.json + Turso (when configured)".to_string());
    })?;

    write_response(vec![json!({
        "test": "dx_onboarding",
        "results": payload,
    })])?;

    prompts::log::success("DX onboarding completed")?;

    Ok(())
}

fn print_usage() {
    eprintln!("Usage:");
    eprintln!("  cargo run                  # run DX onboarding (default)");
    eprintln!("  cargo run -- --dx-onboard [--shared-account <ref>] [--account-email <email>]");
    eprintln!(
        "  cargo run -- --tests       # run full prompt test suite (1-{})",
        prompt_suite::TOTAL_TESTS
    );
    eprintln!(
        "  cargo run -- <1-{}>        # run single numbered test",
        prompt_suite::TOTAL_TESTS
    );
}

async fn async_main() -> Result<()> {
    let args: Vec<String> = env::args().collect();

    // Default behavior: no args -> show DX onboarding
    if args.len() == 1 {
        let parsed = parse_onboard_args(&[]);
        return run_dx_onboarding_flow(parsed).await;
    }

    // When args are present, route explicitly
    match args[1].as_str() {
        "--dx-onboard" | "dx-onboard" | "onboard" => {
            let parsed = parse_onboard_args(&args[2..]);
            run_dx_onboarding_flow(parsed).await?;
            Ok(())
        }
        "--tests" | "--run-tests" => {
            // Explicit request to run the full prompt test suite
            let s = &*prompts::SYMBOLS;
            eprintln!("{}", "┌─ DX CLI Prompt Test Suite".dimmed());
            eprintln!("{}", s.bar.dimmed());
            prompts::section_with_width(
                &format!("Running All Tests (1-{})", prompt_suite::TOTAL_TESTS),
                60,
                |lines| {
                    lines.push("Testing all prompt components sequentially".to_string());
                },
            )?;

            let mut all_results: Vec<Value> = Vec::new();
            for i in 1..=prompt_suite::TOTAL_TESTS {
                let (name, data) = prompt_suite::run_test(i)?;
                all_results.push(json!({ "test": name, "results": data }));
            }

            write_response(all_results)?;
            prompts::log::success(format!("All {} tests completed!", prompt_suite::TOTAL_TESTS))?;
            eprintln!("{}", "└─ Check response.json for results".dimmed());
            Ok(())
        }
        "--help" | "-h" | "help" => {
            print_usage();
            Ok(())
        }
        _ => {
            // Treat as a single test number
            let test_num = args[1].parse::<u32>().unwrap_or(0);
            if test_num == 0 || test_num > prompt_suite::TOTAL_TESTS {
                eprintln!("Invalid test number. Use 1-{}.", prompt_suite::TOTAL_TESTS);
                print_usage();
                return Ok(());
            }
            let (name, data) = prompt_suite::run_test(test_num)?;
            write_response(vec![json!({"test": name, "results": data})])?;
            prompts::log::success(format!(
                "Completed test {test_num}/{}",
                prompt_suite::TOTAL_TESTS
            ))?;
            Ok(())
        }
    }
}

fn main() -> Result<()> {
    let handle = std::thread::Builder::new()
        .name("dx-onboard-main".to_string())
        .stack_size(32 * 1024 * 1024)
        .spawn(|| -> Result<()> {
            let runtime = tokio::runtime::Builder::new_multi_thread().enable_all().build()?;
            runtime.block_on(async_main())
        })
        .map_err(|err| anyhow::anyhow!("failed to spawn onboarding thread: {}", err))?;

    match handle.join() {
        Ok(result) => result,
        Err(_) => Err(anyhow::anyhow!("onboarding thread panicked")),
    }
}

fn write_response(results: Vec<Value>) -> Result<()> {
    let response = json!({
        "suite": "dx-onboard-prompts",
        "total": results.len(),
        "completed_at": Local::now().to_rfc3339(),
        "results": results,
    });
    let root = find_workspace_root();
    let output_path = root.join("response.json");
    fs::write(output_path, serde_json::to_string_pretty(&response)?)?;
    prompts::log::success("Saved to response.json")?;
    Ok(())
}
