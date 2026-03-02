# Models.dev Integration for 75+ LLM Providers

## Overview

This document describes the integration of models.dev API into Zed to support 75+ LLM providers dynamically.

## Architecture

### Components

1. **ModelsDevRegistry** (`crates/language_models/src/provider/models_dev.rs`)
   - Fetches provider data from https://models.dev/api.json
   - Caches provider information locally
   - Provides access to provider metadata

2. **ModelsDevLanguageModelProvider**
   - Dynamic provider that wraps models.dev provider data
   - Implements `LanguageModelProvider` trait
   - Handles authentication via environment variables

3. **ModelsDevLanguageModel**
   - Individual model implementation
   - Uses OpenAI-compatible API format
   - Supports streaming completions

### Supported Providers (75+)

The models.dev API provides access to providers including:
- **evroc**: Llama 3.3, Phi-4, Qwen3, Mistral models
- **zai** (Z.AI): GLM-5, GLM-4.5/4.6/4.7 series
- **zenmux** (ZenMux): MiMo, KAT-Coder, Step, Ling, Ring, Doubao, DeepSeek, Kimi, ERNIE, Gemini, Grok, GPT-5, MiniMax, Claude
- **io-net** (IO.NET): GLM, DeepSeek R1, Qwen Coder, Kimi, Llama, Mistral, GPT-OSS
- **nvidia** (Nvidia): Nemotron, Parakeet, Cosmos models

And many more!

## Implementation Status

### ✅ Completed
- [x] Created `ModelsDevRegistry` for fetching provider data
- [x] Created `ModelsDevLanguageModelProvider` for dynamic providers
- [x] Created `ModelsDevLanguageModel` for individual models
- [x] Added provider module registration

### 🚧 TODO
- [ ] Implement streaming completion using OpenAI-compatible API
- [ ] Add provider registration in `language_models.rs`
- [ ] Create settings UI for enabling/disabling providers
- [ ] Add local caching of provider data
- [ ] Implement automatic provider refresh
- [ ] Add provider filtering by capabilities
- [ ] Create provider configuration UI
- [ ] Add telemetry for models.dev providers
- [ ] Implement rate limiting per provider
- [ ] Add cost tracking for paid models

## Usage

### Enabling Models.dev Providers

1. **Automatic Discovery**: On startup, Zed will fetch the latest provider list from models.dev

2. **Authentication**: Set environment variables for each provider you want to use:
   ```bash
   export EVROC_API_KEY="your-key"
   export ZHIPU_API_KEY="your-key"
   export ZENMUX_API_KEY="your-key"
   # ... etc
   ```

3. **Model Selection**: All models from authenticated providers will appear in the model selector

### Configuration

Add to your Zed settings:

```json
{
  "language_models": {
    "models_dev": {
      "enabled": true,
      "auto_refresh": true,
      "refresh_interval_hours": 24,
      "enabled_providers": [
        "evroc",
        "zai",
        "zenmux",
        "io-net",
        "nvidia"
      ]
    }
  }
}
```

## API Integration

### Models.dev API Structure

```json
{
  "provider_id": {
    "id": "provider_id",
    "name": "Provider Name",
    "api": "https://api.provider.com/v1",
    "env": ["PROVIDER_API_KEY"],
    "models": {
      "model_id": {
        "id": "model_id",
        "name": "Model Name",
        "cost": {
          "input": 0.5,
          "output": 1.5
        },
        "limit": {
          "context": 128000,
          "output": 4096
        },
        "tool_call": true,
        "reasoning": true
      }
    }
  }
}
```

### OpenAI-Compatible API

Most providers use OpenAI-compatible endpoints:

```
POST {api_url}/chat/completions
Authorization: Bearer {api_key}

{
  "model": "model_id",
  "messages": [...],
  "stream": true
}
```

## Benefits

1. **Massive Provider Support**: 75+ providers vs current 13
2. **Automatic Updates**: New providers added to models.dev are automatically available
3. **Unified Interface**: All providers use consistent API format
4. **Cost Transparency**: Model pricing information included
5. **Capability Discovery**: Automatic detection of tool calling, reasoning, multimodal support

## Next Steps

To complete the integration:

1. **Implement Streaming**: Add OpenAI-compatible streaming in `ModelsDevLanguageModel::stream_completion`
2. **Register Providers**: Update `language_models.rs` to fetch and register models.dev providers
3. **Add Settings**: Create UI for managing models.dev providers
4. **Test Integration**: Verify with multiple providers
5. **Documentation**: Add user-facing documentation

## Example: Adding a New Provider

When a new provider is added to models.dev, it becomes available automatically:

1. Provider added to https://models.dev/api.json
2. Zed fetches updated provider list (on startup or refresh)
3. User sets environment variable (e.g., `NEW_PROVIDER_API_KEY`)
4. Models appear in model selector
5. User can start using the models immediately

No code changes required!

## Comparison with OpenCode

| Feature | OpenCode | Zed (with models.dev) |
|---------|----------|----------------------|
| Providers | 75+ | 75+ (same source) |
| API Source | models.dev | models.dev |
| Integration | TypeScript | Rust |
| Caching | Local JSON | In-memory + optional disk |
| Updates | Periodic fetch | On-demand + periodic |
| Configuration | Environment vars | Environment vars + UI |

## Resources

- Models.dev API: https://models.dev/api.json
- OpenCode Integration: `integrations/opencode/packages/opencode/src/provider/models.ts`
- Provider Documentation: https://models.dev/

## Contributing

To add support for a specific provider:

1. Ensure the provider is listed on models.dev
2. Test authentication with the provider's API key
3. Verify OpenAI-compatible API format
4. Report any issues with provider integration

---

**Status**: Initial implementation complete, streaming and registration pending
**Last Updated**: 2026-02-27
