Please list all of the APIs that can generate can talk to user live and have a long live audio and video sessions with users by creating an account in their platform and getting an API key to programmatically create videos. As for DX Rust software we have to be able to create can talk to user live and have a long live audio and video sessions with users but as we don't have any money we will rely on other third-party providers so that the users can log in to those platforms and get an API key and generate can talk to user live and have a long live audio and video sessions with users in our DX Rust software. And if you can then also list awesome GitHub repos listing all of the free video providers that give free credit to users so that the users can use those free credits to create can talk to user live and have a long live audio and video sessions with users on our DX Rust platform.

**Here is a complete, up-to-date (Feb 2026) list of APIs** that enable **real-time live conversational AI sessions** with full **audio + video** (talking avatars that see/hear/respond to the user in real time, supporting long ongoing interactions like video calls or chat sessions).

These are **BYOK-only** (users create free account → generate their own API key → paste into your DX Rust software). Your Rust backend uses `reqwest` + `tokio-tungstenite` (or `axum` WebSocket) to start a session, stream user audio/video input, and receive the AI avatar's live video + audio output. Zero cost to you — billing is entirely on the user's account.

All support **long sessions** (minutes to hours of continuous back-and-forth) via streaming/WebRTC/WebSocket.

### 1. Core Real-Time Conversational Avatar APIs (Live Audio + Video)
| Provider | Key Live Features (Audio + Video) | Free Tier / Credits on Signup | API Key Generation | Rust Integration Notes |
|----------|-----------------------------------|-------------------------------|--------------------|------------------------|
| **D-ID** (Top for real-time streaming) | Real-time low-latency streaming avatars, 100 FPS lip-sync, face-to-face conversational agents, integrates with any LLM/STT for dynamic back-and-forth | 14-day trial (limited minutes) + occasional credits | studio.d-id.com → Account Settings → Generate Key | HTTP POST to start + WebSocket/HTTP streaming for live video; simple polling or event-driven |
| **Tavus** (Best for full face-to-face live) | Conversational Video Interface (CVI): real-time see/hear/respond, photorealistic digital twins, natural turn-taking, low latency, Phoenix-3 rendering | Free developer testing tier / trial credits | tavus.io → Developer Portal (signup → API keys) | WebRTC-ready API; start live session endpoint + stream audio/video |
| **HeyGen** | Streaming Avatars / LiveAvatar: real-time lip-sync + gestures, interactive video agents (works great with Twilio/OpenAI Realtime) | Free plan (limited videos) + API pay-as-you-go (buy credits as needed) | app.heygen.com → API section in dashboard | Streaming endpoints; combine with voice for full live sessions |
| **DeepBrain AI** | Real-time conversation avatars, live kiosk-style deployment, multi-language lip-sync | Trial credits on signup | aistudios.com → API dashboard | Real-time API calls + streaming support |
| **Beyond Presence** | Real-time Audio-to-Video API, expressive avatars, direct LiveKit integration for agents | Free testing credits | bey.dev → API keys | Simple real-time POST + WebSocket; perfect for Rust + LiveKit |
| **Hedra** | Expressive real-time talking heads from audio, LiveKit-native streaming, character-driven conversations | Free tier + credits | hedra.com → Developer API | WebSocket streaming; low-latency avatar output |
| **Simli** | Real-time avatar streaming (audio + video), sub-second latency, long-session capable | $10 free credit + ~50 min/month on free plan | simli.com → Dashboard → API Keys | WebSocket for live sessions; extremely developer-friendly |
| **Anam AI** | Sub-second photorealistic real-time avatars, text/voice input, customizable | Free dev tier | anam.ai → API portal | High-performance streaming API |

### 2. Unified / Voice-First + Avatar Combos (One or Two Keys)
- **LiveKit Agents** + any above avatar (LiveKit has generous free tier; pair with D-ID/Tavus/Hedra via their API keys)
- **OpenAI Realtime API** (voice core, excellent free credits for new users) + D-ID/HeyGen/Tavus for video layer
- **Agora Conversational AI** (supports multiple avatar providers via their API keys)

### How Users Generate Live Sessions in Your DX Rust Software
1. User signs up free on e.g. d-id.com / tavus.io / heygen.com / simli.com.
2. Goes to dashboard → API Keys → copies key (sometimes needs to buy minimal credits for heavy use).
3. Pastes into your Rust app (store per-user, encrypted).
4. Your UI: “Start Live Video Chat with AI Avatar” + provider/model picker + system prompt.
5. Rust backend (example skeleton):
```rust
// Using reqwest + tokio-tungstenite for WebSocket streaming
let client = reqwest::Client::new();
let session = client.post("https://api.d-id.com/talks/stream") // or tavus/simli equivalent
    .header("Authorization", format!("Bearer {}", user_key))
    .json(&json!({ "source_url": avatar, "script": "...", "stream": true }))
    .send().await?;

// Then WebSocket for live bidirectional audio/video
let (mut ws, _) = connect_async("wss://api.simli.com/...").await?;
// Stream user mic → provider → receive avatar video frames + audio
```
Most return a streaming WebSocket or WebRTC token — forward to frontend (Video.js / MediaSource / WebRTC). Sessions can run for 30–60+ minutes continuously.

### Awesome GitHub Repos Listing Free-Credit / Real-Time Providers
These are actively maintained and specifically highlight **real-time conversational avatars**, free tiers, trials, and API notes (perfect for your “Discover Free Credits” tab):

- **https://github.com/fengxiaolonger/awesome-ai-video-tools** → Dedicated **AI Avatars & Talking Heads** section with real-time flags (D-ID, DeepBrain, Hedra, etc.) + free-tier mentions (Vidnoz daily credits, etc.).
- **https://github.com/topics/real-time-avatars** → GitHub topic with repos showing integrations (Anam, Hedra, LiveKit examples).
- **https://github.com/yzfly/awesome-voice-agents** → Best for voice + avatar combos; lists real-time providers with free credits (Simli, OpenAI Realtime + avatars).
- **https://github.com/livekit/agents** → Official LiveKit examples with avatar integrations; generous free tier notes.
- **https://github.com/steven2358/awesome-generative-ai** → Video/Avatar subsection with real-time tags.
- **https://github.com/e2b-dev/awesome-ai-agents** → Includes real-time avatar agents with API credits.

**Pro tip for your Rust platform**: Default to **D-ID** or **Simli** (best free credits + simplest streaming). Add a tab “Free Real-Time Providers” linking these GitHub repos so users can grab extra trial minutes across Tavus, Hedra, etc. Users can rotate keys to maximize free usage before paying.

This covers **every major production-ready real-time live audio + video conversational avatar API** as of Feb 2026. The space moves extremely fast (new low-latency models drop monthly), so the GitHub lists above are the living source.

Need ready-to-copy Rust code for D-ID streaming session, Tavus CVI, or Simli WebSocket? Or a full example with LiveKit? Just tell me which provider and I’ll drop it!
