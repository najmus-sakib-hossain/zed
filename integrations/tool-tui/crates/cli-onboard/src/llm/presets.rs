use crate::llm::types::{AuthRequirement, ProviderCapabilities, ProviderMetadata};

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ProviderPreset {
    pub id: &'static str,
    pub name: &'static str,
    pub category: &'static str,
    pub base_url: &'static str,
    pub api_key_env: &'static str,
    pub default_model: &'static str,
}

impl ProviderPreset {
    pub fn metadata(&self) -> ProviderMetadata {
        ProviderMetadata {
            id: self.id.to_string(),
            name: self.name.to_string(),
            category: self.category.to_string(),
            auth_requirement: if self.api_key_env.is_empty() {
                AuthRequirement::None
            } else {
                AuthRequirement::BearerToken
            },
            capabilities: ProviderCapabilities {
                chat: true,
                streaming: true,
                tools: true,
                vision: true,
                audio_input: true,
                audio_output: false,
                model_listing: true,
            },
            rate_limits: None,
            docs_url: None,
            website: None,
        }
    }
}

pub fn openai_compatible_provider_presets() -> Vec<ProviderPreset> {
    vec![
        // FREE MODELS - No API key required!
        ProviderPreset {
            id: "opencode",
            name: "OpenCode (Free Models)",
            category: "free",
            base_url: "https://api.opencode.ai/v1",
            api_key_env: "", // No API key needed - uses "public" key
            default_model: "zai/glm-4.7-flash",
        },
        ProviderPreset {
            id: "github-copilot",
            name: "GitHub Copilot",
            category: "popular",
            base_url: "https://api.githubcopilot.com",
            api_key_env: "GITHUB_COPILOT_TOKEN",
            default_model: "gpt-4o",
        },
        ProviderPreset {
            id: "openai",
            name: "OpenAI",
            category: "major-cloud",
            base_url: "https://api.openai.com",
            api_key_env: "OPENAI_API_KEY",
            default_model: "gpt-4o-mini",
        },
        ProviderPreset {
            id: "openrouter",
            name: "OpenRouter",
            category: "aggregator",
            base_url: "https://openrouter.ai/api",
            api_key_env: "OPENROUTER_API_KEY",
            default_model: "openai/gpt-oss-20b",
        },
        ProviderPreset {
            id: "groq",
            name: "Groq",
            category: "fast-inference",
            base_url: "https://api.groq.com/openai",
            api_key_env: "GROQ_API_KEY",
            default_model: "llama-3.1-8b-instant",
        },
        ProviderPreset {
            id: "together",
            name: "Together AI",
            category: "fast-inference",
            base_url: "https://api.together.xyz/v1",
            api_key_env: "TOGETHER_API_KEY",
            default_model: "openai/gpt-oss-20b",
        },
        ProviderPreset {
            id: "fireworks",
            name: "Fireworks AI",
            category: "fast-inference",
            base_url: "https://api.fireworks.ai/inference/v1",
            api_key_env: "FIREWORKS_API_KEY",
            default_model: "accounts/fireworks/models/qwen3-30b-a3b",
        },
        ProviderPreset {
            id: "deepinfra",
            name: "DeepInfra",
            category: "fast-inference",
            base_url: "https://api.deepinfra.com/v1/openai",
            api_key_env: "DEEPINFRA_API_KEY",
            default_model: "meta-llama/Meta-Llama-3.1-70B-Instruct",
        },
        ProviderPreset {
            id: "cerebras",
            name: "Cerebras",
            category: "fast-inference",
            base_url: "https://api.cerebras.ai/v1",
            api_key_env: "CEREBRAS_API_KEY",
            default_model: "llama-3.3-70b",
        },
        ProviderPreset {
            id: "hyperbolic",
            name: "Hyperbolic",
            category: "fast-inference",
            base_url: "https://api.hyperbolic.xyz/v1",
            api_key_env: "HYPERBOLIC_API_KEY",
            default_model: "meta-llama/Llama-3.1-70B-Instruct",
        },
        ProviderPreset {
            id: "novita",
            name: "Novita AI",
            category: "fast-inference",
            base_url: "https://api.novita.ai/v3/openai",
            api_key_env: "NOVITA_API_KEY",
            default_model: "meta-llama/llama-3.1-70b-instruct",
        },
        ProviderPreset {
            id: "lepton",
            name: "Lepton AI",
            category: "fast-inference",
            base_url: "https://llm.lepton.ai/api/v1",
            api_key_env: "LEPTON_API_KEY",
            default_model: "llama3-8b-instruct",
        },
        ProviderPreset {
            id: "nebius",
            name: "Nebius",
            category: "fast-inference",
            base_url: "https://api.studio.nebius.com/v1",
            api_key_env: "NEBIUS_API_KEY",
            default_model: "meta-llama/Meta-Llama-3.1-70B-Instruct",
        },
        ProviderPreset {
            id: "friendliai",
            name: "FriendliAI",
            category: "fast-inference",
            base_url: "https://api.friendli.ai/serverless/v1",
            api_key_env: "FRIENDLI_API_KEY",
            default_model: "meta-llama-3.1-70b-instruct",
        },
        ProviderPreset {
            id: "anyscale",
            name: "Anyscale",
            category: "fast-inference",
            base_url: "https://api.endpoints.anyscale.com/v1",
            api_key_env: "ANYSCALE_API_KEY",
            default_model: "meta-llama/Llama-3.1-70B-Instruct",
        },
        ProviderPreset {
            id: "baseten",
            name: "Baseten",
            category: "fast-inference",
            base_url: "https://inference.baseten.co/v1",
            api_key_env: "BASETEN_API_KEY",
            default_model: "meta-llama/Llama-3.1-70B-Instruct",
        },
        ProviderPreset {
            id: "predibase",
            name: "Predibase",
            category: "fast-inference",
            base_url: "https://serving.app.predibase.com/v1",
            api_key_env: "PREDIBASE_API_KEY",
            default_model: "deepseek-r1-distill-llama-70b",
        },
        ProviderPreset {
            id: "galadriel",
            name: "Galadriel",
            category: "fast-inference",
            base_url: "https://api.galadriel.com/v1",
            api_key_env: "GALADRIEL_API_KEY",
            default_model: "openai/gpt-oss-20b",
        },
        ProviderPreset {
            id: "cloudflare-workers-ai",
            name: "Cloudflare Workers AI",
            category: "aggregator",
            base_url: "https://api.cloudflare.com/client/v4/accounts/{account_id}/ai/v1",
            api_key_env: "CLOUDFLARE_API_TOKEN",
            default_model: "@cf/meta/llama-3.1-8b-instruct",
        },
        ProviderPreset {
            id: "vercel-ai-gateway",
            name: "Vercel AI Gateway",
            category: "aggregator",
            base_url: "https://ai-gateway.vercel.sh/v1",
            api_key_env: "AI_GATEWAY_API_KEY",
            default_model: "openai/gpt-4o-mini",
        },
        ProviderPreset {
            id: "litellm-proxy",
            name: "LiteLLM Proxy",
            category: "aggregator",
            base_url: "http://localhost:4000/v1",
            api_key_env: "LITELLM_API_KEY",
            default_model: "gpt-4o-mini",
        },
        ProviderPreset {
            id: "clarifai",
            name: "Clarifai",
            category: "aggregator",
            base_url: "https://api.clarifai.com/v2/ext/openai/v1",
            api_key_env: "CLARIFAI_API_KEY",
            default_model: "openai/gpt-4o-mini",
        },
        ProviderPreset {
            id: "huggingface",
            name: "Hugging Face Inference",
            category: "open-source-host",
            base_url: "https://router.huggingface.co/v1",
            api_key_env: "HF_TOKEN",
            default_model: "meta-llama/Meta-Llama-3.1-70B-Instruct",
        },
        ProviderPreset {
            id: "mistral",
            name: "Mistral",
            category: "open-source-host",
            base_url: "https://api.mistral.ai/v1",
            api_key_env: "MISTRAL_API_KEY",
            default_model: "mistral-small-latest",
        },
        ProviderPreset {
            id: "cohere",
            name: "Cohere",
            category: "open-source-host",
            base_url: "https://api.cohere.com/compatibility/v1",
            api_key_env: "COHERE_API_KEY",
            default_model: "command-r7b-12-2024",
        },
        ProviderPreset {
            id: "deepseek",
            name: "DeepSeek",
            category: "open-source-host",
            base_url: "https://api.deepseek.com/v1",
            api_key_env: "DEEPSEEK_API_KEY",
            default_model: "deepseek-chat",
        },
        ProviderPreset {
            id: "xai",
            name: "xAI",
            category: "open-source-host",
            base_url: "https://api.x.ai/v1",
            api_key_env: "XAI_API_KEY",
            default_model: "grok-3-mini",
        },
        ProviderPreset {
            id: "ai21",
            name: "AI21",
            category: "open-source-host",
            base_url: "https://api.ai21.com/studio/v1",
            api_key_env: "AI21_API_KEY",
            default_model: "jamba-1.5-mini",
        },
        ProviderPreset {
            id: "nlpcloud",
            name: "NLP Cloud",
            category: "open-source-host",
            base_url: "https://api.nlpcloud.io/v1",
            api_key_env: "NLPCLOUD_API_KEY",
            default_model: "finetuned-llama-3-70b",
        },
        ProviderPreset {
            id: "gooseai",
            name: "GooseAI",
            category: "open-source-host",
            base_url: "https://api.goose.ai/v1",
            api_key_env: "GOOSEAI_API_KEY",
            default_model: "gpt-neo-20b",
        },
        ProviderPreset {
            id: "nvidia",
            name: "NVIDIA NIM",
            category: "major-cloud",
            base_url: "https://integrate.api.nvidia.com/v1",
            api_key_env: "NVIDIA_API_KEY",
            default_model: "meta/llama-3.1-70b-instruct",
        },
        ProviderPreset {
            id: "alibaba-qwen",
            name: "Alibaba Qwen",
            category: "major-cloud",
            base_url: "https://dashscope.aliyuncs.com/compatible-mode/v1",
            api_key_env: "DASHSCOPE_API_KEY",
            default_model: "qwen-plus",
        },
        ProviderPreset {
            id: "azure-openai",
            name: "Azure OpenAI",
            category: "major-cloud",
            base_url: "https://{resource}.openai.azure.com/openai",
            api_key_env: "AZURE_OPENAI_API_KEY",
            default_model: "gpt-4o-mini",
        },
        ProviderPreset {
            id: "vertex-ai",
            name: "Google Vertex AI",
            category: "major-cloud",
            base_url: "https://{location}-aiplatform.googleapis.com/v1/projects/{project}/locations/{location}/endpoints/openapi",
            api_key_env: "GOOGLE_API_KEY",
            default_model: "gemini-2.5-flash",
        },
        ProviderPreset {
            id: "snowflake-cortex",
            name: "Snowflake Cortex",
            category: "major-cloud",
            base_url: "https://{account}.snowflakecomputing.com/api/v2/cortex/inference/openai/v1",
            api_key_env: "SNOWFLAKE_API_KEY",
            default_model: "llama3.1-70b",
        },
        ProviderPreset {
            id: "ibm-watsonx",
            name: "IBM watsonx.ai",
            category: "major-cloud",
            base_url: "https://{region}.ml.cloud.ibm.com/ml/v1-beta/openai",
            api_key_env: "WATSONX_API_KEY",
            default_model: "meta-llama/llama-3-70b-instruct",
        },
        ProviderPreset {
            id: "sap-ai-core",
            name: "SAP AI Core",
            category: "major-cloud",
            base_url: "https://api.ai.prod.eu-central-1.aws.ml.hana.ondemand.com/v2/lm/openai/v1",
            api_key_env: "SAP_AI_CORE_API_KEY",
            default_model: "gpt-4o-mini",
        },
        ProviderPreset {
            id: "lmstudio",
            name: "LM Studio",
            category: "local-runner",
            base_url: "http://127.0.0.1:1234/v1",
            api_key_env: "LMSTUDIO_API_KEY",
            default_model: "local-model",
        },
        ProviderPreset {
            id: "llama-cpp",
            name: "Llama.cpp Server",
            category: "local-runner",
            base_url: "http://127.0.0.1:8080/v1",
            api_key_env: "LLAMA_CPP_API_KEY",
            default_model: "local-model",
        },
        ProviderPreset {
            id: "vllm",
            name: "vLLM",
            category: "local-runner",
            base_url: "http://127.0.0.1:8000/v1",
            api_key_env: "VLLM_API_KEY",
            default_model: "local-model",
        },
        ProviderPreset {
            id: "localai",
            name: "LocalAI",
            category: "local-runner",
            base_url: "http://127.0.0.1:8080/v1",
            api_key_env: "LOCALAI_API_KEY",
            default_model: "local-model",
        },
        ProviderPreset {
            id: "tabbyapi",
            name: "TabbyAPI",
            category: "local-runner",
            base_url: "http://127.0.0.1:5000/v1",
            api_key_env: "TABBY_API_KEY",
            default_model: "local-model",
        },
        ProviderPreset {
            id: "docker-model-runner",
            name: "Docker Model Runner",
            category: "local-runner",
            base_url: "http://127.0.0.1:12434/v1",
            api_key_env: "DOCKER_MODEL_RUNNER_API_KEY",
            default_model: "local-model",
        },
        ProviderPreset {
            id: "zenmux",
            name: "ZenMux",
            category: "enterprise",
            base_url: "https://api.zenmux.ai/v1",
            api_key_env: "ZENMUX_API_KEY",
            default_model: "openai/gpt-4o-mini",
        },
        ProviderPreset {
            id: "302-ai",
            name: "302.AI",
            category: "enterprise",
            base_url: "https://api.302.ai/v1",
            api_key_env: "AI302_API_KEY",
            default_model: "openai/gpt-4o-mini",
        },
        ProviderPreset {
            id: "puter",
            name: "Puter",
            category: "open-source-host",
            base_url: "https://api.puter.com/v1",
            api_key_env: "PUTER_API_KEY",
            default_model: "gpt-4o-mini",
        },
        ProviderPreset {
            id: "cloudflare-ai-gateway",
            name: "Cloudflare AI Gateway",
            category: "aggregator",
            base_url: "https://gateway.ai.cloudflare.com/v1/{account_id}/{gateway_id}/openai",
            api_key_env: "CLOUDFLARE_API_TOKEN",
            default_model: "openai/gpt-4o-mini",
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    #[test]
    fn has_large_preset_batch() {
        let presets = openai_compatible_provider_presets();
        assert!(presets.len() >= 36);
    }

    #[test]
    fn preset_ids_are_unique() {
        let presets = openai_compatible_provider_presets();
        let ids: BTreeSet<&str> = presets.iter().map(|preset| preset.id).collect();
        assert_eq!(ids.len(), presets.len());
    }

    #[test]
    fn includes_key_provider_categories() {
        let presets = openai_compatible_provider_presets();
        let ids: BTreeSet<&str> = presets.iter().map(|preset| preset.id).collect();

        for required in [
            "github-copilot",
            "openai",
            "openrouter",
            "groq",
            "together",
            "fireworks",
            "hyperbolic",
            "novita",
            "lepton",
            "deepinfra",
            "cerebras",
            "baseten",
            "predibase",
            "friendliai",
            "anyscale",
            "litellm-proxy",
            "vercel-ai-gateway",
            "cloudflare-workers-ai",
            "cloudflare-ai-gateway",
            "clarifai",
            "huggingface",
            "mistral",
            "cohere",
            "deepseek",
            "xai",
            "ai21",
            "nlpcloud",
            "gooseai",
            "nvidia",
            "alibaba-qwen",
            "azure-openai",
            "vertex-ai",
            "ibm-watsonx",
            "sap-ai-core",
            "snowflake-cortex",
            "lmstudio",
            "llama-cpp",
            "vllm",
            "localai",
            "tabbyapi",
            "docker-model-runner",
            "zenmux",
            "302-ai",
            "puter",
        ] {
            assert!(ids.contains(required), "missing preset: {required}");
        }
    }
}
