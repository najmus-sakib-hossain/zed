# DX Agent Gateway API Guide

## Base endpoints
- `GET /health` — service health status.
- `GET /api/v1/status` — gateway status and config snapshot.
- `GET /api/v1/sessions` — session list.
- `GET /api/v1/clients` — connected websocket clients.
- `GET /api/v1/config` — sanitized config view.
- `GET /api/v1/dashboard` — dashboard data model.
- `POST /api/v1/webchat` — send message from web UI.
- `GET /api/v1/logs` — tail of gateway log lines.
- `GET /api/v1/skills` — bundled/workspace skill inventory.

## WebSocket
- `GET /ws` — main gateway websocket.
- Message envelope uses `dx-agent-protocol` (`GatewayMessage`, `GatewayRequest`, `GatewayResponse`, `GatewayEvent`).

## Security model
- JWT/API-key auth supported by gateway auth manager.
- Rate limiting enforced by per-IP sliding window.
- Audit logging and secret storage available in gateway modules.

## Notes
- Enable file logging in gateway config to populate `/api/v1/logs`.
- Web UI routes are served from `crates/agent/gateway/src/web.rs`.
