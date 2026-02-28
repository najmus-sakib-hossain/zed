# Getting Started Docs

For first-time setup and quick orientation.

## Start Path

1. Main overview and quick start: [../../README.md](../../README.md)
2. One-click setup and dual bootstrap mode: [../one-click-bootstrap.md](../one-click-bootstrap.md)
3. Update or uninstall on macOS: [macos-update-uninstall.md](macos-update-uninstall.md)
4. Set up on Android (Termux/ADB): [../android-setup.md](../android-setup.md)
5. Find commands by tasks: [../commands-reference.md](../commands-reference.md)

## Choose Your Path

| Scenario | Command |
|----------|---------|
| I have an API key, want fastest setup | `dx onboard --api-key sk-... --provider openrouter` |
| I want guided prompts | `dx onboard --interactive` |
| Config exists, just fix channels | `dx onboard --channels-only` |
| Config exists, I intentionally want full overwrite | `dx onboard --force` |
| Using subscription auth | See [Subscription Auth](../../README.md#subscription-auth-openai-codex--claude-code) |

## Onboarding and Validation

- Quick onboarding: `dx onboard --api-key "sk-..." --provider openrouter`
- Interactive onboarding: `dx onboard --interactive`
- Existing config protection: reruns require explicit confirmation (or `--force` in non-interactive flows)
- Ollama cloud models (`:cloud`) require a remote `api_url` and API key (for example `api_url = "https://ollama.com"`).
- Validate environment: `dx status` + `dx doctor`

## Next

- Runtime operations: [../operations/README.md](../operations/README.md)
- Reference catalogs: [../reference/README.md](../reference/README.md)
- macOS lifecycle tasks: [macos-update-uninstall.md](macos-update-uninstall.md)
- Android setup path: [../android-setup.md](../android-setup.md)
