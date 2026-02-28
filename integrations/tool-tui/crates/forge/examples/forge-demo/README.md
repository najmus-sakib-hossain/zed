
# Forge Demo Repository

This is a demonstration repository for the Forge version control system.

## Features Demonstrated

- Binary Blob Storage
- All files stored as compressed blobs
- Cloudflare R2 Integration
- Zero-egress cloud storage
- Traffic Branches
- Red/Yellow/Green deployment branches
- LSP Detection
- Smart change detection via Language Server Protocol
- CRDT Sync
- Conflict-free replicated data types for real-time collaboration
- Platform-Native I/O
- io_uring (Linux), kqueue (macOS), IOCP (Windows)

## Files in This Demo

- `src/main.rs`
- Simple Rust application
- `src/lib.rs`
- Library code
- `Cargo.toml`
- Project manifest
- `README.md`
- This file

## Usage

This repository is initialized with Forge, not Git. All version control operations are handled by Forge and stored in Cloudflare R2.
```bash


# View commit history


forge log


# Create a new traffic branch


forge branch --traffic green


# View storage stats


forge stats ```


## Storage Backend


All blobs are stored in Cloudflare R2 with: -LZ4 compression (10-50x faster than gzip) -SHA-256 content addressing -Zero egress fees -99.999999999% durability


## Platform-Native I/O


+----------+---------+----------+
| Platform | Backend | Features |
+==========+=========+==========+
| Linux    | 5.1+    | io       |
+----------+---------+----------+
```bash

# Check which backend is active

forge info --backend ```
