# DX AGI Requirements

> **Mission**: Build the world's most advanced AI agent platform with three revolutionary pillars:
> 1. **🎯 Token-Optimized Serialization** — 52-73% savings (DX Serializer) + 10-80% (DX Markdown)
> 2. **🤖 Auto-Self-Updating AI Agent** — Installs runtimes, creates WASM plugins, modifies itself
> 3. **🦀 Rust Performance Foundation** — 10-80x faster than TypeScript/Node.js competitors

---

## The DX Competitive Edge

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                        THE DX COMPETITIVE EDGE                               │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  COMPETITORS (OpenClaw, etc.):                                               │
│  ┌────────────────────────────────────────────────────────────────────────┐ │
│  │ • Fixed capabilities (20 hardcoded features)                           │ │
│  │ • JSON everywhere (100% token overhead)                                │ │
│  │ • TypeScript/Node.js (slow, GC pauses)                                 │ │
│  │ • Manual plugin installation                                           │ │
│  │ • Cannot add new messaging apps without code changes                   │ │
│  └────────────────────────────────────────────────────────────────────────┘ │
│                                                                              │
│  DX AGENT:                                                                   │
│  ┌────────────────────────────────────────────────────────────────────────┐ │
│  │ • Infinite capabilities (self-expanding via WASM)                      │ │
│  │ • DX Serializer LLM format (70%+ token savings)                        │ │
│  │ • Rust core (10-80x faster)                                            │ │
│  │ • Auto-installs runtimes, creates WASM plugins on demand               │ │
│  │ • Agent proposes & deploys new integrations autonomously               │ │
│  └────────────────────────────────────────────────────────────────────────┘ │
│                                                                              │
│  RESULT: DX is not just better — it's a different category of AI agent      │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## 1. Core Architecture Requirements

### 1.1 Config-Driven Extensibility
- **REQ-001**: All CLI features MUST be configurable via `.sr` (DX Serializer) files
- **REQ-002**: Adding new commands, integrations, or tools MUST take < 10 seconds via config
- **REQ-003**: Hot-reload MUST apply config changes without CLI restart
- **REQ-004**: Config validation MUST use DX Forge traffic braining (Green/Yellow/Red)

### 1.2 Plugin System
- **REQ-005**: Support WASM plugins via `wasmtime` for sandboxed execution
- **REQ-006**: Support native plugins via `libloading` for performance-critical extensions
- **REQ-007**: Plugin registry MUST use `dashmap` for thread-safe concurrent access
- **REQ-008**: All plugins MUST be signed with Ed25519 for security verification

### 1.3 Command Registry
- **REQ-009**: Commands MUST be defined in `commands.sr`, NOT hardcoded Rust enums
- **REQ-010**: Dynamic command discovery at runtime from `.dx/` folder
- **REQ-011**: Command versioning with backward compatibility (`version=N` field)
- **REQ-012**: Built-in commands MUST be overridable via user config

---

## 2. Theme System Requirements

### 2.1 Unified Theme Architecture
- **REQ-013**: Single `DxTheme` struct unifying all 3 existing theme systems
- **REQ-014**: Theme MUST load from `theme.sr` config file
- **REQ-015**: Support shadcn-ui design tokens (background, foreground, card, popover, primary, secondary, muted, accent, destructive, border, input, ring)
- **REQ-016**: Dark/Light mode toggle with automatic terminal detection

### 2.2 Color Customization
- **REQ-017**: Primary color configurable as solid, gradient, or rainbow animated
- **REQ-018**: Gradient colors MUST support start/end colors with direction
- **REQ-019**: Rainbow animation MUST run at 60fps without blocking main thread
- **REQ-020**: Support OKLCH color space for perceptually uniform gradients

### 2.3 Tailwind-like Atomic Styling
- **REQ-021**: Atomic token enums: `Fg`, `Bg`, `Border`, `Spacing`, `Radius`
- **REQ-022**: Composable style builder: `style.fg_primary().bg_muted().bold()`
- **REQ-023**: All styles MUST resolve through theme, NO hardcoded colors
- **REQ-024**: Style presets loadable from `.sr` files

