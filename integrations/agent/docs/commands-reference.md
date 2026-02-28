# DX Commands Reference

This reference is derived from the current CLI surface (`dx --help`).

Last verified: **February 28, 2026**.

## Top-Level Commands

| Command | Purpose |
|---|---|
| `onboard` | Initialize workspace/config quickly or interactively |
| `agent` | Run interactive chat or single-message mode |
| `gateway` | Start webhook and WhatsApp HTTP gateway |
| `daemon` | Start supervised runtime (gateway + channels + optional heartbeat/scheduler) |
| `service` | Manage user-level OS service lifecycle |
| `doctor` | Run diagnostics and freshness checks |
| `status` | Print current configuration and system summary |
| `estop` | Engage/resume emergency stop levels and inspect estop state |
| `cron` | Manage scheduled tasks |
| `models` | Refresh provider model catalogs |
| `providers` | List provider IDs, aliases, and active provider |
| `channel` | Manage channels and channel health checks |
| `integrations` | Inspect integration details |
| `skills` | List/install/remove skills |
| `migrate` | Import from external runtimes (currently OpenClaw) |
| `config` | Export machine-readable config schema |
| `completions` | Generate shell completion scripts to stdout |
| `hardware` | Discover and introspect USB hardware |
| `peripheral` | Configure and flash peripherals |

## Command Groups

### `onboard`

- `dx onboard`
- `dx onboard --interactive`
- `dx onboard --channels-only`
- `dx onboard --force`
- `dx onboard --api-key <KEY> --provider <ID> --memory <sqlite|lucid|markdown|none>`
- `dx onboard --api-key <KEY> --provider <ID> --model <MODEL_ID> --memory <sqlite|lucid|markdown|none>`
- `dx onboard --api-key <KEY> --provider <ID> --model <MODEL_ID> --memory <sqlite|lucid|markdown|none> --force`

`onboard` safety behavior:

- If `config.toml` already exists and you run `--interactive`, onboarding now offers two modes:
  - Full onboarding (overwrite `config.toml`)
  - Provider-only update (update provider/model/API key while preserving existing channels, tunnel, memory, hooks, and other settings)
- In non-interactive environments, existing `config.toml` causes a safe refusal unless `--force` is passed.
- Use `dx onboard --channels-only` when you only need to rotate channel tokens/allowlists.

### `agent`

- `dx agent`
- `dx agent -m "Hello"`
- `dx agent --provider <ID> --model <MODEL> --temperature <0.0-2.0>`
- `dx agent --peripheral <board:path>`

Tip:

- In interactive chat, you can ask for route changes in natural language (for example “conversation uses kimi, coding uses gpt-5.3-codex”); the assistant can persist this via tool `model_routing_config`.
- In interactive chat, you can also ask to:
  - switch web search provider/fallbacks (`web_search_config`)
  - inspect or update domain access policy (`web_access_config`)

### `gateway` / `daemon`

- `dx gateway [--host <HOST>] [--port <PORT>] [--new-pairing]`
- `dx daemon [--host <HOST>] [--port <PORT>]`

`--new-pairing` clears all stored paired tokens and forces generation of a fresh pairing code on gateway startup.

### `estop`

- `dx estop` (engage `kill-all`)
- `dx estop --level network-kill`
- `dx estop --level domain-block --domain "*.chase.com" [--domain "*.paypal.com"]`
- `dx estop --level tool-freeze --tool shell [--tool browser]`
- `dx estop status`
- `dx estop resume`
- `dx estop resume --network`
- `dx estop resume --domain "*.chase.com"`
- `dx estop resume --tool shell`
- `dx estop resume --otp <123456>`

Notes:

- `estop` commands require `[security.estop].enabled = true`.
- When `[security.estop].require_otp_to_resume = true`, `resume` requires OTP validation.
- OTP prompt appears automatically if `--otp` is omitted.

### `service`

