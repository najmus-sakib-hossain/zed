# DX Master Plan: The Unified Architecture Document

*Comprehensive technical plan combining all subsystems — 100+ LLM providers, LiteLLM features, background agents, computer use, voice input, AI avatar, system-wide grammar checking, edit prediction, and adaptive hardware-aware model selection.*

---

## Section 1: The Core Thesis

DX is a single Rust binary (plus bundled models) that replaces six separate products simultaneously:

| Product Replaced | What DX Does Instead |
|---|---|
| **LiteLLM** | 100+ LLM provider abstraction with cost tracking, budgets, rate limits, fallbacks |
| **Grammarly** | System-wide grammar checking, locally, under 10ms, free, unlimited |
| **Wispr Flow** | Voice dictation + command mode, locally via Whisper, free, unlimited |
| **Zed Edit Prediction** | Tab-accept edit prediction extended to every text field on the OS |
| **OpenClaw / ZeroClaw** | 24/7 background AI agent daemon with messaging channels and scheduling |
| **Anthropic Computer Use** | Real OS control — screenshots, mouse, keyboard, accessibility tree |

No product on Earth combines all six. DX is the first.

---

## Section 2: The Adaptive Hardware Detection & Model Selection System

This is the key differentiator that makes DX universally accessible — from a 10-year-old laptop with 4GB RAM to a Mac Studio with 192GB unified memory.

### 2A. Hardware Profiling Engine

At first launch, DX profiles the user's hardware to determine exactly what can run locally.

**Primary Rust crate: `hardware-query`**

A cross-platform Rust library for querying detailed system hardware information with advanced monitoring and power management capabilities. It offers cross-platform hardware detection (Windows, Linux, macOS), detailed CPU information (cores, threads, cache, features), GPU detection and capabilities (CUDA, ROCm, DirectML support).

The `hardware-query` crate is uniquely powerful because the library provides comprehensive detection for AI/ML-oriented hardware including NPUs (Intel Movidius, GNA, XDNA; Apple Neural Engine; Qualcomm Hexagon), TPUs (Google Cloud TPU and Edge TPU; Intel Habana), ARM Systems (Raspberry Pi, NVIDIA Jetson, Apple Silicon with power management), and FPGAs (Intel/Altera and Xilinx families with AI optimization scoring).

**Supporting crate: `system-analysis`**

A comprehensive Rust library for analyzing system capabilities, workload requirements, and optimal resource allocation. This crate provides tools for determining if a system can run specific workloads, scoring hardware capabilities, and recommending optimal configurations with a focus on AI/ML workloads. Features include comprehensive system analysis, AI/ML specialization with built-in support for AI inference and training workloads with model parameter analysis, workload modeling, compatibility checking, resource utilization prediction, and bottleneck detection.

**Supporting crate: `llmfit`**

Right-size LLM models to your system hardware. Interactive TUI and CLI to match models against available RAM, CPU, and GPU.

**Supporting crate: `silicon-monitor`**

Silicon Monitor is a powerful, cross-platform hardware monitoring utility designed primarily for AI agents and interactive interfaces. It provides deep insights into CPUs, GPUs, memory, disks, motherboards, and network interfaces across Windows, Linux, and macOS. With NVIDIA full NVML integration for all CUDA-capable GPUs, AMD ROCm/sysfs support for RDNA/CDNA architectures, Intel i915/xe driver support for Arc, Iris Xe, and Data Center GPUs, and a unified API with a single interface for all GPU vendors.

### 2B. The Five Device Tiers

DX classifies every device into one of five tiers based on the hardware scan, then selects the optimal model stack for each tier.

---

#### **Tier 1: Ultra-Low-End (2–4GB RAM, no GPU)**
*Examples: Raspberry Pi 4, Chromebooks, 10-year-old laptops, cheap Android phones*

**Target model stack:**

Microsoft's BitNet — ultra-efficient 1.58-bit weight model, exceptional for edge and CPU-only inference. Memory requirement: Only 0.4GB!

SmolLM2 is a family of compact models (135M, 360M, and 1.7B parameters). The 360M version is small enough to run on-device, making it one of the most efficient instruction-following models for mobile AI and embedded systems. It is highly optimized for efficiency and great for low-power devices and real-time AI.

| Purpose | Model | Quantization | RAM | Disk |
|---------|-------|-------------|-----|------|
| Grammar LLM (Tier 3) | SmolLM2-360M | Q4_K_M | ~300MB | ~200MB |
| Edit Prediction | SmolLM2-135M | Q4_K_M | ~150MB | ~100MB |
| Voice STT | Whisper Tiny.en | Native | ~100MB | ~75MB |
| Embeddings (memory) | all-MiniLM-L6-v2 | Native | ~50MB | ~23MB |
| **Total** | | | **~600MB** | **~400MB** |

Grammar checking relies primarily on Harper (rule-based, <1ms). The LLM is only invoked for Tier 3 complex suggestions and voice post-processing.

---

#### **Tier 2: Low-End (4–8GB RAM, no dedicated GPU or iGPU)**
*Examples: Entry-level laptops, older MacBooks, budget desktops, mid-range phones*

Qwen3-0.6B is the smallest dense model in Alibaba's Qwen3 family, released under the Apache 2.0 license. Despite its tiny size, it inherits strong reasoning, improved agent and tool-use capabilities, and broad multilingual support. Architecturally, it's a 0.6B parameter causal LM with 32K context length, supporting a hybrid behavior pattern that can switch between deeper reasoning and faster responses.

By December 2025, Qwen3-0.6B is among the most downloaded text generation models on Hugging Face. Strong capability for a sub-1B model. If you want something meaningfully stronger than "toy" small models, but still lightweight enough for low-cost deployments, Qwen3-0.6B is a solid baseline. In some evaluations, it's competitive even against much larger models like DeepSeek-R1-Distill-Llama-8B.

