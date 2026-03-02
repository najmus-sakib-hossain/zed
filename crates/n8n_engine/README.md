# n8n Engine Integration for Zed

This crate provides a comprehensive integration between Zed's Rust GPUI framework and a forked n8n workflow engine. It implements the "n8n Killer Blueprint" architecture.

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────────────┐
│           RUST GPUI ZED (Main Process)                              │
│                                                                     │
│  ┌────────────────┐  ┌──────────────────┐  ┌───────────────────┐   │
│  │ WorkflowPanel  │  │  AiWorkflowRouter│  │  HybridExecutor   │   │
│  │ (GPUI UI)      │  │  (Rust Routing)  │  │  (Rust + n8n)     │   │
│  └────────┬───────┘  └────────┬─────────┘  └─────────┬─────────┘   │
│           │                   │                      │             │
│           └───────────────────┼──────────────────────┘             │
│                               │                                    │
│                    ┌──────────▼──────────┐                         │
│                    │   N8nSidecar        │                         │
│                    │   (IPC Bridge)      │                         │
│                    └──────────┬──────────┘                         │
└───────────────────────────────┼────────────────────────────────────┘
                                │ IPC (Named Pipe / TCP)
┌───────────────────────────────▼────────────────────────────────────┐
│  n8n EXECUTION SIDECAR (Child Process - Node.js)                   │
│  - Custom IPC Bridge Server                                        │
│  - WorkflowRunner / WorkflowExecute engine                         │
│  - 500+ Node Integrations                                          │
└────────────────────────────────────────────────────────────────────┘
```

## Key Components

### 1. N8nSidecar (`sidecar.rs`)

Manages the n8n Node.js process as a child process and communicates via IPC:

```rust
use n8n_engine::{N8nSidecar, SidecarConfig};

let config = SidecarConfig::default();
let sidecar = N8nSidecar::spawn(config).await?;

let workflow = WorkflowBuilder::new("My Workflow")
    .add_trigger()
    .add_http_request("API Call", "https://api.example.com", "GET")
    .build();

let result = sidecar.execute_workflow(&workflow, json!({})).await?;
```

### 2. AiWorkflowRouter (`workflow_router.rs`)

Maps AI intents to workflow templates at nanosecond speed:

```rust
use n8n_engine::AiWorkflowRouter;

let router = AiWorkflowRouter::new()?;

let decision = json!({
    "intent": "send_slack",
    "parameters": {
        "channel": "#engineering",
        "message": "Hello!"
    }
});

let route = router.route_ai_decision(decision)?;
```

### 3. HybridWorkflowExecutor (`hybrid_executor.rs`)

Executes workflows using native Rust nodes when available, falling back to n8n:

```rust
use n8n_engine::HybridWorkflowExecutor;

let executor = HybridWorkflowExecutor::new(sidecar);

// This will use Rust nodes if all are implemented, otherwise n8n
let result = executor.execute(&workflow, json!({})).await?;

// Check execution stats
let stats = executor.get_stats();
println!("Rust execution rate: {:.1}%", executor.get_rust_percentage());
```

### 4. Native Rust Nodes (`rust_nodes.rs`)

Pure Rust implementations of n8n nodes for maximum performance:

- `RustHttpRequestNode` - HTTP requests via reqwest (~10-50ms vs ~100-500ms in n8n)
- `RustDataTransformNode` - Data transformation
- `RustFilterNode` - Conditional filtering
- `RustMergeNode` - Data merging
- `RustSplitBatchesNode` - Batch processing

### 5. WorkflowPanel (`workflow_panel.rs`)

GPUI-native UI for the workflow engine:

```rust
use n8n_engine::WorkflowPanel;

// In your workspace setup
let panel = cx.new(|cx| WorkflowPanel::new(cx));
panel.update(cx, |panel, cx| {
    panel.start_engine(cx);
});
```

## Protocol Types (`protocol.rs`)

### WorkflowDefinition

```rust
let workflow = WorkflowDefinition {
    id: None,
    name: "My Workflow".to_string(),
    nodes: vec![
        WorkflowNode::new("trigger", "Start", "n8n-nodes-base.manualTrigger", [240, 300]),
        WorkflowNode::new("http", "API", "n8n-nodes-base.httpRequest", [460, 300])
            .with_parameters(json!({"url": "https://api.example.com"})),
    ],
    connections: json!({...}),
    settings: None,
};
```

### WorkflowBuilder (Fluent API)

```rust
let workflow = WorkflowBuilder::new("Slack Notification")
    .add_trigger()
    .add_http_request("Fetch Data", "https://api.example.com", "GET")
    .add_slack_message("Notify", "#channel", "Data fetched!")
    .build();
```

## Setting Up the n8n Fork

1. Fork the n8n repository
2. Add the IPC engine entry point from `templates/ipc-engine.ts` to `packages/cli/src/`
3. Build the n8n fork: `npm run build`
4. Configure the sidecar:

```rust
let config = SidecarConfig {
    n8n_project_path: PathBuf::from("./path/to/n8n-fork"),
    ipc_path: "/tmp/n8n-engine.sock".to_string(), // Unix
    // ipc_path: "127.0.0.1:58765".to_string(),   // Windows
    db_type: "sqlite".to_string(),
    db_connection: "./n8n.db".to_string(),
    env_vars: HashMap::new(),
};
```

## Workflow Templates

Built-in AI intent templates:

| Intent | Description |
|--------|-------------|
| `send_slack` | Send Slack messages |
| `http_request` | Make HTTP requests |
| `send_email` | Send emails |
| `run_code` | Execute JavaScript/Python code |
| `query_database` | Run SQL queries |

Register custom templates:

```rust
router.register_template("my_intent", WorkflowTemplate {
    name: "My Custom Workflow".to_string(),
    builder: |params| {
        WorkflowBuilder::new("Custom")
            .add_trigger()
            // ... build workflow
            .build()
    },
    blocking: true,
    parameter_schema: json!({...}),
});
```

## Performance Profile

| Layer | Technology | Latency | Purpose |
|---|---|---|---|
| AI Classification | Pure Rust | ~10 ns | Classify user intent |
| Workflow Routing | Pure Rust | ~50–200 ns | Decide which workflow to run |
| Rust-Native Exec | Pure Rust (reqwest) | ~1–50 ms | HTTP, filtering, transforms |
| n8n Fallback Exec | IPC → n8n | ~5–50 ms | 500+ integrations |

## Migration Path

1. **Phase 1: Absorb** - Use n8n for all executions via sidecar
2. **Phase 2: Evolve** - Implement high-frequency nodes in Rust
3. **Phase 3: Replace** - Port most-used nodes to native Rust
4. **Phase 4: Surpass** - Full Rust workflow engine with WASM plugins

## License

GPL-3.0-or-later
