# DX CLI Requirements - OpenClaw Feature Parity

> **Vision**: Achieve 100% feature parity with OpenClaw TypeScript implementation while maintaining Rust's safety, performance, and zero-cost abstractions.
>
> **Current State**: ~12.5% feature coverage (50/400 features)
> **Target State**: 100% feature coverage (400/400 features)
> **Timeline**: 100 weeks (24 months with 1 developer, 6-12 months with team of 4+)

---

## Executive Summary

This document outlines requirements for integrating all 350+ missing OpenClaw features into the DX Rust CLI. The implementation follows a systematic 5-phase approach prioritizing core infrastructure, channel implementations, advanced systems, security features, and platform integration.

**Key Performance Requirements:**
- **Performance**: 10-80x faster than TypeScript/Node.js
- **Memory**: <50MB baseline, <128MB under load
- **Startup**: <50ms CLI startup time
- **Reliability**: 99.9% uptime for daemon/gateway
- **Security**: WASM sandbox, capability-based permissions, audit logging

---

## Phase 1: Core Infrastructure (Weeks 1-15)

### 1.1 Gateway RPC Methods Expansion

**Requirement**: Expand from 25 to 70+ RPC methods across 9 categories

**Categories:**
1. **Models & Agents** (6 methods)
   - `models.list` - List available LLM models with capabilities
   - `agents.list` - List configured agents with status
   - `agents.files.list/get/set` - Agent file management
   - `agent.identity.get` - Get agent identity/profile

2. **Chat & Sessions** (6 additional methods)
   - `sessions.preview` - Preview session without full load
   - `sessions.patch` - Update session metadata
   - `sessions.compact` - Compress session history
   - `agent.wait` - Block until agent completes
   - `chat.history` - Get paginated chat history
   - `chat.abort` - Cancel ongoing generation

3. **Exec Approvals** (6 methods)
   - `exec.approvals.get/set` - Global approval rules
   - `exec.approvals.node.get/set` - Per-node approval rules
   - `exec.approval.request` - Request command approval
   - `exec.approval.resolve` - Approve/reject pending command

4. **Node Management** (15 methods)
   - `node.pair.*` - request, list, approve, reject, verify
   - `node.*` - rename, list, describe, invoke, invoke.result, event
   - `device.pair.*` - list, approve, reject
   - `device.token.*` - rotate, revoke

5. **Cron** (7 methods)
   - `cron.*` - list, status, add, update, remove, run, runs

6. **Skills** (4 methods)
   - `skills.*` - status, bins, install, update

7. **TTS** (6 methods)
   - `tts.*` - status, providers, enable, disable, convert, setProvider

8. **Voice Wake** (2 methods)
   - `voicewake.get/set`

9. **Wizard** (4 methods)
   - `wizard.*` - start, next, cancel, status

**Acceptance Criteria:**
- ✅ All 51 new RPC methods implemented and tested
- ✅ Methods follow consistent error handling pattern
- ✅ JSON-RPC 2.0 compliance
- ✅ Comprehensive unit tests (>90% coverage)
- ✅ Integration tests with gateway

### 1.2 Session Management System

**Requirement**: Full session lifecycle management with persistence, compaction, and repair

**Functional Requirements:**
- Session CRUD operations (create, read, update, delete)
- File-based persistence (`~/.dx/sessions/{agent_id}/{session_key}.json`)
- Session compaction for large histories (>1MB)
- Session repair for corrupted data
- Export in multiple formats (JSON, Markdown, HTML)
- Automatic backups on modification
- Session metadata (created, updated, token count, message count)

**Data Structure:**
```rust
pub struct Session {
    key: String,
    agent_id: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    messages: Vec<Message>,
    metadata: HashMap<String, Value>,
    state: SessionState,  // active, paused, archived
    context_length: usize,
    token_count: usize,
}
```

**Performance Requirements:**
- Session load: <100ms for normal sessions
- Session save: <50ms with atomic writes
- Compaction: <5s for 1000+ message sessions
- Search: <200ms for 100+ sessions

**Acceptance Criteria:**
- ✅ All CRUD operations working
- ✅ Atomic writes (temp file + rename)
- ✅ Compression for large sessions
- ✅ Repair functionality for corrupted sessions
- ✅ Export to JSON/Markdown/HTML
- ✅ Unit tests (>90% coverage)

