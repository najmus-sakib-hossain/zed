use crate::llm::generic::GenericProvider;
use crate::llm::openai_compatible::OpenAiCompatibleProvider;
use crate::llm::types::{AuthRequirement, ProviderCapabilities, ProviderMetadata};

pub fn ibm_watsonx_provider(
    api_key: impl Into<String>,
    region: impl Into<String>,
    project_id: impl Into<String>,
    model: impl Into<String>,
) -> OpenAiCompatibleProvider {
    let region = region.into();
    let base_url = format!("https://{}.ml.cloud.ibm.com/ml/v1-beta/openai", region);

    let metadata = ProviderMetadata {
        id: "ibm-watsonx".to_string(),
        name: "IBM watsonx.ai".to_string(),
        category: "major-cloud".to_string(),
        auth_requirement: AuthRequirement::OAuth,
        capabilities: ProviderCapabilities {
            chat: true,
            streaming: true,
            tools: true,
            vision: true,
            audio_input: false,
            audio_output: false,
            model_listing: true,
        },
        rate_limits: None,
        docs_url: Some("https://www.ibm.com/products/watsonx-ai".to_string()),
        website: Some("https://www.ibm.com/watsonx".to_string()),
    };

    OpenAiCompatibleProvider::new("ibm-watsonx", base_url, api_key.into())
        .with_custom_header("x-project-id", project_id.into())
        .with_custom_header("x-model-id", model.into())
        .with_metadata(metadata)
}

pub fn sap_ai_core_provider(
    api_key: impl Into<String>,
    resource_group: impl Into<String>,
) -> OpenAiCompatibleProvider {
    let base_url = "https://api.ai.prod.eu-central-1.aws.ml.hana.ondemand.com/v2/lm/openai/v1";
    let metadata = ProviderMetadata {
        id: "sap-ai-core".to_string(),
        name: "SAP AI Core".to_string(),
        category: "major-cloud".to_string(),
        auth_requirement: AuthRequirement::OAuth,
        capabilities: ProviderCapabilities {
            chat: true,
            streaming: true,
            tools: true,
            vision: true,
            audio_input: false,
            audio_output: false,
            model_listing: true,
        },
        rate_limits: None,
        docs_url: Some("https://help.sap.com/docs/ai-core".to_string()),
        website: Some("https://www.sap.com/products/artificial-intelligence.html".to_string()),
    };

    OpenAiCompatibleProvider::new("sap-ai-core", base_url, api_key.into())
        .with_custom_header("ai-resource-group", resource_group.into())
        .with_metadata(metadata)
}

pub fn snowflake_cortex_provider(
    api_key: impl Into<String>,
    account: impl Into<String>,
    model: impl Into<String>,
) -> OpenAiCompatibleProvider {
    let account = account.into();
    let base_url =
        format!("https://{}.snowflakecomputing.com/api/v2/cortex/inference/openai/v1", account);

    let metadata = ProviderMetadata {
        id: "snowflake-cortex".to_string(),
        name: "Snowflake Cortex".to_string(),
        category: "major-cloud".to_string(),
        auth_requirement: AuthRequirement::HeaderApiKey,
        capabilities: ProviderCapabilities {
            chat: true,
            streaming: true,
            tools: true,
            vision: false,
            audio_input: false,
            audio_output: false,
            model_listing: true,
        },
        rate_limits: None,
        docs_url: Some("https://docs.snowflake.com/en/user-guide/snowflake-cortex".to_string()),
        website: Some("https://www.snowflake.com".to_string()),
    };

    OpenAiCompatibleProvider::new("snowflake-cortex", base_url, api_key.into())
        .with_custom_header("x-snowflake-model", model.into())
        .with_metadata(metadata)
}

pub fn register_enterprise_openai_compatible_from_env() -> Vec<GenericProvider> {
    let mut providers = Vec::new();

    if let (Ok(api_key), Ok(region), Ok(project), Ok(model)) = (
        std::env::var("WATSONX_API_KEY"),
        std::env::var("WATSONX_REGION"),
        std::env::var("WATSONX_PROJECT_ID"),
        std::env::var("WATSONX_MODEL"),
    ) {
        providers.push(ibm_watsonx_provider(api_key, region, project, model).into_inner());
    }

    if let (Ok(api_key), Ok(resource_group)) = (
        std::env::var("SAP_AI_CORE_API_KEY"),
        std::env::var("SAP_AI_CORE_RESOURCE_GROUP"),
    ) {
        providers.push(sap_ai_core_provider(api_key, resource_group).into_inner());
    }

    if let (Ok(api_key), Ok(account), Ok(model)) = (
        std::env::var("SNOWFLAKE_API_KEY"),
        std::env::var("SNOWFLAKE_ACCOUNT"),
        std::env::var("SNOWFLAKE_MODEL"),
    ) {
        providers.push(snowflake_cortex_provider(api_key, account, model).into_inner());
    }

    providers
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm::provider::LlmProvider;

    #[test]
    fn builds_watsonx_custom_provider() {
        let provider = ibm_watsonx_provider("token", "us-south", "project-1", "meta-llama");
        assert_eq!(provider.id(), "ibm-watsonx");
        assert!(provider.base_url().contains("us-south"));
    }

    #[test]
    fn builds_sap_custom_provider() {
        let provider = sap_ai_core_provider("token", "default");
        assert_eq!(provider.id(), "sap-ai-core");
    }

    #[test]
    fn builds_snowflake_custom_provider() {
        let provider = snowflake_cortex_provider("token", "acme-xy123", "llama3.1-70b");
        assert_eq!(provider.id(), "snowflake-cortex");
        assert!(provider.base_url().contains("acme-xy123"));
    }
}
