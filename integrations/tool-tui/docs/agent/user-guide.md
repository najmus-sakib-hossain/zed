# DX Agent User Guide

## Start gateway
- Run gateway and verify health endpoint.
- Open control UI at gateway root URL.

## Onboarding
- Run `dx onboard` to configure profile, model provider, channels, security preferences, and health checks.

## Channels
- Native Rust channels: Telegram, Discord, Slack, WhatsApp Cloud API, Matrix, Teams.
- Optional channels can be enabled by feature flags and configuration.

## Web UI
- Dashboard shows uptime, sessions, and connections.
- WebChat can send test messages into gateway events.
- Log Viewer displays recent gateway log file lines.
- Skill Manager displays bundled and workspace skills.

## Security
- Use authentication and API keys in production.
- Enable rate limits and audit logging.
- Store credentials via gateway secret store module.
