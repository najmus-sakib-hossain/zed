# DX Master TODO — The Universal AI Platform

> Derived from **DX_PLAN.md**. Covers all systems: UI, provider layers, inference engines,
> voice, media generation, grammar, hardware adaptation, daemon, computer use, and polish.

---

## Phase A: Core UI Shell

### Part 1: Center AI Panel + Rounded Input [IN PROGRESS]
- [x] Add `center_ai_mode` state to `Workspace` struct
- [x] Modify `Workspace::render()` to show AgentPanel centered when no files open
- [x] Add `is_centered` prop to `AgentPanel` render path
- [x] Style chat input: `max_w(680px)`, `rounded_xl()`, `border_1()`, `shadow_md()`, `mx_auto()`
- [x] Wire file open/close events to toggle `center_ai_mode`
- [ ] Build and verify

### Part 2: Six AI Profiles [QUEUED]
- [ ] Add PLAN, STUDY, DEEP_RESEARCH, SEARCH profile IDs
- [ ] Create `PlanView` component
- [ ] Create `StudyView` component (3-column: sources/chat/studio)
- [ ] Create `ComingSoonView` stub for Deep Research & Search
- [ ] Profile switcher UI with 6 entries + distinct icons
- [ ] Wire profile switch to transform entire panel content

### Part 3: Notion-Style Left Sidebar [QUEUED]
- [ ] Create `DxSidebar` panel struct
- [ ] Top zone: Home, Search, + New buttons
- [ ] Center zone: Notion-style page tree with sections
- [ ] Bottom zone: Dot-nav workspace switcher
- [ ] Register as default left dock panel (expanded)
- [ ] Embed ProjectPanel as collapsible section

### Part 4: Mood/Media Toggle System [QUEUED]
- [ ] Define `MoodActionSet` per mood (Text/Image/Audio/Video/Live/3D/PDF)
- [ ] Create `MoodActionBar` component
- [ ] Wire mood toggle to swap input action buttons
- [ ] Change send button label per mood
- [ ] Connect each mood to its corresponding media generation engine (Phase D)

### Part 5: Session History Rail [QUEUED]
- [ ] Create `SessionHistoryRail` component
- [ ] Group sessions by date
- [ ] Show in center mode on right side
- [ ] Click to load session

### Part 6: Floating AI Panel (Multi-Mode) [QUEUED]
- [ ] Compact mode (320×480) — quick questions, single-turn
- [ ] Medium mode (480×640) — working sessions, conversation
- [ ] Full mode (640×800) — deep work, multi-tool
- [ ] Support text input, voice input, file drops, screenshot paste
- [ ] Show generation progress (image/video/3D preview as it renders)
- [ ] Resize, move, pin, collapse back to avatar

---

## Phase B: Provider Infrastructure — Universe A (Language Intelligence)

### Part 7: Unified LLM Provider Abstraction (LiteLLM Replacement) [QUEUED]
> Replaces LiteLLM. 100+ LLM providers through a single abstraction layer.

- [ ] Define `LlmProvider` trait: `complete()`, `stream()`, `list_models()`, `embed()`
- [ ] **Tier 1 — Native Adapters (full SDK-level):**
  - [ ] OpenAI (Chat + Responses API)
  - [ ] Anthropic (Messages API)
  - [ ] Google (Gemini + Vertex AI)
  - [ ] AWS Bedrock (SigV4 auth)
  - [ ] Azure OpenAI (versioned endpoints)
  - [ ] Ollama (local)
- [ ] **Tier 2 — Named Adapters (provider-specific quirks):**
  - [ ] Mistral, Cohere, DeepSeek, xAI (Grok), Groq
  - [ ] Fireworks AI, Together AI, Hugging Face Inference
  - [ ] NVIDIA NIM, Replicate, Sagemaker, LM Studio
- [ ] **Tier 3 — OpenAI-Compatible Generic Adapter:**
  - [ ] Single adapter for 40+ providers: Cerebras, Perplexity, Venice AI, Baseten, Deep Infra, IO.NET, Moonshot AI, MiniMax, Nebius, OVHcloud, Scaleway, SiliconFlow, Inference.net, vLLM, GPUStack, llamafile, etc.