---

## 3. UI Component Requirements

### 3.1 Reusable Components
- **REQ-025**: ALL components MUST implement ratatui `Widget` trait
- **REQ-026**: Components MUST use atomic styles from theme, NOT inline colors
- **REQ-027**: Builder pattern for all component construction
- **REQ-028**: Full rustdoc documentation with examples for each component

### 3.2 Required Components
| Component | Status | Requirements |
|-----------|--------|--------------|
| Sidebar | 🔴 New | Collapsible, nested items, badges, gradient border |
| Button | 🟡 Update | Widget trait, theme integration |
| Card | 🟡 Update | Widget trait, theme integration |
| Dialog | 🟡 Update | Widget trait, modal focus |
| Input | 🟡 Update | Widget trait, validation states |
| List | 🟡 Update | Widget trait, virtualization |
| Menu | 🟡 Update | Widget trait, keyboard nav |
| Progress | 🟡 Update | Widget trait, animated |
| Select | 🟡 Update | Widget trait, search/filter |
| Table | 🟡 Update | Widget trait, sorting, pagination |
| Tabs | 🟡 Update | Widget trait, closable |
| Toast | 🟡 Update | Widget trait, auto-dismiss |
| DiffView | ✅ Exists | Enhance with syntax highlighting |
| FileTree | 🔴 New | Lazy loading, icons, expand/collapse |

### 3.3 Sidebar Component
- **REQ-029**: Collapsible to icon-only mode
- **REQ-030**: Nested tree items with expand/collapse
- **REQ-031**: Badge counts for notifications
- **REQ-032**: Keyboard navigation (j/k, Enter)
- **REQ-033**: Gradient/rainbow border from theme.sr
- **REQ-034**: Integration with `ThreeColumnLayout`

---

## 4. Code Editor Requirements

### 4.1 File Browser
- **REQ-035**: Tree-view file explorer with icons
- **REQ-036**: Lazy loading for large directories
- **REQ-037**: File filtering and search
- **REQ-038**: Git status indicators (modified, staged, untracked)

### 4.2 Syntax Highlighting
- **REQ-039**: Use `syntect` for syntax highlighting
- **REQ-040**: Support 100+ languages via TextMate grammars
- **REQ-041**: Theme-aware color schemes
- **REQ-042**: Line numbers with relative mode option

### 4.3 Editor Features
- **REQ-043**: Vim keybindings (configurable via `editor.sr`)
- **REQ-044**: Multiple cursors
- **REQ-045**: Search and replace with regex
- **REQ-046**: Minimap preview
- **REQ-047**: Split panes (horizontal/vertical)

---

## 5. Git Integration Requirements

### 5.1 LazyGit-like UI
- **REQ-048**: Status view showing staged/unstaged/untracked
- **REQ-049**: Commit interface with message editor
- **REQ-050**: Diff viewer with syntax highlighting
- **REQ-051**: Branch management (create, switch, delete, merge)
- **REQ-052**: Stash operations (save, pop, list, drop)

### 5.2 Git Features
- **REQ-053**: Interactive staging (stage hunks, lines)
- **REQ-054**: Commit graph visualization
- **REQ-055**: Remote management (fetch, pull, push)
- **REQ-056**: Conflict resolution UI
- **REQ-057**: Blame view with author info

---

## 6. AI Agent Requirements

### 6.1 Auto-Self-Updating
- **REQ-058**: Agent MUST detect capability gaps ("I need X feature")
- **REQ-059**: Agent MUST generate `.sr` config to add capability
- **REQ-060**: DX Forge MUST validate generated configs
- **REQ-061**: Hot-reload MUST apply new capability < 1 second
- **REQ-062**: Rollback MUST restore previous config on failure