| Purpose | Model | Quantization | RAM | Disk |
|---------|-------|-------------|-----|------|
| General LLM | Qwen3-0.6B | Q4_K_M | ~500MB | ~400MB |
| Grammar LLM | Same Qwen3-0.6B | Shared | — | — |
| Edit Prediction | SmolLM2-360M | Q4_K_M | ~300MB | ~200MB |
| Voice STT | Whisper Tiny.en | Native | ~100MB | ~75MB |
| Embeddings | all-MiniLM-L6-v2 | Native | ~50MB | ~23MB |
| **Total** | | | **~950MB** | **~700MB** |

---

#### **Tier 3: Mid-Range (8–16GB RAM, iGPU or entry GPU)**
*Examples: Modern MacBook Air M1/M2, mid-range gaming PCs, business laptops*

SmolLM2 (1.7B) offers state-of-the-art accuracy among small models, while Gemma 3 (1B) maximizes efficiency and context length. DeepSeek R1 and Qwen 2.5 (both 1.5B) provide excellent reasoning with modest resources, and Llama 3.2 (3B) leverages Meta's optimizations for edge devices.

| Purpose | Model | Quantization | RAM | Disk |
|---------|-------|-------------|-----|------|
| General LLM | Qwen2.5-3B-Instruct | Q4_K_M | ~2.0GB | ~1.8GB |
| Grammar + Complex Suggestions | Same Qwen2.5-3B | Shared | — | — |
| Edit Prediction (code) | Qwen2.5-Coder-1.5B | Q5_K_M | ~1.2GB | ~1.0GB |
| Edit Prediction (prose) | SmolLM2-1.7B | Q4_K_M | ~1.2GB | ~1.0GB |
| Voice STT | Whisper Base.en | Native | ~200MB | ~142MB |
| Embeddings | all-MiniLM-L6-v2 | Native | ~50MB | ~23MB |
| **Total** | | | **~4.7GB** | **~4.0GB** |

---

#### **Tier 4: High-End (16–32GB RAM, discrete GPU with 6–12GB VRAM)**
*Examples: MacBook Pro M3 Pro/Max, RTX 3070/4070, developer workstations*

SmolLM3-3B is a fully open instruct and reasoning model from Hugging Face. At the 3B scale, it outperforms Llama-3.2-3B and Qwen2.5-3B, while staying competitive with many 4B-class alternatives across 12 popular LLM benchmarks. What sets SmolLM3 apart is the level of transparency — Hugging Face published the full engineering blueprint. It supports dual-mode reasoning with /think and /no_think, and a long context window trained to 64K that can stretch to 128K tokens with YaRN extrapolation.

| Purpose | Model | Quantization | RAM/VRAM | Disk |
|---------|-------|-------------|----------|------|
| General LLM | Mistral-7B-Instruct | Q5_K_M | ~6.5GB | ~5.1GB |
| Grammar + Style | SmolLM3-3B | Q5_K_M | ~2.5GB | ~2.0GB |
| Edit Prediction (code) | Zeta (Qwen2.5-Coder-7B) | Q4_K_M | ~4.5GB | ~3.8GB |
| Edit Prediction (prose) | Qwen2.5-3B-Instruct | Q5_K_M | ~2.5GB | ~2.0GB |
| Voice STT | Whisper Small.en | Native | ~400MB | ~244MB |
| Agent Tasks | Shared Mistral-7B | Shared | — | — |
| Embeddings | all-MiniLM-L6-v2 | Native | ~50MB | ~23MB |
| **Total** | | | **~16.5GB** | **~13.2GB** |

---

#### **Tier 5: Ultra-High-End (32GB+ RAM, high-end GPU with 16GB+ VRAM, or Apple Silicon with 64GB+ unified)**
*Examples: Mac Studio M2/M3 Ultra, RTX 4090, multi-GPU workstations*

These users get results comparable to paid cloud APIs — for free, locally.

M3 Max with 128GB can run 70B models at Q4_K_M at approximately 30–40 tokens/sec with full context.

| Purpose | Model | Quantization | RAM/VRAM | Disk |
|---------|-------|-------------|----------|------|
| General LLM | Qwen2.5-72B or Llama3.1-70B | Q4_K_M | ~40GB | ~38GB |
| Grammar + Deep Style | Qwen2.5-14B-Instruct | Q5_K_M | ~10GB | ~9GB |
| Edit Prediction (code) | Zeta / Qwen2.5-Coder-32B | Q4_K_M | ~20GB | ~18GB |
| Edit Prediction (prose) | Mistral-7B-Instruct | Q6_K | ~6GB | ~5.5GB |
| Voice STT | Whisper Large-v3 | Native | ~3GB | ~1.5GB |
| Agent Tasks | Shared 72B model | Shared | — | — |
| Vision (Computer Use) | LLaVA-1.5-7B | Q4_K_M | ~4.5GB | ~3.8GB |
| Embeddings | all-MiniLM-L6-v2 | Native | ~50MB | ~23MB |
| **Total** | | | **~84GB** | **~76GB** |

**This is where DX goes viral for rich users.** A 72B model running locally at Q4_K_M produces output quality approaching GPT-4-class for many tasks. They get cloud-quality AI, offline, unlimited, for free. Nobody else offers this.

### 2C. Quantization Strategy

Q4_K_M is the safe default for phones and lighter Macs. Q5_K_M improves detail and reasoning stability. Q8_0 is for when quality matters most and memory isn't a constraint.

GGML/GGUF Quantization uses an optimized quantization format for CPU inference with various levels (Q2_K, Q3_K, Q4_K, Q5_K, Q6_K, Q8_0) offering a good balance across different precision levels, optimized for CPU-optimized inference and consumer hardware deployment.

The quantization ladder DX follows:

For most users, stick to Q4_K_M → Q5_K_M → Q8_0 as your ladder.

Speed improvements compared to FP16: Q4_K_M achieves 3.0–4.0x faster inference, Q5_K_M achieves 2.5–3.2x faster, Q8_0 achieves 2.0–2.5x faster.

In practice, Q4_K_M is best used when hardware is the main constraint: CPU-only deployments, small GPUs, or situations where you need to squeeze a larger model onto limited VRAM.

