# DX Agent Troubleshooting Guide

This guide covers common runtime issues for gateway, channels, and worker fallbacks.

## Gateway startup failures

### Symptom
Gateway does not start or exits immediately.

### Checks
- Verify config path and syntax.
- Check port conflicts on gateway/websocket ports.
- Run targeted check: `cargo check -p dx-agent-gateway`.

### Fix
- Correct invalid config values.
- Change port or stop conflicting process.
- Re-run with debug logging and inspect startup errors.

## Channel connect failures

### Symptom
Channel remains disconnected.

### Checks
- Confirm channel credentials are present and valid.
- Validate feature flags used at build/run time.
- Inspect logs for provider auth or permission errors.

### Fix
- Rotate and re-apply tokens.
- Ensure required scopes/intents are configured.
- Reconnect channel after credential update.

## WhatsApp Cloud API errors

### Symptom
`send` returns API auth or validation errors.

### Checks
- `DX_WHATSAPP_CLOUD_ACCESS_TOKEN` validity.
- `DX_WHATSAPP_CLOUD_PHONE_NUMBER_ID` matches business account setup.
- Destination number format is correct.

### Fix
- Refresh token and verify app permissions.
- Re-check WhatsApp Business account linkage.
- Retry with a known-good recipient in test environment.

## WhatsApp Baileys fallback issues

### Symptom
Baileys mode fails to connect or times out.

### Checks
- Node.js installed and reachable (`DX_NODE_BIN` if needed).
- Runner path valid (`DX_WHATSAPP_RUNNER`).
- Auth directory writable (`DX_WHATSAPP_AUTH_DIR`) and QR pairing completed.

### Fix
- Reinstall/update bridge dependencies.
- Delete stale auth state and re-pair.
- Restart channel and verify `state=connected` events.

## Worker process instability

### Symptom
Workers restart repeatedly.

### Checks
- Review worker logs and restart policy settings.
- Confirm IPC endpoint availability (named pipe/socket).
- Check sandbox resource ceilings.

### Fix
- Increase resource limits for valid workloads.
- Resolve IPC conflicts and stale endpoints.
- Tighten health checks to avoid false positives.

## Performance regressions

### Symptom
Latency or throughput regresses after changes.

### Checks
- Run focused benchmarks only for affected crates.
- Compare Cloud API vs Baileys using the WhatsApp benchmark example.
- Capture p95 latency and failure rates.

### Fix
- Roll back suspected changes and bisect.
- Prefer Rust-native path when parity exists.
- Keep benchmark outputs in versioned docs for trend tracking.