### 6.2 Multi-Runtime WASM Bridge (Self-Expanding Capabilities)
- **REQ-063**: Agent MUST install required runtimes automatically (Node.js, Python, Go, Rust, etc.)
- **REQ-064**: Agent MUST generate WASM plugins for new integrations
- **REQ-065**: WASM preferred for security, native IPC available for performance
- **REQ-066**: All WASM plugins MUST communicate via DX Machine Format (zero-copy)
- **REQ-067**: Plugin memory/CPU limits enforced via wasmtime sandbox
- **REQ-068**: Runtime installation MUST request user permission

### 6.3 Memory System
- **REQ-069**: Conversation memory using DX Serializer machine format
- **REQ-070**: ~48ns serialization/deserialization via RKYV
- **REQ-071**: Vector embeddings for semantic search
- **REQ-072**: Memory pruning based on relevance and age
- **REQ-073**: Cross-session persistence

### 6.4 LLM Communication (Token Optimization - CRITICAL)
- **REQ-074**: ALL LLM communication MUST use DX Serializer LLM format (52-73% savings)
- **REQ-075**: ALL markdown context MUST use DX Markdown (10-80% savings)
- **REQ-076**: JSON → DX LLM format auto-conversion for external APIs
- **REQ-077**: Streaming response support with chunked parsing
- **REQ-078**: Multi-provider support (OpenAI, Anthropic, local Ollama)
- **REQ-079**: Token counting, cost tracking, and savings reporting

---

## 7. Token Efficiency Requirements (MANDATORY)

### 7.1 Three-Format System (DX Serializer)

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                    MANDATORY FORMAT USAGE                                    │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  HUMAN FORMAT (.sr)                                                          │
│  ├── Config files                                                            │
│  ├── Skill definitions                                                       │
│  ├── Knowledge base entries                                                  │
│  └── User-editable settings                                                  │
│                                                                              │
│  LLM FORMAT (70%+ token savings)                                             │
│  ├── All prompts to LLM                                                      │
│  ├── All context sent to LLM                                                 │
│  ├── Expected response format                                                │
│  └── Memory/knowledge retrieval                                              │
│                                                                              │
│  MACHINE FORMAT (RKYV ~48ns)                                                 │
│  ├── Internal data structures                                                │
│  ├── IPC between processes                                                   │
│  ├── Persistent storage                                                      │
│  └── WASM plugin communication                                               │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

### 7.2 Token Savings Targets

| Context Type | Minimum Savings | Target Savings |
|--------------|-----------------|----------------|
| Simple config | 40% | 52% |
| Nested objects | 55% | 65% |
| Deep structures | 65% | 73% |
| Skill definitions | 60% | 70% |
| Memory context | 65% | 73% |
| Knowledge retrieval | 70% | 80% |

### 7.3 LLM Format Specification

```
# DX LLM Format Rules (Already Implemented in crates/serializer)

STRUCTURE:
section:count[key=value key2=value2]
section.subsection:count[nested=data]

ARRAYS:
array[N]=item1 item2 item3     # Space-separated
array[N]=item1|item2|item3     # Pipe for items with spaces

TYPE CODES (Short):
:str → string
:int → integer
:f   → float
:bool → boolean (T/F)
:path → file path
:url → URL
:b64 → base64 data

OPTIONALITY:
field? → optional field
field! → required field

EXAMPLES:
skill:1[id=pdf-parser runtime=python deps[2]=pypdf pillow]
mem:3[F|24h "fact 1" P|48h "pref 1" C|1h "conv 1"]
```

- **REQ-080**: All new features MUST use DX LLM format for LLM communication
- **REQ-081**: JSON responses from external APIs MUST be auto-converted
- **REQ-082**: Token savings MUST be logged and reported

---

## 8. Browser Automation Requirements

### 8.1 Config-Driven Automation
- **REQ-083**: Actions defined in `browser.sr` config
- **REQ-084**: Support navigate, click, type, screenshot, scrape
- **REQ-085**: Wait conditions (element, timeout, network idle)
- **REQ-086**: Cookie and session management