Smaller models are generally less capable, so heavily compressing them increases the risk of a noticeable quality drop. This means for Tier 1 and Tier 2 devices, DX should prefer the smallest viable model at Q4_K_M rather than aggressively quantizing a larger model down to Q2_K.

Always budget ~1.2x the quantized model file size for actual memory usage, to allow for prompt and intermediate calculation storage.

### 2D. Smart Model Download & Management

**The first-launch experience:**

1. DX runs hardware profiling (takes < 2 seconds)
2. Classifies the device into a tier
3. Shows the user a single screen: "Your device can run [Tier X]. DX will download [total size]. This gives you: [capabilities list]. Download now?"
4. Downloads only the models needed for that tier
5. If the user later upgrades hardware (or plugs in an eGPU), DX re-scans and offers to upgrade models

**Progressive downloading:**
- Grammar engine (Harper) works immediately — zero download needed (bundled, <5MB)
- Whisper Tiny downloads first (~75MB) — voice input available within 30 seconds
- Edit prediction model downloads next
- General LLM downloads last (largest file)
- User has a functional product within 60 seconds of install, even on slow internet

**Model storage:**
- All models stored in `~/.dx/models/`
- Models shared with Ollama when detected (no duplicate downloads)
- Automatic cleanup of unused models after 30 days
- Total disk footprint clearly shown in settings

### 2E. Dynamic Model Swapping

DX doesn't just pick models at install time. It continuously monitors system resources and swaps models dynamically:

