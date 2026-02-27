**Here is a comprehensive, up-to-date (as of Feb 2026) list of video-generation APIs that support programmatic access via account signup + personal API key.** These are ideal for your DX Rust software: users sign up on the third-party platform, get their own API key (free or paid), paste it into your app, and your Rust backend (using `reqwest` + JSON serde) calls the provider's REST/HTTP endpoints directly on their behalf. No cost to you — it's pure BYOK (Bring Your Own Key).

All require a free account signup on their site/dashboard → generate API key/token (usually in settings or developer portal). Most are REST JSON APIs (easy in Rust); some offer SDKs (Python/JS) you can port or just use raw HTTP.

### 1. Core Generative Text/Image/Video-to-Video APIs (the "Sora-like" ones)
These create original AI videos from prompts.

| Provider / API | Key Models/Features | Free Tier / Credits on Signup | API Docs / Signup | Notes for Rust Integration |
|---------------|---------------------|-------------------------------|-------------------|----------------------------|
| **Runway ML** | Gen-4 / Gen-4.5 Turbo (text-to-video, image-to-video, motion brush) | Limited free credits (often 125+ one-time + monthly on free plan) | runwayml.com/api or dev.runwayml.com | Official SDK example; simple POST tasks |
| **Luma AI Dream Machine / Ray** | Dream Machine (text/image-to-video, 3D-aware, cinematic) | Free plan: ~8 draft videos/month; paid upgrades | docs.lumalabs.ai/docs/api (keys at lumalabs.ai/dream-machine/api/keys) | Async polling by request ID; very straightforward |
| **Kling AI (Kuaishou/ByteDance)** | Kling 2.0 (photorealistic, long clips, styles, virtual try-on) | Generous free daily/monthly credits on freemium | Official API or via WaveSpeedAI gateway | High quality; API-first design |
| **Pika Labs** | Pikaformance / Pika 2.x (creative motion, lip-sync, effects) | Limited free tier | Via fal.ai (search "pika" on fal.ai) or third-party wrappers | Fast; Fal.ai unifies many |
| **Hailuo AI** | Hailuo text-to-video | Generous free tier (one of the best free options) | Open API, developer portal | Cheap + free credits |
| **Minimax** | Multimodal video | Limited free tier | Developer registration portal | Good for multi-input |
| **OpenAI Sora 2 / Sora Pro** | Sora 2 (narrative, physics, long-form) | No broad free tier (paid credits); some Pro access via ChatGPT+ | platform.openai.com (or gateways like Evolink.ai) | Now generally available |
| **Google Veo 3 / Flow** | Veo 3 via Vertex AI or Google AI Studio/Gemini API | 100+ free credits/month (more in some regions); $300 new-user Cloud credits | ai.google.dev or cloud.google.com/vertex-ai | Excellent free monthly quota |
| **Stability AI** | Stable Video Diffusion (image-to-video, SVD) + others | Pay-per-use; occasional trials/credits | platform.stability.ai | Stable, production-ready |

### 2. Unified / Aggregator Platforms (one key → many models)
Great for your software — users pick model/provider inside your UI.

- **Fal.ai** — Hosts Pika, Kling, Luma, Hailuo, Minimax, Veo, etc. Super-fast inference. Signup → API key. Pay-per-use + trial credits. (fal.ai)
- **Replicate.com** — Hosts 50k+ models incl. Runway Gen-4.5, PixVerse, Grok Imagine Video, Stable Video, open-source (Wan, AnimateDiff, etc.). Signup with GitHub/Google → API token. **Try-for-Free collection** (limited free runs without card on many video models). (replicate.com)
- **WaveSpeedAI** — Unified gateway for Kling 2.0, Seedance, etc. Single key for multiple. Free credits for testing. (wavespeed.ai)
- **Runware.ai** — One API for text-to-video + image-to-video across models. Low-cost. (runware.ai)
- **Hugging Face Inference API / Endpoints** — Open-source video models (SVD, etc.). Free tier with rate limits; paid for dedicated. hf.co (API token in settings)
- **ModelsLab** — Dedicated video generation API. (modelslab.com)

