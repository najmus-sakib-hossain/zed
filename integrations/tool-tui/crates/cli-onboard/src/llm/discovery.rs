use crate::llm::error::ProviderError;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};
use tokio::task::JoinHandle;
use tokio::time::sleep;

const MODELS_DEV_URL: &str = "https://models.dev/api.json";
const LITELLM_URL: &str =
    "https://raw.githubusercontent.com/BerriAI/litellm/main/model_prices_and_context_window.json";
const OPENROUTER_MODELS_URL: &str = "https://openrouter.ai/api/v1/models";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DiscoveredProvider {
    pub id: String,
    pub name: String,
    pub source: String,
    pub category: String,
    pub base_url: Option<String>,
    pub model_count: usize,
    pub openai_compatible: bool,
    pub supports_tools: bool,
    pub supports_vision: bool,
    pub supports_audio: bool,
    pub max_context_window: Option<u32>,
    pub avg_input_price_per_million: Option<f64>,
    pub avg_output_price_per_million: Option<f64>,
    pub docs_url: Option<String>,
    pub website: Option<String>,
    pub sample_models: Vec<String>,
}

impl DiscoveredProvider {
    pub fn default_api_key_env(&self) -> Option<String> {
        let normalized = self.id.to_ascii_uppercase().replace('-', "_");
        match normalized.as_str() {
            "OPENAI" => Some("OPENAI_API_KEY".to_string()),
            "OPENROUTER" => Some("OPENROUTER_API_KEY".to_string()),
            "GROQ" => Some("GROQ_API_KEY".to_string()),
            "ANTHROPIC" => Some("ANTHROPIC_API_KEY".to_string()),
            "GOOGLE" | "GEMINI" => Some("GEMINI_API_KEY".to_string()),
            "OLLAMA" | "LMSTUDIO" | "LOCALAI" | "VLLM" | "LLAMA_CPP" => None,
            _ => Some(format!("{}_API_KEY", normalized)),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct CatalogFilter {
    pub query: Option<String>,
    pub category: Option<String>,
    pub openai_compatible_only: bool,
    pub max_input_price_per_million: Option<f64>,
    pub min_context_window: Option<u32>,
    pub requires_tools: bool,
    pub requires_vision: bool,
    pub requires_audio: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct DiscoveryCatalog {
    pub generated_at_unix: u64,
    pub providers: Vec<DiscoveredProvider>,
}

fn cache_root_dir() -> PathBuf {
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    home.join(".dx").join("cache")
}

fn ensure_cache_dir() -> Result<PathBuf, ProviderError> {
    let cache_dir = cache_root_dir();
    fs::create_dir_all(&cache_dir).map_err(|err| ProviderError::InvalidConfig {
        provider: "discovery".to_string(),
        detail: format!("failed to create cache directory {}: {err}", cache_dir.display()),
    })?;
    Ok(cache_dir)
}

fn now_unix_seconds() -> u64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|dur| dur.as_secs())
        .unwrap_or(0)
}

fn should_refresh_cache(path: &Path, max_age: Duration) -> bool {
    if !path.exists() {
        return true;
    }

    let modified = match fs::metadata(path).and_then(|metadata| metadata.modified()) {
        Ok(mtime) => mtime,
        Err(_) => return true,
    };

    match SystemTime::now().duration_since(modified) {
        Ok(age) => age > max_age,
        Err(_) => true,
    }
}

async fn fetch_json_with_cache(
    client: &reqwest::Client,
    url: &str,
    cache_file_name: &str,
    max_age: Duration,
) -> Result<Value, ProviderError> {
    let cache_dir = ensure_cache_dir()?;
    let cache_file = cache_dir.join(cache_file_name);

    if !should_refresh_cache(&cache_file, max_age) {
        let content =
            fs::read_to_string(&cache_file).map_err(|err| ProviderError::InvalidConfig {
                provider: "discovery".to_string(),
                detail: format!("failed reading cache file {}: {err}", cache_file.display()),
            })?;
        let parsed: Value = serde_json::from_str(&content)?;
        return Ok(parsed);
    }

    let response = client.get(url).send().await?;
    if !response.status().is_success() {
        let status = response.status().as_u16();
        let body = response.text().await.unwrap_or_default();
        return Err(ProviderError::HttpStatus {
            provider: "discovery".to_string(),
            status,
            body,
        });
    }

    let raw = response.text().await?;
    fs::write(&cache_file, &raw).map_err(|err| ProviderError::InvalidConfig {
        provider: "discovery".to_string(),
        detail: format!("failed writing cache file {}: {err}", cache_file.display()),
    })?;

    let parsed = serde_json::from_str::<Value>(&raw)?;
    Ok(parsed)
}

fn classify_provider(id: &str) -> String {
    let normalized = id.to_ascii_lowercase();

    if matches!(
        normalized.as_str(),
        "openai" | "anthropic" | "google" | "azure" | "aws" | "bedrock"
    ) {
        return "major-cloud".to_string();
    }

    if normalized.contains("openrouter")
        || normalized.contains("litellm")
        || normalized.contains("vercel")
        || normalized.contains("router")
    {
        return "aggregator".to_string();
    }

    if normalized.contains("ollama")
        || normalized.contains("lmstudio")
        || normalized.contains("llama.cpp")
        || normalized.contains("vllm")
        || normalized.contains("local")
    {
        return "local-runner".to_string();
    }

    if normalized.contains("groq")
        || normalized.contains("together")
        || normalized.contains("fireworks")
        || normalized.contains("cerebras")
        || normalized.contains("deepinfra")
        || normalized.contains("hyperbolic")
    {
        return "fast-inference".to_string();
    }

    "specialized".to_string()
}

fn parse_models_dev_json(data: Value) -> Vec<DiscoveredProvider> {
    let Some(root) = data.as_object() else {
        return Vec::new();
    };

    root.iter()
        .map(|(provider_id, value)| {
            let name = value.get("name").and_then(Value::as_str).unwrap_or(provider_id).to_string();
            let base_url = value.get("api").and_then(Value::as_str).map(ToString::to_string);
            let docs_url = value.get("doc").and_then(Value::as_str).map(ToString::to_string);
            let website = value.get("website").and_then(Value::as_str).map(ToString::to_string);

            let models = value.get("models").and_then(Value::as_object);
            let model_count = models.map(|all| all.len()).unwrap_or(0);
            let mut supports_tools = false;
            let mut supports_vision = false;
            let mut supports_audio = false;
            let mut max_context_window: Option<u32> = None;
            let mut input_price_sum = 0.0f64;
            let mut output_price_sum = 0.0f64;
            let mut priced_count = 0usize;
            let mut sample_models = Vec::new();

            if let Some(models) = models {
                for (model_id, model_meta) in models {
                    if sample_models.len() < 8 {
                        sample_models.push(model_id.to_string());
                    }

                    supports_tools |=
                        model_meta.get("tool_call").and_then(Value::as_bool).unwrap_or(false);

                    if let Some(inputs) = model_meta
                        .get("modalities")
                        .and_then(|modalities| modalities.get("input"))
                        .and_then(Value::as_array)
                    {
                        supports_vision |= inputs.iter().any(|m| m.as_str() == Some("image"));
                        supports_audio |= inputs.iter().any(|m| m.as_str() == Some("audio"));
                    }

                    if let Some(context) = model_meta
                        .get("limit")
                        .and_then(|limit| limit.get("context"))
                        .and_then(Value::as_u64)
                        .and_then(|value| u32::try_from(value).ok())
                    {
                        max_context_window = Some(max_context_window.unwrap_or(0).max(context));
                    }

                    let input = model_meta
                        .get("cost")
                        .and_then(|cost| cost.get("input"))
                        .and_then(Value::as_f64);
                    let output = model_meta
                        .get("cost")
                        .and_then(|cost| cost.get("output"))
                        .and_then(Value::as_f64);
                    if let (Some(input), Some(output)) = (input, output) {
                        input_price_sum += input;
                        output_price_sum += output;
                        priced_count += 1;
                    }
                }
            }

            let avg_input_price_per_million = if priced_count > 0 {
                Some(input_price_sum / priced_count as f64)
            } else {
                None
            };
            let avg_output_price_per_million = if priced_count > 0 {
                Some(output_price_sum / priced_count as f64)
            } else {
                None
            };

            let openai_compatible = value
                .get("npm")
                .and_then(Value::as_str)
                .map(|npm| npm.contains("openai-compatible") || npm.contains("@ai-sdk/openai"))
                .unwrap_or(false)
                || base_url.as_ref().map(|url| url.contains("/v1")).unwrap_or(false);

            DiscoveredProvider {
                id: provider_id.to_string(),
                name,
                source: "models.dev".to_string(),
                category: classify_provider(provider_id),
                base_url,
                model_count,
                openai_compatible,
                supports_tools,
                supports_vision,
                supports_audio,
                max_context_window,
                avg_input_price_per_million,
                avg_output_price_per_million,
                docs_url,
                website,
                sample_models,
            }
        })
        .collect()
}

fn parse_litellm_json(data: Value) -> Vec<DiscoveredProvider> {
    let Some(root) = data.as_object() else {
        return Vec::new();
    };

    let mut provider_map: BTreeMap<String, usize> = BTreeMap::new();
    let mut context_map: BTreeMap<String, u32> = BTreeMap::new();
    let mut input_sum: BTreeMap<String, f64> = BTreeMap::new();
    let mut output_sum: BTreeMap<String, f64> = BTreeMap::new();
    let mut price_count: BTreeMap<String, usize> = BTreeMap::new();
    let mut tools_map: BTreeMap<String, bool> = BTreeMap::new();
    let mut vision_map: BTreeMap<String, bool> = BTreeMap::new();
    let mut audio_map: BTreeMap<String, bool> = BTreeMap::new();
    let mut sample_models: BTreeMap<String, Vec<String>> = BTreeMap::new();

    for (model_id, model_info) in root {
        if let Some(provider_name) = model_info
            .get("litellm_provider")
            .and_then(Value::as_str)
            .or_else(|| model_info.get("provider").and_then(Value::as_str))
        {
            *provider_map.entry(provider_name.to_string()).or_insert(0) += 1;

            if let Some(models) = sample_models.get_mut(provider_name) {
                if models.len() < 8 {
                    models.push(model_id.to_string());
                }
            } else {
                sample_models.insert(provider_name.to_string(), vec![model_id.to_string()]);
            }

            if let Some(context) = model_info
                .get("max_input_tokens")
                .or_else(|| model_info.get("max_tokens"))
                .and_then(Value::as_u64)
                .and_then(|value| u32::try_from(value).ok())
            {
                context_map
                    .entry(provider_name.to_string())
                    .and_modify(|current| *current = (*current).max(context))
                    .or_insert(context);
            }

            let input = model_info
                .get("input_cost_per_token")
                .or_else(|| model_info.get("input_cost_per_token_usd"))
                .and_then(Value::as_f64)
                .map(|value| value * 1_000_000.0);
            let output = model_info
                .get("output_cost_per_token")
                .or_else(|| model_info.get("output_cost_per_token_usd"))
                .and_then(Value::as_f64)
                .map(|value| value * 1_000_000.0);

            if let (Some(input), Some(output)) = (input, output) {
                *input_sum.entry(provider_name.to_string()).or_insert(0.0) += input;
                *output_sum.entry(provider_name.to_string()).or_insert(0.0) += output;
                *price_count.entry(provider_name.to_string()).or_insert(0) += 1;
            }

            let supports_tools = model_info
                .get("supports_function_calling")
                .or_else(|| model_info.get("supports_tool_calling"))
                .and_then(Value::as_bool)
                .unwrap_or(false);
            let supports_vision =
                model_info.get("supports_vision").and_then(Value::as_bool).unwrap_or(false);
            let supports_audio = model_info
                .get("supports_audio_input")
                .or_else(|| model_info.get("supports_audio"))
                .and_then(Value::as_bool)
                .unwrap_or(false);

            tools_map
                .entry(provider_name.to_string())
                .and_modify(|value| *value |= supports_tools)
                .or_insert(supports_tools);
            vision_map
                .entry(provider_name.to_string())
                .and_modify(|value| *value |= supports_vision)
                .or_insert(supports_vision);
            audio_map
                .entry(provider_name.to_string())
                .and_modify(|value| *value |= supports_audio)
                .or_insert(supports_audio);
        }
    }

    provider_map
        .into_iter()
        .map(|(id, model_count)| {
            let provider_id = id.clone();
            DiscoveredProvider {
                name: provider_id.clone(),
                category: classify_provider(&provider_id),
                source: "litellm".to_string(),
                base_url: None,
                openai_compatible: true,
                id: provider_id.clone(),
                model_count,
                supports_tools: tools_map.get(&provider_id).copied().unwrap_or(false),
                supports_vision: vision_map.get(&provider_id).copied().unwrap_or(false),
                supports_audio: audio_map.get(&provider_id).copied().unwrap_or(false),
                max_context_window: context_map.get(&provider_id).copied(),
                avg_input_price_per_million: price_count.get(&provider_id).copied().and_then(
                    |count| {
                        if count == 0 {
                            None
                        } else {
                            Some(input_sum.get(&provider_id).copied().unwrap_or(0.0) / count as f64)
                        }
                    },
                ),
                avg_output_price_per_million: price_count.get(&provider_id).copied().and_then(
                    |count| {
                        if count == 0 {
                            None
                        } else {
                            Some(
                                output_sum.get(&provider_id).copied().unwrap_or(0.0) / count as f64,
                            )
                        }
                    },
                ),
                docs_url: None,
                website: None,
                sample_models: sample_models.remove(&provider_id).unwrap_or_default(),
            }
        })
        .collect()
}

pub fn extract_unique_litellm_providers_not_in_models_dev(
    models_dev: &[DiscoveredProvider],
    litellm: &[DiscoveredProvider],
) -> Vec<DiscoveredProvider> {
    let known: BTreeSet<String> = models_dev.iter().map(|provider| provider.id.clone()).collect();
    litellm
        .iter()
        .filter(|provider| !known.contains(&provider.id))
        .cloned()
        .collect()
}

fn parse_openrouter_json(data: Value) -> Vec<DiscoveredProvider> {
    let models = data.get("data").and_then(Value::as_array).cloned().unwrap_or_default();

    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    let mut supports_tools = BTreeMap::<String, bool>::new();
    let mut supports_vision = BTreeMap::<String, bool>::new();
    let mut supports_audio = BTreeMap::<String, bool>::new();
    let mut max_context = BTreeMap::<String, u32>::new();
    let mut input_sum = BTreeMap::<String, f64>::new();
    let mut output_sum = BTreeMap::<String, f64>::new();
    let mut price_count = BTreeMap::<String, usize>::new();
    let mut sample_models = BTreeMap::<String, Vec<String>>::new();

    for model in models {
        let Some(model_id) = model.get("id").and_then(Value::as_str) else {
            continue;
        };

        let provider_id = model_id
            .split('/')
            .next()
            .map(ToString::to_string)
            .unwrap_or_else(|| "unknown".to_string());

        *counts.entry(provider_id.clone()).or_insert(0) += 1;

        if let Some(items) = sample_models.get_mut(&provider_id) {
            if items.len() < 8 {
                items.push(model_id.to_string());
            }
        } else {
            sample_models.insert(provider_id.clone(), vec![model_id.to_string()]);
        }

        if let Some(tool) = model
            .get("supported_parameters")
            .and_then(Value::as_array)
            .map(|params| params.iter().any(|parameter| parameter.as_str() == Some("tools")))
        {
            let entry = supports_tools.entry(provider_id.clone()).or_insert(false);
            *entry |= tool;
        }

        if let Some(input_mods) = model
            .get("architecture")
            .and_then(|arch| arch.get("input_modalities"))
            .and_then(Value::as_array)
        {
            let has_vision = input_mods.iter().any(|m| m.as_str() == Some("image"));
            let has_audio = input_mods.iter().any(|m| m.as_str() == Some("audio"));
            supports_vision
                .entry(provider_id.clone())
                .and_modify(|value| *value |= has_vision)
                .or_insert(has_vision);
            supports_audio
                .entry(provider_id.clone())
                .and_modify(|value| *value |= has_audio)
                .or_insert(has_audio);
        }

        if let Some(context) = model
            .get("context_length")
            .and_then(Value::as_u64)
            .and_then(|value| u32::try_from(value).ok())
        {
            max_context
                .entry(provider_id.clone())
                .and_modify(|current| *current = (*current).max(context))
                .or_insert(context);
        }

        let prompt = model
            .get("pricing")
            .and_then(|pricing| pricing.get("prompt"))
            .and_then(Value::as_str)
            .and_then(|value| value.parse::<f64>().ok())
            .map(|value| value * 1_000_000.0);
        let completion = model
            .get("pricing")
            .and_then(|pricing| pricing.get("completion"))
            .and_then(Value::as_str)
            .and_then(|value| value.parse::<f64>().ok())
            .map(|value| value * 1_000_000.0);
        if let (Some(prompt), Some(completion)) = (prompt, completion) {
            *input_sum.entry(provider_id.clone()).or_insert(0.0) += prompt;
            *output_sum.entry(provider_id.clone()).or_insert(0.0) += completion;
            *price_count.entry(provider_id.clone()).or_insert(0) += 1;
        }
    }

    counts
        .into_iter()
        .map(|(id, model_count)| DiscoveredProvider {
            name: id.clone(),
            category: classify_provider(&id),
            source: "openrouter".to_string(),
            base_url: Some("https://openrouter.ai/api/v1".to_string()),
            openai_compatible: true,
            supports_tools: supports_tools.get(&id).copied().unwrap_or(false),
            supports_vision: supports_vision.get(&id).copied().unwrap_or(false),
            supports_audio: supports_audio.get(&id).copied().unwrap_or(false),
            max_context_window: max_context.get(&id).copied(),
            avg_input_price_per_million: price_count.get(&id).copied().and_then(|count| {
                if count == 0 {
                    None
                } else {
                    Some(input_sum.get(&id).copied().unwrap_or(0.0) / count as f64)
                }
            }),
            avg_output_price_per_million: price_count.get(&id).copied().and_then(|count| {
                if count == 0 {
                    None
                } else {
                    Some(output_sum.get(&id).copied().unwrap_or(0.0) / count as f64)
                }
            }),
            docs_url: None,
            website: None,
            sample_models: sample_models.remove(&id).unwrap_or_default(),
            id,
            model_count,
        })
        .collect()
}

pub async fn fetch_models_dev_providers(
    client: &reqwest::Client,
) -> Result<Vec<DiscoveredProvider>, ProviderError> {
    let json = fetch_json_with_cache(
        client,
        MODELS_DEV_URL,
        "models_dev.json",
        Duration::from_secs(60 * 60 * 24),
    )
    .await?;
    Ok(parse_models_dev_json(json))
}

pub async fn fetch_litellm_providers(
    client: &reqwest::Client,
) -> Result<Vec<DiscoveredProvider>, ProviderError> {
    let json = fetch_json_with_cache(
        client,
        LITELLM_URL,
        "litellm.json",
        Duration::from_secs(60 * 60 * 24),
    )
    .await?;
    Ok(parse_litellm_json(json))
}

pub async fn fetch_openrouter_providers(
    client: &reqwest::Client,
) -> Result<Vec<DiscoveredProvider>, ProviderError> {
    let json = fetch_json_with_cache(
        client,
        OPENROUTER_MODELS_URL,
        "openrouter.json",
        Duration::from_secs(60 * 60 * 24),
    )
    .await?;
    Ok(parse_openrouter_json(json))
}

pub fn merge_discovery_sources(
    models_dev: Vec<DiscoveredProvider>,
    litellm: Vec<DiscoveredProvider>,
    openrouter: Vec<DiscoveredProvider>,
) -> Vec<DiscoveredProvider> {
    let mut merged: BTreeMap<String, DiscoveredProvider> = BTreeMap::new();

    for provider in models_dev.into_iter().chain(litellm.into_iter()).chain(openrouter.into_iter())
    {
        let entry = merged.entry(provider.id.clone()).or_insert_with(|| provider.clone());

        entry.model_count = entry.model_count.max(provider.model_count);
        entry.openai_compatible |= provider.openai_compatible;
        entry.supports_tools |= provider.supports_tools;
        entry.supports_vision |= provider.supports_vision;
        entry.supports_audio |= provider.supports_audio;
        entry.max_context_window = Some(
            entry
                .max_context_window
                .unwrap_or(0)
                .max(provider.max_context_window.unwrap_or(0)),
        )
        .filter(|context| *context > 0);

        if entry.avg_input_price_per_million.is_none() {
            entry.avg_input_price_per_million = provider.avg_input_price_per_million;
        }
        if entry.avg_output_price_per_million.is_none() {
            entry.avg_output_price_per_million = provider.avg_output_price_per_million;
        }
        if entry.docs_url.is_none() {
            entry.docs_url = provider.docs_url.clone();
        }
        if entry.website.is_none() {
            entry.website = provider.website.clone();
        }
        if entry.sample_models.is_empty() && !provider.sample_models.is_empty() {
            entry.sample_models = provider.sample_models.clone();
        }

        if entry.base_url.is_none() {
            entry.base_url = provider.base_url.clone();
        }

        let mut source_parts: BTreeSet<String> = entry
            .source
            .split('+')
            .filter(|part| !part.is_empty())
            .map(ToString::to_string)
            .collect();
        for source in provider.source.split('+') {
            if !source.is_empty() {
                source_parts.insert(source.to_string());
            }
        }
        entry.source = source_parts.into_iter().collect::<Vec<_>>().join("+");
    }

    merged.into_values().collect()
}

pub fn search_catalog(
    catalog: &DiscoveryCatalog,
    query: Option<&str>,
    category: Option<&str>,
    openai_compatible_only: bool,
) -> Vec<DiscoveredProvider> {
    let query_lower = query.map(|q| q.to_ascii_lowercase());

    catalog
        .providers
        .iter()
        .filter(|provider| {
            if let Some(category_filter) = category
                && provider.category != category_filter
            {
                return false;
            }

            if openai_compatible_only && !provider.openai_compatible {
                return false;
            }

            if let Some(q) = &query_lower {
                let target = format!("{} {} {}", provider.id, provider.name, provider.source)
                    .to_ascii_lowercase();
                target.contains(q)
            } else {
                true
            }
        })
        .cloned()
        .collect()
}

pub fn search_catalog_advanced(
    catalog: &DiscoveryCatalog,
    filter: CatalogFilter,
) -> Vec<DiscoveredProvider> {
    let query_lower = filter.query.map(|q| q.to_ascii_lowercase());

    catalog
        .providers
        .iter()
        .filter(|provider| {
            if let Some(category) = &filter.category
                && &provider.category != category
            {
                return false;
            }

            if filter.openai_compatible_only && !provider.openai_compatible {
                return false;
            }

            if filter.requires_tools && !provider.supports_tools {
                return false;
            }
            if filter.requires_vision && !provider.supports_vision {
                return false;
            }
            if filter.requires_audio && !provider.supports_audio {
                return false;
            }

            if let Some(max_price) = filter.max_input_price_per_million {
                match provider.avg_input_price_per_million {
                    Some(price) if price <= max_price => {}
                    _ => return false,
                }
            }

            if let Some(min_context) = filter.min_context_window {
                match provider.max_context_window {
                    Some(context) if context >= min_context => {}
                    _ => return false,
                }
            }

            if let Some(query) = &query_lower {
                let target = format!(
                    "{} {} {} {}",
                    provider.id,
                    provider.name,
                    provider.source,
                    provider.sample_models.join(" ")
                )
                .to_ascii_lowercase();
                target.contains(query)
            } else {
                true
            }
        })
        .cloned()
        .collect()
}

pub async fn refresh_discovery_catalog(
    client: &reqwest::Client,
) -> Result<DiscoveryCatalog, ProviderError> {
    let models_dev = fetch_models_dev_providers(client).await?;
    let litellm = fetch_litellm_providers(client).await.unwrap_or_default();
    let openrouter = fetch_openrouter_providers(client).await.unwrap_or_default();

    let providers = merge_discovery_sources(models_dev, litellm, openrouter);

    let catalog = DiscoveryCatalog {
        generated_at_unix: now_unix_seconds(),
        providers,
    };

    let cache_dir = ensure_cache_dir()?;
    let cache_file = cache_dir.join("providers.json");
    let content = serde_json::to_string_pretty(&catalog)?;
    fs::write(&cache_file, content).map_err(|err| ProviderError::InvalidConfig {
        provider: "discovery".to_string(),
        detail: format!("failed writing merged provider cache {}: {err}", cache_file.display()),
    })?;

    Ok(catalog)
}

pub fn start_catalog_auto_refresh(client: reqwest::Client, interval: Duration) -> JoinHandle<()> {
    tokio::spawn(async move {
        loop {
            let _ = refresh_discovery_catalog(&client).await;
            sleep(interval).await;
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parses_models_dev_shape() {
        let input = json!({
            "openai": {
                "name": "OpenAI",
                "api": "https://api.openai.com/v1",
                "npm": "@ai-sdk/openai",
                "models": {
                    "gpt-4o": {},
                    "gpt-4o-mini": {}
                }
            }
        });

        let providers = parse_models_dev_json(input);
        assert_eq!(providers.len(), 1);
        assert_eq!(providers[0].id, "openai");
        assert_eq!(providers[0].model_count, 2);
        assert!(providers[0].openai_compatible);
    }

    #[test]
    fn merges_and_deduplicates_sources() {
        let a = vec![DiscoveredProvider {
            id: "openai".to_string(),
            name: "OpenAI".to_string(),
            source: "models.dev".to_string(),
            category: "major-cloud".to_string(),
            base_url: Some("https://api.openai.com/v1".to_string()),
            model_count: 2,
            openai_compatible: true,
            supports_tools: true,
            supports_vision: true,
            supports_audio: false,
            max_context_window: Some(128000),
            avg_input_price_per_million: Some(0.5),
            avg_output_price_per_million: Some(1.5),
            docs_url: Some("https://example.com".to_string()),
            website: None,
            sample_models: vec!["gpt-4o".to_string()],
        }];

        let b = vec![DiscoveredProvider {
            id: "openai".to_string(),
            name: "OpenAI".to_string(),
            source: "litellm".to_string(),
            category: "major-cloud".to_string(),
            base_url: None,
            model_count: 8,
            openai_compatible: true,
            supports_tools: true,
            supports_vision: false,
            supports_audio: false,
            max_context_window: Some(200000),
            avg_input_price_per_million: None,
            avg_output_price_per_million: None,
            docs_url: None,
            website: Some("https://openai.com".to_string()),
            sample_models: vec!["gpt-5".to_string()],
        }];

        let merged = merge_discovery_sources(a, b, Vec::new());
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].model_count, 8);
        assert!(merged[0].source.contains("models.dev"));
        assert!(merged[0].source.contains("litellm"));
        assert_eq!(merged[0].max_context_window, Some(200000));
        assert!(merged[0].supports_tools);
    }

    #[test]
    fn advanced_filter_works() {
        let catalog = DiscoveryCatalog {
            generated_at_unix: 0,
            providers: vec![DiscoveredProvider {
                id: "openai".to_string(),
                name: "OpenAI".to_string(),
                source: "models.dev".to_string(),
                category: "major-cloud".to_string(),
                base_url: Some("https://api.openai.com/v1".to_string()),
                model_count: 3,
                openai_compatible: true,
                supports_tools: true,
                supports_vision: true,
                supports_audio: true,
                max_context_window: Some(200000),
                avg_input_price_per_million: Some(2.0),
                avg_output_price_per_million: Some(8.0),
                docs_url: None,
                website: None,
                sample_models: vec!["gpt-4o-mini".to_string()],
            }],
        };

        let filtered = search_catalog_advanced(
            &catalog,
            CatalogFilter {
                query: Some("gpt-4o".to_string()),
                category: Some("major-cloud".to_string()),
                openai_compatible_only: true,
                max_input_price_per_million: Some(3.0),
                min_context_window: Some(100000),
                requires_tools: true,
                requires_vision: true,
                requires_audio: false,
            },
        );

        assert_eq!(filtered.len(), 1);
    }

    #[test]
    fn extracts_unique_litellm_providers() {
        let models_dev = vec![DiscoveredProvider {
            id: "openai".to_string(),
            name: "OpenAI".to_string(),
            source: "models.dev".to_string(),
            category: "major-cloud".to_string(),
            base_url: Some("https://api.openai.com/v1".to_string()),
            model_count: 10,
            openai_compatible: true,
            supports_tools: true,
            supports_vision: true,
            supports_audio: false,
            max_context_window: Some(200000),
            avg_input_price_per_million: Some(1.0),
            avg_output_price_per_million: Some(4.0),
            docs_url: None,
            website: None,
            sample_models: vec!["gpt-4o-mini".to_string()],
        }];

        let litellm = vec![
            DiscoveredProvider {
                id: "openai".to_string(),
                name: "OpenAI".to_string(),
                source: "litellm".to_string(),
                category: "major-cloud".to_string(),
                base_url: None,
                model_count: 10,
                openai_compatible: true,
                supports_tools: true,
                supports_vision: true,
                supports_audio: false,
                max_context_window: None,
                avg_input_price_per_million: None,
                avg_output_price_per_million: None,
                docs_url: None,
                website: None,
                sample_models: vec![],
            },
            DiscoveredProvider {
                id: "friendliai".to_string(),
                name: "FriendliAI".to_string(),
                source: "litellm".to_string(),
                category: "fast-inference".to_string(),
                base_url: None,
                model_count: 5,
                openai_compatible: true,
                supports_tools: true,
                supports_vision: false,
                supports_audio: false,
                max_context_window: None,
                avg_input_price_per_million: None,
                avg_output_price_per_million: None,
                docs_url: None,
                website: None,
                sample_models: vec![],
            },
        ];

        let unique = extract_unique_litellm_providers_not_in_models_dev(&models_dev, &litellm);
        assert_eq!(unique.len(), 1);
        assert_eq!(unique[0].id, "friendliai");
    }
}
