
# DCP - Development Context Protocol

A high-performance binary protocol designed to replace MCP (Model Context Protocol) with 10-1000x performance improvements while maintaining full backward compatibility.

## Why DCP?

+---------+--------+-----------+
| Metric  | MCP    | (JSON-RPC |
+=========+========+===========+
| Message | Header | ~100+     |
+---------+--------+-----------+



## Features

- Binary Message Envelope (BME)
- 8-byte fixed header with O(1) parsing via pointer casting
- Zero-Copy Tool Invocation
- Direct memory access without serialization overhead
- O(1) Binary Trie Router
- Constant-time tool dispatch by ID
- Lock-Free Streaming
- Ring buffer with backpressure signaling
- XOR Delta Sync
- Efficient state synchronization with run-length encoding
- Ed25519 Security
- Signed tool definitions and replay protection
- Full MCP Compatibility
- JSON-RPC adapter for seamless migration

## Architecture

+-----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------+
| ┌─────────────────────────────────────────────────────────────────────────┐                                                                                                                                                       |
+===================================================================================================================================================================================================================================+
| │DCP                                                                                                                                                                                                                              |
+-----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------+



## Installation

Add to your `Cargo.toml`:
```toml
[dependencies]
dcp = "0.1.0"
```
Or build from source:
```bash
git clone https://github.com/your-org/dcp.git cd dcp cargo build --release ```


## Quick Start



### As a Library


```rust
use dcp::{ DcpServer, ServerConfig, McpAdapter, BinaryMessageEnvelope, MessageType, Flags, };
// Create a DCP server let config = ServerConfig::default();
let server = DcpServer::new(config);
// Handle MCP requests (backward compatible)
let adapter = McpAdapter::new();
let response = adapter.handle_request(json_rpc_request)?;
// Or use native binary protocol let envelope = BinaryMessageEnvelope::new( MessageType::Tool, Flags::empty(), payload.len() as u32, );
```


### CLI Usage


```bash

# Start DCP server

dcp serve --port 8080

# Convert MCP schema to DCP

dcp convert --input mcp-schema.json --output dcp-schema.bin

# Show protocol info

dcp info

# Validate a DCP message

dcp validate message.bin ```

## Core Components

### Binary Message Envelope

8-byte header for all DCP messages:
```rust


#[repr(C, packed)]


pub struct BinaryMessageEnvelope { pub magic: u16, // 0xDC01 for DCP v1 pub message_type: u8, // Tool, Resource, Prompt, Response, Error, Stream pub flags: u8, // streaming, compressed, signed pub payload_len: u32, // Payload length in bytes }
```

### Tool Invocation

Zero-copy tool calls:
```rust


#[repr(C)]


pub struct ToolInvocation { pub tool_id: u32, // Pre-resolved tool ID pub arg_layout: u64, // Argument type bitfield pub args_offset: u32, // Offset in shared memory pub args_len: u32, // Argument length }
```

### Capability Manifest

Bitset-based capability negotiation:
```rust
let manifest = CapabilityManifest::new();
manifest.set_tool(42, true);
manifest.set_resource(7, true);
// O(1) capability intersection let common = client_manifest.intersect(&server_manifest);
```

## MCP Migration Guide

DCP provides seamless migration from MCP: -Drop-in Adapter: Use `McpAdapter` to handle JSON-RPC requests -Hybrid Mode: Run both protocols simultaneously during transition -Schema Conversion: Use `dcp convert` to migrate tool schemas -Session Preservation: Upgrade connections without losing state ```rust // Existing MCP handler let mcp_request = r#"{"jsonrpc":"2.0","method":"tools/call","params":{"name":"read_file"},"id":1}"#;
// Works with DCP adapter let adapter = McpAdapter::new();
let response = adapter.handle_request(mcp_request)?;
```


## Testing


DCP includes comprehensive testing with 263 tests:
```bash

# Run all tests

cargo test --release

# Run property-based tests only

cargo test props --release

# Run with verbose output

cargo test --release -- --nocapture ```
Test coverage includes: -150 unit tests for specific functionality -113 property-based tests using `proptest` -16 correctness properties validated

## Performance

Benchmarks comparing DCP vs MCP JSON-RPC:
```bash
cargo bench ```
Key metrics: -Message size: 6-12x smaller than JSON-RPC -Parsing: O(1) pointer cast vs O(n) JSON parsing -Memory: Zero allocations for message handling -Dispatch: O(1) tool routing by ID


## Security


- Ed25519 Signatures: Tool definitions and invocations can be cryptographically signed
- Replay Protection: Nonce-based protection with timestamp expiration
- Capability Manifests: Fine-grained permission control via bitsets


## Project Structure


@tree:src[]


## License


MIT License - see LICENSE (LICENSE) for details.


## Contributing


Contributions welcome! Please read our contributing guidelines and submit PRs. -Fork the repository -Create a feature branch -Add tests for new functionality -Ensure all tests pass: `cargo test --release` -Run clippy: `cargo clippy` -Submit a pull request
