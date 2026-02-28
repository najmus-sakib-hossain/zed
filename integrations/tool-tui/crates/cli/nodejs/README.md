# DX Messaging Channels (Node.js Integration)

This directory contains Node.js-based messaging channel integrations for the DX CLI, ported from Dx.

## Overview

The DX CLI is primarily built in Rust, but messaging platform integrations benefit from Node.js due to:
- Rich ecosystem of messaging SDKs (Discord.js, Slack SDK, Telegram Bot API, etc.)
- Better WebSocket/real-time connection support
- Mature OAuth and authentication libraries
- Active maintenance of platform-specific packages

## Architecture

```
crates/cli/
├── src/
│   ├── channels/          # Rust channel coordination
│   ├── gateway/           # Rust gateway (existing)
│   └── nodejs/            # Node.js bridge (this directory)
│       ├── channels/      # Channel core logic
│       └── extensions/    # Platform-specific integrations
├── nodejs/                # Node.js runtime
│   ├── extensions/        # Messaging platform extensions
│   ├── channels/          # Channel implementation
│   ├── package.json       # Node.js dependencies
│   └── tsconfig.json      # TypeScript configuration
├── scripts/               # Build and utility scripts
├── skills/                # AI agent skills
├── swabble/               # Swift utilities (macOS)
└── pi/                    # Platform integrations
```

## Supported Platforms

### Messaging Apps
- **Discord** - Discord bot integration
- **Slack** - Slack workspace integration
- **Telegram** - Telegram bot API
- **WhatsApp** - WhatsApp Business API
- **Signal** - Signal messenger
- **Matrix** - Matrix protocol
- **Mattermost** - Self-hosted team chat
- **MS Teams** - Microsoft Teams
- **Google Chat** - Google Workspace chat
- **Line** - Line messenger
- **Feishu** - Lark/Feishu (ByteDance)
- **Twitch** - Twitch chat
- **Nextcloud Talk** - Nextcloud chat
- **Nostr** - Decentralized protocol
- **Tlon** - Urbit messaging
- **Zalo** - Vietnamese messenger
- **BlueBubbles** - iMessage bridge
- **iMessage** - Native macOS iMessage

## Integration with Rust

The Node.js extensions communicate with the Rust CLI through:

1. **IPC (Inter-Process Communication)** - JSON-RPC over stdin/stdout
2. **Shared Memory** - For high-performance data transfer
3. **WebSocket Bridge** - For real-time event streaming

### Message Flow

```
Messaging Platform → Node.js Extension → Channel Registry → Rust Gateway → DX CLI
```

## Development

### Prerequisites

```bash
# Install Node.js dependencies
cd crates/cli/nodejs
npm install

# Build TypeScript
npm run build

# Run tests
npm test
```

### Adding a New Channel

1. Create extension directory: `nodejs/extensions/your-platform/`
2. Implement channel interface in TypeScript
3. Register in `channels/registry.ts`
4. Add Rust bridge in `src/channels/`
5. Update documentation

### Configuration

Each extension has a `dx.plugin.json` (renamed from `dx.plugin.json`) with:

```json
{
  "name": "platform-name",
  "version": "1.0.0",
  "type": "channel",
  "runtime": "nodejs",
  "entry": "index.ts"
}
```

## Skills Integration

The `skills/` directory contains AI agent capabilities that can be invoked through messaging channels:

- **1password** - Password management
- **github** - GitHub integration
- **notion** - Notion workspace
- **obsidian** - Obsidian notes
- **slack** - Slack operations
- **discord** - Discord operations
- **weather** - Weather information
- **voice-call** - Voice call handling
- And 40+ more...

## Scripts

Build and utility scripts in `scripts/`:

- `build-and-run-mac.sh` - macOS build script
- `protocol-gen.ts` - Generate protocol bindings
- `sync-plugin-versions.ts` - Sync plugin versions
- `test-*.ts` - Various test utilities

## Platform-Specific Notes

### macOS (Swabble)

The `swabble/` directory contains Swift code for native macOS integrations:
- iMessage native support
- System notifications
- Keychain access
- AppleScript automation

### Pi Extensions

The `pi/` directory contains platform integration extensions:
- Git hooks
- Custom prompts
- Extension configurations

## Migration from Dx

This code was ported from Dx with the following changes:

1. **Renamed** all `dx` references to `dx`
2. **Integrated** with existing DX Rust gateway
3. **Removed** duplicate functionality (daemon, gateway already exist in Rust)
4. **Kept** only messaging channel implementations
5. **Updated** configuration format to match DX conventions

## License

See LICENSE files in parent directories.