### 8.2 Implementation
- **REQ-087**: Use `chromiumoxide` for CDP protocol
- **REQ-088**: Headless mode by default, headed option
- **REQ-089**: Screenshot and PDF export
- **REQ-090**: DOM inspection and modification
- **REQ-091**: Network interception

---

## 9. Messaging & Integration Requirements

### 9.1 Channel Abstraction
- **REQ-092**: `Channel` trait for all messaging platforms
- **REQ-093**: Channels configurable via `channels.sr`
- **REQ-094**: Message type abstraction (text, image, audio, template)
- **REQ-095**: Webhook support as generic channel type

### 9.2 Pre-installed Channels
| Channel | Config File | Features |
|---------|-------------|----------|
| WhatsApp | `whatsapp.sr` | Business API, Personal gateway |
| Telegram | `telegram.sr` | Bot API, inline keyboards |
| Discord | `discord.sr` | Slash commands, embeds |
| Slack | `slack.sr` | App manifest, blocks |
| Webhook | `webhooks.sr` | HTTP POST/GET, auth headers |

### 9.3 Self-Expanding Channel System (NEW)
- **REQ-096**: Agent MUST create new channel adapters via WASM plugins
- **REQ-097**: For WebSocket-heavy APIs, use Node.js → WASM compilation
- **REQ-098**: For ML/data processing, use Python → WASM (Pyodide)
- **REQ-099**: For performance-critical, use Rust → WASM
- **REQ-100**: All channel plugins implement `ChannelAdapter` interface

### 9.4 Adding New Channels
- **REQ-101**: Add new channel by creating `<name>.sr` config
- **REQ-102**: Channel handler can be WASM plugin or native IPC
- **REQ-103**: Auto-discovery of channel configs in `.dx/channels/`

---

## 9. Protocol Requirements

### 9.1 LSP (Language Server Protocol)
- **REQ-089**: Full LSP 3.17 compliance
- **REQ-090**: Binary-first transport option (DCP)
- **REQ-091**: Multi-language support

### 9.2 MCP (Model Context Protocol)
- **REQ-092**: Full MCP compliance for tool/resource exposure
- **REQ-093**: Backward compatibility with MCP clients

### 9.3 DCP (Development Context Protocol)
- **REQ-094**: 10-1000x faster than JSON-RPC MCP
- **REQ-095**: Zero-copy tool dispatch
- **REQ-096**: Binary message format

---

## 10. Community & Forge Requirements

### 10.1 Contribution Tracking
- **REQ-097**: Track plugin authors via git user info
- **REQ-098**: Push contributions to main DX repo
- **REQ-099**: Display credits with GitHub username
- **REQ-100**: Leaderboard of top contributors

### 10.2 DX Forge Validation
- **REQ-101**: Traffic braining: Green (safe) / Yellow (review) / Red (reject)
- **REQ-102**: Automated security scanning of plugins
- **REQ-103**: Performance benchmarking of new integrations
- **REQ-104**: Breaking change detection

---

## 11. Performance Requirements

- **REQ-105**: CLI startup < 50ms
- **REQ-106**: Hot-reload < 100ms
- **REQ-107**: Theme animation 60fps without frame drops
- **REQ-108**: Memory usage < 50MB baseline
- **REQ-109**: Zero GC pauses (stack-only allocation in hot paths)
- **REQ-110**: SIMD optimizations where applicable

---

## 12. Platform Apps Requirements

### 12.1 macOS App
- **REQ-111**: Menu bar app with status indicator and quick actions
- **REQ-112**: Voice Wake with configurable wake word ("Hey DX")
- **REQ-113**: Talk Mode overlay for continuous conversation
- **REQ-114**: Canvas host for visual workspace (A2UI)
- **REQ-115**: WebChat embedded view
- **REQ-116**: System notifications integration
- **REQ-117**: Code signing with Apple Developer certificate

