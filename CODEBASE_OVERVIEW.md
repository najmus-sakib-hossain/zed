# DX Codebase — Complete Reference

> **Last updated:** March 1, 2026  
> **Base project:** Zed Code Editor (forked by DX Industries)  
> **Language:** Rust (Cargo workspace, `edition = "2024"`)  
> **Primary binary:** `crates/zed` → `cargo run -p zed --locked`

---

## Table of Contents

1. [What Is This Codebase?](#1-what-is-this-codebase)
2. [The Eight Products DX Replaces](#2-the-eight-products-dx-replaces)
3. [High-Level Architecture](#3-high-level-architecture)
4. [Workspace Structure](#4-workspace-structure)
5. [Core Editor Capabilities (Inherited from Zed)](#5-core-editor-capabilities-inherited-from-zed)
6. [DX-Specific Subsystems](#6-dx-specific-subsystems)
7. [AI Provider Universe A — Language Intelligence (100+ LLM Providers)](#7-ai-provider-universe-a--language-intelligence-100-llm-providers)
8. [AI Provider Universe B — Media Generation (50+ Providers)](#8-ai-provider-universe-b--media-generation-50-providers)
9. [Local Inference Engine](#9-local-inference-engine)
10. [Hardware Adaptive System — 5 Device Tiers](#10-hardware-adaptive-system--5-device-tiers)
11. [Grammar Engine (Grammarly Replacement)](#11-grammar-engine-grammarly-replacement)
12. [Voice Engine (Wispr Flow + ElevenLabs Replacement)](#12-voice-engine-wispr-flow--elevenlabs-replacement)
13. [OS-Level Input Interception](#13-os-level-input-interception)
14. [Computer Use / OS Control](#14-computer-use--os-control)
15. [Background Agent Daemon](#15-background-agent-daemon)
16. [DX UI Components](#16-dx-ui-components)
17. [GPUI — The Custom UI Framework](#17-gpui--the-custom-ui-framework)
18. [All Crates — Complete Directory](#18-all-crates--complete-directory)
19. [Key DX Crates — Details](#19-key-dx-crates--details)
20. [External Dependencies — Key Crates](#20-external-dependencies--key-crates)
21. [Build System](#21-build-system)
22. [Configuration & Extensibility](#22-configuration--extensibility)
23. [Implementation Status (Phase Tracking)](#23-implementation-status-phase-tracking)
24. [Multi-Agent Coordination Protocol](#24-multi-agent-coordination-protocol)

---

## 1. What Is This Codebase?

This is **DX** — a fork of the [Zed code editor](https://zed.dev) that has been dramatically extended into a **Universal AI Platform**. It is a single Rust binary that simultaneously functions as:

- A high-performance, GPU-accelerated **code editor** (everything from the original Zed codebase)
- A **100+ LLM provider aggregator** (replacing LiteLLM)
- A **system-wide grammar checker** running locally in <10ms (replacing Grammarly)
- A **voice dictation + command mode** engine running offline via Whisper (replacing Wispr Flow)
- A **tab-completion / edit prediction** engine extended to every text field on the OS (extending Zed's Zeta)
- A **24/7 background AI agent daemon** with messaging channels and cron scheduling (replacing OpenClaw/ZeroClaw)
- A **computer use / OS control** system that reads the accessibility tree, takes screenshots, moves the mouse, and types (replacing Anthropic Computer Use)
- A **media generation hub** for image, video, audio, music, 3D, and document generation across 50+ providers (replacing Fal.ai / Replicate / ElevenLabs / Stability AI)

The project vision is to deliver **$552+ / year in subscription savings** per user — entirely free and mostly local, requiring no internet beyond initial model downloads.

---

## 2. The Eight Products DX Replaces

| Product Replaced | Annual Cost | DX Equivalent | DX Cost |
|---|---|---|---|
| **LiteLLM** (100+ LLM providers) | $0–Enterprise | Unified provider abstraction, cost tracking, fallbacks, proxy | **$0** |
| **Grammarly** (writing assistant) | $144/yr | System-wide grammar: Harper + nlprule + LLM, <10ms, private | **$0** |
| **Wispr Flow** (voice dictation) | $144/yr | Local Whisper STT + Piper/Chatterbox TTS, offline | **$0** |
| **Zed Edit Prediction** (code tab-complete) | Paid | Tab-accept edit prediction in every OS text field | **$0** |
| **OpenClaw / ZeroClaw** (AI agent) | $0 | 24/7 background daemon: cron, Telegram, Discord, Slack, WhatsApp | **$0** |
| **Anthropic Computer Use** (OS control) | API cost | Screenshots, mouse, keyboard, accessibility tree — locally | **$0** |
| **ElevenLabs** (voice generation) | $264/yr | Local TTS via Piper/Chatterbox/Kokoro (beats ElevenLabs in blind tests on Tier 4+) | **$0 local** |
| **Fal.ai / Replicate / Stability AI** (media) | Pay-per-use | Image, video, audio, music, 3D, PDF across 50+ providers | **$0 local / cloud on-demand** |

---

## 3. High-Level Architecture

```
┌─────────────────────────────────────────────────────┐
│                  DX Binary (Rust)                    │
│                                                      │
│  ┌──────────┐  ┌──────────┐  ┌──────────────────┐   │
│  │  Code    │  │  AI Chat │  │  Media Generation │   │
│  │  Editor  │  │  Panel   │  │  Hub               │   │
│  │  (Zed)   │  │  (DX)    │  │  (DX)             │   │
│  └──────────┘  └──────────┘  └──────────────────┘   │
│                                                      │
│  ┌──────────────────────────────────────────────┐   │
│  │              GPUI (Custom UI Framework)        │   │
│  │       GPU-accelerated via wgpu (WGSL)         │   │
│  └──────────────────────────────────────────────┘   │
│                                                      │
│  ┌──────────┐  ┌──────────────┐  ┌──────────────┐   │
│  │  Grammar │  │  Voice       │  │  Agent        │   │
│  │  Engine  │  │  Engine      │  │  Daemon       │   │
│  │  (DX)    │  │  (DX)        │  │  (DX)         │   │
│  └──────────┘  └──────────────┘  └──────────────┘   │
│                                                      │
│  ┌──────────────────────────────────────────────┐   │
│  │        dx_core — shared foundation types      │   │
│  │  LlmProvider, MediaProvider, TtsProvider,     │   │
│  │  DeviceTier, CostTracker, RateLimiter, etc.   │   │
│  └──────────────────────────────────────────────┘   │
│                                                      │
│  ┌──────────┐  ┌──────────────┐  ┌──────────────┐   │
│  │  100+    │  │  50+ Media   │  │  Local GGUF  │   │
│  │  LLM     │  │  Providers   │  │  Inference   │   │
│  │  Providers│  │  (Universe B)│  │  (Candle)    │   │
│  └──────────┘  └──────────────┘  └──────────────┘   │
└─────────────────────────────────────────────────────┘
```

**Two Universes of AI Providers:**

- **Universe A — Language Intelligence:** 100+ LLM providers unified under a single `LlmProvider` trait. They think, reason, write, code, plan, analyze.
- **Universe B — Media Generation:** 50+ creation providers unified under a `MediaProvider` trait. They create images, video, audio, music, 3D, documents.

---

## 4. Workspace Structure

This is a **Cargo workspace** with `default-members = ["crates/zed"]`. The workspace root `Cargo.toml` lists ~220 member crates organized into:

```
f:\Desktop\
├── Cargo.toml               ← workspace root (220+ member crates)
├── crates/                  ← all library crates (see section 18)
├── assets/                  ← fonts, icons (SVGs), themes, keymaps, sounds, prompts
│   ├── fonts/
│   ├── icons/
│   ├── keymaps/
│   ├── themes/
│   ├── sounds/
│   └── settings/
├── extensions/              ← bundled extensions (glsl, html, proto, etc.)
├── docs/                    ← mdBook documentation
├── tooling/                 ← perf tooling, xtask
├── essence/                 ← supplemental essence/design files
└── integrations/            ← agent integrations
```

---

## 5. Core Editor Capabilities (Inherited from Zed)

Everything Zed can do is present in this codebase. Key capabilities:

### Code Editing
- **Multi-buffer editing** (`crates/multi_buffer`) — edit multiple files in one surface
- **Rope-based text engine** (`crates/rope`, `crates/text`) — O(log n) edits on any file size
- **Tree-sitter syntax highlighting** for 50+ languages (`crates/languages`)
- **LSP support** (`crates/lsp`, `crates/language`) — hover, goto-definition, diagnostics, completions, formatting
- **Diagnostics panel** (`crates/diagnostics`) — aggregated errors and warnings
- **Buffer diffs** (`crates/buffer_diff`, `crates/streaming_diff`) — inline and panel views
- **Multi-cursor editing** — full support inside `crates/editor`
- **Code actions, refactoring** — LSP-driven
- **Outline panel** (`crates/outline`, `crates/outline_panel`) — symbol navigation
- **Project panel / file explorer** (`crates/project_panel`) — file tree with truncation
- **Breadcrumbs** (`crates/breadcrumbs`) — path indicator in editor header
- **Tab switcher** (`crates/tab_switcher`)
- **Go-to-line** (`crates/go_to_line`)

### Search
- **Project-wide search** (`crates/search`) — regex, case-sensitive, file filters
- **File finder** (`crates/file_finder`) — fuzzy file search
- **Project symbols** (`crates/project_symbols`)
- **Fuzzy matching engine** (`crates/fuzzy`) — used everywhere

### Git Integration
- **Git blame, diffs, staging** (`crates/git`, `crates/git_ui`)
- **Git graph** (`crates/git_graph`) — commit history visualization
- **Git hosting providers** (`crates/git_hosting_providers`) — GitHub, GitLab, Bitbucket links

### Collaboration (original Zed multiplayer)
- **Real-time collaboration** (`crates/collab`, `crates/collab_ui`) — shared editing sessions
- **LiveKit audio/video calling** (`crates/call`, `crates/livekit_client`, `crates/livekit_api`)
- **Channel system** (`crates/channel`) — team channels with messages
- **Notifications** (`crates/notifications`)

### Worktrees
- **Worktree manager** (`crates/worktree`) — multi-agent git worktree support so multiple agents can work on the same repo without conflicts

### Terminal
- **Integrated terminal** (`crates/terminal`, `crates/terminal_view`) — full PTY support

### REPL
- **Jupyter-style REPL** (`crates/repl`) — execute code inline

### Debugger
- **DAP-based debugger** (`crates/dap`, `crates/dap_adapters`, `crates/debugger_ui`, `crates/debugger_tools`)
- **Debug adapter extension hook** (`crates/debug_adapter_extension`)

### Extensions
- **Extension host** (`crates/extension_host`, `crates/extension`) — WebAssembly-sandboxed extensions
- **Extension CLI** (`crates/extension_cli`)
- **Extension API** (`crates/extension_api`)
- **Extensions UI** (`crates/extensions_ui`)
- **Bundled extensions:** GLSL, HTML, Proto, plus slash command examples and test extensions

### Edit Prediction (Zeta)
- **Zeta-style tab-accept completions** (`crates/edit_prediction`, `crates/edit_prediction_ui`, `crates/edit_prediction_types`, `crates/edit_prediction_context`)
- **CLI for prediction** (`crates/edit_prediction_cli`)

### Vim Mode
- **Complete Vim emulation** (`crates/vim`, `crates/vim_mode_setting`) — normal, insert, visual modes, motions, registers, macros

### Themes & UI Customization
- **Theme engine** (`crates/theme`, `crates/theme_extension`, `crates/theme_importer`, `crates/theme_selector`)
- **Settings system** (`crates/settings`, `crates/settings_ui`, `crates/settings_json`, `crates/settings_content`, `crates/settings_profile_selector`)
- **Keymap editor** (`crates/keymap_editor`)

### Miscellaneous Editor Features
- **Markdown preview** (`crates/markdown_preview`, `crates/markdown`)
- **SVG preview** (`crates/svg_preview`)
- **Image viewer** (`crates/image_viewer`)
- **Journal** (`crates/journal`)
- **Snippets** (`crates/snippet`, `crates/snippets_ui`, `crates/snippet_provider`)
- **Tasks** (`crates/task`, `crates/tasks_ui`) — run/build task system
- **Command palette** (`crates/command_palette`, `crates/command_palette_hooks`)
- **Dev containers** (`crates/dev_container`)
- **Prettier integration** (`crates/prettier`)
- **Node runtime** (`crates/node_runtime`) — for JS-based LSP servers
- **Encoding selector** (`crates/encoding_selector`)
- **Line ending selector** (`crates/line_ending_selector`)
- **Toolchain selector** (`crates/toolchain_selector`)
- **Language selector** (`crates/language_selector`)
- **Which-key overlay** (`crates/which_key`)
- **Auto-update** (`crates/auto_update`, `crates/auto_update_ui`, `crates/auto_update_helper`)
- **Crash reporter** (`crates/crashes`)

---

## 6. DX-Specific Subsystems

All DX-specific code lives in crates prefixed with `dx_`:

| Crate | Purpose |
|---|---|
| `dx_core` | Shared types: `LlmProvider`, `MediaProvider`, `TtsProvider`, `DeviceTier`, `CostTracker`, `RateLimiter`, `Mood`, `AiProfile`, `SessionEntry`, `DxConfig` |
| `dx_ai_ui` | AI-specific UI components |
| `dx_computer_use` | OS automation: mouse, keyboard, screenshot, accessibility tree |
| `dx_daemon` | Background agent daemon: supervisor, cron, channel routing, VPS deploy |
| `dx_grammar` | Three-tier grammar pipeline: Harper → nlprule → LLM |
| `dx_hardware` | Hardware detection, NPU detection, model swapper, system analysis |
| `dx_inference` | Local ML inference: Candle backend, llama.cpp backend, model cache, GPU memory manager |
| `dx_input_intercept` | OS-level input interception for system-wide grammar/prediction |
| `dx_llm_adapters` | Additional LLM adapter wiring |
| `dx_llm_bridge` | Bridge between dx_core trait and provider implementations |
| `dx_local_inference` | Standalone local inference management |
| `dx_media` | All media generation adapters (50+ providers for image/video/audio/3D/PDF) |
| `dx_orchestrator` | Unified `generate()` orchestration: decomposer, parallel executor, cost summary |
| `dx_providers` | 21 LLM provider adapters wired to `LlmProvider` trait |
| `dx_ui` | All new DX UI components (see section 16) |
| `dx_voice` | Voice conversation loop: full-duplex engine, quality router |

Additional supporting crates:
| Crate | Purpose |
|---|---|
| `providers` | Free provider adapters (OpenCode, ML Voca, Pollinations, etc.) |
| `n8n_engine` | N8N workflow automation engine integration |
| `scheduler` | Task scheduling inside DX |
| `rules_library` | Rules-based AI constraint system |
| `eval` / `eval_utils` | Model evaluation and benchmarking utilities |
| `nc` | Utility crate (likely "new concept" or similar) |

---

## 7. AI Provider Universe A — Language Intelligence (100+ LLM Providers)

All providers implement the `LlmProvider` trait in `dx_core`:
```rust
trait LlmProvider {
    fn complete(&self, request: LlmRequest) -> Future<LlmResponse>;
    fn stream(&self, request: LlmRequest) -> Stream<LlmDelta>;
    fn list_models(&self) -> Future<Vec<ModelInfo>>;
    fn embed(&self, texts: Vec<String>) -> Future<Vec<Vec<f32>>>;
}
```

### Tier 1 — Native Adapters (Full SDK-Level)
| Provider | Crate | Adapter |
|---|---|---|
| OpenAI (Chat + Responses API) | `crates/open_ai` | `dx_providers/openai_adapter.rs` |
| Anthropic (Messages API) | `crates/anthropic` | `dx_providers/anthropic_adapter.rs` |
| Google Gemini + Vertex AI | `crates/google_ai` | `dx_providers/google_ai_adapter.rs` |
| AWS Bedrock (SigV4 auth) | `crates/bedrock` | `dx_providers/bedrock_adapter.rs` |
| Azure OpenAI | — | `dx_providers/azure_openai_adapter.rs` |
| Ollama (local) | `crates/ollama` | `dx_providers/ollama_adapter.rs` |

### Tier 2 — Named Adapters (Provider-Specific Quirks)
| Provider | Adapter |
|---|---|
| Mistral | `mistral_adapter.rs` (via `crates/mistral`) |
| DeepSeek | `deepseek_adapter.rs` (via `crates/deepseek`) |
| xAI Grok | `x_ai_adapter.rs` (via `crates/x_ai`) |
| Groq | `groq_adapter.rs` |
| Fireworks AI | `fireworks_adapter.rs` |
| Together AI | `together_adapter.rs` |
| Cohere | `cohere_adapter.rs` |
| NVIDIA NIM | `nvidia_nim_adapter.rs` |
| LM Studio | `lm_studio_adapter.rs` (via `crates/lmstudio`) |
| Hugging Face Inference | `huggingface_adapter.rs` |
| Replicate (LLM) | `replicate_llm_adapter.rs` |
| Codestral | `crates/codestral` |

### Tier 3 — OpenAI-Compatible Generic Adapter (40+ Providers)
Single adapter `openai_compatible.rs` handles: Cerebras, Perplexity, Venice AI, Baseten, Deep Infra, IO.NET, Moonshot AI, MiniMax, Nebius, OVHcloud, Scaleway, SiliconFlow, Inference.net, vLLM, GPUStack, llamafile, and every compatible endpoint.

Also includes: `crates/open_router`, `crates/vercel` (Vercel AI Gateway), Cloudflare AI Gateway, Helicone, Cortecs, ZenMux, 302.AI.

### Tier 4 — Aggregator Multipliers
| Aggregator | Crate | What It Adds |
|---|---|---|
| OpenRouter | `crates/open_router` | Access to 300+ models through one key |
| Vercel AI Gateway | `crates/vercel` | Unified gateway with caching |
| Cloudflare, Helicone | via generic | Rate limiting, logging overlay |

### Tier 5 — Local / Free Models
- **Free providers** (`crates/providers`): OpenCode (7 free models), ML Voca (TinyLlama, DeepSeek R1 1.5B), Pollinations (OpenAI Fast)
- **Local GGUF**: via Ollama or direct Candle/llama.cpp inference
- All 7 free models are always shown at the top of the model picker — even without API key configuration

### Additional Provider Crates
- `crates/supermaven` + `crates/supermaven_api` — Supermaven code completion
- `crates/copilot` + `crates/copilot_chat` + `crates/copilot_ui` — GitHub Copilot integration
- `crates/cloud_llm_client`, `crates/cloud_api_client`, `crates/cloud_api_types` — Zed cloud services
- `crates/language_model`, `crates/language_models` — unified LangModel abstraction layer
- `crates/agent`, `crates/agent_ui`, `crates/agent_settings`, `crates/agent_servers` — Zed's native agent system
- `crates/acp_thread`, `crates/acp_tools` — ACP protocol thread/tool integration

### Provider Features
- **Fallback chains**: Provider A → Provider B → Provider C (automatic on failure)
- **Cost tracking**: Per-token pricing per provider in `dx_core/src/cost.rs`
- **Rate limiting**: RPM sliding-window limiter in `dx_core/src/rate_limiter.rs`
- **Budget limits & alerts**: `BudgetConfig` in `dx_core/src/cost.rs`
- **Provider registry**: `DxProviderRegistry` with health monitoring

---

## 8. AI Provider Universe B — Media Generation (50+ Providers)

All providers implement the `MediaProvider` trait in `dx_core`:
```rust
trait MediaProvider {
    fn generate(&self, request: MediaGenerationRequest) -> Stream<MediaGenerationProgress>;
    fn list_models(&self) -> Future<Vec<MediaModelInfo>>;
    fn estimate_cost(&self, request: &MediaGenerationRequest) -> MicroCost;
}
```

### Image Generation (12 providers + local)
| Provider | Specialty | Location |
|---|---|---|
| OpenAI DALL-E 3 / GPT-Image | Semantic understanding | via `replicate.rs` |
| Fal.ai (600+ models) | Fastest, largest selection | `dx_media/fal_ai.rs` (planned) |
| Stability AI (SDXL, SD3.5) | Open-source, self-hostable | via `replicate.rs` |
| Replicate (200+ community) | Community fine-tunes | `dx_media/replicate.rs` ✅ |
| Google Imagen (Vertex AI) | Multi-modal input | via `replicate.rs` |
| Midjourney | Aesthetic/stylized | via API |
| Adobe Firefly | Commercially cleared | via API |
| DeepSeek Janus Pro | Quality/cost ratio | via API |
| Black Forest Labs / Flux 2 | Best photorealism | via `fal.ai` |
| Recraft V3/V4 | Logos, SVG, design | via API |
| Ideogram 3.0 | Text-in-image rendering | via API |
| Local SDXL / Flux.1 | Free, offline (Tier 4+) | via Candle scaffold |

### Video Generation (12 providers)
| Provider | Specialty |
|---|---|
| Runway Gen-3 Alpha | Industry standard | `dx_media/runway.rs` ✅ |
| Kling AI (Kuaishou) | High-quality, long-form | `dx_media/kling_ai.rs` ✅ |
| Pika | Creative/stylized | `dx_media/pika.rs` ✅ |
| Luma AI Dream Machine | Photorealistic motion | `dx_media/luma_ai.rs` ✅ |
| Minimax / Hailuo | Fast generation | `dx_media/minimax_video.rs` ✅ |
| Synthesia | AI avatar video | `dx_media/synthesia.rs` ✅ |
| HeyGen | Avatar + dubbing | planned via `replicate.rs` |
| Google Veo (Vertex AI) | Google flagship | planned |
| OpenAI Sora | Text-to-video | planned |
| Stability Video Diffusion | Open-source video | via `replicate.rs` |
| Replicate video models | Community | `dx_media/replicate.rs` ✅ |

### Audio & Voice Generation (15 providers)
| Provider | Type | Speed |
|---|---|---|
| **Piper TTS (local)** | Free, offline | Real-time on Pi |
| **Chatterbox-Turbo (local)** | Free, offline | Wins vs ElevenLabs blind tests |
| **Kokoro (local)** | Free, offline, 6 voices | Ultra-fast, CPU-only |
| ElevenLabs | Cloud, 1200+ voices | — |
| Fish Audio | Cloud, #1 TTS-Arena | 80% cheaper than ElevenLabs |
| Cartesia | Cloud, 40ms latency | Voice cloning from 3s |
| PlayHT | Cloud, 1000+ voices | 142+ languages |
| Deepgram Aura | Cloud, enterprise | — |
| Google Cloud TTS | Cloud, 380+ voices | 50+ languages |
| Amazon Polly | Cloud, free tier | 5M chars/month |
| Azure Speech | Cloud, neural | SSML support |
| WellSaid Labs | Cloud, brand | Studio quality |
| Murf AI | Cloud, production | Audio/video editor |
| Lovo AI | Cloud, video | 500 voices |
| OpenAI TTS | Cloud, simple | GPT-quality voice |

### Music Generation (7 providers)
| Provider | Specialty | Location |
|---|---|---|
| Suno AI | Full songs (vocals + instruments) | `dx_media/suno_ai.rs` ✅ |
| Udio | High-quality music | `dx_media/udio.rs` ✅ |
| Stability Audio | Open-source | `dx_media/stability_audio.rs` ✅ |
| Meta MusicGen | Open-source | via `replicate.rs` ✅ |
| Google MusicFX | Text-to-music | planned |
| AIVA | Classical/cinematic | planned |
| Mubert | Royalty-free, real-time | planned |

### 3D Asset Generation (8 providers + local)
| Provider | Specialty | Location |
|---|---|---|
| Meshy | Text-to-3D, PBR textures | `dx_media/meshy.rs` ✅ |
| Tripo AI | Fast 3D generation | `dx_media/tripo_ai.rs` ✅ |
| Luma AI Genie | 3D from text/image | planned |
| Stability TripoSR | Open-source 3D | planned |
| OpenAI Shap-E | 3D from text | planned |
| CSM | Image-to-3D world | planned |
| Kaedim | Production 3D from images | planned |
| Rodin AI | 3D avatar generation | planned |
| Local TripoSR | Free, offline (Tier 4+) | via Candle scaffold |

### Document & PDF Generation (8 cloud + local Rust rendering)
| Cloud Provider | Location |
|---|---|
| Adobe PDF | `dx_media/adobe_pdf.rs` ✅ |
| ApiTemplate | `dx_media/apitemplate.rs` ✅ |
| Carbone | `dx_media/carbone.rs` ✅ |
| CraftMyPDF | `dx_media/craftmypdf.rs` ✅ |
| DocRaptor | `dx_media/docraptor.rs` ✅ |
| PDF.co | `dx_media/pdf_co.rs` ✅ |
| PDFShift | `dx_media/pdfshift.rs` ✅ |
| QuickChart | `dx_media/quickchart.rs` ✅ |

Local rendering crates (LLM writes content → Rust renders):
- `genpdf` — PDF with layouts, images, tables
- `printpdf` — full PDF spec, vector graphics
- `typst` — LaTeX-quality typesetting
- `rust_xlsxwriter` — Excel files with charts
- `plotters` — 2D/3D chart generation
- `pulldown-cmark` + `maud` — Markdown→HTML
- `resvg` + `usvg` — SVG rendering (already in Zed)
- `image` — image processing (already in Zed)

### Multi-Media Orchestration (`dx_orchestrator`)
One-call `generate()` that composes multiple providers in parallel:

> "Generate a product landing page PDF with a hero image, 3D mockup, and background music"

1. LLM writes copy (Universe A)
2. Fal.ai generates hero image (Universe B image)
3. Meshy generates 3D product mockup (Universe B 3D)
4. Suno AI generates background music (Universe B audio)
5. `printpdf`/`genpdf` assembles PDF locally
6. Piper/Chatterbox reads summary to user (voice)

All steps run in parallel where possible (max concurrency = 4), with cost tracking and a unified progress dashboard.

---

## 9. Local Inference Engine

Located in `crates/dx_inference`. Supports fully offline AI with GGUF quantized models.

### Primary Engine: Candle (Hugging Face)
- `candle-core` + `candle-transformers` + `candle-nn`
- CUDA + Metal + CPU backends
- GGUF quantization: Q2_K through F16
- Supported architectures: LLaMA v1/v2/v3, Falcon, GLM4, Gemma v1/v2, Phi-1 through Phi-3, StableLM, Mamba, Mistral 7B, CodeGeeX4, RecurrentGemma

### Secondary Engine: llama-cpp-rs / llama-cpp-2
- FFI bindings to llama.cpp
- Maximum GGUF ecosystem compatibility
- Safety net for architectures Candle doesn't yet support

### Model Cache Manager (`dx_inference/model_cache.rs`)
- 14 pre-configured models across all 5 device tiers
- SHA256 verification of downloads
- Automatic cleanup of unused quantizations after 30 days
- Storage: `~/.dx/models/`
- Shared with Ollama when detected (no duplicate downloads)

### GPU Memory Manager (`dx_inference/gpu_memory.rs`)
- Concurrent model loading
- Shares GPU/CPU memory across grammar + prediction + voice simultaneously
- One model, three use cases

### Progressive Download Strategy
| Time | What Downloads |
|---|---|
| t=0s | Binary installs (~10MB) |
| t=5s | Hardware scan → tier classified |
| t=10s | Harper grammar loads (bundled, ~5MB) |
| t=15s | Piper TTS tiny (~15MB) — voice immediately available |
| t=45s | Whisper Tiny (~75MB) — voice input available |
| t=90s | SmolLM2/Qwen3 (~200–400MB) |
| t=180s | Full model suite downloaded |

### Model Download UI (`dx_ui/model_download_ui.rs`)
- Progress bars per model
- Pause/Resume/Cancel per download
- Disk space warnings
- Queue management

---

## 10. Hardware Adaptive System — 5 Device Tiers

Located in `crates/dx_hardware` and `crates/dx_core/src/device_tier.rs`.

At first launch, DX profiles hardware (<2 second scan) and classifies into one of five tiers.

### Detection Capabilities
- RAM via `sysinfo`
- GPU VRAM: NVIDIA via `nvidia-smi`, AMD via `rocm-smi`, macOS via `system_profiler`, Windows via PowerShell/WMIC, Linux via `lspci` + sysfs
- CUDA / ROCm / Metal / DirectML flags
- Apple Silicon unified memory (75% of RAM as effective VRAM)
- NPU detection: Intel Movidius/GNA/XDNA, Qualcomm Hexagon, Apple Neural Engine, AMD XDNA (`dx_hardware/npu.rs`)
- Battery/power state: macOS `pmset`, Windows `Win32_Battery`, Linux `/sys/class/power_supply/`
- Disk space for model storage budget
- AI workload scoring and bottleneck detection (`dx_hardware/system_analysis.rs`)

### The Five Tiers

| Tier | Hardware | RAM | Model Stack |
|---|---|---|---|
| **1 — Ultra-Low-End** | Raspberry Pi, old laptops, Chromebooks | 2–4GB, no GPU | BitNet, SmolLM2-360M, Whisper Tiny.en, MiniLM — ~600MB total |
| **2 — Low-End** | Entry laptops, older MacBooks | 4–8GB, no dGPU | Qwen3-0.6B, SmolLM2-360M, Whisper Tiny.en, MiniLM — ~950MB total |
| **3 — Mid-Range** | MacBook Air M1/M2, mid gaming PCs | 8–16GB, iGPU | Qwen2.5-3B, Qwen2.5-Coder-1.5B, Whisper Base.en — ~4.7GB total |
| **4 — High-End** | MacBook Pro M3, RTX 4070 | 16–32GB, 6–12GB VRAM | Mistral-7B, SmolLM3-3B, Zeta/Qwen2.5-Coder-7B, Chatterbox-Turbo, Whisper Small.en — ~16.5GB total |
| **5 — Ultra-High-End** | Mac Studio, RTX 4090, 32GB+ | 32GB+ RAM, 16GB+ VRAM | Qwen2.5-72B or Llama3.1-70B, Qwen2.5-14B grammar, Zeta/Qwen2.5-Coder-32B, Chatterbox-Turbo + voice cloning, Whisper Large-v3, LLaVA-1.5-7B vision — ~84GB total |

### Dynamic Model Swapping (`dx_hardware/model_swapper.rs`)
- RAM pressure detected → swap Q5_K_M → Q4_K_M → unload edit prediction temporarily
- User plugged in laptop → enable GPU acceleration + load larger models
- User on battery → swap to smaller models, reduce prediction frequency
- User idle (daemon mode) → load larger model for scheduled tasks
- Multiple DX features active → share single model across grammar + prediction + voice

### Quantization Ladder
- **Q4_K_M**: Default for most users — 3.0–4.0x faster than FP16, fits constrained hardware
- **Q5_K_M**: Better detail and reasoning stability
- **Q8_0**: Maximum quality, used when memory isn't a constraint
- Always budget ~1.2× the quantized file size for actual memory usage

### Config Persistence
- Configuration saved to `~/.dx/dx_config.json`
- `DxConfig` contains: `CachedHardwareProfile`, `ProviderKeyRef`, `UserPreferences`, `ModelDownloadState`
- 7-day cache freshness — auto-rescan if stale
- `DX_HOME` env var overrides config location
- `ProviderKeyRef`: env var / OS keychain / inline key resolution

---

## 11. Grammar Engine (Grammarly Replacement)

Located in `crates/dx_grammar`. Three-tier pipeline — entirely local, <10ms on Tier 1, private.

### Tier 1 — Harper (`harper-core`)
- **Speed**: <10ms
- **Memory**: 1/50th of LanguageTool
- **Catches**: spelling, punctuation, grammar rules, passive voice, wordiness, sentence structure
- **Privacy**: everything runs on-device
- Displayed as 🔴 red squiggly (definitive errors) and 🟡 yellow squiggly (suggestions)

### Tier 2 — nlprule + Hunspell (`zspell`)
- **Speed**: <50ms
- **Catches**: 4000+ LanguageTool rule patterns (offline, no Java), multi-language spell check
- Displayed as 🔵 blue squiggly (style suggestions)

### Tier 3 — Local LLM (hardware-tiered)
- **Speed**: <500ms (debounced on paste/pause)
- **Catches**: tone mismatch, subtle awkwardness, restructuring suggestions, context-aware corrections
- Displayed as 💜 purple squiggly (AI insight)

### Supporting Systems
- Language detection: `whichlang` / `lingua-rs` — auto-switch engine per language
- Unicode segmentation: `unicode-segmentation` — proper word/sentence boundaries
- Fuzzy spelling correction: `analiticcl` — edit distance, phonetic codes

### Context-Aware Writing Profiles (`dx_grammar/app_detection.rs`)
Auto-detects active application and adjusts:
| App Category | Grammar Level | Tone | Prediction |
|---|---|---|---|
| Email client | High | Professional | Full sentence |
| Slack/Discord | Low | Casual | Short phrase |
| Code editor | Off for code / High for comments | Technical | Zeta-style code |
| Terminal | Off | — | None |
| Document editor | Maximum | Match document | Paragraph continuations |
| Social media | Medium | Casual-Professional | Short-form |

---

## 12. Voice Engine (Wispr Flow + ElevenLabs Replacement)

Located in `crates/dx_voice` and `crates/audio`.

### Speech-to-Text (planned)
- `whisper-rs` — GPU-accelerated (Metal/CUDA)
- `whisper-cpp-plus` — real-time streaming + Silero VAD
- `cpal` — cross-platform audio I/O
- `rubato` — resampling to 16kHz
- `webrtc-vad` — standalone VAD

### Text-to-Speech (partially implemented)
| Implementation | Status | File |
|---|---|---|
| Piper TTS | ✅ DONE | `dx_voice/piper_tts.rs` |
| Chatterbox-Turbo (paralinguistic tags) | ✅ DONE | `dx_voice/chatterbox_tts.rs` |
| Kokoro (6 voice presets, CPU-only) | ✅ DONE | `dx_voice/kokoro_tts.rs` |
| ElevenLabs | ✅ DONE | `dx_voice/elevenlabs_tts.rs` |
| Fish Audio | ✅ DONE | `dx_voice/fish_audio_tts.rs` |
| Cartesia | ✅ DONE | `dx_voice/cartesia_tts.rs` |
| PlayHT | ✅ DONE | `dx_voice/playht_tts.rs` |
| Deepgram Aura | ✅ DONE | `dx_voice/deepgram_tts.rs` |
| Google Cloud TTS | ✅ DONE | `dx_voice/google_cloud_tts.rs` |
| Amazon Polly | ✅ DONE | `dx_voice/amazon_polly_tts.rs` |
| Azure Speech | ✅ DONE | `dx_voice/azure_speech_tts.rs` |
| OpenAI TTS | ✅ DONE | `dx_voice/openai_tts.rs` |
| WellSaid / Murf / Lovo | ✅ DONE | `dx_voice/extra_cloud_tts.rs` |
| Quality Router | ✅ DONE | `dx_voice/quality_router.rs` |

### Full-Duplex Voice Conversation Loop (`dx_voice/full_duplex.rs`)
1. User speaks → Whisper transcribes locally
2. LLM post-processes transcription (course correction)
3. LLM generates response
4. TTS speaks back (streaming — starts speaking before full response generated)
5. User can interrupt (energy-based VAD detects speech → TTS stops)
6. Conversation history maintained for multi-turn sessions

### Quality Router Logic
- Short UI responses → fast local Piper (zero cloud cost)
- Long narration → Chatterbox-Turbo (human quality, free)
- Premium/cloned voice requests → cloud TTS

---

## 13. OS-Level Input Interception

Located in `crates/dx_input_intercept`. Extends grammar-checking and edit prediction to **every text field on the OS**, not just inside DX.

| Platform | Technology | Status |
|---|---|---|
| macOS | CGEventTap + IMK, AXUIElement for text access, transparent NSWindow overlay | ✅ DONE |
| Windows | TSF + low-level hooks, UI Automation API, WS_EX_LAYERED overlay | ✅ DONE |
| Linux X11 | IBus + XInput2, AT-SPI2, override-redirect GPUI/Vulkan window | ✅ DONE |
| Linux Wayland | Fcitx5 + input-method-v2, AT-SPI2, layer shell protocol | ✅ DONE |

Supporting crates:
- `arboard` — cross-platform clipboard read/write
- `get-selected-text` — read selected text across all apps
- `global-hotkey` — cross-platform hotkey bindings (from Tauri ecosystem)

---

## 14. Computer Use / OS Control

Located in `crates/dx_computer_use`.

### Capabilities
| Feature | Implementation | Status |
|---|---|---|
| Mouse movement / click / drag | `dx_computer_use/input.rs` (Windows: user32, macOS: cliclick, Linux: xdotool) | ✅ DONE |
| Keyboard typing + key press | `dx_computer_use/input.rs` | ✅ DONE |
| Full screen / region / window capture | `dx_computer_use/capture.rs` (Windows CopyFromScreen, macOS screencapture, Linux scrot) | ✅ DONE |
| Accessibility tree traversal | `dx_computer_use/platform_accessibility.rs` (Windows UIA COM, macOS python3+osascript, Linux AT-SPI2) | ✅ DONE |
| Template matching (find element on screen) | via `rustautogui` built-in — Segmented Normalized Cross-Correlation | ✅ DONE |
| Open application | `dx_computer_use/actions.rs` | ✅ DONE |
| Run shell command | `dx_computer_use/actions.rs` | ✅ DONE |
| Vision model (screenshot understanding) | LLaVA-1.5-7B Q4_K_M (Tier 5) / GPT-4V / Claude Vision | QUEUED |
| Safety boundaries / allowlists | Per-app permissions config | QUEUED |

---

## 15. Background Agent Daemon

Located in `crates/dx_daemon`. A 24/7 system service.

### OS Service Installation
- Linux: systemd unit (real `systemctl enable`)
- macOS: launchd plist (real `launchctl load`)
- Windows: SC.exe service (real Windows Service registration)
- One command: `dx service install`

### Supervisor (`dx_daemon/supervisor.rs`)
- Manages named agent processes
- Auto-restart crashed agents with exponential backoff (max 5 min)
- Supervises HashMap of `SupervisedProcess` entries

### Cron Scheduler (`dx_daemon/cron.rs`)
- Full 5-field cron expression parser
- Fields: Any (`*`), Value, List, Range, Step
- Named shortcuts: `@yearly`, `@monthly`, `@weekly`, `@daily`, `@hourly`
- `.matches(datetime)` — check if a cron fires at a given time
- `.describe()` — human-readable description
- Example: `"Fetch news every 8am"`, `"Summarize emails at 6pm"`

### Channel Router (`dx_daemon/channel.rs`)
Real HTTP API implementations for 7 messaging channels:
| Channel | Implementation |
|---|---|
| Telegram | Bot API — send_telegram |
| Discord | Webhook with 2000-char splitting |
| Slack | Incoming webhook |
| Email | SendGrid + sendmail fallback |
| Generic Webhook | JSON payload |
| SMS | Twilio API |
| WhatsApp | Meta Cloud API v18.0 |

### VPS Deploy (`dx_daemon/vps.rs`)
- `dx deploy --host user@server` — one command provisions + deploys
- Real cloud API provisioning: Hetzner, DigitalOcean, Fly.io
- SCP binary upload, SSH systemd unit creation
- Waits for SSH health polling before deploying
- `check_health()` + `get_logs()` via SSH

### Agent Identity (`dx_daemon/agent_identity.rs`)
- AIEOS-compatible JSON persona
- IDENTITY.md parsing and migration from OpenClaw format
- `CommunicationStyle` preferences per agent

### Remote Monitor (`dx_daemon/remote_monitor.rs`)
- Heartbeat timeout detection
- `RemoteAgent` + `RemoteSystemInfo` structs
- `SecureChannel` SSH tunnel abstraction

---

## 16. DX UI Components

Located in `crates/dx_ui/src/`. All built with GPUI.

| Component | File | Description |
|---|---|---|
| `ProfileSwitcher` | `profile_switcher.rs` | Cycles between 6 AI profiles with icons |
| `PlanView` | `plan_view.rs` | Plan mode: PlanItem list, completion toggle, generation state |
| `StudyView` | `study_view.rs` | 3-column: sources / chat / studio (NotebookLM-style) |
| `ComingSoonView` | `coming_soon_view.rs` | 6 feature stubs for Deep Research & Search |
| `DxSidebar` | `dx_sidebar.rs` | Notion-style left sidebar: page tree, workspace dot-nav, home/search/new |
| `MoodActionBar` | `mood_action_bar.rs` | 7 mood toggles (Text/Image/Audio/Video/Live/3D/PDF) + action buttons |
| `SessionHistoryRail` | `session_history_rail.rs` | Chat session history grouped by Today/Yesterday/ThisWeek/Older |
| `FloatingAiPanel` | `floating_ai_panel.rs` | 3 sizes: Compact 320×480, Medium 480×640, Full 640×800 — move/resize/pin/collapse |
| `FlowBarUi` | `flow_bar_ui.rs` | Persistent bottom-center voice widget: Idle/Listening/Transcribing/PostProcessing/Result/Speaking states |
| `AiFaceWidget` | `ai_face_widget.rs` | GPU-rendered procedural avatar: eye tracking, blinking, mouth animation, 7 expressions, glow ring |
| `MediaPreview` | `media_preview.rs` | Unified preview for Image/Video/Audio/3D/PDF/Document with progress bars |
| `TierDisplay` | `tier_display.rs` | Hardware summary + 5-tier buttons + auto/manual tier override |
| `ModelDownloadUi` | `model_download_ui.rs` | Download queue, progress bars, pause/resume/cancel, disk warnings |
| `SocialShareUi` | `social_share_ui.rs` | Share popover: Facebook, WhatsApp, Twitter, Telegram, Discord, Signal, + more |
| `VisualPolish` | `visual_polish.rs` | Design tokens: `DxSpacing` (8 steps), `DxTypography` (8 levels), `DxLabels`, `DxZIndex`, `DxDuration`, `DxEasing` |

### AI Chat Input Features
- Rounded center-focused input box (not sidebar)
- **Top left**: Attach icon (add files/screenshots/clipboard), Follow agent button
- **Top right**: Zoom icon, Enhance prompt icon, Context pie chart
- **Bottom left**: Target selector (Local/Background/Cloud), AI profile selector, AI model selector
- **Bottom right**: Follow Zed agent button, Submit/Stop button
- 7-mood toolbar (Text/Image/Video/Live/Audio/3D/PDF) next to submit button — shows 4 + More icon
- Session scroll rail (right side) — vertical lines for user input (thin) and AI responses (thick), with tooltip on hover, click-to-scroll
- All selectors have border with primary color + background highlight when selected

### AI Profiles
1. **Agent** — chat-mode with tool use
2. **Ask** — simple Q&A
3. **Plan** — structured plan generation with completion tracking
4. **Study** — NotebookLM-style 3-column research workspace
5. **Deep Research** — extended web + document research (coming soon)
6. **Search** — SearXNG-style search results (coming soon)

---

## 17. GPUI — The Custom UI Framework

Located in `crates/gpui` (core), `crates/gpui_wgpu`, `crates/gpui_windows`, `crates/gpui_macos`, `crates/gpui_linux`, `crates/gpui_platform`, `crates/gpui_tokio`, `crates/gpui_util`, `crates/gpui_macros`, `crates/gpui_web`.

### What GPUI Is
A **hybrid immediate and retained mode, GPU-accelerated UI framework** built entirely in Rust for the Zed editor. It provides:
- State management via **Entities** (`Entity<T>`) with `.read()`, `.update()`, `.update_in()`
- Declarative UI via **Views** and **Elements**
- GPU-accelerated rendering via **wgpu** (WGSL shaders)
- **Tailwind-inspired** method chaining API
- Cross-platform: macOS (Metal), Windows (DirectX 12/WGPU), Linux (Vulkan/WGPU), Web (WASM)

### Core Element Types
```rust
div()            // Swiss-army container — flex, padding, colors, borders
h_flex()         // Horizontal flex (shorthand)
v_flex()         // Vertical flex (shorthand)
Label::new()     // Text display
Button::new()    // Interactive button
Icon::new()      // SVG icon display
```

### Styling API (Tailwind-style)
```rust
div()
    .flex()
    .flex_col()
    .gap_2()         // 8px gap
    .p_4()           // 16px padding
    .rounded_xl()    // 12px border-radius
    .bg(cx.theme().colors().background)
    .border_1()
    .shadow_md()
    .overflow_x_hidden()
    .min_w_0()       // enables text truncation
    .child(Label::new("Hello").truncate())
```

### Context Types
| Context | Purpose |
|---|---|
| `App` | Application-level operations |
| `Context<T>` | Entity-specific reads/writes |
| `Window` | Window-level: focus, actions, drawing |
| `AsyncApp` | Async tasks with app access |
| `AsyncWindowContext` | Async tasks with window access |

### Entity System
```rust
// Create entity
let model = cx.new(|_cx| MyModel::default());

// Read
let value = model.read(cx).some_field;

// Update (triggers re-render)
model.update(cx, |model, cx| {
    model.some_field = new_value;
    cx.notify();
});
```

### Rendering Trait
```rust
impl Render for MyView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .child(Label::new("Hello World"))
    }
}
```

### Current GPUI Capabilities
- Linear gradients ✅
- Box shadows ✅
- Fade/slide animations via `AnimationExt` ✅
- SVG rendering (static) via `resvg` ✅
- Canvas element for custom drawing (`canvas()`) ✅
- Radial/mesh gradients ❌ (not yet implemented)
- Inner glow / backdrop blur ❌ (not yet implemented)
- SVG animations (SMIL/CSS) ❌ (resvg/usvg don't support them)
- Transform matrices ❌ (limited)

### Icon System
DX has a custom `icon` CLI tool for managing 100,000+ icons from 250+ packs:
```bash
icon search <query>              # search icons
icon export <query> <dir>        # export to directory
icon desktop search:lucide home:solar   # export to assets/icons/
icon packs                       # list available packs
```
Popular packs: `lucide`, `solar`, `heroicons`, `feather`, `material-symbols`, `tabler`, `carbon`

---

## 18. All Crates — Complete Directory

<details>
<summary>Click to expand — full list of 220+ crates</summary>

### Editor & Text Core
- `editor` — main editor view
- `multi_buffer` — multi-file editing surface
- `rope` — O(log n) text rope
- `text` — text primitives
- `buffer_diff` — buffer-level diff computation
- `streaming_diff` — streaming diff for AI edits
- `language` — language server protocol
- `languages` — bundled language grammar configs
- `lsp` — LSP client
- `markdown` — Markdown parser
- `markdown_preview` — Markdown preview panel

### UI Framework
- `gpui` — core GPUI framework
- `gpui_wgpu` — WGPU rendering backend
- `gpui_windows` — Windows platform layer
- `gpui_macos` — macOS platform layer
- `gpui_linux` — Linux platform layer
- `gpui_platform` — platform abstraction
- `gpui_tokio` — Tokio integration
- `gpui_util` — GPUI utilities
- `gpui_macros` — GPUI procedural macros
- `gpui_web` — WASM/web target
- `ui` — base UI components (Label, Button, Icon)
- `ui_input` — text input components
- `ui_macros` — UI procedural macros
- `ui_prompt` — prompt/dialog components
- `component` — reusable component library
- `component_preview` — component storybook preview
- `storybook` — interactive component browser
- `story` — story framework
- `inspector_ui` — GPUI inspector
- `miniprofiler_ui` — performance profiler UI

### AI & LLM
- `agent` — Zed native agent
- `agent_ui` — agent panel UI
- `agent_settings` — agent configuration
- `agent_servers` — agent server connections
- `acp_thread` — ACP protocol threads
- `acp_tools` — ACP protocol tools
- `language_model` — unified language model abstraction
- `language_models` — concrete language model configs
- `anthropic` — Anthropic API client
- `open_ai` — OpenAI API client
- `google_ai` — Google AI API client
- `bedrock` — AWS Bedrock API client
- `codestral` — Codestral (Mistral code) API
- `deepseek` — DeepSeek API client
- `x_ai` — xAI Grok API client
- `mistral` — Mistral API client
- `open_router` — OpenRouter API client
- `ollama` — Ollama local API client
- `lmstudio` — LM Studio API client
- `vercel` — Vercel AI Gateway client
- `copilot` — GitHub Copilot integration
- `copilot_chat` — Copilot Chat
- `copilot_ui` — Copilot UI
- `supermaven` — Supermaven completions
- `supermaven_api` — Supermaven API
- `cloud_llm_client` — Zed cloud LLM
- `cloud_api_client` — Zed cloud API
- `cloud_api_types` — Zed cloud types
- `context_server` — MCP context server
- `prompt_store` — prompt management
- `assistant_slash_command` — slash command base
- `assistant_slash_commands` — slash command library
- `assistant_text_thread` — text thread for AI
- `rules_library` — AI rules constraints
- `zeta_prompt` — Zeta edit prediction prompts

### Edit Prediction
- `edit_prediction` — Zeta edit prediction engine
- `edit_prediction_cli` — CLI for predictions
- `edit_prediction_context` — context extraction
- `edit_prediction_types` — prediction types
- `edit_prediction_ui` — prediction UI (ghost text)

### DX Subsystems
- `dx_core` — shared foundation
- `dx_ai_ui` — AI UI components
- `dx_computer_use` — OS control
- `dx_daemon` — background agent
- `dx_grammar` — grammar pipeline
- `dx_hardware` — hardware detection
- `dx_inference` — local ML inference
- `dx_input_intercept` — OS input interception
- `dx_llm_adapters` — additional LLM adapters
- `dx_llm_bridge` — LLM bridge
- `dx_local_inference` — local inference management
- `dx_media` — media generation
- `dx_orchestrator` — multi-modal orchestration
- `dx_providers` — 21 LLM provider adapters
- `dx_ui` — DX UI components
- `dx_voice` — voice engine

### Collaboration & Networking
- `collab` — collab server
- `collab_ui` — collaboration UI
- `call` — audio/video calling
- `channel` — team channels
- `livekit_client` — LiveKit client
- `livekit_api` — LiveKit API
- `client` — Zed cloud client
- `rpc` — RPC protocol
- `proto` — protobuf definitions
- `remote` — remote development
- `remote_connection` — remote connections
- `remote_server` — remote server daemon
- `net` — network utilities
- `reqwest_client` — HTTP client wrapper
- `http_client` — HTTP abstractions
- `http_client_tls` — TLS HTTP client
- `aws_http_client` — AWS-signed HTTP
- `social_sharing` — social media sharing

### Git
- `git` — git operations
- `git_ui` — git UI (blame, diff, staging)
- `git_graph` — commit history
- `git_hosting_providers` — GitHub/GitLab/Bitbucket

### File System & Projects
- `project` — project management
- `worktree` — worktree management
- `fs` — filesystem abstraction
- `paths` — path utilities
- `watch` — file watching
- `project_panel` — file explorer
- `outline_panel` — symbol outline

### Terminal & REPL
- `terminal` — PTY/terminal backend
- `terminal_view` — terminal UI
- `repl` — Jupyter-style REPL

### Debugger
- `dap` — Debug Adapter Protocol
- `dap_adapters` — DAP adapter impls
- `debugger_ui` — debugger UI
- `debugger_tools` — debugger utilities
- `debug_adapter_extension` — extension debug adapters

### Extensions
- `extension` — extension runtime
- `extension_host` — extension host (WASM sandbox)
- `extension_api` — public extension API
- `extension_cli` — extension CLI
- `extensions_ui` — extensions manager UI
- `language_extension` — language extension support
- `theme_extension` — theme extension support

### Themes & Settings
- `theme` — theme engine
- `theme_importer` — import VSCode/etc. themes
- `theme_selector` — theme picker
- `settings` — settings engine
- `settings_ui` — settings UI
- `settings_json` — JSON settings parsing
- `settings_content` — settings content types
- `settings_macros` — settings proc macros
- `settings_profile_selector` — profile switcher

### Search & Navigation
- `search` — project/buffer search
- `file_finder` — fuzzy file finder
- `project_symbols` — symbol search
- `outline` — document outline
- `fuzzy` — fuzzy matching engine
- `go_to_line` — go to line
- `recent_projects` — recent project history
- `tab_switcher` — tab management

### Viewers
- `image_viewer` — image preview
- `markdown_preview` — Markdown preview
- `svg_preview` — SVG preview

### Miscellaneous Editor
- `breadcrumbs` — editor breadcrumb trail
- `diagnostics` — diagnostics panel
- `task` — task system
- `tasks_ui` — tasks UI
- `snippet` — snippet engine
- `snippets_ui` — snippets management
- `snippet_provider` — snippet providers
- `vim` — Vim mode
- `vim_mode_setting` — Vim setting
- `prettier` — Prettier formatting
- `journal` — daily journal
- `feedback` — in-app feedback
- `keymap_editor` — keymap editor UI
- `dev_container` — Dev Container support
- `which_key` — keybinding overlay
- `notifications` — notification system
- `onboarding` — first-launch onboarding
- `ai_onboarding` — AI onboarding
- `language_onboarding` — language onboarding
- `auto_update` — auto-update system
- `auto_update_ui` — auto-update UI
- `auto_update_helper` — platform-specific update helpers
- `crashes` — crash reporting
- `telemetry` — usage analytics
- `telemetry_events` — telemetry event types
- `session` — session management
- `feature_flags` — feature flag system
- `release_channel` — update channel (stable/preview/nightly)
- `migrator` — settings/schema migration

### Platform-Specific
- `install_cli` — install CLI symlink
- `askpass` — SSH password helper
- `credentials_provider` — OS keychain abstraction
- `etw_tracing` — Windows ETW tracing
- `explorer_command_injector` — Windows Explorer integration
- `platform_title_bar` — native title bar
- `title_bar` — DX title bar

### System & Utilities
- `util` — general utilities
- `util_macros` — utility macros
- `collections` — custom collections (BTreeMap enhancements)
- `sum_tree` — persistent B-tree for text
- `rope` — rope data structure
- `clock` — hybrid logical clocks
- `time_format` — time formatting
- `refineable` — settings refinement system
- `rich_text` — rich text rendering
- `icons` — icon asset management
- `file_icons` — file-type icons
- `audio` — audio playback (system sounds)
- `media` — media types
- `encoding_selector` — file encoding detection/selector
- `line_ending_selector` — CRLF/LF selector
- `language_selector` — syntax language selector
- `toolchain_selector` — language toolchain selector
- `menu` — context menu
- `picker` — generic picker/autocomplete
- `panel` — panel docking system
- `sidebar` — sidebar management
- `workspace` — top-level workspace layout

### Database & Storage
- `db` — local SQLite database
- `sqlez` — async SQLite wrapper
- `sqlez_macros` — sqlez proc macros

### Observability
- `zlog` — structured logging
- `zlog_settings` — log settings
- `ztracing` — distributed tracing
- `ztracing_macro` — tracing macros

### Networking & Providers
- `n8n_engine` — N8N workflow integration
- `web_search` — web search engine integration
- `web_search_providers` — web search providers (SearXNG, etc.)
- `scheduler` — task scheduler
- `nc` — utility
- `html_to_markdown` — HTML→Markdown conversion
- `json_schema_store` — JSON schema registry
- `denoise` — audio denoising
- `providers` — free AI provider adapters
- `eval` — model evaluation
- `eval_utils` — evaluation utilities
- `zed_actions` — top-level action definitions
- `zed_env_vars` — environment variable handling
- `system_specs` — system specification reporting

### Benchmarks
- `project_benchmarks` — project system benchmarks
- `worktree_benchmarks` — worktree benchmarks
- `fs_benchmarks` — filesystem benchmarks

</details>

---

## 19. Key DX Crates — Details

### `dx_core`
**The shared foundation.** Both UI agents (Alpha) and backend agents (Beta) depend on this.

Key modules:
| Module | Contents |
|---|---|
| `cost.rs` | `MicroCost`, `TokenPricing`, `MediaPricing`, `CostTracker`, `BudgetConfig` |
| `device_tier.rs` | `DeviceTier`, `HardwareProfile`, `ModelRecommendation`, full hardware detection |
| `llm_provider.rs` | `LlmProvider` trait, `LlmFallbackChain`, `OpenAiCompatibleConfig` |
| `media_provider.rs` | `MediaProvider` trait, all well-known provider IDs |
| `tts_provider.rs` | `TtsProvider` trait, `TtsFallbackChain`, all well-known TTS IDs |
| `mood.rs` | `Mood` enum, `MoodActionSet`, `actions_for_mood()` |
| `profile.rs` | `AiProfile` enum: Chat, Code, Plan, Study, DeepResearch, Search |
| `provider_registry.rs` | `DxProviderRegistry` (LLM + Media + TTS combined) |
| `rate_limiter.rs` | `RateLimiter` (sliding-window RPM) |
| `session.rs` | `SessionEntry`, `SessionGroup`, `group_sessions_by_date()` |
| `config.rs` | `DxConfig`, `CachedHardwareProfile`, `ProviderKeyRef`, `ModelDownloadState` |
| `dx_core.rs` | `init()` — first launch hardware detection + logging |

### `dx_providers` (21 LLM Adapters)
```
openai_adapter.rs        anthropic_adapter.rs    google_ai_adapter.rs
ollama_adapter.rs        openai_compatible.rs    bedrock_adapter.rs
azure_openai_adapter.rs  mistral_adapter.rs      deepseek_adapter.rs
x_ai_adapter.rs          groq_adapter.rs         fireworks_adapter.rs
together_adapter.rs      cohere_adapter.rs       nvidia_nim_adapter.rs
lm_studio_adapter.rs     huggingface_adapter.rs  replicate_llm_adapter.rs
open_router_adapter.rs   vercel_adapter.rs       provider_bridge.rs
```

### `dx_inference` (6 Files)
```
candle_backend.rs    — Primary Candle ML backend (CUDA/Metal/CPU, GGUF)
llama_backend.rs     — llama.cpp FFI backend (maximum GGUF compatibility)
model_cache.rs       — 14 pre-configured models, download manager, SHA256 verify
download_manager.rs  — hf-hub integration, progressive download
gpu_memory.rs        — concurrent model loading, memory sharing
```

### `dx_media` (Cloud Adapters)
```
runway.rs            kling_ai.rs     pika.rs         luma_ai.rs
minimax_video.rs     synthesia.rs    replicate.rs    suno_ai.rs
udio.rs              stability_audio.rs  meshy.rs    tripo_ai.rs
adobe_pdf.rs         apitemplate.rs  carbone.rs      craftmypdf.rs
docraptor.rs         pdf_co.rs       pdfshift.rs     quickchart.rs
document_generator.rs   output_cache.rs
```

### `dx_orchestrator` (4 Files)
```
plan.rs          — GenerationPlan: task graph with dependencies
decomposer.rs    — RequestDecomposer: keyword-based task detection, 7 task types
executor.rs      — ParallelExecutor: max_concurrency=4, topological ordering
cost_summary.rs  — CostSummary: per-provider + per-task-type + format_report()
```

### `dx_daemon` (6+ Files)
```
service.rs           — OS service install (systemd/launchd/sc.exe)
supervisor.rs        — SupervisedProcess, max 5min backoff, Supervisor
cron.rs              — CronExpression 5-field parser, named shortcuts
channel.rs           — 7 channel APIs (Telegram, Discord, Slack, email, SMS, WhatsApp, webhook)
agent_identity.rs    — AgentIdentity, IDENTITY.md parsing, CommunicationStyle
remote_monitor.rs    — RemoteMonitor, RemoteAgent, SecureChannel
vps.rs               — Real cloud provisioning (Hetzner/DO/Fly.io), SCP, SSH
```

### `dx_grammar` (7 Files)
```
harper_tier.rs       — Harper <10ms tier (rule-based)
nlprule_tier.rs      — nlprule <50ms tier (4000+ LanguageTool patterns)
llm_tier.rs          — Local LLM <500ms tier (context-aware)
detection.rs         — Language detection (whichlang/lingua-rs)
segmentation.rs      — Unicode word/sentence boundaries
fuzzy_match.rs       — edit_distance, phonetic_code, suggest_corrections (analiticcl)
app_detection.rs     — AppCategory, AppWritingProfile, detect_category()
```

### `dx_hardware` (4 Files)
```
model_swapper.rs       — PowerState, ResourceSnapshot, SwapDecision, ModelSwapper
npu.rs                 — NPU detection: Intel/Qualcomm/Apple/AMD NPU
system_analysis.rs     — AI workload scoring, bottleneck detection
(+ dx_core handles HardwareProfile::detect())
```

### `dx_input_intercept` (6 Files)
```
platform_intercept.rs  — macOS/Windows/X11/Wayland input hooks
text_field_access.rs   — AXUIElement/UIA text field reading
clipboard.rs           — arboard + get-selected-text
hotkey.rs              — global-hotkey bindings
overlay.rs             — transparent GPUI overlay window
```

### `dx_computer_use` (8 Files)
```
automation.rs              — Cross-platform automation orchestrator
screen_capture.rs          — Screenshot coordination
accessibility_toolkit.rs   — AccessKit tree traversal
platform_accessibility.rs  — Platform-specific AX tree (Windows UIA, macOS, Linux)
input.rs                   — Mouse/keyboard: user32/cliclick/xdotool
capture.rs                 — Screen capture: CopyFromScreen/screencapture/scrot
actions.rs                 — execute(), open_application(), run_shell_command()
screenshot.rs              — capture_full()/capture_region() wrappers
```

---

## 20. External Dependencies — Key Crates

### Inference & AI
| Crate | Purpose |
|---|---|
| `candle-core` | Primary ML framework (Hugging Face) |
| `candle-transformers` | Pre-built model architectures |
| `candle-nn` | Neural network layers |
| `llama-cpp-rs` / `llama-cpp-2` | llama.cpp FFI (GGUF compat) |
| `hf-hub` | Hugging Face model downloads |
| `tokenizers` | Hugging Face tokenizers |
| `tiktoken-rs` | OpenAI token counting |

### Grammar
| Crate | Purpose |
|---|---|
| `harper-core` | Primary grammar engine <10ms |
| `nlprule` | 4000+ LanguageTool patterns, no Java |
| `zspell` | Hunspell spell check, 100+ languages |
| `analiticcl` | Fuzzy string matching |
| `unicode-segmentation` | Word/sentence boundaries |
| `whichlang` / `lingua-rs` | Language detection |

### Voice
| Crate | Purpose |
|---|---|
| `piper-rs` | Piper TTS local voice synthesis |
| `whisper-rs` | Whisper STT (GPU: Metal/CUDA) |
| `whisper-cpp-plus` | Streaming Whisper + Silero VAD |
| `cpal` | Cross-platform audio I/O |
| `rubato` | Audio resampling |
| `rodio` | Audio playback |
| `webrtc-vad` | Voice activity detection |

### OS Integration
| Crate | Purpose |
|---|---|
| `sysinfo` | CPU, memory, disk, processes |
| `global-hotkey` | Cross-platform hotkeys |
| `get-selected-text` | Read selected text globally |
| `arboard` | Clipboard read/write |
| `rustautogui` | Mouse/keyboard/template matching |
| `accesskit` | Cross-platform accessibility |
| `tray-icon` | System tray icon |
| `windows` crate | Windows API (TSF, UIA, Win32) |
| `objc2` | macOS Objective-C FFI |
| `atspi` | Linux AT-SPI2 accessibility |
| `signal-hook` | Unix signal handling |

### Networking & HTTP
| Crate | Purpose |
|---|---|
| `reqwest` | HTTP client for all API calls |
| `eventsource-stream` | SSE parsing for streaming |
| `async-stream` | Async streams |
| `tokio` | Async runtime |
| `axum` + `tower` | AI Gateway HTTP server |
| `aws-sigv4` | SigV4 signing for Bedrock |
| `serde` + `serde_json` | JSON serialization |

### Rate Limiting & Cost
| Crate | Purpose |
|---|---|
| `governor` | Token-bucket rate limiting |
| `backoff` / `again` | Exponential backoff with jitter |
| `dashmap` | Concurrent in-memory maps |
| `sqlx` | Database for virtual keys, spend tracking |
| `redis` | Cache + rate limit state |
| `argon2` / `sha2` | Key hashing |

### UI Rendering
| Crate | Purpose |
|---|---|
| `wgpu` | GPU rendering backend |
| `similar` | Text diffs for ghost text |
| `aho-corasick` | Multi-pattern matching for guardrails |

### Document Generation
| Crate | Purpose |
|---|---|
| `genpdf` | High-level PDF generation |
| `printpdf` | Full PDF spec control |
| `typst` | LaTeX-quality typesetting |
| `rust_xlsxwriter` | Excel generation |
| `plotters` | 2D/3D chart generation |
| `pulldown-cmark` | Markdown→HTML |
| `resvg` / `usvg` | SVG rendering |

### Messaging (Daemon)
| Crate | Purpose |
|---|---|
| `teloxide` | Telegram Bot API |
| `serenity` | Discord Bot |
| `slack-morphism` | Slack API |
| `matrix-sdk` | Matrix/Element protocol |

### Observability
| Crate | Purpose |
|---|---|
| `tracing` + `tracing-subscriber` | Structured logging |
| `opentelemetry` | Distributed tracing |
| `prometheus` | Metrics for proxy server |
| `thiserror` + `anyhow` | Error handling |

---

## 21. Build System

This is a **Windows-first** development environment (Git Bash on Windows 11 Pro).

### Mandatory Build Command
```bash
CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=1 CARGO_PROFILE_DEV_CODEGEN_UNITS=1 CARGO_PROFILE_DEV_BUILD_OVERRIDE_CODEGEN_UNITS=1 cargo run -p zed --locked
```

**NEVER** run `cargo build -p <crate>` or `cargo check -p <crate>` individually — each creates a separate artifact tree wasting gigabytes on the limited F: drive (~5GB free, 95% used).

### Build Environment
- **OS**: Windows 11 Pro
- **Shell**: Git Bash
- **RAM**: 7.3GB (critical constraint)
- **Disk**: F: drive, ~5GB free
- **Target dir**: `F:\Desktop\target`
- **Toolchain**: Stable Rust (see `rust-toolchain.toml`)
- **Requirements**: MSVC Build Tools, long paths enabled

### Recommended Persistent Settings (PowerShell profile)
```powershell
$env:CARGO_TARGET_DIR = "F:\zed-target"
$env:CARGO_BUILD_JOBS = "1"
$env:CARGO_INCREMENTAL = "1"
$env:CARGO_PROFILE_DEV_CODEGEN_UNITS = "1"
$env:CARGO_PROFILE_DEV_BUILD_OVERRIDE_CODEGEN_UNITS = "1"
```

### Clippy & Code Quality
```bash
./script/clippy     # Run clippy (use this, not cargo clippy directly)
```

All Clippy warnings from changed code must be treated as errors and fixed.

### Workspace Config
- `resolver = "2"` (Rust 2024 edition features)
- `publish = false` for all workspace members
- `default-members = ["crates/zed"]` — only zed binary built by default
- Lockfile always respected (`--locked`)

---

## 22. Configuration & Extensibility

### Settings System
- JSON-based settings files at `~/.config/zed/settings.json`
- Refineable settings (workspace overrides global)
- Profile-based settings (`crates/settings_profile_selector`)
- Settings UI editor (`crates/settings_ui`)
- Settings migration (`crates/migrator`)
- JSON schema validation (`crates/json_schema_store`)

### Keymaps
- JSON-based keymaps at `assets/keymaps/`
- Default, Vim, Emacs, JetBrains, VS Code, Atom keymaps bundled
- Keymap editor UI (`crates/keymap_editor`)
- Which-key overlay for keybinding discovery (`crates/which_key`)
- Global command palette (`crates/command_palette`)

### Themes
- JSON theme files at `assets/themes/`
- Theme importer for VSCode/Atom themes
- Theme selector UI
- Extension-loadable themes

### Extensions (WebAssembly)
- Extensions sandboxed in WASM (`crates/extension_host`)
- Extension API: language server support, themes, snippets, slash commands, debug adapters
- Extensions UI for browsing/installing (`crates/extensions_ui`)
- Bundled: GLSL, HTML, Proto, test-extension

### Context Servers (MCP)
- Model Context Protocol support (`crates/context_server`)
- AI tools through MCP extensions

### Task System
- Custom build/run tasks in `.zed/tasks.json`
- Task runner with terminal integration

---

## 23. Implementation Status (Phase Tracking)

### Phase A — Core UI Shell ✅ DONE (build pending verification)
- Center AI panel with rounded input
- Six AI profiles (Agent, Ask, Plan, Study, DeepResearch, Search)
- Notion-style left sidebar
- Mood/media toggle system (7 moods)
- Session history rail
- Floating AI panel (3 sizes)

### Phase B — LLM Provider Infrastructure ✅ DONE
- 21 LLM provider adapters
- Fallback chains, cost tracking, rate limiting
- 100+ providers via Tier 3 generic adapter
- Free models (7 total) always visible

### Phase C — Media Generation Infrastructure ✅ DONE
- Image, video, audio, music, 3D, PDF cloud adapters
- Output caching, rate limiting, parallel generation
- In-panel previews for all media types

### Phase D — Hardware Adaptive Intelligence ✅ DONE
- Real hardware detection (RAM, VRAM, GPU, NPU, battery)
- 5-tier classification + model recommendations
- Config persistence (`~/.dx/dx_config.json`)
- Dynamic model swapping (power/RAM/idle awareness)
- Model download UI with progress/pause/resume

### Phase E — Grammar Engine ✅ DONE
- Three-tier pipeline (Harper → nlprule → LLM)
- 4-color severity system
- Language detection, Unicode segmentation, fuzzy correction
- Context-aware writing profiles (6 app categories)
- OS-level input interception (macOS/Windows/X11/Wayland)

### Phase F — Voice Engine (PARTIALLY DONE)
- ✅ Local TTS: Piper, Chatterbox-Turbo, Kokoro (6 presets)
- ✅ 13 cloud TTS providers + quality router
- ✅ Full-duplex conversation loop (with interrupt detection)
- 🔄 STT integration (whisper-rs, cpal, VAD): QUEUED
- 🔄 Audio playback (rodio): QUEUED

### Phase G — Voice UI ✅ DONE (integration pending)
- Flow bar with 6 states (Idle/Listening/Transcribing/PostProcessing/Result/Speaking)
- AI face widget (eye tracking, blinking, mouth animation, 7 expressions)

### Phase H — Background Daemon ✅ DONE
- Real OS service install (systemd/launchd/sc.exe)
- Supervisor with exponential backoff
- Full cron parser (5-field + named shortcuts)
- 7 messaging channel APIs
- VPS provisioning (Hetzner, DigitalOcean, Fly.io)
- Remote monitoring + SSH tunnel

### Phase I — Computer Use ✅ DONE
- Mouse/keyboard automation (Windows/macOS/Linux)
- Screen capture (all platforms)
- Accessibility tree traversal (Windows UIA, macOS AX, Linux AT-SPI2)
- Dry-run / live toggle

### Phase J — Social Sharing ✅ (partial)
- Social share UI (6 platforms)
- Backend REST: QUEUED

### Phase K — Visual Polish ✅ DONE
- Design tokens: `DxSpacing`, `DxTypography`, `DxZIndex`, `DxDuration`, `DxEasing`
- Dark/light theme support: QUEUED
- Responsive layouts: QUEUED

### Phase Orchestration ✅ DONE
- `dx_orchestrator`: decomposer, parallel executor, cost summary
- Handles multi-modal generation requests end-to-end

---

## 24. Multi-Agent Coordination Protocol

Two AI agents work on this codebase simultaneously:

| Agent | Owns | Phases |
|---|---|---|
| **Agent Alpha (🅰️)** | UI / frontend | A, G, J, K |
| **Agent Beta (🅱️)** | Backend / infrastructure | B, C, D, E, F, H, I |

### Coordination Rules
- A task marked `[IN PROGRESS 🅰️]` — Agent Beta must NOT touch it
- A task marked `[IN PROGRESS 🅱️]` — Agent Alpha must NOT touch it
- Mark as `[DONE]` immediately when finished
- `[BLOCKED ON 🅰️]` / `[BLOCKED ON 🅱️]` — signal cross-lane dependency

### Shared Crates (Coordinate Before Editing)
- `crates/dx_core/` — foundation types used by both lanes
- `crates/zed/` — main app wiring
- `Cargo.toml` (workspace root)
- `crates/workspace/` — workspace rendering (Alpha leads, Beta consults)

### Worktree Solution
DX uses Zed's native worktree system (`crates/worktree`) to allow multiple agents to work on the same Git repo simultaneously without merge conflicts — each agent gets its own Git worktree branch with automatic conflict-free merging back to main.

---

## Appendix A: Key Documentation Files

| File | Purpose |
|---|---|
| [README.md](README.md) | Project overview, installation, contributing |
| [DX_PLAN.md](DX_PLAN.md) | Full technical architecture (1010 lines) |
| [DX_MASTERPLAN.md](DX_MASTERPLAN.md) | Comprehensive subsystem design + crate reference |
| [DX_TODO.md](DX_TODO.md) | Phase-by-phase task tracking (811 lines) |
| [GPUI_GUIDE.md](GPUI_GUIDE.md) | GPUI framework complete reference |
| [BUILD.md](BUILD.md) | Windows-specific build instructions |
| [ICONS.md](ICONS.md) | Icon CLI tool documentation |
| [WORKTREE.md](WORKTREE.md) | Multi-agent worktree strategy |
| [CONTRIBUTING.md](CONTRIBUTING.md) | Contribution guidelines |
| [AUDIO.md](AUDIO.md) | Audio subsystem plans |
| [IMAGE.md](IMAGE.md) | Image generation plans |
| [VIDEO.md](VIDEO.md) | Video generation plans |
| [LIVE.md](LIVE.md) | Live streaming plans |
| [PDF_DOCS_CHARTS.md](PDF_DOCS_CHARTS.md) | Document generation plans |
| [3D_AR_VR_XR_MR.md](3D_AR_VR_XR_MR.md) | 3D/AR/VR plans |
| [LITELLM.md](LITELLM.md) | LiteLLM replacement architecture |
| [N8N.md](N8N.md) | N8N workflow integration |
| [SYSTEM_QUICK_REFERENCE.md](SYSTEM_QUICK_REFERENCE.md) | Quick system reference |
| [MODELS_DEV_IMPLEMENTATION_COMPLETE.md](MODELS_DEV_IMPLEMENTATION_COMPLETE.md) | Model dev implementation status |
| [COMPLETED.md](COMPLETED.md) | Completed task log |

---

*This document was auto-generated on March 1, 2026, from full codebase analysis of the DX fork of Zed (`f:\Desktop`).*
