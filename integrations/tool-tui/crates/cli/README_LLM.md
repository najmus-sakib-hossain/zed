
# Local LLM Integration

Fast local LLM inference in Rust with multiple backend support.

## Quick Start

```bash

# Initialize LLM configuration

dx llm init --backend candle


# Download a model from Hugging Face

dx llm download google/gemma-2-2b-it


# Test inference

dx llm test --prompt "Hello, how are you?" --stream


# List downloaded models

dx llm list


# View configuration

dx llm config
```

## Backends

### 1. Candle (Pure Rust)

- **Pros**: No external dependencies, CUDA/Metal support
- **Cons**: Slightly slower than llama.cpp for some models
- **Best for**: Distribution, cross-platform compatibility

```bash
dx llm init --backend candle --model google/gemma-2-2b-it
```

### 2. Ollama (API)

- **Pros**: Easy setup, model management included
- **Cons**: Requires Ollama server running
- **Best for**: Development, quick testing

```bash

# Start Ollama first: ollama serve

dx llm init --backend ollama
dx llm test --prompt "Hello"
```

### 3. Khroma (API)

- **Pros**: Custom API support
- **Cons**: Requires external service
- **Best for**: Production deployments

```bash
dx llm init --backend khroma
```

## Configuration

Config file: `~/.config/dx/llm.toml`

See `llm.example.toml` for all options.

## Chat Integration

The chat UI (`dx chat`) automatically uses the configured LLM backend:

```bash
dx chat
```

## Performance

- **Candle**: ~20-30 tokens/sec (CPU), ~100+ tokens/sec (GPU)
- **Ollama**: Depends on server configuration
- **llama.cpp**: Coming soon (fastest quantized inference)

## Models

Recommended models for local inference:

- `google/gemma-2-2b-it` - 2B params, fast, good quality
- `microsoft/phi-2` - 2.7B params, excellent reasoning
- `TinyLlama/TinyLlama-1.1B-Chat-v1.0` - 1.1B params, very fast

## Troubleshooting

**Model download fails:**
```bash

# Set HF token for private models

export HF_TOKEN=hf_...
dx llm download model-name
```

**Out of memory:**
- Use smaller models (2B or less)
- Enable quantization (coming soon)
- Use Ollama with quantized models

**Slow inference:**
- Enable CUDA: `cargo build --features cuda`
- Use GPU-optimized models
- Try Ollama with GGUF quantized models
