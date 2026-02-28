
# dx-www-server

The Holographic Server - SSR, Binary Streaming, and Delta Patching.

## Overview

This crate provides the server-side runtime for dx-www, including: -Server-side rendering (SSR) -Binary content streaming -Delta patching for efficient updates -WebSocket support for real-time features

## Installation

```toml
[dependencies]
dx-www-server = { path = "../server" }
```

## Features

- `query` (default)
- Data fetching support
- `auth` (default)
- Authentication support
- `fallback` (default)
- HTML fallback mode
- `db`
- Database integration
- `sync`
- Real-time sync
- `offline`
- Offline support
- `full`
- All features

## License

MIT OR Apache-2.0
