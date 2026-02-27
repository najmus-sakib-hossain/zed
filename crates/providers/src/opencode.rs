use anyhow::{Context as _, Result};
use http_client::{AsyncBody, HttpClient, Method, Request};
use serde_json::Value;
use smol::io::AsyncReadExt;

pub const OPENCODE_API: &str = "https://opencode.ai/zen/v1";
pub const MODELS_DEV_API: &str = "https://models.dev/api.json";

/// Free models recommended for Zed users (no API key required).
///
/// OpenCode Zen provides an OpenAI-compatible `chat/completions` endpoint and a public key
/// for promotional access.
///
/// These 3 models are verified working as of February 2026:
/// - trinity-large-preview-free: 131K context, fast responses
/// - big-pickle: 200K context, reasoning model
/// - minimax-m2.5-free: 204K context, reasoning model
pub const FREE_MODELS: [&str; 3] = [
    "trinity-large-preview-free",
    "big-pickle",
    "minimax-m2.5-free",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenCodeModelDescriptor {
    pub id: String,
    pub display_name: String,
    pub context_window: Option<u64>,
    pub max_output_tokens: Option<u64>,
    pub supports_tools: bool,
    pub supports_vision: bool,
    pub supports_audio: bool,
}

pub fn default_free_models() -> Vec<OpenCodeModelDescriptor> {
    FREE_MODELS
        .iter()
        .map(|id| OpenCodeModelDescriptor {
            id: (*id).to_string(),
            display_name: format!("{} (recommended)", id),
            context_window: None,
            max_output_tokens: None,
            supports_tools: true,
            supports_vision: false,
            supports_audio: false,
        })
        .collect()
}

/// Fetch metadata for OpenCode free models from models.dev.
///
/// If the network request or parsing fails, callers should fall back to `default_free_models()`.
pub async fn fetch_free_models(http_client: &dyn HttpClient) -> Result<Vec<OpenCodeModelDescriptor>> {
    let request = Request::builder()
        .method(Method::GET)
        .uri(MODELS_DEV_API)
        .header("User-Agent", "zed-opencode")
        .body(AsyncBody::empty())
        .context("build models.dev request")?;

    let mut response = http_client
        .send(request)
        .await
        .context("send models.dev request")?;

    let mut body = String::new();
    response
        .body_mut()
        .read_to_string(&mut body)
        .await
        .context("read models.dev response")?;

    let response_json: Value = serde_json::from_str(&body).context("parse models.dev json")?;

    let provider_data = response_json
        .get("opencode")
        .context("models.dev payload missing 'opencode'")?;

    let provider_models = provider_data
        .get("models")
        .and_then(|m| m.as_object())
        .context("models.dev payload missing 'opencode.models' map")?;

    let mut models = Vec::new();
    for model_id in FREE_MODELS {
        let Some(model_data) = provider_models.get(model_id) else {
            continue;
        };

        if model_data
            .get("status")
            .and_then(|v| v.as_str())
            .is_some_and(|status| status.eq_ignore_ascii_case("deprecated"))
        {
            continue;
        }

        let input_cost = model_data
            .get("cost")
            .and_then(|c| c.get("input"))
            .and_then(|v| v.as_f64());
        let output_cost = model_data
            .get("cost")
            .and_then(|c| c.get("output"))
            .and_then(|v| v.as_f64());

        if input_cost != Some(0.0) || output_cost != Some(0.0) {
            continue;
        }

        // OpenCode Zen serves GPT-* via `/responses`, while we use `/chat/completions`.
        if model_id.starts_with("gpt-") {
            continue;
        }

        let name = model_data
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or(model_id);

        let context_window = model_data
            .get("limit")
            .and_then(|l| l.get("context"))
            .and_then(|v| v.as_u64());

        let max_output_tokens = model_data
            .get("limit")
            .and_then(|l| l.get("output"))
            .and_then(|v| v.as_u64());

        let supports_tools = model_data
            .get("tool_call")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);

        let supports_vision = model_data
            .get("modalities")
            .and_then(|m| m.get("input"))
            .and_then(|v| v.as_array())
            .is_some_and(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str())
                    .any(|mode| mode.eq_ignore_ascii_case("image"))
            });

        let supports_audio = model_data
            .get("modalities")
            .and_then(|m| m.get("input"))
            .and_then(|v| v.as_array())
            .is_some_and(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str())
                    .any(|mode| mode.eq_ignore_ascii_case("audio"))
            });

        models.push(OpenCodeModelDescriptor {
            id: model_id.to_string(),
            display_name: format!("{} (recommended)", name),
            context_window,
            max_output_tokens,
            supports_tools,
            supports_vision,
            supports_audio,
        });
    }

    // Keep stable ordering matching FREE_MODELS.
    models.sort_by_key(|m| {
        FREE_MODELS
            .iter()
            .position(|id| id == &m.id)
            .unwrap_or(usize::MAX)
    });

    Ok(models)
}