### 1.3 Configuration System Overhaul

**Requirement**: Migrate from basic TOML to advanced YAML config with validation, hot-reload, and encryption

**Functional Requirements:**
- YAML configuration format
- Environment variable substitution (`${VAR:-default}`)
- File includes (`includes: ["./other.yaml"]`)
- Deep merge for included configs
- JSON Schema generation and validation
- Hot-reload with file watcher (100ms debounce)
- Config migrations (legacy TOML → YAML)
- Encrypted secrets (AES-256-GCM)
- Per-agent/channel/provider overrides
- Automatic backups

**Config Structure:**
```yaml
version: "2.0"
includes:
  - "./channels.yaml"
  - "${DX_CONFIG_DIR}/custom.yaml"

gateway:
  host: "${DX_HOST:-0.0.0.0}"
  port: ${DX_PORT:-31337}
  mdns_enabled: true

agents:
  - id: "default"
    name: "DX Assistant"
    model: "claude-sonnet-4"
    workspace: "~/workspace"

providers:
  anthropic:
    api_key: "${ANTHROPIC_API_KEY}"

memory:
  backend: "lancedb"
  path: "~/.dx/memory"
```

**Performance Requirements:**
- Config load: <50ms
- Hot-reload: <100ms from file change
- Validation: <20ms
- Schema generation: <100ms

**Acceptance Criteria:**
- ✅ YAML parsing with environment variables
- ✅ File includes with deep merge
- ✅ Hot-reload without restart
- ✅ JSON Schema validation
- ✅ AES-256 secret encryption
- ✅ Migration from legacy formats
- ✅ Unit + integration tests

### 1.4 Memory & Persistence Layer

**Requirement**: Vector search, embeddings, and multiple storage backends

**Storage Backends:**
1. **SQLite** - Full-text search (FTS5)
2. **LanceDB** - Vector similarity search
3. **File** - Simple file-based backend

**Embedding Providers:**
1. **OpenAI** - text-embedding-3-small
2. **Voyage** - via Anthropic
3. **Local** - ONNX models (all-MiniLM-L6-v2)

**Functional Requirements:**
- Document storage with metadata
- Full-text search
- Vector embedding generation
- Semantic search with similarity scores
- Automatic indexing (background task)
- Document pruning (by age, relevance)
- Compression for large documents
- Deduplication
- Cache layer

**Data Structure:**
```rust
pub struct Document {
    id: String,
    content: String,
    embedding: Option<Vec<f32>>,
    metadata: HashMap<String, Value>,
    timestamp: DateTime<Utc>,
    source: DocumentSource,  // session, file, web
}
```

**Performance Requirements:**
- Document store: <10ms
- Text search: <50ms for 10k docs
- Vector search: <100ms for 100k vectors
- Embedding generation: <500ms per batch

**Acceptance Criteria:**
- ✅ All 3 backends implemented
- ✅ All 3 embedding providers working
- ✅ Background indexing task
- ✅ Pruning with configurable policies
- ✅ Benchmarks demonstrate performance
- ✅ Unit + integration tests

### 1.5 Plugin System Core

**Requirement**: Dynamic plugin loading with WASM sandbox and native extensions

**Plugin Types:**
1. **WASM** - Sandboxed, cross-platform
2. **Native** - High-performance, requires signature
3. **Script** - Node.js/Python/Shell scripts

**Functional Requirements:**
- Plugin manifest (YAML/TOML)
- Dynamic loading/unloading
- Hot-reload support
- Plugin registry
- Capability-based permissions
- Resource limits (memory, CPU)
- Version management
- Plugin discovery
- Sandboxed execution (WASM)
- Host interface (network, fs, kv, logging)

**Plugin Manifest:**
```yaml
name: "my-plugin"
version: "0.1.0"
author: "Author Name"
description: "Plugin description"
dx_version: ">=0.1.0"

hooks:
  - type: "channel"
    id: "custom-channel"

permissions:
  - "network"
  - "filesystem:read"

entry:
  wasm: "./plugin.wasm"
```

**Performance Requirements:**
- Plugin load: <100ms for WASM
- Plugin call: <5ms overhead
- Memory limit: 256MB default
- CPU timeout: 30s per call