### 3. Avatar / Talking-Head Video APIs (script/text → realistic person video)
These are full video generators too.

- **HeyGen** — 500+ avatars, cloning, 175+ languages translation. **3 free video credits/month** recurring. API from $99/mo but free tier exists. docs.heygen.com
- **Synthesia** — 230+ avatars, templates. API on higher plans. synthesia.io/docs
- **D-ID** — Talking heads + real-time streaming API. d-id.com
- **Colossyan** — Script-to-training video. Free plan + API on Business. colossyan.com
- **Deepbrain AI**, **Tavus**, etc. (listed in awesome repos below)

### How Users Use These in Your DX Rust Software
1. User creates free account on e.g. runwayml.com / lumalabs.ai / replicate.com / heygen.com.
2. Goes to dashboard → "API Keys" → copies key.
3. Pastes into your Rust app (store encrypted per-user).
4. Your app: dropdown "Provider: Runway / Luma / Kling / Fal / Replicate..." + model selector + prompt form.
5. Rust backend: `reqwest` POST to their endpoint with `Authorization: Bearer <user_key>` + JSON payload (prompt, duration, ratio, etc.).
6. Poll status or use webhooks for completion → return video URL/file to user.

Most endpoints are simple (create task → poll ID). Rate limits and billing are on the user's account.

### Awesome GitHub Repos Listing Providers (Especially Free-Credit Ones)
These are actively maintained curated lists. Many explicitly note free tiers, credits, APIs, and which give signup/daily/monthly free usage (perfect for users who don't want to pay immediately):

- **https://github.com/foss42/awesome-generative-ai-apis** — Best focused list of GenAI APIs (including video section: Deepbrain, HeyGen, Synthesia, Colossyan, etc.). Marks free/no-key where applicable.
- **https://github.com/steven2358/awesome-generative-ai** — Huge list with dedicated Video Generation section (Runway, Kling, Luma, Pika, HeyGen, Synthesia, Hailuo, etc.).
- **https://github.com/fengxiaolonger/awesome-ai-video-tools** — 150+ video-specific tools, has **APIs & Developer Tools** section (12+ entries) + free-tier notes (e.g., HeyVid AI aggregator with free + credits).
- **https://github.com/jayeshmepani/Media-AI** — Ultimate media gen list (image/video/audio); covers 110+ tools with free-tier mentions and API notes.
- **https://github.com/cporter202/API-mega-list** — Massive (10k+ APIs); has dedicated "videos-apis-979" folder with hundreds of video-related APIs.
- **https://github.com/topics/ai-video-generation** — GitHub topic page (browse repos for more tools/wrappers).
- Others worth starring: **mahseema/awesome-ai-tools**, **eudk/awesome-ai-tools** (video sections), **AlonzoLeeeooo/awesome-video-generation** (more research/papers but links to implementations).

Many of these repos highlight providers with **free credits** like:
- HeyGen (3 monthly videos)
- Kling/Hailuo (daily/generous free)
- Replicate (limited free runs on video models)
- Google Veo (monthly credits)
- Luma/Runway (free tiers)
- Aggregators (Fal/Replicate/WaveSpeed) that inherit free credits from underlying models

**Pro tip for your Rust app**: Start with Replicate or Fal.ai as defaults (one key unlocks dozens of models). Add a "Free Tier Providers" tab linking to these GitHub lists so users can discover more signup bonuses easily.

This covers essentially **all major production-ready video-generation APIs** available in Feb 2026. Things evolve fast (new models drop monthly), so the GitHub awesome lists are the living source. Your users can rotate keys across providers for more free credits or better quality. Let me know if you need Rust code snippets for a specific provider's endpoint!