- [ ] **Tier 4 — Aggregator Multipliers:**
  - [ ] OpenRouter, Cloudflare AI Gateway, Vercel AI Gateway, Helicone, Cortecs, ZenMux, 302.AI
- [ ] **Tier 5 — Local Models:**
  - [ ] Ollama, LM Studio, llama.cpp, GPUStack, llamafile, Candle-native (embedded)
- [ ] Unified cost tracking per-provider (token-based pricing)
- [ ] Rate limiting (RPM limits per API key)
- [ ] Fallback chains (Provider A → Provider B → Provider C)
- [ ] Budget limits and alerts
- [ ] Provider health monitoring and auto-failover

### Part 8: Local Inference Engine [QUEUED]
> Embedded ML inference for offline/free operation.

- [ ] Integrate `candle-core` + `candle-transformers` + `candle-nn` as primary framework
  - [ ] CUDA support, Metal support, CPU fallback
  - [ ] GGUF quantization loading
- [ ] Integrate `llama-cpp-rs` / `llama-cpp-2` for maximum GGUF compatibility
- [ ] Integrate `hf-hub` for programmatic Hugging Face model downloads
- [ ] Model cache manager (download, verify, clean unused quantizations)
- [ ] Concurrent model loading (share GPU memory across grammar + prediction + voice)
- [ ] Progressive download strategy:
  - [ ] Second 0: Binary installs (~10MB)
  - [ ] Second 5: Hardware scan → tier classified
  - [ ] Second 10: Harper grammar loads (bundled, ~5MB)
  - [ ] Second 15: Piper TTS tiny downloads (~15MB)
  - [ ] Second 45: Whisper Tiny downloads (~75MB)
  - [ ] Second 90: SmolLM2/Qwen3 downloads (~200–400MB)
  - [ ] Second 180: Full model suite downloaded

---

## Phase C: Provider Infrastructure — Universe B (Media Generation)

### Part 9: Unified Media Provider Abstraction [QUEUED]
> Separate provider registry, separate cost tracking, separate API patterns from Universe A.

- [ ] Define `MediaProvider` trait: `generate()`, `stream_progress()`, `list_models()`, `estimate_cost()`
- [ ] Media type enum: Image, Video, Audio, Music, ThreeD, Document
- [ ] Per-provider cost tracking (per-image, per-second, per-request pricing)
- [ ] Rate limiting per API key
- [ ] Output caching (identical prompt + settings → cached result)
- [ ] Parallel generation orchestration (multiple media types simultaneously)

### Part 10: Image Generation Engine [QUEUED]
- [ ] **Local (Free, Unlimited, Offline):**
  - [ ] Stable Diffusion XL via Candle (Tier 4+ hardware, 6GB+ VRAM)
  - [ ] Flux.1 Schnell via Candle (local, open-source)
- [ ] **Cloud Adapters:**
  - [ ] OpenAI (DALL-E 3, GPT-Image-1.5)
  - [ ] Fal.ai (600+ models, fastest inference)
  - [ ] Stability AI (SDXL, SD3.5)
  - [ ] Replicate (200+ community models)
  - [ ] Google Imagen (via Vertex AI)
  - [ ] Midjourney (via API)
  - [ ] Adobe Firefly (commercially cleared)
  - [ ] DeepSeek Janus Pro
  - [ ] Black Forest Labs / Flux 2 (via fal.ai)
  - [ ] Recraft V3/V4 (logos, SVG, design assets)
  - [ ] Ideogram 3.0 (text rendering in images)
- [ ] Image preview panel in GPUI (inline rendering as generation completes)
- [ ] Prompt enhancement via LLM before sending to image provider

### Part 11: Video Generation Engine [QUEUED]
> Cloud only (for now) — video generation requires massive GPU.

- [ ] Runway Gen-3 Alpha adapter
- [ ] Kling AI (by Kuaishou) adapter
- [ ] Pika adapter
- [ ] Luma AI Dream Machine adapter
- [ ] Google Veo (via Vertex AI) adapter
- [ ] OpenAI Sora adapter
- [ ] Minimax / Hailuo adapter
- [ ] Synthesia adapter (AI avatar video)
- [ ] HeyGen adapter (AI avatar video, dubbing)
- [ ] Replicate video models adapter
- [ ] Fal.ai video models adapter
- [ ] Unified `generate_video()` interface with progress tracking and streaming
- [ ] Video preview panel in GPUI

