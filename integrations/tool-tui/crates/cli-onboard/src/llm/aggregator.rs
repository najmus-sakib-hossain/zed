use crate::llm::error::ProviderError;
use serde::Deserialize;
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum AggregatorFormat {
    OpenAiModels,
    LiteLlmModelInfo,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct AggregatorEndpoint {
    pub provider_id: String,
    pub base_url: String,
    pub api_key: String,
    pub format: AggregatorFormat,
}

#[derive(Debug, Clone, Default, Eq, PartialEq)]
pub struct AggregatorModelIndex {
    pub provider_to_models: BTreeMap<String, Vec<String>>,
    pub model_to_providers: BTreeMap<String, Vec<String>>,
}

impl AggregatorModelIndex {
    pub fn route_provider_for_model(&self, model: &str) -> Option<String> {
        self.model_to_providers
            .get(model)
            .and_then(|providers| providers.first().cloned())
    }
}

#[derive(Debug, Deserialize)]
struct OpenAiModelsResponse {
    data: Vec<OpenAiModel>,
}

#[derive(Debug, Deserialize)]
struct OpenAiModel {
    id: String,
}

pub async fn fetch_aggregator_model_lists(
    client: &reqwest::Client,
    endpoints: &[AggregatorEndpoint],
) -> Result<AggregatorModelIndex, ProviderError> {
    let mut index = AggregatorModelIndex::default();

    for endpoint in endpoints {
        let models = match endpoint.format {
            AggregatorFormat::OpenAiModels => {
                fetch_openai_models(client, &endpoint.base_url, &endpoint.api_key).await?
            }
            AggregatorFormat::LiteLlmModelInfo => {
                fetch_litellm_models(client, &endpoint.base_url, &endpoint.api_key).await?
            }
        };

        for model in models {
            index
                .provider_to_models
                .entry(endpoint.provider_id.clone())
                .or_default()
                .push(model.clone());
            index
                .model_to_providers
                .entry(model)
                .or_default()
                .push(endpoint.provider_id.clone());
        }
    }

    for providers in index.model_to_providers.values_mut() {
        providers.sort();
        providers.dedup();
    }

    for models in index.provider_to_models.values_mut() {
        models.sort();
        models.dedup();
    }

    Ok(index)
}

async fn fetch_openai_models(
    client: &reqwest::Client,
    base_url: &str,
    api_key: &str,
) -> Result<Vec<String>, ProviderError> {
    let url = format!("{}/models", base_url.trim_end_matches('/'));
    let response = client.get(url).bearer_auth(api_key).send().await?;
    if !response.status().is_success() {
        let status = response.status().as_u16();
        let body = response.text().await.unwrap_or_default();
        return Err(ProviderError::HttpStatus {
            provider: "aggregator-model-list".to_string(),
            status,
            body,
        });
    }

    let payload: OpenAiModelsResponse = response.json().await?;
    Ok(payload.data.into_iter().map(|model| model.id).collect())
}

async fn fetch_litellm_models(
    client: &reqwest::Client,
    base_url: &str,
    api_key: &str,
) -> Result<Vec<String>, ProviderError> {
    let url = format!("{}/model/info", base_url.trim_end_matches('/'));
    let response = client.get(url).bearer_auth(api_key).send().await?;
    if !response.status().is_success() {
        let status = response.status().as_u16();
        let body = response.text().await.unwrap_or_default();
        return Err(ProviderError::HttpStatus {
            provider: "aggregator-model-list".to_string(),
            status,
            body,
        });
    }

    let payload: Value = response.json().await?;
    let models = payload
        .get("data")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|item| {
                    item.get("model_name")
                        .or_else(|| item.get("model"))
                        .and_then(Value::as_str)
                        .map(ToString::to_string)
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    Ok(models)
}

#[cfg(test)]
mod tests {
    use super::*;
    use httpmock::Method::GET;
    use httpmock::MockServer;

    #[tokio::test]
    async fn fetches_and_routes_aggregator_models() {
        let server = MockServer::start();

        let _openai_models = server.mock(|when, then| {
            when.method(GET).path("/v1/models").header("authorization", "Bearer token-a");
            then.status(200).header("content-type", "application/json").body(
                r#"{"data":[{"id":"openai/gpt-4o-mini"},{"id":"anthropic/claude-sonnet-4.5"}]}"#,
            );
        });

        let _litellm_info = server.mock(|when, then| {
            when.method(GET)
                .path("/model/info")
                .header("authorization", "Bearer token-b");
            then.status(200)
                .header("content-type", "application/json")
                .body(
                    r#"{"data":[{"model_name":"openai/gpt-4o-mini"},{"model_name":"groq/llama-3.1-8b-instant"}]}"#,
                );
        });

        let client = reqwest::Client::new();
        let index = fetch_aggregator_model_lists(
            &client,
            &[
                AggregatorEndpoint {
                    provider_id: "vercel-ai-gateway".to_string(),
                    base_url: format!("{}/v1", server.base_url()),
                    api_key: "token-a".to_string(),
                    format: AggregatorFormat::OpenAiModels,
                },
                AggregatorEndpoint {
                    provider_id: "litellm-proxy".to_string(),
                    base_url: server.base_url(),
                    api_key: "token-b".to_string(),
                    format: AggregatorFormat::LiteLlmModelInfo,
                },
            ],
        )
        .await
        .expect("model index");

        assert!(index.provider_to_models.contains_key("vercel-ai-gateway"));
        assert!(index.provider_to_models.contains_key("litellm-proxy"));
        assert_eq!(
            index.route_provider_for_model("groq/llama-3.1-8b-instant"),
            Some("litellm-proxy".to_string())
        );
        assert_eq!(
            index.route_provider_for_model("openai/gpt-4o-mini"),
            Some("litellm-proxy".to_string())
        );
    }
}
