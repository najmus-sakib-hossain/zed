# WhatsApp Integration Evaluation (Rust-first)

## Summary
- Preferred current path: keep `crates/agent/channels/src/whatsapp.rs` (Cloud API via Rust `reqwest`).
- Fallback path: Baileys worker only if protocol coverage is insufficient.
- Decision: use Rust by default for security/performance; use Node worker for non-Cloud WhatsApp Web-specific flows.

## Rust Option
- Implementation: WhatsApp Cloud API in Rust.
- Advantages:
  - Memory-safe runtime and simpler deployment.
  - Native integration with DX auth, rate limits, audit logs.
  - Lower operational complexity versus long-running JS worker sessions.
- Limitation:
  - Cloud API feature envelope may not cover every WhatsApp Web automation use case.

## Node.js Fallback (Baileys)
- Source: `integrations/openclaw/extensions/whatsapp/src/` plus DX bridge runner at `crates/cli/src/bridge/whatsapp/runner.mjs`.
- Use when needed for protocol parity not exposed in Cloud API.
- Keep fallback isolated behind IPC worker boundary.

## What web research showed
- `whatsapp_handler` exists for Rust Cloud API and is usable for typed webhook/message flows, but has a smaller ecosystem footprint.
- Baileys is actively maintained and widely used, but has breaking changes in recent versions and is explicitly a reverse-engineered WhatsApp Web stack.
- Rust-native channels such as `teloxide` and `serenity` show strong maturity/usage; this supports Rust-first for channels where parity is available.
- Recommendation remains Rust-first, fallback to Baileys for gaps.

## Recommended DX strategy
1. Keep Rust Cloud API as default production path.
2. Add optional Node worker mode under explicit config flag.
3. Benchmark both modes in channel benchmarks before final default selection.
4. Keep business logic in Rust; keep JS worker thin.
5. Reuse OpenClaw normalization/routing behavior where possible, then map to DX channel abstractions.