### Part 12: Audio & Music Generation Engine [QUEUED]
- [ ] **Local:**
  - [ ] Sound effects via local diffusion models (Stability Audio Small, via Candle)
  - [ ] Basic music via local MusicGen (small) on Tier 4+ devices
- [ ] **Cloud Music Adapters:**
  - [ ] Suno AI (full song generation: vocals + instruments)
  - [ ] Udio (high-quality music)
  - [ ] Stability Audio
  - [ ] Meta MusicGen (via Replicate)
  - [ ] Google MusicFX
  - [ ] AIVA (classical/cinematic)
  - [ ] Mubert (real-time royalty-free)
- [ ] Audio waveform preview in GPUI
- [ ] `rodio` for playback of generated audio

### Part 13: 3D Asset Generation & Interactive Viewer [QUEUED]
- [ ] **Local:**
  - [ ] TripoSR (open-source, via Candle) for text-to-3D on Tier 4+ devices
- [ ] **Cloud Adapters:**
  - [ ] Meshy (text-to-3D, image-to-3D with PBR textures)
  - [ ] Tripo AI (fast 3D generation)
  - [ ] Luma AI Genie (3D from text/image)
  - [ ] Stability TripoSR
  - [ ] OpenAI Shap-E (3D from text)
  - [ ] CSM / Common Sense Machines (image-to-3D world)
  - [ ] Kaedim (production-ready 3D from images)
  - [ ] Rodin AI (3D avatar generation)
- [ ] `gltf` / `easy-gltf` crate integration for glTF 2.0 loading/writing
- [ ] Interactive 3D viewer in GPUI via `wgpu` (rotate, zoom, inspect)
- [ ] Export to glTF, OBJ, STL formats

### Part 14: PDF & Document Generation Engine [QUEUED]
> Entirely local. Zero cloud dependency. LLM generates structured content, Rust renders it.

- [ ] `genpdf` — high-level PDF generation with layouts, images, tables
- [ ] `printpdf` — full PDF spec control, vector graphics
- [ ] `typst` — LaTeX-quality typesetting, programmable documents
- [ ] `rust_xlsxwriter` — full Excel files with charts, formatting
- [ ] `csv` — high-performance CSV reading/writing
- [ ] `pulldown-cmark` + `maud` — Markdown→HTML rendering
- [ ] `resvg` + `usvg` — SVG vector rendering
- [ ] `plotters` — 2D/3D charts, data visualization
- [ ] `image` — image processing and format conversion
- [ ] Unified `generate_document()` call that orchestrates LLM + rendering
- [ ] In-panel PDF/document preview

---

## Phase D: Hardware-Adaptive Intelligence

### Part 15: Hardware Detection & Device Tier Classification [QUEUED]
> At first launch, DX profiles hardware and classifies into 5 tiers.

- [ ] Integrate `hardware-query` crate for full profiling: CPU, GPU, NPU, TPU, RAM, CUDA/ROCm/DirectML
- [ ] `system-analysis` crate for AI workload scoring and bottleneck detection
- [ ] Define 5 device tiers:
  - [ ] **Tier 1** (2–4GB RAM, No GPU): SmolLM2-360M, Piper tiny, Whisper Tiny — ~615MB total
  - [ ] **Tier 2** (4–8GB RAM, No GPU): Qwen3-0.6B, Piper medium, Whisper Tiny — ~1.0GB total
  - [ ] **Tier 3** (8–16GB RAM, iGPU): Qwen2.5-3B, Piper high + Kokoro, Whisper Base — ~4.8GB total
  - [ ] **Tier 4** (16–32GB RAM, dGPU 6–12GB): Mistral-7B, Chatterbox-Turbo, SDXL Turbo — ~20.5GB total
  - [ ] **Tier 5** (32GB+ RAM, 16GB+ VRAM): Qwen2.5-72B, Chatterbox + cloning, Flux.1 Dev, TripoSR — ~100GB total
- [ ] UI for tier display and manual override
- [ ] `llmfit` integration for interactive model-to-hardware fitting