**Acceptance Criteria:**
- ✅ WASM runtime (wasmtime) working
- ✅ Native loader (libloading) working
- ✅ Capability enforcement
- ✅ Resource limits enforced
- ✅ Plugin manifest validation
- ✅ Hot-reload working
- ✅ Example plugins created
- ✅ Unit + integration tests

---

## Phase 2: Channel Implementations (Weeks 16-40)

### 2.1 Core 5 Channels Enhancement

**Requirement**: Complete WhatsApp, Telegram, Discord, Slack, Signal with full features

**Current State:** Basic bridge protocol exists
**Target State:** Full feature parity with OpenClaw

**Features per Channel (15 total):**
1. Text messaging (✅ done)
2. Media (images, videos, audio, documents)
3. Reactions/Emojis
4. Threads/Replies
5. Forwarded messages
6. Quoted messages
7. Location sharing
8. Contact sharing
9. Typing indicators
10. Read receipts
11. Delivery status
12. Message editing
13. Message deletion
14. Pinned messages
15. Archived conversations

**Bridge Protocol Enhancement:**
```rust
pub enum BridgeMessage {
    Text { to: String, text: String },  // Existing
    Media { to: String, media_type: MediaType, url: String, caption: Option<String> },
    Reaction { message_id: String, emoji: String },
    Reply { to: String, text: String, reply_to: String },
    Location { to: String, lat: f64, lng: f64, name: Option<String> },
    Edit { message_id: String, new_text: String },
    Delete { message_id: String },
    // ... more types
}
```

**Node.js Bridge Enhancements:**
- WhatsApp: Use @whiskeysockets/baileys with media support
- Telegram: Use grammy with full Bot API features
- Discord: Use discord.js with slash commands
- Slack: Use @slack/bolt with Block Kit
- Signal: Use signal-cli wrapper

**Performance Requirements:**
- Message send: <500ms
- Message receive: Real-time via WebSocket
- Media upload: Support up to 100MB files
- Bridge startup: <5s per channel

**Acceptance Criteria:**
- ✅ All 5 channels have 15/15 features
- ✅ Media handling working
- ✅ Node.js bridge stable
- ✅ Error recovery implemented
- ✅ Integration tests for each channel

### 2.2 Business Platform Channels

**Requirement**: Add Microsoft Teams, Feishu/Lark, Matrix, Mattermost

**Channels (4 total):**
1. **Microsoft Teams**
   - Teams Bot Framework integration
   - Adaptive Cards support
   - Channel/Team routing
   - File attachments
   - @mentions

2. **Feishu/Lark**
   - Event subscription
   - Interactive cards
   - File upload/download
   - Bot management

3. **Matrix**
   - Homeserver connection
   - E2E encryption (optional)
   - Media uploads
   - Read receipts

4. **Mattermost**
   - WebSocket connection
   - Channel messaging
   - Direct messages
   - Slash commands

**Performance Requirements:**
- Message latency: <1s
- WebSocket reconnect: <5s
- File upload: Support up to 50MB

**Acceptance Criteria:**
- ✅ All 4 channels implemented
- ✅ Full feature set per channel
- ✅ Documentation for setup
- ✅ Integration tests

### 2.3 Specialized Channels

**Requirement**: Add LINE, Twitch, Zalo, Nostr, Urbit, Open Prose, Nextcloud Talk, BlueBubbles

**Channels (8 total):**
1. LINE - Messaging API
2. Twitch - IRC + API
3. Zalo - Official Account API
4. Nostr - Protocol (NIP-01)
5. Urbit (Tlon) - HTTP API
6. Open Prose - XMPP
7. Nextcloud Talk - API
8. BlueBubbles - iMessage relay

**Acceptance Criteria:**
- ✅ All 8 channels implemented
- ✅ Basic messaging working
- ✅ Documentation for each
- ✅ Integration tests

---

## Phase 3: Advanced Systems (Weeks 41-70)

### 3.1 Skills System

**Requirement**: Skill discovery, installation, management, allowlists

**Skill Components:**
- Skill manifest (YAML)
- System prompts
- Tool allowlists
- Required binaries
- Installation scripts

**Skill Manifest:**
```yaml
name: "code-review"
version: "1.0.0"
description: "Code review assistant"

prompts:
  system: "You are a code review expert..."
  user_template: "Review this code:\n{code}"

tools:
  - "read_file"
  - "grep"
  - "git_diff"

bins:
  - name: "eslint"
    version: ">=9.0.0"
```

