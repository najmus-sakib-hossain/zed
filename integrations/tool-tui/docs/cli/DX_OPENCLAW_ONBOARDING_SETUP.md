# DX CLI Onboarding Setup

## Purpose

This document defines the DX CLI onboarding flow for `crates/cli-onboard` using DX-native onboarding concepts and the onboarding prompt design-system.

> Security notice: DX is an active development project and can make mistakes. Treat outputs and automatic changes with caution — do not run in production without review.

## Entry Modes

- **Prompt test suite:** `cargo run -- <1-36>`
- **DX onboarding flow:** `cargo run -- --dx-onboard` (also the crate default)
- **Shared-account onboarding:**
  - `cargo run -- --dx-onboard --shared-account <ref>`
  - `cargo run -- --dx-onboard --account-email <email>`

## Runtime Environment Branching

The onboarding flow detects and confirms the runtime target:

1. `real_os` - local workstation (desktop app + extension path)
2. `vps` - cloud/server VM (remote gateway path)
3. `container` - Docker/Podman style environment
4. `restricted` - CI or restricted shell, non-interactive-friendly path

### Detection Inputs

- `CI=true|1` => `restricted`
- `/.dockerenv` or `/proc/1/cgroup` containing `docker|containerd` => `container`
- cloud env vars (`VERCEL`, `RAILWAY_ENVIRONMENT`, `FLY_APP_NAME`, `HEROKU_APP_NAME`) => `vps`
- fallback => `real_os`

## Account Detection and Auth Branches

### Shared Account from CLI args

If `--shared-account` is present, onboarding uses the shared account directly.

### Interactive Account Branches

If no shared account arg is passed:

- ask whether user already has a DX account
- if yes, choose login method:
  - `email_password`
  - `device_code`
  - `api_key`
- if no, choose account creation mode:
  - `quick_signup`
  - `team_invite`
  - `local_sandbox`

## Provider Options

Current provider list surfaced by the onboarding flow:

- OpenAI
- Anthropic
- Google
- xAI
- OpenRouter
- vLLM
- LiteLLM
- Venice
- Hugging Face
- Together
- Qwen
- Z.AI
- Qianfan
- Moonshot / Kimi
- GitHub Copilot
- Vercel AI Gateway
- Cloudflare AI Gateway
- OpenCode Zen
- Custom (OpenAI-compatible endpoint)

## Messaging Channel Options

Current channel list surfaced by the onboarding flow:

- Telegram
- WhatsApp
- Discord
- Slack
- Google Chat
- Signal
- iMessage
- IRC
- Mattermost
- Matrix
- Microsoft Teams
- WeChat
- Messenger
- LINE

## Select/MultiSelect Hint UX

`select` and `multiselect` render active-item contextual hints as a **muted right-side hint chip**.

- only the active row displays the chip
- chip uses dim text + muted background styling
- row keeps existing border/symbol rules from `ONBOARDING_PROMPT_DESIGN_SYSTEM.md`

## Current Scope

The onboarding flow is intentionally non-production for flow validation and UX review. It records structured output to `response.json` for evaluation before wiring account/session/channel backends.