### Part 16: Dynamic Model Swapping & Resource Management [QUEUED]
- [ ] `silicon-monitor` / `nvml-wrapper` for runtime GPU/CPU/memory monitoring
- [ ] RAM pressure detection → swap Q5_K_M → Q4_K_M, unload edit prediction temporarily
- [ ] Power state detection: plugged in → GPU acceleration + larger models; battery → smaller models
- [ ] Idle detection: daemon mode → load larger model for scheduled agent tasks
- [ ] Multi-feature active → share single model across grammar + prediction + voice
- [ ] Hardware upgrade detection → re-scan, offer model tier upgrade
- [ ] Disk space low → offer to remove unused model quantizations
- [ ] Model download manager with progress UI and resume support

---

## Phase E: System-Wide Writing Engine (Grammarly Replacement)

### Part 17: Three-Tier Grammar Pipeline [QUEUED]
> Replaces Grammarly. Local, <10ms, free, unlimited, privacy-preserving.

- [ ] **Tier 1 — Harper (`harper-core`):** <10ms, spelling, punctuation, grammar rules, passive voice, wordiness
- [ ] **Tier 2 — nlprule + Hunspell:** <50ms, 4000+ LanguageTool patterns offline, multi-language spell check via `zspell`
- [ ] **Tier 3 — Local LLM (tiered):** <500ms, tone mismatch, restructuring, context-aware suggestions
- [ ] Severity rendering:
  - [ ] 🔴 Red squiggly — definitive errors (misspellings, broken grammar)
  - [ ] 🟡 Yellow squiggly — suggestions (wordiness, passive voice)
  - [ ] 🔵 Blue squiggly — style (stronger synonyms, conciseness)
  - [ ] 💜 Purple squiggly — AI insight (restructuring, tone adjustment)
- [ ] Language detection via `whichlang` / `lingua-rs`
- [ ] Unicode word/sentence boundaries via `unicode-segmentation`
- [ ] `analiticcl` for fuzzy string matching spelling correction

### Part 18: OS Input Interception & System-Wide Text Fields [QUEUED]
> Extends edit prediction and grammar to EVERY app on the OS, not just Zed.

- [ ] **macOS:** CGEventTap + Input Method Kit (IMK) for input interception; AXUIElement for text field access; transparent NSWindow overlay (GPUI-rendered)
- [ ] **Windows:** Text Services Framework (TSF) + low-level hooks; UI Automation API; layered window (WS_EX_LAYERED), GPUI/DirectX overlay
- [ ] **Linux X11:** IBus + XInput2; AT-SPI2 accessibility; override-redirect window, GPUI/Vulkan
- [ ] **Linux Wayland:** Fcitx5 + input-method-v2; AT-SPI2; layer shell protocol, GPUI/Vulkan
- [ ] Cross-platform clipboard integration via `arboard`
- [ ] `get-selected-text` for selected text access
- [ ] `global-hotkey` for cross-platform hotkey bindings

### Part 19: Context-Aware Writing Profiles [QUEUED]
- [ ] Email client → High grammar, Professional tone, full-sentence prediction
- [ ] Slack/Discord → Low grammar, Casual tone, short-phrase prediction
- [ ] Code editor → Off for code / High for comments, Technical tone, Zeta-style code prediction
- [ ] Terminal → Grammar off, no text prediction
- [ ] Document editor → Maximum grammar, match document tone, paragraph continuations
- [ ] Social media → Medium grammar, Casual-Professional, short-form optimized
- [ ] Auto-detect app category and apply matching profile

---

## Phase F: Voice Conversation Engine (Wispr Flow + ElevenLabs Replacement)

### Part 20: Local Speech-to-Text (Whisper) [QUEUED]
> Replaces Wispr Flow. Free, unlimited, offline voice input.

- [ ] Integrate `whisper-rs` (GPU-accelerated: Metal/CUDA)
- [ ] Integrate `whisper-cpp-plus` for streaming Whisper + Silero VAD
- [ ] `cpal` for cross-platform audio I/O (CoreAudio, WASAPI, ALSA/PulseAudio)
- [ ] `rubato` for audio resampling to 16kHz
- [ ] `webrtc-vad` for standalone Voice Activity Detection
- [ ] Tiered Whisper models:
  - [ ] Tier 1–2: Whisper Tiny.en (~75MB)
  - [ ] Tier 3: Whisper Base.en (~142MB)
  - [ ] Tier 4: Whisper Small.en (~244MB)
  - [ ] Tier 5: Whisper Large-v3 (~1.5GB)