**Functional Requirements:**
- Skill discovery (local + remote registry)
- Installation with dependency resolution
- Version management
- Skill bundling (merge multiple skills)
- Validation (syntax, security)
- Hot-reload

**Acceptance Criteria:**
- ✅ Skill manager implemented
- ✅ Discovery working
- ✅ Installation working
- ✅ Validation working
- ✅ 5+ example skills created
- ✅ Unit + integration tests

### 3.2 Hooks & Automation System

**Requirement**: Event-driven automation with bundled and custom hooks

**Hook Types:**
1. **Trigger**: cron, event, webhook, file watcher
2. **Action**: message, exec, rpc, agent

**Hook Definition:**
```yaml
name: "gmail-to-agent"
description: "Forward Gmail to agent"

trigger:
  type: "gmail"
  config:
    labels: ["AI-INBOX"]
    poll_interval: "5m"

actions:
  - type: "agent"
    agent_id: "default"
    prompt: "Summarize this email: {{body}}"
```

**Bundled Hooks:**
- Gmail watcher (IMAP)
- GitHub webhooks
- Cron scheduler
- File watcher
- HTTP webhook receiver

**Acceptance Criteria:**
- ✅ Hook manager implemented
- ✅ All bundled hooks working
- ✅ Custom hooks supported
- ✅ Templating working
- ✅ Integration tests

### 3.3 Browser Automation

**Requirement**: Full Playwright integration for browser control

**Functional Requirements:**
- Browser launch (Chrome, Firefox)
- Headless/headed modes
- Page navigation
- Element interaction (click, type)
- Screenshot capture
- PDF generation
- Network monitoring
- JavaScript execution
- Cookie/session management

**Performance Requirements:**
- Browser launch: <3s
- Page load wait: Configurable timeout
- Screenshot: <1s
- PDF generation: <2s

**Acceptance Criteria:**
- ✅ Playwright integrated
- ✅ All actions implemented
- ✅ Config-driven automation
- ✅ Session management
- ✅ Integration tests

### 3.4 Terminal UI (TUI)

**Requirement**: Interactive chat interface with session management

**Components:**
- Chat view (main)
- Session list (sidebar)
- Command palette (Ctrl+P)
- Diff viewer
- File browser

**Features:**
- Split panes
- Syntax highlighting (syntect)
- Vim-style keybindings
- Mouse support
- Inline image display
- Theme support

**Performance Requirements:**
- Render: 60fps
- Input latency: <16ms
- Syntax highlight: <100ms

**Acceptance Criteria:**
- ✅ All components implemented
- ✅ Keybindings working
- ✅ Performance targets met
- ✅ Theme integration
- ✅ Unit tests

### 3.5 Onboarding Wizard

**Requirement**: Interactive setup for new users

**Wizard Steps:**
1. Prerequisites check (Node.js, Rust)
2. Gateway configuration
3. Channel setup with auth
4. Agent configuration
5. Plugin installation
6. Daemon setup (optional)
7. Validation

**Acceptance Criteria:**
- ✅ Wizard flow implemented
- ✅ All steps working
- ✅ Validation per step
- ✅ Rollback on failure
- ✅ Integration tests

### 3.6 Diagnostics & Troubleshooting

**Requirement**: Comprehensive health checks and auto-fix

**Diagnostic Checks:**
- System (platform, resources)
- Config validation
- Gateway health
- Channel connectivity
- Sandbox checks
- Auth diagnostics
- Network diagnostics
- Performance metrics

**Auto-Fix Capabilities:**
- Config syntax errors
- Missing dependencies
- Permission issues
- Port conflicts

**Acceptance Criteria:**
- ✅ All checks implemented
- ✅ Auto-fix working
- ✅ Reporting clear
- ✅ Integration tests

### 3.7 Daemon Management

**Requirement**: Native system service integration

**Platforms:**
1. **macOS** - launchd
2. **Linux** - systemd
3. **Windows** - Task Scheduler

**Functional Requirements:**
- Service installation/uninstallation
- Start/stop/restart
- Status checking
- Log viewing
- Auto-start on boot
- Service upgrade
- Rollback capability

