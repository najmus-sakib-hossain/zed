Please list all of the APIs that can generate image by creating an account in their platform and getting an API key to programmatically create videos. As for DX Rust software we have to be able to create image but as we don't have any money we will rely on other third-party providers so that the users can log in to those platforms and get an API key and generate image in our DX Rust software. And if you can then also list awesome GitHub repos listing all of the free video providers that give free credit to users so that the users can use those free credits to create image on our DX Rust platform.

**Here is a complete, up-to-date (Feb 2026) list of image-generation APIs** that let users create a free account, generate an API key, and call the service programmatically. Perfect for your DX Rust app: users paste their own key (BYOK), your Rust backend (`reqwest` + `serde_json`) sends the prompt, and the provider bills **only the user** — zero cost to you.

All are standard REST/JSON endpoints (or OpenAI-compatible), so integration is trivial in Rust. Many also have official Python/JS SDKs you can mirror with `reqwest`.

### 1. Core Text-to-Image / Image-to-Image APIs
| Provider | Key Models / Features | Free Tier / Credits on Signup | API Docs & Key Generation | Rust Notes |
|----------|-----------------------|-------------------------------|---------------------------|------------|
| **OpenAI** | GPT-Image-1.5, DALL·E 3 (HD, variations, editing) | New-user credits (~$5–18 sometimes); then pay-per-image (~$0.04–0.08) | platform.openai.com → API keys | Official OpenAI Rust crate or raw POST to `https://api.openai.com/v1/images/generations` |
| **Google Gemini / Imagen 3** | Gemini 3 Pro Image, Imagen 3 (best text-in-image, style control) | Extremely generous free tier (high daily image quota in most regions, no card needed) | aistudio.google.com/apikey or ai.google.dev | OpenAI-compatible endpoint; free tier is one of the best |
| **Stability AI** | SD 3.5 Large/Turbo, Stable Image Ultra, ControlNet, upscaling | Signup credits + recurring free on free plan | platform.stability.ai/account/keys | Simple POST `/v2beta/stable-image/generate` |
| **Leonardo.ai** | Phoenix, Leonardo Vision XL (cinematic, product, anime styles) | Free plan: daily tokens + **$5–10 API credit** on signup | leonardo.ai → API section in dashboard | Excellent creative controls; async + webhooks |
| **xAI Grok** | grok-imagine-image / grok-2-image-1212 (Flux-based, uncensored, fast) | Pay-per-image (~$0.07) but very cheap; new users often get test credits | console.x.ai → API keys | OpenAI-compatible; `base_url = "https://api.x.ai/v1"` |
| **Ideogram** | Ideogram 2.0 (best text rendering in images) | Free daily generations + API credits on signup | ideogram.ai/api or developer portal | High-quality text-in-image |
| **Bria.ai** | Bria 2.0 (commercial-safe, product photography) | **1,000 free API calls** on signup | bria.ai/api | Enterprise-grade, safe for commercial use |
| **DeepAI** | Text-to-Image, Image-to-Image, inpainting | **100 free credits** + cheap pay-per | api.deepai.org | Very simple REST, great for starters |

### 2. Unified / Aggregator Platforms (one key → 50+ models)
Best choice for your DX Rust software — let users pick “Flux”, “SD 3.5”, “Imagen”, “Leonardo”, etc. from a dropdown.

| Aggregator | Models Available | Free Tier | API Link |
|------------|------------------|-----------|----------|
| **Replicate** | Flux 2 Pro/Dev/Schnell, SD 3.5, Ideogram, Imagen, Grok, 1000+ open models | “Try for Free” collection (many models run free without card); limited runs/month | replicate.com/account/api-tokens |
| **Fal.ai** | Flux 2, Grok Imagine, SD, Leonardo, etc. (fastest inference) | $10–20 free credits on signup | fal.ai/keys |
| **Hugging Face Inference API** | All open Flux/SD models + custom | Free tier with rate limits; paid for high volume | hf.co/settings/tokens |
| **Together.ai / Fireworks.ai / SiliconFlow** | Flux 2 Dev/Pro (open weights), SD variants | Free credits/trial for new devs | together.ai → API keys |
| **WaveSpeedAI** | Flux, Seedream, Hunyuan Image, etc. | Free testing credits | wavespeed.ai |
| **AIMLAPI.com** | Flux 1.1/2, GPT-Image Mini, 200+ models | Explicit “Free Image Generation API” tier | aimlapi.com |

### How Users Generate Images in Your DX Rust Software
1. User signs up on any of the above (all free).
2. Goes to dashboard → “API Keys” → copies key.
3. Pastes into your Rust app (store encrypted per-user).
4. Your UI: dropdown of providers/models + prompt, aspect ratio, style, etc.
5. Rust backend example (works for 95 % of them):
```rust
let client = reqwest::Client::new();
let res = client.post("https://api.openai.com/v1/images/generations")  // or replicate/fal/etc.
    .header("Authorization", format!("Bearer {}", user_api_key))
    .json(&serde_json::json!({
        "model": "gpt-image-1.5",
        "prompt": "a cyberpunk cat in neon Tokyo",
        "n": 1,
        "size": "1024x1024"
    }))
    .send().await?;
```
Most return a direct image URL or base64. Poll async jobs on Leonardo/Replicate if needed.

### Awesome GitHub Repos Listing Free-Credit Image Providers
These are actively maintained and specifically highlight **free tiers / signup credits** (perfect for your users who want to generate images for free initially):

- **https://github.com/foss42/awesome-generative-ai-apis** → Best developer-focused list; every entry marks “free tier / credits” and direct API links (OpenAI, Stability, Leonardo, Replicate, Google, etc.).
- **https://github.com/steven2358/awesome-generative-ai** → Huge curated list with dedicated **Image** section; notes free daily credits on Leonardo, Google, Replicate, DeepAI, etc.
- **https://github.com/filipecalegario/awesome-generative-ai** → Another excellent one with image-synthesis subsection and free-tier tags.
- **https://github.com/mahseema/awesome-ai-tools** → “Generative AI Images” category with free-credit providers.
- **https://github.com/BeingCoders/1000-AI-collection-tools** → 1000+ tools, many marked “free AI image generator + API”.
- **https://github.com/ai-collection/ai-collection** → Landscape view with direct links to free-credit services.

**Pro tip for your Rust app**: Default to **Replicate** or **Fal.ai** (one key unlocks Flux + everything). Add a “Discover Free Credits” tab that links to the GitHub lists above so users can quickly sign up for Leonardo’s daily tokens + Google’s huge free quota + Replicate’s Try-for-Free models.

This covers **every major production-ready image-generation API** available right now. The landscape moves fast (new Flux versions and unified gateways appear monthly), so star those GitHub repos — they stay updated. Your users can rotate keys across providers to maximize free credits before going paid.

Need Rust code examples for a specific provider (e.g., Leonardo async polling or Replicate webhook)? Just say the word and I’ll drop ready-to-copy snippets!