- [ ] Real-time streaming transcription with VAD

### Part 21: Local Text-to-Speech (Piper / Chatterbox) [QUEUED]
> Replaces ElevenLabs. Local TTS that wins blind tests on Tier 4+ hardware.

- [ ] Integrate `piper-rs` for Piper TTS models
- [ ] Integrate Chatterbox-Turbo (paralinguistic tags: [cough], [laugh], [sigh])
- [ ] Integrate Kokoro as zero-cost offline alternative
- [ ] `rodio` for audio playback
- [ ] `natural-tts` as multi-backend fallback
- [ ] Tiered TTS models:
  - [ ] Tier 1: Piper tiny (~15MB) — functional, clear, real-time on Pi
  - [ ] Tier 2: Piper medium (~65MB) — good quality, natural
  - [ ] Tier 3: Piper high + Kokoro (~100MB) — near-human, expressive
  - [ ] Tier 4: Chatterbox-Turbo (~500MB) — wins blind tests vs ElevenLabs
  - [ ] Tier 5: Chatterbox-Turbo + voice cloning (~1GB) — indistinguishable from human
- [ ] Audio caching (identical text + voice + settings → cached audio)

### Part 22: Cloud Voice APIs (Unified TTS Abstraction) [QUEUED]
> Same trait-based pattern as Universe A. Every TTS provider implements one interface.