- `dx service install`
- `dx service start`
- `dx service stop`
- `dx service restart`
- `dx service status`
- `dx service uninstall`

### `cron`

- `dx cron list`
- `dx cron add <expr> [--tz <IANA_TZ>] <command>`
- `dx cron add-at <rfc3339_timestamp> <command>`
- `dx cron add-every <every_ms> <command>`
- `dx cron once <delay> <command>`
- `dx cron remove <id>`
- `dx cron pause <id>`
- `dx cron resume <id>`

Notes:

- Mutating schedule/cron actions require `cron.enabled = true`.
- Shell command payloads for schedule creation (`create` / `add` / `once`) are validated by security command policy before job persistence.

### `models`

- `dx models refresh`
- `dx models refresh --provider <ID>`
- `dx models refresh --force`

`models refresh` currently supports live catalog refresh for provider IDs: `openrouter`, `openai`, `anthropic`, `groq`, `mistral`, `deepseek`, `xai`, `together-ai`, `gemini`, `ollama`, `llamacpp`, `sglang`, `vllm`, `astrai`, `venice`, `fireworks`, `cohere`, `moonshot`, `glm`, `zai`, `qwen`, `volcengine` (`doubao`/`ark` aliases), `siliconflow`, and `nvidia`.

### `doctor`

- `dx doctor`
- `dx doctor models [--provider <ID>] [--use-cache]`
- `dx doctor traces [--limit <N>] [--event <TYPE>] [--contains <TEXT>]`
- `dx doctor traces --id <TRACE_ID>`

Provider connectivity matrix CI/local helper:

- `python3 scripts/ci/provider_connectivity_matrix.py --binary target/release-fast/dx --contract .github/connectivity/probe-contract.json`

`doctor traces` reads runtime tool/model diagnostics from `observability.runtime_trace_path`.

### `channel`

- `dx channel list`
- `dx channel start`
- `dx channel doctor`
- `dx channel bind-telegram <IDENTITY>`
- `dx channel add <type> <json>`
- `dx channel remove <name>`

Runtime in-chat commands while channel server is running:

- Telegram/Discord sender-session routing:
  - `/models`
  - `/models <provider>`
  - `/model`
  - `/model <model-id>`
  - `/new`
- Supervised tool approvals (all non-CLI channels):
  - `/approve-request <tool-name>` (create pending approval request)
  - `/approve-confirm <request-id>` (confirm pending request; same sender + same chat/channel only)
  - `/approve-pending` (list pending requests in current sender+chat/channel scope)
  - `/approve <tool-name>` (direct one-step grant + persist to `autonomy.auto_approve`, compatibility path)
  - `/unapprove <tool-name>` (revoke + remove from `autonomy.auto_approve`)
  - `/approvals` (show runtime + persisted approval state)
  - Natural-language approval behavior is controlled by `[autonomy].non_cli_natural_language_approval_mode`:
    - `direct` (default): `授权工具 shell` / `approve tool shell` immediately grants
    - `request_confirm`: natural-language approval creates pending request, then confirm with request ID
    - `disabled`: natural-language approval commands are ignored (slash commands only)
  - Optional per-channel override: `[autonomy].non_cli_natural_language_approval_mode_by_channel`

Approval safety behavior:

- Runtime approval commands are parsed and executed **before** LLM inference in the channel loop.
- Pending requests are sender+chat/channel scoped and expire automatically.
- Confirmation requires the same sender in the same chat/channel that created the request.
- Once approved and persisted, the tool remains approved across restarts until revoked.
- Optional policy gate: `[autonomy].non_cli_approval_approvers` can restrict who may execute approval-management commands.

Startup behavior for multiple channels:
- `dx channel start` starts all configured channels in one process.
- If one channel fails initialization, other channels continue to start.
- If all configured channels fail initialization, startup exits with an error.

Channel runtime also watches `config.toml` and hot-applies updates to:
- `default_provider`
- `default_model`
- `default_temperature`
- `api_key` / `api_url` (for the default provider)
- `reliability.*` provider retry settings

