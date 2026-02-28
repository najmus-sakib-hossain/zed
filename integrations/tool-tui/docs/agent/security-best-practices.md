# DX Agent Security Best Practices

This document is the operational baseline for secure DX agent deployments.

## 1) Secrets and credentials

- Store provider tokens only in encrypted storage (`dx-agent-gateway` secrets manager).
- Never commit `.env`, auth state folders, or token files to Git.
- Rotate credentials on schedule and immediately after incidents.
- Use least-privilege scopes for all provider tokens.

## 2) Channel isolation

- Enable per-channel allowlists/denylists for all production channels.
- Require DM pairing approval for high-risk channels.
- Disable channels that are not actively used.
- Keep separate credentials for dev/staging/prod environments.

## 3) Sandbox and execution safety

- Run untrusted tool execution in `dx-agent-sandbox` with resource limits.
- Restrict network egress for sandboxed sessions when possible.
- Set CPU/memory/PID ceilings to prevent denial-of-service from workloads.
- Prefer capability-based actions over unrestricted shell execution.

## 4) Transport and gateway hardening

- Require token auth for gateway and web endpoints.
- Enforce rate limits on API and WebSocket routes.
- Terminate TLS at reverse proxy or directly in gateway deployment.
- Restrict management endpoints to trusted networks/VPN.

## 5) Logging and auditability

- Keep audit logging enabled for security-relevant actions.
- Redact secrets from logs and diagnostics before storage/export.
- Configure retention policy for operational and security logs.
- Forward logs to SIEM/centralized monitoring for alerting.

## 6) Dependency and build hygiene

- Prefer Rust-native crates over Node workers when feasible.
- Keep dependency updates regular and verify advisories.
- Run `cargo clippy` and targeted tests before release candidates.
- Use debug builds for development; keep release artifacts reproducible.

## 7) Incident response basics

- Have a runbook for credential compromise and channel abuse.
- Add one-click credential revocation in ops scripts.
- Preserve forensic logs and event timelines before cleanup.
- Post-incident: patch root cause, rotate keys, and backfill tests.
