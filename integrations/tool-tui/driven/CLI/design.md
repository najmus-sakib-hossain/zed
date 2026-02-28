# DX CLI Design Document - OpenClaw Feature Integration

> **Architecture Goal**: Maintain Rust's safety guarantees while achieving full OpenClaw feature parity with superior performance and reliability.

---

## Table of Contents

1. [System Architecture](#system-architecture)
2. [Core Design Principles](#core-design-principles)
3. [Component Design](#component-design)
4. [Data Flow](#data-flow)
5. [Security Architecture](#security-architecture)
6. [Performance Optimization](#performance-optimization)
7. [Technology Stack](#technology-stack)
8. [Migration Strategy](#migration-strategy)

---

## System Architecture

### High-Level Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                      DX CLI Architecture                      │
├─────────────────────────────────────────────────────────────┤
│                                                               │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐      │
│  │ CLI Commands │  │  TUI Client  │  │ Platform Apps│      │
│  │  (clap 4.x)  │  │  (ratatui)   │  │ (iOS/Android)│      │
│  └──────┬───────┘  └──────┬───────┘  └──────┬───────┘      │
│         │                  │                  │              │
│         ├──────────────────┴──────────────────┘              │
│         │                                                     │
│  ┌──────▼──────────────────────────────────────────────┐    │
│  │            Gateway (Axum WebSocket/HTTP)             │    │
│  │  ┌────────────────────────────────────────────┐     │    │
│  │  │         RPC Method Registry (70+)          │     │    │
│  │  └────────────────────────────────────────────┘     │    │
│  └──────┬───────────────────┬───────────────────┬──────┘    │
│         │                   │                   │            │
│  ┌──────▼─────┐    ┌───────▼──────┐    ┌──────▼──────┐     │
│  │  Session   │    │   Config     │    │   Plugin    │     │
│  │  Manager   │    │   Manager    │    │   Manager   │     │
│  └──────┬─────┘    └───────┬──────┘    └──────┬──────┘     │
│         │                   │                   │            │
│  ┌──────▼──────────────────▼───────────────────▼──────┐     │
│  │              State Management Layer                 │     │
│  │  ┌────────┐  ┌────────┐  ┌────────┐  ┌────────┐  │     │
│  │  │ Memory │  │ Agents │  │Channels│  │ Skills │  │     │
│  │  └────────┘  └────────┘  └────────┘  └────────┘  │     │
│  └────────────────────────────────────────────────────┘     │
│         │                   │                   │            │
│  ┌──────▼─────┐    ┌───────▼──────┐    ┌──────▼──────┐     │
│  │  Storage   │    │   Bridge     │    │   External  │     │
│  │  (SQLite/  │    │   (Node.js   │    │   Services  │     │
│  │  LanceDB)  │    │   Subprocess)│    │   (APIs)    │     │
│  └────────────┘    └──────────────┘    └─────────────┘     │
│                                                               │
└─────────────────────────────────────────────────────────────┘
```

### Component Diagram

```
┌───────────────────────────────────────────────────────────────┐
│                        CLI Layer                               │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐     │
│  │  gateway │  │ channel  │  │  agent   │  │  config  │     │
│  │  start   │  │ connect  │  │  chat    │  │  get/set │     │
│  └──────────┘  └──────────┘  └──────────┘  └──────────┘     │
└───────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌───────────────────────────────────────────────────────────────┐
│                      Gateway Layer                             │
│  ┌──────────────────────────────────────────────────────┐     │
│  │  WebSocket/HTTP Server (Axum)                        │     │
│  │  - JSON-RPC 2.0 handler                              │     │
│  │  - Connection management                             │     │
│  │  - Event broadcasting                                │     │
│  └──────────────────────────────────────────────────────┘     │
│  ┌──────────────────────────────────────────────────────┐     │
│  │  RPC Method Registry                                 │     │
│  │  - Method registration                               │     │
│  │  - Request routing                                   │     │
│  │  - Error handling                                    │     │
│  └──────────────────────────────────────────────────────┘     │
└───────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌───────────────────────────────────────────────────────────────┐
│                    Service Layer                               │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐           │
│  │  Session    │  │   Config    │  │   Plugin    │           │
│  │  Manager    │  │   Manager   │  │   Manager   │           │
│  │             │  │             │  │             │           │
│  │ - CRUD ops  │  │ - Load/save │  │ - Load/exec │           │
│  │ - Compact   │  │ - Hot-reload│  │ - Sandbox   │           │
│  │ - Export    │  │ - Validate  │  │ - Registry  │           │
│  └─────────────┘  └─────────────┘  └─────────────┘           │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐           │
│  │   Memory    │  │   Agent     │  │  Channel    │           │
│  │   Manager   │  │   Manager   │  │  Manager    │           │
│  │             │  │             │  │             │           │
│  │ - Embed gen │  │ - Execution │  │ - Bridge    │           │
│  │ - Search    │  │ - Tools     │  │ - Events    │           │
│  │ - Index     │  │ - Models    │  │ - Media     │           │
│  └─────────────┘  └─────────────┘  └─────────────┘           │
└───────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌───────────────────────────────────────────────────────────────┐
│                   Persistence Layer                            │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐           │
│  │   SQLite    │  │  LanceDB    │  │    Files    │           │
│  │   (FTS5)    │  │   (Vector)  │  │   (JSON)    │           │
│  └─────────────┘  └─────────────┘  └─────────────┘           │
└───────────────────────────────────────────────────────────────┘
```

---

## Core Design Principles

### 1. **Safety First**
- Leverage Rust's ownership system for memory safety
- Use `Result<T, E>` for all fallible operations
- No `unwrap()` in production code (use `expect()` with context)
- WASM sandbox for untrusted code execution
- Capability-based permissions

### 2. **Performance by Design**
- Zero-copy where possible (Cow, &str vs String)
- Async I/O everywhere (tokio runtime)
- Connection pooling (database, HTTP clients)
- Lazy initialization of heavy resources
- Efficient serialization (serde with zero-copy)

### 3. **Modularity**
- Clear separation of concerns
- Trait-based abstractions
- Dependency injection via Arc<T>
- Plugin architecture for extensibility

### 4. **Developer Experience**
- Consistent error messages
- Comprehensive logging (tracing crate)
- Hot-reload for configuration
- Rich CLI with colors and progress bars

### 5. **Interoperability**
- Node.js bridge for channel integrations
- Stable JSON-RPC 2.0 interface
- OpenAPI/gRPC support for external clients
- Cross-platform native apps

---

## Component Design

### 1. Gateway (RPC Server)

**File**: `crates/cli/src/gateway/mod.rs`

```rust
pub struct Gateway {
    state: Arc<GatewayState>,
    registry: Arc<MethodRegistry>,
    config: GatewayConfig,
}

pub struct GatewayState {
    pub clients: Arc<DashMap<String, WebSocketClient>>,
    pub session_manager: Arc<SessionManager>,
    pub config_manager: Arc<ConfigManager>,
    pub plugin_manager: Arc<PluginManager>,
    pub memory_manager: Arc<MemoryManager>,
    pub agent_manager: Arc<AgentManager>,
    pub channel_manager: Arc<ChannelManager>,
    pub approval_manager: Arc<ApprovalManager>,
}

impl Gateway {
    pub async fn start(&self) -> Result<()> {
        // 1. Initialize all managers
        // 2. Start HTTP/WebSocket server (Axum)
        // 3. Start background tasks (session cleanup, indexing)
        // 4. Start channel bridges
        // 5. Register signal handlers (SIGTERM, SIGINT)
    }
}
```

**Design Decisions:**
- **Axum** over Actix for better async/await ergonomics
- **WebSocket** for real-time bidirectional communication
- **JSON-RPC 2.0** for standardized RPC protocol
- **DashMap** for concurrent client storage (lock-free reads)
- **Arc** for shared state (cheap clone, thread-safe)

### 2. RPC Method Registry

**File**: `crates/cli/src/gateway/rpc.rs`

```rust
pub struct MethodRegistry {
    handlers: HashMap<String, Box<dyn RpcHandler>>,
    categories: HashMap<String, Vec<String>>,
}

#[async_trait]
pub trait RpcHandler: Send + Sync {
    async fn handle(
        &self,
        state: Arc<GatewayState>,
        client_id: String,
        params: Value,
    ) -> Result<Value, RpcError>;
}

impl MethodRegistry {
    pub fn register<F, Fut>(&mut self, method: &str, handler: F)
    where
        F: Fn(Arc<GatewayState>, String, Value) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<Value, RpcError>> + Send + 'static,
    {
        // Box the handler and store in HashMap
    }

    pub async fn invoke(
        &self,
        state: Arc<GatewayState>,
        client_id: &str,
        request: JsonRpcRequest,
    ) -> JsonRpcResponse {
        // 1. Find handler by method name
        // 2. Call handler with params
        // 3. Handle errors gracefully
        // 4. Return response
    }
}
```

**Design Decisions:**
- **Dynamic dispatch** via trait objects for flexibility
- **Async trait** for async method handlers
- **Closure-based** registration for ergonomics
- **Category grouping** for documentation

### 3. Session Manager

**File**: `crates/cli/src/session/mod.rs`

```rust
pub struct SessionManager {
    storage: Arc<dyn SessionStorage>,
    active: DashMap<String, Arc<RwLock<Session>>>,
    config: SessionConfig,
}

#[async_trait]
pub trait SessionStorage: Send + Sync {
    async fn load(&self, key: &str) -> Result<Session>;
    async fn save(&self, session: &Session) -> Result<()>;
    async fn delete(&self, key: &str) -> Result<()>;
    async fn list(&self, filter: SessionFilter) -> Result<Vec<SessionInfo>>;
}

pub struct FileSessionStorage {
    base_path: PathBuf,
}

impl SessionManager {
    pub async fn create(&self, agent_id: &str) -> Result<Session> {
        let key = self.generate_key();
        let session = Session {
            key: key.clone(),
            agent_id: agent_id.to_string(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            messages: Vec::new(),
            metadata: HashMap::new(),
            state: SessionState::Active,
            context_length: 0,
            token_count: 0,
        };

        // Store in memory
        self.active.insert(key.clone(), Arc::new(RwLock::new(session.clone())));

        // Persist to disk
        self.storage.save(&session).await?;

        Ok(session)
    }

    pub async fn compact(&self, key: &str) -> Result<Session> {
        // 1. Load session
        // 2. Summarize old messages
        // 3. Replace with summary
        // 4. Update token count
        // 5. Save
    }
}
```

**Design Decisions:**
- **Trait-based storage** for pluggable backends
- **Dual storage**: In-memory (hot) + Disk (cold)
- **RwLock** for concurrent read access
- **Atomic writes**: Temp file + rename
- **ULID keys** for time-sortable IDs

### 4. Config Manager

**File**: `crates/cli/src/config/mod.rs`

```rust
pub struct ConfigManager {
    path: PathBuf,
    config: Arc<RwLock<DxConfig>>,
    watcher: Option<FileWatcher>,
    reload_tx: broadcast::Sender<()>,
}

#[derive(Deserialize, Serialize, JsonSchema)]
pub struct DxConfig {
    pub version: String,
    pub gateway: GatewayConfig,
    pub agents: Vec<AgentConfig>,
    pub channels: HashMap<String, ChannelConfig>,
    pub providers: HashMap<String, ProviderConfig>,
    pub memory: MemoryConfig,
    pub plugins: PluginConfig,
}

impl ConfigManager {
    pub async fn load(path: PathBuf) -> Result<Self> {
        // 1. Read file
        // 2. Substitute env vars
        // 3. Process includes
        // 4. Deep merge
        // 5. Validate against schema
        // 6. Start file watcher
    }

    pub async fn reload(&self) -> Result<()> {
        // 1. Re-load config
        // 2. Validate
        // 3. Update Arc<RwLock<DxConfig>>
        // 4. Broadcast reload event
    }
}
```

**Design Decisions:**
- **YAML format** for human-readability
- **File watcher** (notify crate) for hot-reload
- **Broadcast channel** for reload notifications
- **JSON Schema** generation for validation
- **Environment variables** with `${VAR:-default}` syntax

### 5. Plugin Manager

**File**: `crates/cli/src/plugins/mod.rs`

```rust
pub struct PluginManager {
    registry: Arc<RwLock<PluginRegistry>>,
    loaded: DashMap<String, LoadedPlugin>,
    wasm_runtime: Arc<WasmRuntime>,
    config: PluginConfig,
}

pub struct LoadedPlugin {
    manifest: PluginManifest,
    instance: PluginInstance,
    permissions: PermissionSet,
}

pub enum PluginInstance {
    Wasm(WasmInstance),
    Native(NativeInstance),
    Script(ScriptInstance),
}

impl PluginManager {
    pub async fn load(&self, path: &Path) -> Result<PluginId> {
        // 1. Read manifest
        // 2. Validate signature (native only)
        // 3. Check permissions
        // 4. Load binary (WASM/Native/Script)
        // 5. Initialize plugin
        // 6. Register hooks
    }

    pub async fn invoke(
        &self,
        id: &PluginId,
        hook: &str,
        args: Value,
    ) -> Result<Value> {
        // 1. Check permissions
        // 2. Apply resource limits
        // 3. Call plugin
        // 4. Handle timeout
        // 5. Return result
    }
}
```

**Design Decisions:**
- **WASM-first** for security and portability
- **Wasmtime** for WASM runtime (mature, fast)
- **Capability-based permissions** (network, fs, kv)
- **Resource limits** (memory, CPU time)
- **Hot-reload** support
- **Signature verification** for native plugins

### 6. Memory Manager

**File**: `crates/cli/src/memory/mod.rs`

```rust
pub struct MemoryManager {
    backend: Arc<dyn MemoryBackend>,
    embedder: Arc<dyn EmbeddingProvider>,
    indexer: Arc<BackgroundIndexer>,
}

#[async_trait]
pub trait MemoryBackend: Send + Sync {
    async fn store(&self, doc: Document) -> Result<String>;
    async fn search_text(&self, query: &str, limit: usize) -> Result<Vec<Document>>;
    async fn search_vector(&self, embedding: &[f32], limit: usize) -> Result<Vec<(Document, f32)>>;
}

pub struct SqliteBackend {
    pool: SqlitePool,
}

pub struct LanceDbBackend {
    db: LanceDb,
    table: Table,
}

impl MemoryManager {
    pub async fn store(&self, content: String, metadata: HashMap<String, Value>) -> Result<String> {
        // 1. Create document
        // 2. Store in backend
        // 3. Queue for embedding generation (background)
    }

    pub async fn search(&self, query: &str, use_semantic: bool) -> Result<Vec<Document>> {
        if use_semantic {
            // 1. Generate query embedding
            // 2. Vector search
        } else {
            // Full-text search
        }
    }
}
```

**Design Decisions:**
- **Pluggable backends** (SQLite, LanceDB, file)
- **Background indexing** for embedding generation
- **Hybrid search** (text + vector)
- **Connection pooling** (sqlx)
- **Configurable embedders** (OpenAI, Voyage, local ONNX)

### 7. Channel Manager & Bridge

**File**: `crates/cli/src/gateway/bridge.rs`

```rust
pub struct ChannelManager {
    bridges: DashMap<String, ChannelBridge>,
    event_tx: broadcast::Sender<ChannelEvent>,
}

pub struct ChannelBridge {
    channel_id: String,
    process: Child,
    stdin: ChildStdin,
    stdout_rx: UnboundedReceiver<BridgeMessage>,
    config: ChannelConfig,
}

impl ChannelManager {
    pub async fn start_channel(&self, channel_id: &str, config: ChannelConfig) -> Result<()> {
        // 1. Spawn Node.js process
        // 2. Pass config via stdin
        // 3. Start stdout reader task
        // 4. Monitor for crashes
        // 5. Auto-restart on failure
    }

    pub async fn send(&self, channel_id: &str, message: BridgeMessage) -> Result<()> {
        // 1. Find bridge
        // 2. Serialize message
        // 3. Write to stdin
        // 4. Flush
    }

    fn handle_event(&self, event: BridgeEvent) {
        // 1. Deserialize event
        // 2. Broadcast to subscribers
        // 3. Log
    }
}
```

**Design Decisions:**
- **Node.js subprocess** for channel integrations (reuse ecosystem)
- **JSON-RPC** over stdin/stdout for IPC
- **Auto-restart** on crash with exponential backoff
- **Broadcast channel** for events
- **Process monitoring** with health checks

---

## Data Flow

### Message Flow: User → Agent → Channel

```
┌────────┐     1. HTTP POST      ┌─────────┐
│  User  │ ────────────────────► │ Gateway │
│ (CLI)  │                        │ (Axum)  │
└────────┘                        └────┬────┘
                                       │
                                  2. Parse JSON-RPC
                                       │
                                       ▼
                              ┌────────────────┐
                              │ RPC Registry   │
                              │ invoke(...)    │
                              └────────┬───────┘
                                       │
                                  3. Route to handler
                                       │
                      ┌────────────────┼────────────────┐
                      ▼                                  ▼
            ┌─────────────────┐              ┌─────────────────┐
            │ Session Manager │              │ Agent Manager   │
            │ get_session()   │              │ execute(...)    │
            └────────┬────────┘              └────────┬────────┘
                     │                                │
                4. Load session                  5. Call LLM API
                     │                                │
                     │                                ▼
                     │                       ┌─────────────────┐
                     │                       │   LLM Provider  │
                     │                       │   (Anthropic)   │
                     │                       └────────┬────────┘
                     │                                │
                     │                        6. Stream response
                     │                                │
                     ▼                                ▼
            ┌─────────────────┐              ┌─────────────────┐
            │ Session Manager │              │ Channel Manager │
            │ append_message()│              │ send_message()  │
            └────────┬────────┘              └────────┬────────┘
                     │                                │
                7. Update local                  8. Write to bridge
                     │                                │
                     ▼                                ▼
            ┌─────────────────┐              ┌─────────────────┐
            │  File Storage   │              │ Node.js Bridge  │
            │  (JSON)         │              │ (WhatsApp)      │
            └─────────────────┘              └────────┬────────┘
                                                      │
                                              9. Send to platform
                                                      │
                                                      ▼
                                             ┌─────────────────┐
                                             │    WhatsApp     │
                                             │   Server API    │
                                             └─────────────────┘
```

### Configuration Hot-Reload Flow

```
┌─────────┐   1. File change   ┌──────────┐
│  User   │ ──────────────────►│  notify  │
│ edits   │                     │  watcher │
│ config  │                     └─────┬────┘
└─────────┘                           │
                                 2. Debounce (100ms)
                                      │
                                      ▼
                            ┌──────────────────┐
                            │ Config Manager   │
                            │ reload()         │
                            └─────┬────────────┘
                                  │
                         3. Parse + Validate
                                  │
                    ┌─────────────┼─────────────┐
                    ▼                            ▼
          ┌──────────────┐          ┌──────────────────┐
          │ Update state │          │ Broadcast reload │
          │ (RwLock)     │          │ event            │
          └──────────────┘          └─────┬────────────┘
                                          │
                                     4. Notify subscribers
                                          │
                  ┌───────────────────────┼───────────────────┐
                  ▼                       ▼                   ▼
        ┌─────────────┐        ┌──────────────┐   ┌────────────────┐
        │  Gateway    │        │ Channel Mgr   │   │   Agent Mgr    │
        │ update port │        │ restart bridge│   │ reload models  │
        └─────────────┘        └───────────────┘   └────────────────┘
```

### Plugin Invocation Flow

```
┌──────────┐  1. Call plugin   ┌──────────────┐
│  Agent   │ ─────────────────►│ Plugin Mgr   │
│(tool use)│                    │ invoke(...)  │
└──────────┘                    └──────┬───────┘
                                       │
                              2. Check permissions
                                       │
                                       ▼
                              ┌─────────────────┐
                              │ Permission Check │
                              │ - network?       │
                              │ - filesystem?    │
                              └────────┬─────────┘
                                       │
                               3. Pass ✓ / Fail ✗
                                       │
                        ┌──────────────┴──────────────┐
                        ▼                              ▼
              ┌──────────────────┐          ┌──────────────────┐
              │ WASM Instance    │          │ Native Instance  │
              │ (wasmtime)       │          │ (libloading)     │
              └────────┬─────────┘          └────────┬─────────┘
                       │                              │
                  4. Execute                     4. Execute
                       │                              │
              5. Apply limits                  5. Apply limits
                 (memory, CPU)                   (timeout)
                       │                              │
                       └──────────┬───────────────────┘
                                  │
                             6. Return result
                                  │
                                  ▼
                          ┌────────────────┐
                          │  Agent         │
                          │ (continue)     │
                          └────────────────┘
```

---

## Security Architecture

### 1. WASM Sandbox

**Isolation Model:**
- **Memory isolation**: Each plugin has isolated linear memory
- **CPU limits**: Configurable fuel (instruction count)
- **Timeout**: 30s default per call
- **No network by default**: Must request permission

**Host Interface (WASI):**
```rust
// Limited host functions exposed to WASM
fn dx_log(level: u32, message: &str);
fn dx_http_request(url: &str, method: &str) -> Result<Response>;
fn dx_kv_get(key: &str) -> Option<Vec<u8>>;
fn dx_kv_set(key: &str, value: &[u8]) -> Result<()>;
```

**Permissions:**
```rust
pub enum Permission {
    Network,
    FileSystem { path: PathBuf, read: bool, write: bool },
    KeyValue,
    Process,
}
```

### 2. Exec Approval System

**Rule Evaluation:**
```rust
pub struct ApprovalRule {
    pattern: Regex,          // e.g., r"^rm\s+-rf"
    action: ApprovalAction,  // AutoApprove, Request, Deny
    scope: RuleScope,        // Global, PerAgent, PerSession
}

pub enum ApprovalAction {
    AutoApprove,
    Request { timeout: Duration },
    Deny { reason: String },
}
```

**Approval Flow:**
1. Agent requests tool execution
2. Approval manager checks rules
3. If `Request`: Block and prompt user
4. User approves/denies
5. Log to audit trail
6. Execute or reject

### 3. Secret Encryption

**Encryption:**
- **Algorithm**: AES-256-GCM
- **Key derivation**: Argon2 from master password
- **Salt**: Random 16 bytes per secret
- **Nonce**: Random 12 bytes per encryption

**Storage:**
```json
{
  "version": 1,
  "secrets": {
    "anthropic_api_key": {
      "ciphertext": "...",
      "salt": "...",
      "nonce": "..."
    }
  }
}
```

### 4. Code Signing (Native Plugins)

**Signing:**
- **Algorithm**: Ed25519
- **Key pair**: Generated per developer
- **Signature**: Detached, stored in manifest

**Verification:**
1. Load manifest
2. Extract signature
3. Hash plugin binary
4. Verify signature with public key
5. Reject if invalid

---

## Performance Optimization

### 1. Async Everything

**Tokio Runtime:**
```rust
#[tokio::main(flavor = "multi_thread", worker_threads = 4)]
async fn main() -> Result<()> {
    // All I/O operations are async
    // No blocking in async context
}
```

**Best Practices:**
- Use `spawn_blocking` for CPU-bound work
- Connection pooling for database
- HTTP client pooling
- Channel buffering (bounded channels)

### 2. Zero-Copy Deserialization

**Serde with Cow:**
```rust
#[derive(Deserialize)]
pub struct Message<'a> {
    #[serde(borrow)]
    pub text: Cow<'a, str>,
    #[serde(borrow)]
    pub metadata: Cow<'a, str>,
}
```

### 3. Lazy Initialization

**OnceCell for heavy resources:**
```rust
use once_cell::sync::Lazy;

static EMBEDDER: Lazy<Arc<Embedder>> = Lazy::new(|| {
    Arc::new(Embedder::new())
});
```

### 4. Caching Strategies

**LRU Cache:**
```rust
use lru::LruCache;

pub struct ResponseCache {
    cache: Arc<Mutex<LruCache<CacheKey, CachedResponse>>>,
    ttl: Duration,
}
```

**Cache Layers:**
- **L1**: In-memory (hot data)
- **L2**: SQLite (warm data)
- **L3**: Disk files (cold data)

### 5. Benchmarking

**Criterion for benchmarks:**
```rust
use criterion::{criterion_group, criterion_main, Criterion};

fn bench_session_load(c: &mut Criterion) {
    c.bench_function("session_load", |b| {
        b.iter(|| {
            // Benchmark session loading
        });
    });
}
```

**Targets:**
- Session load: <100ms
- RPC call: <10ms
- Config reload: <100ms
- Plugin invoke: <20ms

---

## Technology Stack

### Core Technologies

| Component | Technology | Justification |
|-----------|-----------|---------------|
| Language | Rust 1.85+ | Safety, performance, ecosystem |
| Async runtime | Tokio 1.40 | Industry standard, mature |
| HTTP/WebSocket | Axum 0.8 | Ergonomic, type-safe |
| CLI framework | clap 4.5 | Derive macros, completions |
| Serialization | serde 1.0 | Zero-copy, fast |
| Database | SQLx 0.8 | Compile-time SQL checking |
| Vector DB | LanceDB 0.14 | Rust-native, fast |
| WASM runtime | Wasmtime 27.0 | Mature, secure |
| Logging | tracing 0.1 | Structured, async |
| Testing | cargo test | Built-in, fast |

### Node.js Bridge

| Component | Technology | Justification |
|-----------|-----------|---------------|
| WhatsApp | @whiskeysockets/baileys | Active, feature-rich |
| Telegram | grammy | Type-safe, modern |
| Discord | discord.js | Official, comprehensive |
| Slack | @slack/bolt | Official SDK |
| Signal | signal-cli | CLI wrapper |

### Platform Apps

| Platform | Technology | Justification |
|----------|-----------|---------------|
| iOS | SwiftUI | Native, declarative |
| Android | Jetpack Compose | Native, declarative |
| macOS | AppKit + SwiftUI | Native, menu bar support |

---

## Migration Strategy

### Phase 1: Foundation (Current)

✅ **Completed:**
- CLI structure (clap)
- Gateway RPC (basic)
- Channel bridge framework
- Basic commands

🚧 **In Progress:**
- RPC method expansion (51 new methods)
- Session management
- Config system overhaul

### Phase 2: Core Services

**Goals:**
- Memory manager
- Plugin system
- Enhanced channels
- Skills system

**Migration Path:**
1. Implement Rust modules
2. Add unit tests
3. Add integration tests
4. Document APIs
5. Deploy

### Phase 3: Advanced Features

**Goals:**
- Browser automation
- TUI
- Onboarding wizard
- Diagnostics

**Migration Path:**
1. Prototype in separate branch
2. Review with team
3. Merge incrementally
4. Monitor performance

### Phase 4: Platform Apps

**Goals:**
- iOS/Android/macOS apps
- Cross-platform testing
- App store preparation

**Migration Path:**
1. Develop apps in parallel
2. Beta testing (TestFlight, Play Store Beta)
3. Public release
4. Gather feedback

---

## Error Handling Strategy

### Error Types

```rust
#[derive(Debug, thiserror::Error)]
pub enum DxError {
    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Session not found: {0}")]
    SessionNotFound(String),

    #[error("Plugin error: {0}")]
    Plugin(String),

    #[error("Channel error: {0}")]
    Channel(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}
```

### Error Propagation

```rust
// Use ? operator for propagation
pub async fn load_session(key: &str) -> Result<Session, DxError> {
    let path = self.session_path(key);
    let content = tokio::fs::read_to_string(&path).await?;
    let session = serde_json::from_str(&content)?;
    Ok(session)
}
```

### User-Facing Errors

```rust
// Pretty error messages with context
impl Display for DxError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            DxError::SessionNotFound(key) => {
                write!(f, "Session '{}' not found. Use 'dx sessions list' to see available sessions.", key)
            }
            // ... more cases
        }
    }
}
```

---

## Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_session_create() {
        let manager = SessionManager::new_test().await;
        let session = manager.create("default").await.unwrap();
        assert_eq!(session.agent_id, "default");
    }
}
```

### Integration Tests

```rust
#[cfg(test)]
mod integration {
    #[tokio::test]
    async fn test_gateway_rpc_call() {
        let gateway = Gateway::new_test().await;
        let response = gateway
            .invoke("sessions.list", json!({}))
            .await
            .unwrap();
        assert!(response["sessions"].is_array());
    }
}
```

### End-to-End Tests

```bash
#!/bin/bash
# Test complete workflow
dx gateway start &
sleep 2
dx channel connect whatsapp
dx agent chat "Hello"
dx sessions list
dx gateway stop
```

---

## Deployment Architecture

### Development

```
Developer Machine
├── dx CLI (debug build)
├── Gateway (localhost:31337)
├── Channel bridges (Node.js)
└── SQLite database (~/.dx/dev.db)
```

### Production

```
User Machine
├── dx CLI (release build, installed via cargo/brew)
├── Gateway (daemon, systemd/launchd)
├── Channel bridges (managed by gateway)
├── SQLite database (~/.dx/dx.db)
└── Logs (~/.dx/logs/)
```

### Cloud Deployment (Optional)

```
Cloud Instance (AWS/GCP/Azure)
├── Gateway (docker container)
├── PostgreSQL (managed service)
├── Redis (session cache)
└── S3/GCS (file storage)
```

---

## Monitoring & Observability

### Logging

```rust
use tracing::{info, warn, error, debug};

info!("Gateway started on port {}", port);
warn!("Channel {} reconnecting after failure", channel_id);
error!("Failed to load session {}: {}", key, err);
debug!("RPC call: {} with params {:?}", method, params);
```

### Metrics

```rust
use prometheus::{Counter, Histogram};

lazy_static! {
    static ref RPC_CALLS: Counter = Counter::new("rpc_calls_total", "Total RPC calls").unwrap();
    static ref RPC_DURATION: Histogram = Histogram::new("rpc_duration_seconds", "RPC duration").unwrap();
}
```

### Tracing (Distributed)

```rust
use opentelemetry::trace::Tracer;

let span = tracer.start("session.load");
// ... do work ...
span.end();
```

---

## Conclusion

This design document provides a comprehensive architecture for integrating all OpenClaw features into the DX Rust CLI while maintaining safety, performance, and maintainability. The modular design allows for incremental implementation and testing, ensuring high quality at each phase.

**Key Takeaways:**
1. **Safety**: Rust's type system + WASM sandbox
2. **Performance**: Async I/O, zero-copy, caching
3. **Modularity**: Trait-based, plugin architecture
4. **Interoperability**: Node.js bridge, JSON-RPC
5. **Observability**: Structured logging, metrics, tracing

**Next Steps:**
1. Complete Phase 1 (Core Infrastructure)
2. Incremental rollout of Phase 2-5
3. Continuous testing and benchmarking
4. Community feedback and iteration