`add/remove` currently route you back to managed setup/manual config paths (not full declarative mutators yet).

### `integrations`

- `dx integrations info <name>`

### `skills`

- `dx skills list`
- `dx skills audit <source_or_name>`
- `dx skills install <source>`
- `dx skills remove <name>`

`<source>` accepts:

| Format | Example | Notes |
|---|---|---|
| **ClawhHub profile URL** | `https://clawhub.ai/steipete/summarize` | Auto-detected by domain; downloads zip from ClawhHub API |
| **ClawhHub short prefix** | `clawhub:summarize` | Short form; slug is the skill name on ClawhHub |
| **Direct zip URL** | `zip:https://example.com/skill.zip` | Any HTTPS URL returning a zip archive |
| **Local zip file** | `/path/to/skill.zip` | Zip file already downloaded to local disk |
| **Registry packages** | `namespace/name` or `namespace/name@version` | Fetched from the configured registry (default: ZeroMarket) |
| **Git remotes** | `https://github.com/…`, `git@host:owner/repo.git` | Cloned with `git clone --depth 1` |
| **Local filesystem paths** | `./my-skill` or `/abs/path/skill` | Directory copied and audited |

**ClawhHub install examples:**

```bash
# Install by profile URL (slug extracted from last path segment)
dx skill install https://clawhub.ai/steipete/summarize

# Install using short prefix
dx skill install clawhub:summarize

# Install from a zip already downloaded locally
dx skill install ~/Downloads/summarize-1.0.0.zip
```

If the ClawhHub API returns 429 (rate limit) or requires authentication, set `clawhub_token` in `[skills]` config (see [config reference](config-reference.md#skills)).

**Zip-based install behavior:**
- If the zip contains `_meta.json` (OpenClaw convention), name/version/author are read from it.
- A minimal `SKILL.toml` is written automatically if neither `SKILL.toml` nor `SKILL.md` is present in the zip.

Registry packages are installed to `~/.dx/workspace/skills/<name>/`.

`skills install` always runs a built-in static security audit before the skill is accepted. The audit blocks:
- symlinks inside the skill package
- script-like files (`.sh`, `.bash`, `.zsh`, `.ps1`, `.bat`, `.cmd`)
- high-risk command snippets (for example pipe-to-shell payloads)
- markdown links that escape the skill root, point to remote markdown, or target script files

> **Note:** The security audit applies to directory-based installs (local paths, git remotes). Zip-based installs (ClawhHub, direct zip URLs, local zip files) perform path-traversal safety checks during extraction but do not run the full static audit — review zip contents manually for untrusted sources.

Use `skills audit` to manually validate a candidate skill directory (or an installed skill by name) before sharing it.

Skill manifests (`SKILL.toml`) support `prompts` and `[[tools]]`; both are injected into the agent system prompt at runtime, so the model can follow skill instructions without manually reading skill files.

### `migrate`

- `dx migrate openclaw [--source <path>] [--dry-run]`

### `config`

- `dx config schema`

`config schema` prints a JSON Schema (draft 2020-12) for the full `config.toml` contract to stdout.

### `completions`

- `dx completions bash`
- `dx completions fish`
- `dx completions zsh`
- `dx completions powershell`
- `dx completions elvish`

`completions` is stdout-only by design so scripts can be sourced directly without log/warning contamination.

### `hardware`

- `dx hardware discover`
- `dx hardware introspect <path>`
- `dx hardware info [--chip <chip_name>]`

### `peripheral`

- `dx peripheral list`
- `dx peripheral add <board> <path>`
- `dx peripheral flash [--port <serial_port>]`
- `dx peripheral setup-uno-q [--host <ip_or_host>]`
- `dx peripheral flash-nucleo`

## Validation Tip

To verify docs against your current binary quickly:

```bash
dx --help
dx <command> --help
```