- **RAM pressure detected:** Swap from Q5_K_M to Q4_K_M variant, or unload edit prediction model temporarily
- **User plugged in laptop:** Enable GPU acceleration, load larger models
- **User on battery:** Swap to smaller models, reduce prediction frequency
- **User idle (daemon mode):** Load larger model for scheduled tasks (memory available since user isn't using apps)
- **Multiple DX features active simultaneously:** Share a single model for grammar + prediction + voice post-processing (one model, three use cases)

---

## Section 3: The Rust Inference Engine Strategy

### 3A. Primary Engine: Candle (by Hugging Face)

Candle's core goal is to make serverless inference possible. Full machine learning frameworks like PyTorch are very large, which makes creating instances on a cluster slow. Candle allows deployment of lightweight binaries. Secondly, Candle lets you remove Python from production workloads. Python overhead can seriously hurt performance, and the GIL is a notorious source of headaches. Finally, Rust is cool!

Candle provides quantization support using the llama.cpp quantized types.

Candle has an optimized CPU backend with optional MKL support for x86 and Accelerate for Macs. CUDA backend for efficiently running on GPUs, multiple GPU distribution via NCCL. Plus WASM support to run your models in a browser.

Building LLM applications in Rust using candle and llm crates reveals candle as the more viable choice due to its active development and broader hardware support. Candle 0.4.0 with CUDA 12.1 enables GPU acceleration for tensor operations. The llm crate, being archived and limited to GGMLv3 models, lacks support for modern formats like GGUF and newer hardware. For new projects, prioritize candle.

Candle supports an enormous range of models: LLaMA v1, v2, and v3, Falcon, Codegeex4, GLM4, Gemma v1 and v2, RecurrentGemma, Phi-1 through Phi-3, StableLM, Mamba, and Mistral 7b.

### 3B. Secondary Engine: Crane (Built on Candle)

Crane focuses on accelerating LLM inference speed with the power of kernels in the candle framework, while reducing development overhead, making it portable and fast to run models on both CPU and GPU. Crane is a Candle-based Rust Accelerated Neural Engine — a high-performance inference framework.

Crane bridges the gap through the Candle Framework combining Rust's efficiency with PyTorch-like ergonomics, Cross-Platform Acceleration with Metal GPU support achieving 3–5x speedup over CPU-only, and Simplified Deployment with the ability to add new models with less than 100 lines of code in most cases.

### 3C. Fallback Engine: llama-cpp-rs (FFI bindings to llama.cpp)

For maximum compatibility with the GGUF ecosystem and when Candle doesn't support a specific model architecture yet. This is the safety net.

### 3D. Pure-Rust Alternative: llama-gguf

A high-performance Rust implementation of llama.cpp — an LLM inference engine with full GGUF and ONNX support. Full GGUF support to load any GGUF model file compatible with llama.cpp, plus ONNX support to load HuggingFace Optimum ONNX exports.

### 3E. Candle Ecosystem Libraries

Kalosm provides a multi-modal meta-framework for interfacing with local pre-trained models, supporting controlled generation, custom samplers, and in-memory vector databases.

Atoma-infer is a Rust library for fast inference at scale, leveraging FlashAttention2 for efficient attention computation, PagedAttention for efficient KV-cache memory management, and multi-GPU support. It is OpenAI API compatible.

vllm.rs is a minimalist vLLM implementation in Rust based on Candle.

---

## Section 4: The Grammar Engine Deep Dive

### 4A. Primary Engine: Harper

Harper is an English grammar checker designed to be just right. Grammarly was too expensive and too overbearing. Its suggestions lacked context, and were often just plain wrong. Not to mention: it's a privacy nightmare. Everything you write with Grammarly is sent to their servers. LanguageTool is great, if you have gigabytes of RAM to spare and are willing to download the ~16GB n-gram dataset. Besides the memory requirements, LanguageTool was too slow: it would take several seconds to lint even a moderate-size document.

Harper's performance is extraordinary: Not only does it take milliseconds to lint a document, take less than 1/50th of LanguageTool's memory footprint, but it is also completely private. Harper is even small enough to load via WebAssembly.

Since Harper runs on your devices, it's able to serve up suggestions in under 10 milliseconds.

Harper is available as a language server, JavaScript library through WebAssembly, and Rust crate, so you can get fantastic grammar checking anywhere you work.

The core crate is `harper-core`: harper-core is the fundamental engine behind Harper, the grammar checker for developers. harper-core is available on crates.io.

Harper has a sophisticated internal architecture: Patterns are one of the more powerful ways to query text inside Harper, especially for beginners. They are a simplified abstraction over Expr. It includes code for performing dictionary lookups and spellchecking (i.e. fuzzy dictionary lookups) as well as Weir — a programming language for finding errors in natural language.

**Current limitation:** Harper currently only supports English, but the core is extensible to support other languages, so we welcome contributions that allow for other language support.

**DX's opportunity:** Contribute multi-language support to Harper upstream, or fork and extend internally. For non-English languages, fall back to the LLM-based Tier 3 grammar checking.

### 4B. Three-Tier Grammar Pipeline

| Tier | Engine | Speed | What It Catches | Always Active |
|------|--------|-------|----------------|---------------|
| **Tier 1** | Harper (`harper-core`) | <10ms | Spelling, punctuation, grammar rules, sentence structure, passive voice, wordiness | ✅ Yes |
| **Tier 2** | nlprule + Hunspell | <50ms | LanguageTool rule patterns (4000+ rules), multi-language spell check | ✅ Yes |
| **Tier 3** | Local LLM (tiered by device) | <500ms | Tone mismatch, subtle awkwardness, restructuring, context-aware suggestions, course correction | Debounced, on pause |

---

## Section 5: Complete Rust Crate Reference

This is the definitive, researched crate list for every DX subsystem.

### 5A. Hardware Detection & Model Selection

| Purpose | Crate | Notes |
|---------|-------|-------|
| **Full hardware profiling** | `hardware-query` | CPU, GPU, NPU, TPU, RAM, CUDA/ROCm/DirectML detection, power management |
| **AI workload analysis** | `system-analysis` | Determines if system can run specific models, bottleneck detection |
| **Model-to-hardware fitting** | `llmfit` | CLI/TUI to match models against available RAM, CPU, GPU |
| **Runtime monitoring** | `silicon-monitor` | Continuous GPU/CPU/memory monitoring for dynamic model swapping |
| **System info (lightweight)** | `sysinfo` | Basic CPU, memory, disk info — stable, well-maintained |
| **GPU info (NVIDIA)** | `nvml-wrapper` | NVIDIA Management Library bindings for detailed GPU metrics |

### 5B. Inference Engines

| Purpose | Crate | Notes |
|---------|-------|-------|
| **Primary ML framework** | `candle-core` + `candle-transformers` + `candle-nn` | Hugging Face's Rust ML framework, CUDA + Metal + CPU, GGUF quantization |
| **High-level inference** | `crane` (Candle-based) | Higher-level API over Candle, easy model integration |
| **llama.cpp FFI** | `llama-cpp-rs` or `llama-cpp-2` | Bindings to llama.cpp for maximum GGUF compatibility |
| **Pure-Rust GGUF** | `llama-gguf` | Native Rust llama.cpp reimplementation |
| **Multi-modal** | `kalosm` | Meta-framework: controlled generation, custom samplers, vector DB |
| **Fast inference** | `atoma-infer` | FlashAttention2 + PagedAttention + multi-GPU |
| **Model hub** | `hf-hub` | Download models from Hugging Face programmatically |

### 5C. Grammar & Language

| Purpose | Crate | Notes |
|---------|-------|-------|
| **Primary grammar engine** | `harper-core` | Under 10ms, 1/50th of LanguageTool's memory, Rust-native |
| **LanguageTool rules offline** | `nlprule` | 4000+ rules from LanguageTool, no Java dependency |
| **LanguageTool HTTP (optional)** | `languagetool-rust` | For users who want the full LT server |
| **Spell checking (multi-lang)** | `zspell` | Hunspell-compatible, 100+ languages |
| **Fuzzy matching** | `analiticcl` | Approximate string matching for spelling correction |
| **Text segmentation** | `unicode-segmentation` | Proper word/sentence boundaries for all Unicode |
| **Language detection** | `whichlang` or `lingua-rs` | Detect typing language, switch engine |

### 5D. Voice / Speech-to-Text

| Purpose | Crate | Notes |
|---------|-------|-------|
| **Whisper STT** | `whisper-rs` | Bindings to whisper.cpp, GPU-accelerated (Metal/CUDA) |
| **Streaming STT** | `whisper-cpp-plus` | Real-time PCM streaming, Silero VAD built-in |
| **Audio capture** | `cpal` | Cross-platform audio I/O (CoreAudio, WASAPI, ALSA/Pulse) |
| **Audio resampling** | `rubato` | High-quality resampling to 16kHz for Whisper |
| **DSP / processing** | `dasp` | Digital signal processing primitives |
| **VAD (standalone)** | `webrtc-vad` | Voice Activity Detection if not using whisper-cpp-plus |

### 5E. OS Integration (Input Interception, Accessibility, Overlay)

| Purpose | Crate / API | Notes |
|---------|-------------|-------|
| **Global hotkeys** | `global-hotkey` | Cross-platform (macOS, Windows, Linux), from Tauri ecosystem |
| **Selected text** | `get-selected-text` | Get selected text across all platforms |
| **Clipboard** | `arboard` | Cross-platform clipboard read/write |
| **macOS Accessibility** | `accessibility` / `macos-accessibility-client` | AXUIElement bindings for reading/writing text fields |
| **macOS ObjC FFI** | `objc2` | Safe Objective-C interop for IMK, CGEventTap |
| **Windows API** | `windows` crate | TSF, UI Automation, Win32 hooks |
| **Linux accessibility** | `atspi` | AT-SPI2 bindings for reading/writing text fields |
| **Linux input** | `input-linux` | Low-level input event handling |
| **System tray** | `tray-icon` | Cross-platform system tray icon |

### 5F. Computer Use (OS Control)

| Purpose | Crate | Notes |
|---------|-------|-------|
| **Mouse/keyboard (cross-platform)** | `rustautogui` | Works on Windows, Linux, macOS; no OpenCV needed |
| **Alternative automation** | `autopilot-rs` | Cross-platform GUI automation |
| **Screenshot** | `screenshots` crate | Cross-platform screenshot capture |
| **Template matching** | `rustautogui` built-in | Segmented Normalized Cross-Correlation |
| **Accessibility tree** | `accesskit` | Cross-platform accessibility toolkit |

### 5G. Background Agent Daemon

| Purpose | Crate | Notes |
|---------|-------|-------|
| **Async runtime** | `tokio` | Foundation for everything async |
| **Cron scheduling** | `cron` | Parse cron expressions for scheduled tasks |
| **Signal handling** | `signal-hook` | Unix signal handling for graceful shutdown |
| **Local database** | `rusqlite` | SQLite for agent memory, conversation history |
| **Telegram channel** | `teloxide` | Telegram Bot API |
| **Discord channel** | `serenity` | Discord Bot |
| **Slack channel** | `slack-morphism` | Slack API |
| **Matrix channel** | `matrix-sdk` | Matrix/Element protocol |

### 5H. 100+ LLM Provider Abstraction

| Purpose | Crate | Notes |
|---------|-------|-------|
| **HTTP client** | `reqwest` | All provider API calls |
| **SSE streaming** | `eventsource-stream` | Server-Sent Events parsing for streaming |
| **Async streams** | `async-stream` | Create async streams for streaming responses |
| **Rate limiting** | `governor` | Token-bucket rate limiting (RPM/TPM) |
| **Retry/backoff** | `backoff` or `again` | Exponential backoff with jitter |
| **AWS auth (Bedrock)** | `aws-sigv4` + `aws-credential-types` | SigV4 signing for Bedrock |
| **JSON serialization** | `serde` + `serde_json` | All API request/response handling |
| **YAML config** | `serde_yaml` | Proxy server YAML config |
| **TOML config** | `toml` | DX config files |

### 5I. Cost Tracking, Budgets, Proxy Server

| Purpose | Crate | Notes |
|---------|-------|-------|
| **Web framework (proxy)** | `axum` + `tower` | AI Gateway HTTP server |
| **Database** | `sqlx` | PostgreSQL for virtual keys, spend tracking |
| **Redis** | `redis` | Cache backend, rate limit state |
| **Concurrent maps** | `dashmap` | In-memory caches, rate limit counters |
| **Key hashing** | `argon2` / `sha2` | Virtual key hashing |
| **Tokenization** | `tiktoken-rs` | OpenAI token counting |
| **HF Tokenizers** | `tokenizers` | Hugging Face tokenizer library for non-OpenAI models |

### 5J. Observability

| Purpose | Crate | Notes |
|---------|-------|-------|
| **Structured logging** | `tracing` + `tracing-subscriber` | Entire DX logging infrastructure |
| **OpenTelemetry** | `opentelemetry` | Distributed tracing, Langfuse integration |
| **Prometheus** | `prometheus` | Metrics for proxy server |
| **Error handling** | `thiserror` + `anyhow` | Typed errors + ad-hoc errors |

### 5K. GUI / Rendering

| Purpose | Crate / Technology | Notes |
|---------|-------------------|-------|
| **Desktop UI** | GPUI (Zed's framework) | GPU-accelerated rendering |
| **Diff computation** | `similar` | Text diffs for edit prediction ghost text |
| **Pattern matching** | `aho-corasick` | Fast multi-pattern matching for guardrails |
| **Regex** | `regex` | Guardrails, PII detection |

---

## Section 6: The Complete Data Flow Architecture

### Flow A: User Types in Any App (System-Wide)

```
Keystroke → OS Input Interception (platform-specific)
    │
    ├─[< 1ms]─→ Harper (Tier 1 Grammar)
    │               └→ Red/Yellow underlines via Overlay
    │
    ├─[< 10ms]─→ nlprule (Tier 2 Grammar)
    │               └→ Additional underlines via Overlay
    │
    ├─[Debounced 300ms]─→ Edit Prediction Engine
    │   ├─ Detect context: code? prose? email? slack?
    │   ├─ Select model: Zeta for code, prose model for text
    │   ├─ Build context window: current paragraph + app name + clipboard
    │   ├─ Run inference on local LLM (tiered by device)
    │   └→ Ghost text via Overlay (Tab to accept)
    │
    ├─[Debounced 1s]─→ LLM Tier 3 Grammar (on sentence completion)
    │   ├─ Tone analysis (is this too casual for email?)
    │   ├─ Restructuring suggestions
    │   └→ Purple underlines via Overlay
    │
    └─[Background]─→ Writing Memory
        ├─ Record typing patterns
        ├─ Learn personal vocabulary
        └─ Update error frequency map
```

### Flow B: Voice Dictation

```
Hotkey Press → Flow Bar appears (GPUI overlay, bottom-center)
    → Audio capture (cpal)
    → VAD filters silence (Silero via whisper-cpp-plus)
    → Whisper transcribes in real-time
    → Partial results displayed in Flow Bar
    │
Hotkey Release → Full transcript assembled
    → LLM post-processor:
        ├─ Course correction ("no wait" → replace)
        ├─ Filler word removal ("um", "like")
        ├─ Context-aware formatting (detect active app)
        └─ Tone matching (email = formal, Slack = casual)
    → Final cleaned text
    → Insert at cursor via Accessibility API
    → Writing Memory records the dictation
```

### Flow C: Edit Prediction (System-Wide)

```
User pauses typing (debounce: 300ms) → Prediction Pipeline
    │
    ├─ Read context: current line + surrounding paragraph
    ├─ Detect app context (email client? code editor? chat?)
    ├─ Select appropriate model:
    │   ├─ Code editor → Zeta/Coder model (code completion)
    │   ├─ Email → Prose model (formal sentence completion)
    │   ├─ Slack → Prose model (casual phrase completion)
    │   └─ Terminal → No prediction (let shell handle it)
    │
    ├─ Run inference (target: P50 <100ms, P90 <300ms)
    │
    ├─ Compute diff between current text and predicted text
    │
    └─ Render ghost text via Overlay
        ├─ Tab → Accept prediction, insert text
        ├─ Tab again → Chain to next prediction (Zed-style)
        ├─ Any other key → Dismiss prediction
        └─ Writing Memory records accept/dismiss
```

### Flow D: 24/7 Background Agent

```
DX Daemon (systemd/launchd) runs 24/7
    │
    ├─ Channel Router
    │   ├─ Telegram message arrives → Route to Agent A
    │   ├─ Slack message arrives → Route to Agent B
    │   └─ Scheduled cron task fires → Route to Agent C
    │
    ├─ Agent processes message:
    │   ├─ Load identity/persona
    │   ├─ Query memory engine (vector + keyword search)
    │   ├─ Run LLM inference (shared with system-wide models)
    │   ├─ Execute tool calls (MCP/ACP/A2A)
    │   ├─ [If needed] Computer Use (screenshots, clicks, typing)
    │   └─ Store results in memory
    │
    ├─ Supervisor monitors agents:
    │   ├─ Crash detected → Auto-restart with exponential backoff
    │   ├─ Health check fails → Alert user via system notification
    │   └─ Resource pressure → Swap to smaller model
    │
    └─ Resource Manager
        ├─ User active → Agent uses shared model, lower priority
        ├─ User idle → Agent can load larger model, higher priority
        └─ Battery mode → Agent pauses non-critical tasks
```

### Flow E: Cloud Provider Fallback (When Local Isn't Enough)

```
User requests complex task → Local LLM attempts
    │
    ├─ Response quality assessment (confidence score)
    │   ├─ High confidence → Return local result
    │   └─ Low confidence → Offer cloud fallback
    │
    └─ Cloud Fallback (if user has API keys configured):
        ├─ DX Router selects best provider
        ├─ Apply RLM token compression (80-90% savings)
        ├─ Apply DX Serializer for tool calls (70-90% savings)
        ├─ Route through retry/fallback chain
        ├─ Track cost (LiteLLM cost map integration)
        ├─ Check budget limits
        └─ Return result + cost to user
```

---

## Section 7: The Overlay Rendering System

### How DX Renders Grammar Underlines + Ghost Text in EVERY App

The fundamental challenge: rendering visual feedback (underlines, ghost text, suggestion cards) on top of text fields that DX doesn't own.

**Approach per platform:**

**macOS:**
- Create a transparent, click-through `NSWindow` overlay positioned precisely over the active text field
- Use the Accessibility API (`AXUIElement`) to get:
  - The text field's position and size on screen
  - The current text content
  - The cursor position (character index)
  - The text's visual layout (line breaks, scroll position)
- GPUI renders underlines and ghost text at GPU speed onto this overlay
- The overlay window has `NSWindowLevel.floating` so it's always above the app
- Mouse events pass through the overlay to the underlying app
- Only intercepts clicks when the user clicks on an underline (then shows suggestion card)

**Windows:**
- Layered window (`WS_EX_LAYERED | WS_EX_TRANSPARENT | WS_EX_TOPMOST`)
- UI Automation for text field bounds and cursor position
- DirectX/GPUI for overlay rendering
- Use `SetWindowLong` with `WS_EX_TRANSPARENT` for click-through

**Linux:**
- X11: Override-redirect window with `_NET_WM_WINDOW_TYPE_NOTIFICATION`
- Wayland: `layer-shell` protocol for overlay surfaces (`zwlr_layer_shell_v1`)
- GPUI/Vulkan for rendering

### Rendering Pipeline

```
Text Field Change Detected (via Accessibility API)
    │
    ├─ Get text field bounds (screen coordinates)
    ├─ Get current text content
    ├─ Get cursor position
    ├─ Get font metrics (size, family, line height)
    │
    ├─ Run grammar pipeline → list of {span, severity, fix}
    ├─ Run edit prediction → predicted text + insertion point
    │
    ├─ Compute overlay geometry:
    │   ├─ Map character spans to pixel positions
    │   ├─ Account for scroll offset
    │   ├─ Account for zoom level
    │   └─ Clip to visible area of text field
    │
    └─ GPUI renders to overlay window:
        ├─ Squiggly underlines (red/yellow/blue/purple)
        ├─ Ghost text (semi-transparent, ahead of cursor)
        ├─ Suggestion cards (on hover/click of underline)
        └─ Flow Bar (bottom-center, voice state)
```

---

## Section 8: The First-Launch Experience Flow

This is crucial for virality. The first 60 seconds determine if someone keeps DX.

```
Second 0: User opens DX
    → "Welcome to DX. Let's set you up."

Second 5: Hardware scan completes
    → "Your [MacBook Air M2 / ThinkPad T480 / RTX 4090 PC]
       can run [Tier 3 / Tier 2 / Tier 5] models."
    → "This means: [Grammar ✅ Voice ✅ Edit Prediction ✅ Agent ✅]"

Second 10: "Download [4GB / 700MB / 76GB] of AI models?"
    → User clicks "Download"
    → Progress bar with estimated time

Second 15: Harper loaded (bundled, no download)
    → Grammar checking is ALREADY WORKING
    → "Try typing something in any app. DX is already checking your writing."

Second 45: Whisper downloaded
    → "Voice input is ready. Press [Cmd+Shift+Space] in any app."

Second 90: Edit prediction model downloaded
    → "Edit prediction is active. Watch for ghost text when you pause."

Second 180: Full LLM downloaded
    → "All systems online. DX is fully operational."
    → Avatar face appears at bottom-center, smiles

Background: Agent daemon starts running
    → "Want DX to run 24/7? [Install as system service]"
```

---

## Section 9: Competitive Destruction Matrix

| Capability | Grammarly | Wispr Flow | Zed EP | OpenClaw | ZeroClaw | **DX** |
|---|---|---|---|---|---|---|
| Grammar checking | ✅ Cloud | ❌ | ❌ | ❌ | ❌ | ✅ **Local, <10ms** |
| Edit prediction | ❌ | ❌ | ✅ Zed only | ❌ | ❌ | ✅ **Every app** |
| Voice dictation | ❌ | ✅ Cloud | ❌ | ✅ Cloud | ❌ | ✅ **Local** |
| Voice commands | ❌ | ✅ Cloud | ❌ | ❌ | ❌ | ✅ **Local** |
| 24/7 agent | ❌ | ❌ | ❌ | ✅ Node.js | ✅ Rust | ✅ **Rust + GUI** |
| Computer use | ❌ | ❌ | ❌ | Browser only | ❌ | ✅ **Full OS** |
| 100+ LLM providers | ❌ | ❌ | ~12 | ~5 | ~22 | ✅ **100+** |
| Offline | ❌ | ❌ | Partial | ❌ | ✅ | ✅ **Full** |
| Free & unlimited | ❌ $144/yr | ❌ $144/yr | ❌ | ✅ | ✅ | ✅ |
| Privacy (zero cloud) | ❌ | ❌ | Partial | ❌ | ✅ | ✅ |
| Low-end device support | ❌ Heavy | ❌ Heavy | ❌ | ❌ | ✅ 3.4MB | ✅ **Adaptive** |
| High-end experience | ❌ Same | ❌ Same | ❌ Same | ❌ Same | ❌ Same | ✅ **70B models** |
| Code + prose | Prose only | Prose + basic | Code only | General | General | ✅ **Both** |
| AI Avatar | ❌ | ❌ | ❌ | ❌ | ❌ | ✅ **GPU-rendered** |
| Works everywhere | Extensions | macOS/Win | Zed only | CLI + messaging | CLI only | ✅ **5 OS + extensions** |

---

## Section 10: The Virality Engine

### Why DX Goes Viral

**For low-end device users (billions of people):**
- "I have a $200 laptop and DX gives me free Grammarly + free voice dictation + free AI agent"
- The 360M SmolLM2 model runs on a Raspberry Pi. Nobody else does this.
- Word of mouth in developing countries, schools, universities

**For rich users / power users:**
- "I have a Mac Studio and DX gives me 72B model quality for free — that's better than ChatGPT Plus"
- Flex factor: "I'm running a 70B model locally, unlimited, zero latency"
- These users post benchmarks and comparisons on Twitter/Reddit

**For privacy-conscious users:**
- "Everything runs locally. Zero data leaves my device. Ever."
- Post-Snowden, post-AI-training-on-user-data era — this resonates deeply
- Enterprise/government users who can't use cloud AI

**For developers:**
- "DX is the first tool that does edit prediction outside of a code editor"
- "I dictate code with my voice in VS Code, and DX auto-formats it"
- 100+ LLM providers — developer flexibility that nobody matches

### The Viral Loop

```
User installs DX → Free, works immediately (Harper grammar)
    → "Wow, this catches errors Grammarly misses, and it's free"
    → Tells friend
    → Friend installs → Same experience
    → Both discover voice dictation ("This is Wispr Flow but free?!")
    → Both discover edit prediction ("This works in Slack too?!")
    → Power user discovers 24/7 agent + computer use
    → Power user makes YouTube video
    → Video goes viral ("Free AI that beats $288/year in subscriptions")
    → Mass adoption
```

---

## Section 11: Complete Crate Directory (Alpha-Sorted Quick Reference)

| Crate | Purpose | Category |
|-------|---------|----------|
| `accessibility` | macOS accessibility bindings | OS Integration |
| `aho-corasick` | Fast multi-pattern matching | Guardrails |
| `again` | Retry with backoff | LLM Provider |
| `analiticcl` | Fuzzy string matching | Grammar |
| `anyhow` | Ad-hoc error handling | Infrastructure |
| `arboard` | Cross-platform clipboard | OS Integration |
| `argon2` | Key hashing | Security |
| `async-stream` | Async stream construction | LLM Provider |
| `atoma-infer` | Flash/Paged attention inference | Inference |
| `atspi` | Linux accessibility (AT-SPI2) | OS Integration |
| `autopilot-rs` | Cross-platform GUI automation | Computer Use |
| `aws-sigv4` | AWS SigV4 signing (Bedrock) | LLM Provider |
| `axum` | HTTP framework (proxy server) | LLM Provider |
| `backoff` | Exponential backoff | LLM Provider |
| `bincode` | Binary serialization (cache) | Infrastructure |
| `candle-core` | ML tensor framework | Inference |
| `candle-nn` | Neural network layers | Inference |
| `candle-transformers` | Pre-built model architectures | Inference |
| `cpal` | Cross-platform audio I/O | Voice |
| `cron` | Cron expression parsing | Agent |
| `dasp` | Digital signal processing | Voice |
| `dashmap` | Concurrent hash map | LLM Provider |
| `eventsource-stream` | SSE parsing | LLM Provider |
| `get-selected-text` | Get selected text (all OS) | OS Integration |
| `global-hotkey` | Cross-platform global hotkeys | OS Integration |
| `governor` | Token-bucket rate limiting | LLM Provider |
| `hardware-query` | Full hardware profiling | Hardware |
| `harper-core` | Grammar checker engine | Grammar |
| `hf-hub` | Hugging Face model downloads | Inference |
| `kalosm` | Multi-modal meta-framework | Inference |
| `lingua-rs` | Language detection | Grammar |
| `llama-cpp-rs` | llama.cpp FFI bindings | Inference |
| `llama-gguf` | Pure-Rust GGUF inference | Inference |
| `llmfit` | Model-to-hardware fitting | Hardware |
| `matrix-sdk` | Matrix protocol | Agent |
| `nlprule` | LanguageTool rules (offline) | Grammar |
| `notify` | File watching (config reload) | Infrastructure |
| `nvml-wrapper` | NVIDIA GPU management | Hardware |
| `objc2` | macOS Objective-C FFI | OS Integration |
| `opentelemetry` | Distributed tracing | Observability |
| `prometheus` | Metrics collection | Observability |
| `redis` | Cache / rate limit backend | LLM Provider |
| `regex` | Pattern matching | Guardrails |
| `reqwest` | HTTP client | LLM Provider |
| `rubato` | Audio resampling | Voice |
| `rustautogui` | Mouse/keyboard automation | Computer Use |
| `rusqlite` | SQLite (agent memory, DB) | Agent |
| `screenshots` | Screen capture | Computer Use |
| `serenity` | Discord Bot API | Agent |
| `serde` / `serde_json` / `serde_yaml` / `toml` | Serialization | Infrastructure |
| `sha2` | Hashing (cache keys) | Security |
| `signal-hook` | Unix signal handling | Agent |
| `silicon-monitor` | Hardware monitoring for AI | Hardware |
| `similar` | Text diffing | Edit Prediction |
| `slack-morphism` | Slack API | Agent |
| `sqlx` | PostgreSQL (proxy server) | LLM Provider |
| `system-analysis` | AI workload compatibility | Hardware |
| `sysinfo` | Basic system info | Hardware |
| `teloxide` | Telegram Bot API | Agent |
| `thiserror` | Typed error definitions | Infrastructure |
| `tiktoken-rs` | OpenAI tokenization | LLM Provider |
| `tokenizers` | HF tokenizer library | Inference |
| `tokio` | Async runtime | Infrastructure |
| `tower` | HTTP middleware (proxy) | LLM Provider |
| `tracing` / `tracing-subscriber` | Structured logging | Observability |
| `tray-icon` | System tray icon | OS Integration |
| `unicode-segmentation` | Text segmentation | Grammar |
| `webrtc-vad` | Voice activity detection | Voice |
| `whisper-rs` | Whisper STT bindings | Voice |
| `whisper-cpp-plus` | Streaming Whisper + VAD | Voice |
| `windows` | Windows API bindings | OS Integration |
| `zspell` | Hunspell-compatible spellcheck | Grammar |

---

## Section 12: Build Order (Priority-Sequenced)

### Phase 1: "It Works in 60 Seconds" (Weeks 1–6)
- Hardware profiling (`hardware-query` + `system-analysis`)
- Tier classification algorithm
- Harper integration for instant grammar checking
- Model download manager (progressive, tiered)
- Basic transparent overlay on macOS (GPUI)
- Squiggly underlines rendering
- System tray icon

### Phase 2: Edit Prediction Goes System-Wide (Weeks 7–12)
- OS text field interception (macOS Accessibility API first)
- Candle inference engine integration
- GGUF model loading for tiered models
- Ghost text rendering in overlay
- Tab-to-accept via keyboard hook
- Debouncing, caching, context detection
- Code vs. prose detection

### Phase 3: Voice Engine (Weeks 13–16)
- Whisper integration (whisper-rs / whisper-cpp-plus)
- Audio capture (cpal)
- Voice Activity Detection
- Flow Bar UI (GPUI bottom-center widget)
- LLM post-processing (course correction, filler removal)
- Text insertion via Accessibility API

### Phase 4: AI Avatar + Floating Panel (Weeks 17–20)
- GPU-rendered avatar face (GPUI canvas)
- Avatar state machine (idle, listening, thinking, speaking)
- Eye tracking, blink animation
- Floating AI panel (PopUp window)
- Panel ↔ DX daemon communication

### Phase 5: 100+ LLM Providers (Weeks 21–26)
- Unified provider abstraction (Tier 1–5 adapters)
- Models.dev registry integration
- LiteLLM cost map integration
- Router with retry/fallback/load balancing
- Cost tracking + budget management
- Rate limiting (RPM/TPM)
- Virtual key management
- Proxy server (axum-based AI Gateway)

### Phase 6: Background Agent Daemon (Weeks 27–32)
- Daemon architecture with supervisor
- Cron scheduler
- Channel integrations (Telegram, Discord, Slack)
- Agent identity system (AIEOS compatible)
- Memory engine (local vector DB + keyword search)
- Service installation (systemd/launchd)
- VPS deployment

### Phase 7: Computer Use (Weeks 33–36)
- Screenshot engine (cross-platform)
- Mouse/keyboard control (rustautogui)
- Accessibility tree traversal
- Security policy (allowlists, confirmation prompts)
- Vision model integration for screenshot reasoning

### Phase 8: Cross-Platform (Weeks 37–44)
- Windows: TSF + UI Automation + overlay
- Linux: IBus/Fcitx5 + AT-SPI2 + overlay
- Dynamic model swapping based on battery/power state
- Platform-specific edge case handling
- Performance optimization across all platforms

### Phase 9: Polish & Ship (Weeks 45–48)
- First-launch experience (60-second onboarding)
- Writing Memory system (personal dictionary, patterns)
- Context-aware tone switching
- Performance targets: P50 <50ms for all operations
- Memory targets: <2GB minimum viable footprint
- Documentation
- Launch

---

## Section 13: The Numbers That Matter

| Metric | Target | Why It Matters |
|--------|--------|---------------|
| **Install to first grammar check** | < 10 seconds | Harper is bundled, zero download needed |
| **Install to full functionality** | < 5 minutes (Tier 2) / < 30 min (Tier 5) | Progressive download |
| **Grammar check latency (Tier 1)** | < 10ms | Faster than keystroke-to-screen rendering |
| **Grammar check latency (Tier 3 LLM)** | < 500ms | Only on sentence completion, debounced |
| **Edit prediction latency** | P50 < 100ms, P90 < 300ms | Must feel instant |
| **Voice transcription latency** | < 200ms per chunk | Real-time streaming |
| **Minimum RAM (Tier 1)** | ~600MB for DX models | Runs on 2GB RAM systems |
| **Maximum quality (Tier 5)** | 70B Q4_K_M | Approaches GPT-4 class |
| **Binary size** | < 10MB (without models) | Fast download, fast install |
| **Idle CPU usage** | < 1% | Rust daemon efficiency |
| **Battery impact** | < 3% per hour (typing) | Must not drain battery |
| **Subscription cost** | $0 | Forever. The point. |

---

This is the plan. Every crate is researched. Every tier is quantified. Every flow is documented. Every competitor is mapped. Build it.