### 12.2 iOS App
- **REQ-118**: SwiftUI-based Canvas with A2UI support
- **REQ-119**: Camera integration (photo capture, video clips)
- **REQ-120**: Voice Wake using iOS Speech Framework
- **REQ-121**: Talk Mode with on-device processing
- **REQ-122**: Bonjour discovery for gateway connection
- **REQ-123**: Location services for `location.get` command

### 12.3 Android App
- **REQ-124**: Jetpack Compose Canvas with Material 3
- **REQ-125**: CameraX integration for photo/video
- **REQ-126**: MediaProjection screen recording
- **REQ-127**: Foreground service for persistent connection
- **REQ-128**: NSD (Network Service Discovery) for gateway
- **REQ-129**: Optional SMS gateway support

### 12.4 Windows Support
- **REQ-130**: WSL2 integration for full feature support
- **REQ-131**: Native Windows CLI binary
- **REQ-132**: PowerShell integration
- **REQ-133**: Windows Terminal compatibility

### 12.5 Linux Support
- **REQ-134**: Native binary with systemd integration
- **REQ-135**: io_uring support for high-performance I/O
- **REQ-136**: X11/Wayland display server support
- **REQ-137**: PulseAudio/PipeWire audio integration

### 12.6 Gateway Protocol
- **REQ-138**: Unified WebSocket protocol for all platform apps
- **REQ-139**: mDNS/Bonjour discovery
- **REQ-140**: One-time pairing codes with expiration
- **REQ-141**: Node command execution (camera.snap, canvas.render, etc.)

---

## 13. Integration Requirements

### 13.1 Protocol Integrations
- **REQ-142**: LSP server with full LSP 3.17 compliance
- **REQ-143**: MCP server with tools, resources, and prompts
- **REQ-144**: Webhook receiver with bearer/HMAC auth
- **REQ-145**: WebSocket server for real-time events

### 13.2 Voice & Audio Integrations
- **REQ-146**: ElevenLabs TTS with voice_id and model selection
- **REQ-147**: Voice Wake with Whisper.cpp local processing
- **REQ-148**: Shazam-like song recognition
- **REQ-149**: Spotify playback control via OAuth
- **REQ-150**: Sonos multi-room audio control

### 13.3 Automation Integrations
- **REQ-151**: Zapier webhook triggers
- **REQ-152**: N8N workflow execution
- **REQ-153**: Cron scheduler with timezone support
- **REQ-154**: Gmail Pub/Sub for real-time email notifications
- **REQ-155**: Answer Call via Twilio/similar providers

### 13.4 Productivity Integrations
- **REQ-156**: Notion API (pages, databases, blocks)
- **REQ-157**: GitHub CLI wrapper (issues, PRs, runs)
- **REQ-158**: Obsidian vault management
- **REQ-159**: Trello board/card operations
- **REQ-160**: Things 3 task management (macOS)
- **REQ-161**: Bear Notes integration (macOS)
- **REQ-162**: Apple Notes integration (macOS/iOS)
- **REQ-163**: Apple Reminders integration (macOS/iOS)

### 13.5 Communication Integrations
- **REQ-164**: Email send/receive via SMTP/IMAP
- **REQ-165**: Twitter/X posting, replies, and search

### 13.6 Media Integrations
- **REQ-166**: Photo/Video capture from device cameras
- **REQ-167**: Screen capture and recording
- **REQ-168**: GIF search via Giphy/Tenor
- **REQ-169**: Weather forecasts via wttr.in/Open-Meteo

### 13.7 Security & Home Integrations
- **REQ-170**: 1Password CLI integration with tmux session
- **REQ-171**: IoT/Smart Home via HomeAssistant/Hue

---

## 14. Omnibus Architecture Requirements

### 14.1 Token Optimization
- **REQ-172**: 52-73% token savings via DX Serializer LLM format
- **REQ-173**: 10-80% markdown optimization via DX Markdown
- **REQ-174**: Three-format system (Human .sr, LLM compact, Machine binary)
- **REQ-175**: ~48ns serialization via RKYV zero-copy

