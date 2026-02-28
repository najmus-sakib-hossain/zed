# Node.js Integration Components

This directory contains Node.js components integrated from OpenClaw that are easier to implement in Node.js than Rust due to ecosystem maturity.

## Why Node.js for These Components?

While DX is primarily Rust-based, certain features benefit from Node.js:

### 1. **TTS (Text-to-Speech)** - `tts/`
- Uses `node-edge-tts` for Microsoft Edge TTS API
- Mature Node.js library with streaming support
- Rust alternatives lack feature parity

### 2. **Media Understanding** - `media-understanding/`
- PDF parsing with `pdfjs-dist`
- Image processing with `sharp`
- Video frame extraction
- Audio transcription integration
- Rich ecosystem of media libraries

### 3. **Link Understanding** - `link-understanding/`
- Web scraping with `@mozilla/readability`
- HTML parsing with `linkedom`
- Content extraction and cleaning
- Better DOM manipulation in Node.js

### 4. **Cron/Scheduling** - `cron/`
- Uses `croner` for advanced cron expressions
- Isolated agent execution
- Job persistence and recovery
- Simpler than Rust cron implementations

### 5. **Auto-Reply System** - `auto-reply/`
- Complex message routing logic
- Command detection and parsing
- Heartbeat and typing indicators
- Model selection and directives
- Easier to iterate in TypeScript

### 6. **Hooks System** - `hooks/`
- Dynamic hook loading
- Gmail integration with OAuth
- Plugin hooks
- Frontmatter parsing
- Flexible event system

### 7. **Plugin SDK** - `plugin-sdk/`
- Dynamic plugin loading
- TypeScript-based plugin API
- Hot reloading support
- Easier for third-party developers

### 8. **Canvas Host** - `canvas-host/`
- A2UI rendering
- Express/Hono server
- WebSocket connections
- Better web framework support

### 9. **Providers** - `providers/`
- GitHub Copilot OAuth
- Qwen Portal OAuth
- Google API integrations
- OAuth flows easier in Node.js

### 10. **Memory System** - `memory/`
- Vector database integration
- Session management
- Context persistence
- Better async/await patterns

### 11. **Agents** - `agents/`
- Pi Agent integration
- Multi-agent orchestration
- Agent protocol implementations

### 12. **Sessions** - `sessions/`
- Session state management
- Conversation history
- Context tracking

### 13. **Routing** - `routing/`
- Message routing logic
- Channel multiplexing
- Priority handling

### 14. **Pairing** - `pairing/`
- Device pairing flows
- QR code generation
- Pairing store management

### 15. **Node Host** - `node-host/`
- Node.js process management
- IPC bridge to Rust
- Subprocess coordination

## Architecture

```
Rust CLI (crates/cli/src/)
    ↓ IPC/Bridge
Node.js Runtime (crates/cli/src/nodejs/)
    ↓ Channels
Messaging Platforms (crates/cli/nodejs/extensions/)
```

## Communication with Rust

The Node.js components communicate with Rust through:

1. **JSON-RPC over stdio** - Primary IPC mechanism
2. **Shared memory** - For high-throughput data
3. **WebSocket bridge** - For real-time events
4. **File-based queues** - For async job processing

## Dependencies

Key Node.js packages used:

- `@mozilla/readability` - Content extraction
- `sharp` - Image processing
- `pdfjs-dist` - PDF parsing
- `node-edge-tts` - Text-to-speech
- `croner` - Cron scheduling
- `@whiskeysockets/baileys` - WhatsApp protocol
- `grammy` - Telegram bot framework
- `@slack/bolt` - Slack integration
- `discord-api-types` - Discord types
- `playwright-core` - Browser automation (if needed)
- `sqlite-vec` - Vector search
- `express` / `hono` - Web servers

## UI Components

The `ui/` directory contains a Lit-based web UI for:
- Agent configuration
- Channel management
- Session monitoring
- Plugin management

## Packages

The `packages/` directory contains:
- `clawdbot/` - Bot implementation
- `moltbot/` - Alternative bot

## Development

```bash
# Install dependencies
cd crates/cli/nodejs
pnpm install

# Build TypeScript
pnpm build

# Run tests
pnpm test

# Watch mode
pnpm dev
```

## Integration Points

### From Rust to Node.js
- Spawn Node.js process from Rust
- Send commands via stdin (JSON-RPC)
- Receive responses via stdout
- Handle errors via stderr

### From Node.js to Rust
- Call Rust functions via IPC
- Access Rust gateway
- Use Rust serializer
- Leverage Rust performance

## Future Considerations

As Rust ecosystem matures, we may migrate:
- PDF parsing (when `pdf-extract` improves)
- Image processing (when `image` crate adds features)
- OAuth flows (when `oauth2` crate simplifies)

For now, Node.js provides the best developer experience for these components.
