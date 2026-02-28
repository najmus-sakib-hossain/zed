# WhatsApp Benchmark Guide

This guide compares DX WhatsApp delivery paths:

- Cloud API mode (pure Rust `reqwest` path)
- Baileys mode (Node worker fallback for WhatsApp Web)

## Prerequisites

- Build target: debug mode only
- Workspace root: `F:/Dx`
- For Cloud API mode:
  - `DX_WHATSAPP_CLOUD_ACCESS_TOKEN`
  - `DX_WHATSAPP_CLOUD_PHONE_NUMBER_ID`
- For Baileys mode:
  - Node.js installed
  - `@whiskeysockets/baileys` dependencies available for the runner
  - Optional `DX_WHATSAPP_RUNNER`, `DX_NODE_BIN`, `DX_WHATSAPP_AUTH_DIR`

If live credentials/runtime are unavailable, the benchmark can automatically fall back to synthetic mode to keep CI/dev benchmarking unblocked.

## Run benchmark

PowerShell example:

```powershell
$env:DX_WHATSAPP_BENCH_MODE = "both"
$env:DX_WHATSAPP_BENCH_TO = "15551234567"
$env:DX_WHATSAPP_BENCH_ITERATIONS = "20"
$env:DX_WHATSAPP_BENCH_TEXT = "DX benchmark"
$env:DX_WHATSAPP_BENCH_SYNTHETIC_FALLBACK = "1"
$env:DX_WHATSAPP_BENCH_OUTPUT = "docs/agent/whatsapp-benchmark-results.json"

cargo run -p dx-agent-channels --features whatsapp --example whatsapp_benchmark
```

## Output

The benchmark prints:

- `mode`: `cloud` or `baileys`
- `kind`: `live` or `synthetic`
- `iter`: message send attempts
- `ok` / `fail`: delivery status counts
- `total_ms`: wall-clock runtime for the mode
- `avg_ms`: average per-message latency
- `p95_ms`: 95th percentile per-message latency

The runner also supports `resolveTarget` RPC with OpenClaw-style allowlist/mode semantics (`explicit`, `implicit`, `heartbeat`) for parity checks in fallback mode.

If `DX_WHATSAPP_BENCH_OUTPUT` is set, results are saved as JSON.

## OpenClaw vs DX comparator (normalize path)

PowerShell example:

```powershell
$env:DX_WA_NORMALIZE_BENCH_ITERS = "200000"
node crates/cli/scripts/benchmark-openclaw-vs-dx-whatsapp-normalize.mjs
```

Generated artifacts:

- `docs/agent/openclaw-vs-dx-normalize-benchmark.json`
- `docs/agent/openclaw-vs-dx-normalize-benchmark.md`

This comparator benchmarks WhatsApp target normalization behavior with parity checks between DX and an OpenClaw-derived reference implementation.

## Interpretation

- Prefer Cloud API when it satisfies product requirements and latency is competitive.
- Keep Baileys fallback for unsupported workflows (e.g., WhatsApp Web-only behaviors).
- Use synthetic mode as baseline only; use live mode for production decision-making.
- Re-run after channel/runtime optimizations and compare trend lines over time.
