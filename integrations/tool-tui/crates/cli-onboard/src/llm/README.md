# DX LLM Provider System

## OpenCode Integration - Free Models! 🎉

DX now includes **free LLM access** through OpenCode's promotional models. No API keys, no signup, just instant AI-powered development.

### Quick Start

```rust
use dx_onboard::llm::{ProviderRegistry, OPENCODE_FREE_MODELS};

// Register all providers (including OpenCode)
let mut registry = ProviderRegistry::new();
registry.register_openai_compatible_presets();

// Get OpenCode provider
let provider = registry.get("opencode").unwrap();

// Use any of the 3 free models
println!("Free models: {:?}", OPENCODE_FREE_MODELS);
// ["zai/glm-4.7-flash", "nvidia/llama-3.1-nemotron-70b-instruct", "zenmux/xiaomi/mimo-v2-flash-free", "zenmux/z-ai/glm-4.6v-flash-free"]
```

### Available Free Models

1. **GLM 4.7 Flash** (`zai/glm-4.7-flash`)
   - Fast, reliable, great for code generation
   - 200K+ context window
   - Recommended for most use cases

2. **Llama 3.1 Nemotron 70B** (`nvidia/llama-3.1-nemotron-70b-instruct`)
   - Powerful reasoning capabilities
   - 128K context window
   - Best for complex tasks

3. **MiMo V2 Flash** (`zenmux/xiaomi/mimo-v2-flash-free`)
   - Fast and efficient
   - Good for quick responses
   - Lightweight option

4. **GLM 4.6V Flash** (`zenmux/z-ai/glm-4.6v-flash-free`)
   - Vision support (multimodal)
   - Image understanding capabilities
   - Great for visual tasks

### Why These Models Are Free

OpenCode partners with model providers to offer free access during feedback phases. This is a **legitimate promotional strategy**, not a security vulnerability. The models are:

- ✅ Intentionally public
- ✅ Approved by model providers
- ✅ Part of OpenCode's open-source platform (MIT licensed)
- ✅ Used for collecting feedback and improving models

### Usage in Onboarding

The OpenCode provider is automatically registered when you call:

```rust
registry.register_openai_compatible_presets();
```

No API key required! The provider uses OpenCode's "public" key internally.

### Example: Chat Completion

```rust
use dx_onboard::llm::{ChatMessage, ChatRequest, MessageContent, LlmProvider};

let request = ChatRequest {
    model: "zai/glm-4.7-flash".to_string(),
    messages: vec![ChatMessage {
        role: "user".to_string(),
        content: MessageContent::Text("Write a hello world in Rust".to_string()),
        name: None,
    }],
    temperature: Some(0.7),
    max_tokens: Some(500),
    ..Default::default()
};

let response = provider.chat(request).await?;
println!("Response: {}", response.content);
```

### Attribution

When using OpenCode's free models, DX automatically provides attribution:

```
✓ Using free model via OpenCode Zen
  Powered by: https://opencode.ai
  Model: GLM 4.7 Flash (Z.AI)
```

### Benefits for DX Users

1. **Zero Friction** - No API keys, no signup, instant access
2. **High Quality** - Production-grade models from xAI, Z.AI, NVIDIA, Xiaomi
3. **Free Forever** - During promotional periods (check OpenCode docs for updates)
4. **Integrated** - Works seamlessly with DX's binary-first tooling
5. **Vision Support** - GLM 4.6V includes multimodal capabilities

### Learn More

- [OpenCode Website](https://opencode.ai)
- [OpenCode GitHub](https://github.com/anomalyco/opencode) (100k+ stars)
- [OpenCode Zen Pricing](https://opencode.ai/docs/zen/#pricing)
- [Free Models Discussion](https://github.com/anomalyco/opencode/issues/7479)

### Run the Demo

```bash
cargo run --example opencode_demo
```

This will test the OpenCode integration and show you how to use the free models in your code.