- [ ] Define `TtsProvider` trait: `speak()`, `stream_speak()`, `list_voices()`, `clone_voice()`
- [ ] Cloud TTS adapters:
  - [ ] ElevenLabs (1200+ voices, 29 languages)
  - [ ] Fish Audio (#1 TTS-Arena, 80% cheaper than ElevenLabs)
  - [ ] Cartesia (40ms latency, voice cloning from 3 seconds)
  - [ ] PlayHT (1000+ voices, 142+ languages)
  - [ ] Deepgram Aura (production-grade)
  - [ ] Google Cloud TTS (380+ voices, 50+ languages)
  - [ ] Amazon Polly (5M chars/month free tier)
  - [ ] Azure Speech via `aspeak` (neural voices, SSML support)
  - [ ] OpenAI TTS
  - [ ] WellSaid Labs, Murf AI, Lovo AI
- [ ] Fallback chain: Local Piper → Cloud provider → Different cloud provider
- [ ] Per-character cost tracking (identical to LLM token tracking)
- [ ] Quality routing: short UI responses → fast local Piper; long narration → Chatterbox; premium → cloud

### Part 23: Voice Conversation Loop [QUEUED]
> User speaks → Whisper transcribes → LLM processes → TTS speaks back → User responds.

- [ ] Full-duplex conversation mode
- [ ] LLM course-correction pass on transcription before processing
- [ ] Streaming TTS playback (start speaking before full response generated)
- [ ] Conversation history context (multi-turn voice sessions)
- [ ] Interrupt detection (user speaks while DX is speaking → stop TTS, process new input)

---

## Phase G: DX Voice Experience UI (Flow Bar + Avatar)

### Part 24: Flow Bar (Persistent Bottom-Center Widget) [QUEUED]
> Small pill-shaped widget at screen bottom center, rendered by GPUI at GPU speed.

- [ ] **Idle state:** Small AI avatar face (48×48px), subtle blue glow → click to open AI panel
- [ ] **Listening state:** Expanded pill (320px), red pulsing dot, waveform
- [ ] **Transcribing state:** Spinning dots, "Processing..."
- [ ] **Post-processing state:** Purple glow, "Cleaning up..." (LLM course correction)
- [ ] **Result state:** Green border, cleaned text preview, Accept/Cancel → Tab to insert
- [ ] **Speaking state:** Avatar mouth animated, green glow
- [ ] Hotkey trigger system via `global-hotkey`
- [ ] Waveform/orb visualization via GPUI `canvas()`

### Part 25: AI Face Widget (Procedural GPU-Rendered Avatar) [QUEUED]
> Not an image — procedurally generated face via GPUI `canvas()`.

- [ ] Port SVG face from www-forge-token to GPUI procedural rendering
- [ ] **Eyes** track mouse cursor in real-time
- [ ] **Blink** every 3–7 seconds (randomized, natural)
- [ ] **Mouth** animates with speech amplitude when DX is talking
- [ ] **Expression** changes: curious (listening), focused (thinking), happy (done)
- [ ] **Glow ring** color shifts: blue (idle), red (recording), purple (thinking), green (speaking)
- [ ] **Breathing animation** — subtle scale pulse when idle
- [ ] Click to open floating AI panel (Part 6)
- [ ] Bottom-center always-visible placement
- [ ] System tray icon via `tray-icon`

---

## Phase H: Background Agent Daemon

### Part 26: Daemon Service Architecture [QUEUED]
> Runs as system service: systemd (Linux), launchd (macOS), Windows Service.

- [ ] `dx service install` — one command, runs forever
- [ ] Supervisor: auto-restart crashed agents with exponential backoff
- [ ] Cron scheduler: "Fetch news every 8am", "Summarize emails at 6pm"
- [ ] Channel router: Telegram, Discord, Slack, WhatsApp, Signal, Matrix, CLI
- [ ] Memory engine: local vector DB (HNSW) + keyword search (BM25), zero external dependencies
- [ ] Agent identity: AIEOS-compatible JSON persona, OpenClaw IDENTITY.md migration
- [ ] 24/7 background Ollama model
- [ ] Agent management UI in DX panel

### Part 27: VPS Deploy & Remote Agents [QUEUED]
- [ ] `dx deploy --host user@server` — SCP binary, install systemd service
- [ ] Remote agent health monitoring from DX desktop
- [ ] Secure channel between local DX ↔ remote daemon
- [ ] Cost tracking for remote compute

---

## Phase I: Computer Use Integration

### Part 28: OS Control (Mouse/Keyboard/Screen) [QUEUED]
- [ ] `rustautogui` — cross-platform mouse/keyboard, template matching (no OpenCV)
- [ ] `autopilot-rs` — cross-platform GUI automation
- [ ] `screenshots` — cross-platform screen capture
- [ ] `accesskit` — cross-platform accessibility toolkit
- [ ] Safety boundaries and allowlists (configurable per-app permissions)
- [ ] Vision model for understanding screenshots:
  - [ ] Local: LLaVA-1.5-7B Q4_K_M on Tier 5 hardware
  - [ ] Cloud: GPT-4V, Claude Vision, Gemini Vision
- [ ] Accessibility tree traversal for structured app understanding

---

## Phase J: Social & Collaboration

### Part 29: Social Sharing (GPUI) [QUEUED]
- [ ] Create `social_sharing` crate
- [ ] Port REST implementations from integrations/agent/src/channels/
- [ ] Create `SocialShareService` GPUI Global
- [ ] Connect Accounts settings page
- [ ] Wire share popover to actual send logic

---

## Phase K: Visual Polish & Finalization

### Part 30: Visual Polish Pass [QUEUED]
- [ ] Spacing refinements across all panels
- [ ] Typography hierarchy (headings, body, code, captions)
- [ ] New theme color tokens for DX-specific UI
- [ ] Animation transitions (150ms ease-out)
- [ ] Dark/light theme support for all new components
- [ ] Responsive layouts for different window sizes

### Part 31: Unified `generate()` Orchestration [QUEUED]
> "DX, generate a product landing page PDF with a hero image, 3D mockup, and background music"

- [ ] Orchestrator that decomposes multi-media requests:
  - [ ] LLM writes copy (Universe A)
  - [ ] Image provider generates hero image (Universe B — image)
  - [ ] 3D provider generates product mockup (Universe B — 3D)
  - [ ] Music provider generates background audio (Universe B — audio)
  - [ ] Rust rendering engine assembles PDF locally
  - [ ] TTS reads result summary back to user
- [ ] Parallel execution of independent media generation calls
- [ ] Unified cost summary across all providers used
- [ ] Progress dashboard showing all concurrent generation tasks