### 14.2 Deployment
- **REQ-176**: Single binary < 20MB
- **REQ-177**: Docker scratch image < 20MB
- **REQ-178**: Run on 128MB RAM micro-instances
- **REQ-179**: One-click deploy to VPS/cloud

### 14.3 Memory System
- **REQ-180**: Embedded LanceDB for vector storage
- **REQ-181**: Semantic search with local embeddings
- **REQ-182**: Encrypted at-rest with Argon2 key derivation
- **REQ-183**: Cross-session persistence

### 14.4 Security & Sandboxing
- **REQ-184**: WASM sandbox for untrusted code
- **REQ-185**: Capability-based permissions (shell, files, network)
- **REQ-186**: Audit logging of all agent actions
- **REQ-187**: Firejail/bubblewrap sandboxing option

### 14.5 Skill Synthesis
- **REQ-188**: Agent self-synthesis of new skills
- **REQ-189**: Skill validation pipeline (syntax, security, test)
- **REQ-190**: Hot-reload of skills without restart
- **REQ-191**: Skill specs in .sr format (human-readable)

---

## 15. Documentation Requirements

- **REQ-192**: Rustdoc for all public APIs with examples
- **REQ-193**: `UI_COMPONENTS.md` catalog with screenshots
- **REQ-194**: `PLUGIN_DEVELOPMENT.md` guide
- **REQ-195**: `CONTRIBUTING.md` with Forge workflow
- **REQ-196**: Inline code comments following Rust best practices
- **REQ-197**: `platforms.md` for OS-specific app documentation
- **REQ-198**: `integrations.md` for all service integrations
- **REQ-199**: `omnibus.md` for complete AGI architecture

---

## Summary

| Category | Requirements | Priority |
|----------|--------------|----------|
| Config Extensibility | REQ-001 to REQ-012 | 🔴 Critical |
| Theme System | REQ-013 to REQ-024 | 🔴 Critical |
| UI Components | REQ-025 to REQ-034 | 🟡 High |
| Code Editor | REQ-035 to REQ-047 | 🟡 High |
| Git Integration | REQ-048 to REQ-057 | 🟡 High |
| AI Agent | REQ-058 to REQ-072 | 🔴 Critical |
| Browser Automation | REQ-073 to REQ-081 | 🟢 Medium |
| Messaging | REQ-082 to REQ-088 | 🟡 High |
| Protocols | REQ-089 to REQ-096 | 🟢 Medium |
| Community | REQ-097 to REQ-104 | 🟢 Medium |
| Performance | REQ-105 to REQ-110 | 🔴 Critical |
| Platform Apps | REQ-111 to REQ-141 | 🟡 High |
| Integrations | REQ-142 to REQ-171 | 🟡 High |
| Omnibus Architecture | REQ-172 to REQ-191 | 🔴 Critical |
| Documentation | REQ-192 to REQ-199 | 🟡 High |

**Total: 199 Requirements**

---

## Platform Build Matrix

| Platform | Build Status | Requirements |
|----------|--------------|-------------|
| macOS | 🟡 Planned | REQ-111 to REQ-117 |
| iOS | 🟡 Planned | REQ-118 to REQ-123 |
| Android | 🟢 Buildable | REQ-124 to REQ-129 |
| Windows | 🟢 Buildable | REQ-130 to REQ-133 |
| Linux | 🟢 Buildable | REQ-134 to REQ-137 |

## Integration Categories

| Category | Requirements | Count |
|----------|--------------|-------|
| Protocols | REQ-142 to REQ-145 | 4 |
| Voice & Audio | REQ-146 to REQ-150 | 5 |
| Automation | REQ-151 to REQ-155 | 5 |
| Productivity | REQ-156 to REQ-163 | 8 |
| Communication | REQ-164 to REQ-165 | 2 |
| Media | REQ-166 to REQ-169 | 4 |
| Security & Home | REQ-170 to REQ-171 | 2 |