**Acceptance Criteria:**
- ✅ All 3 platforms implemented
- ✅ Service management working
- ✅ Logs accessible
- ✅ Integration tests per platform

---

## Phase 4: Security & Advanced Features (Weeks 71-85)

### 4.1 Exec Approval System

**Requirement**: Command approval workflow for security

**Approval Flow:**
1. Command intercept
2. Rule evaluation
3. User approval (if needed)
4. Execution or denial
5. Audit logging

**Approval Rules:**
```rust
pub struct ApprovalRule {
    pattern: Regex,
    action: ApprovalAction,  // AutoApprove, Request, Deny
    scope: RuleScope,        // Global, Per-agent, Per-session
}
```

**Acceptance Criteria:**
- ✅ Approval manager implemented
- ✅ Rules engine working
- ✅ UI for approval prompts
- ✅ Audit logging
- ✅ Integration tests

### 4.2 Secrets Management

**Requirement**: Encrypted storage for sensitive data

**Backends:**
1. **File** - AES-256-GCM encrypted
2. **Keychain** - OS native (macOS/Windows)
3. **Vault** - HashiCorp Vault integration

**Functional Requirements:**
- Secret CRUD operations
- Master key derivation (Argon2)
- Secret rotation
- Secret detection in code
- Environment variable integration

**Acceptance Criteria:**
- ✅ All 3 backends implemented
- ✅ Encryption working
- ✅ Detection working
- ✅ Rotation working
- ✅ Unit + integration tests

### 4.3 Advanced LLM Features

**Requirement**: Thinking modes, streaming, caching, failover

**Features:**
1. **Extended Thinking** - Budget tokens for reasoning
2. **Streaming** - Server-Sent Events (SSE)
3. **Response Caching** - LRU cache with TTL
4. **Model Failover** - Primary + fallback chain
5. **Session Compaction** - Summarize old messages

**Acceptance Criteria:**
- ✅ All features implemented
- ✅ Thinking mode working
- ✅ Streaming working
- ✅ Cache hit rate >50%
- ✅ Failover tested
- ✅ Integration tests

### 4.4 Media Processing

**Requirement**: Image/video/audio processing and optimization

**Capabilities:**
- Image resize, compress, convert
- Video transcode, compress, extract frames
- Audio convert, compress
- PDF to images
- Document text extraction

**Dependencies:**
- image crate
- ffmpeg-next
- pdf crate

**Acceptance Criteria:**
- ✅ All formats supported
- ✅ Processing working
- ✅ Performance acceptable
- ✅ Integration tests

---

## Phase 5: Platform Integration (Weeks 86-100)

### 5.1 iOS Native App

**Requirement**: Swift app for iOS with gateway communication

**Features:**
- Gateway discovery (Bonjour)
- Secure pairing with QR code
- Real-time chat
- Push notifications
- Session history
- Camera integration
- Location services

**Tech Stack:**
- SwiftUI
- URLSession (WebSocket)
- Bonjour/mDNS
- AVFoundation (camera)

**Acceptance Criteria:**
- ✅ App builds and runs
- ✅ Gateway connection working
- ✅ Chat functional
- ✅ Notifications working
- ✅ App Store ready

### 5.2 Android Native App

**Requirement**: Kotlin app for Android

**Features:**
- Gateway discovery (NSD)
- QR code pairing
- Material Design 3 UI
- Background service
- Push notifications (FCM)
- Camera integration
- Screen recording

**Tech Stack:**
- Jetpack Compose
- OkHttp (WebSocket)
- Network Service Discovery
- CameraX

**Acceptance Criteria:**
- ✅ App builds and runs
- ✅ Gateway connection working
- ✅ Chat functional
- ✅ Notifications working
- ✅ Play Store ready

### 5.3 macOS Menu Bar App

**Requirement**: Native macOS menu bar integration

**Features:**
- Menu bar icon
- Popover chat interface
- Global keyboard shortcut
- Notification center integration
- Auto-start on login
- Quick actions

**Tech Stack:**
- SwiftUI
- AppKit
- Notification Center

**Acceptance Criteria:**
- ✅ App builds and runs
- ✅ Menu bar working
- ✅ Chat functional
- ✅ Shortcuts working
- ✅ Mac App Store ready

---

## Non-Functional Requirements

### Performance

| Metric | Target | Critical |
|--------|--------|----------|
| CLI startup | <50ms | <100ms |
| Config load | <50ms | <100ms |
| Hot-reload | <100ms | <200ms |
| RPC call | <10ms | <50ms |
| Session load | <100ms | <200ms |
| Memory baseline | <50MB | <128MB |
| Gateway throughput | 1000 req/sec | 500 req/sec |

### Reliability

| Metric | Target |
|--------|--------|
| Uptime (daemon) | 99.9% |
| Crash rate | <0.01% |
| Data loss | 0% |
| Error recovery | Automatic with retry |

### Security

| Requirement | Implementation |
|-------------|----------------|
| WASM sandbox | Memory + CPU limits |
| Secret encryption | AES-256-GCM |
| Audit logging | All privileged ops |
| Permission system | Capability-based |
| Code signing | Ed25519 for native plugins |

### Scalability

| Metric | Target |
|--------|--------|
| Concurrent sessions | 100+ |
| Message throughput | 10k msg/sec |
| File size limit | 100MB |
| Session history | Unlimited with compaction |

### Compatibility

| Platform | Min Version |
|----------|-------------|
| macOS | 12.0+ |
| iOS | 15.0+ |
| Android | 8.0+ (API 26) |
| Windows | 10+ |
| Linux | Kernel 5.4+ |

---

## Testing Requirements

### Unit Tests
- Coverage: >90% for all modules
- Framework: `cargo test`
- Mocking: Mock external dependencies

### Integration Tests
- All RPC methods
- All channels
- Gateway communication
- Plugin loading
- Config hot-reload

### End-to-End Tests
- Complete user workflows
- Platform app integration
- Multi-channel scenarios
- Failure recovery

### Performance Tests
- Benchmarks for critical paths
- Load testing for gateway
- Memory profiling
- Startup time validation

---

## Documentation Requirements

### Code Documentation
- Rustdoc for all public APIs
- Examples in docs
- Architecture diagrams

### User Documentation
- Getting started guide
- Configuration reference
- API documentation
- Integration guides
- Troubleshooting guide

### Developer Documentation
- Contributing guide
- Plugin development guide
- Channel integration guide
- Platform app development

---

## Success Metrics

### Feature Parity
- ✅ 90%+ of OpenClaw features implemented
- ✅ All core channels working
- ✅ All platform apps functional

### Performance
- ✅ 10x faster than TypeScript baseline
- ✅ <50MB memory usage
- ✅ <50ms startup time

### Quality
- ✅ <5% bug rate post-release
- ✅ >90% test coverage
- ✅ Zero critical security issues

### Adoption
- ✅ 1000+ active users (6 months post-release)
- ✅ 50+ community plugins
- ✅ 4.5+ GitHub stars rating

---

## Dependencies

### Rust Crates
```toml
# Core
tokio = "1.40"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
serde_yaml = "0.9"

# Async
async-trait = "0.1"
futures = "0.3"

# Config
notify = "7.0"
toml = "0.8"

# Database
sqlx = { version = "0.8", features = ["sqlite"] }
lancedb = "0.14"

# Plugin
wasmtime = "27.0"
libloading = "0.8"

# Security
aes-gcm = "0.10"
argon2 = "0.5"
ed25519-dalek = "2.1"

# Compression
flate2 = "1.0"

# Platform
[target.'cfg(target_os = "macos")']
darwin-libproc = "0.3"

[target.'cfg(target_os = "linux")']
libc = "0.2"
```

### Node.js Dependencies
```json
{
  "@whiskeysockets/baileys": "^6.7.0",
  "grammy": "^1.29.0",
  "discord.js": "^14.16.0",
  "@slack/bolt": "^3.22.0"
}
```

---

## Timeline Summary

| Phase | Weeks | Features |
|-------|-------|----------|
| 1. Core Infrastructure | 15 | RPC, Sessions, Config, Memory, Plugins |
| 2. Channels | 25 | 26 new channels, full features |
| 3. Advanced Systems | 30 | Skills, Hooks, Browser, TUI, Onboarding |
| 4. Security & Features | 15 | Approvals, Secrets, LLM, Media |
| 5. Platform Apps | 15 | iOS, Android, macOS apps |
| **Total** | **100** | **350+ features** |

**Target Completion**: 2 years (1 developer), 6-12 months (4+ developers)
