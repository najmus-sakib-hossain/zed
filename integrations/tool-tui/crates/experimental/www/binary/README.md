
# dx-www-binary

The binary protocol that killed JSON and HTML.

## Overview

This crate implements the HTIP (Hypertext Transfer in Binary Protocol) for dx-www, providing zero-copy binary serialization for web content delivery.

## Installation

```toml
[dependencies]
dx-www-binary = { path = "../binary" }
```

## Features

- `client` (default)
- Client-side WASM runtime
- `server`
- Server-side binary generation

## License

MIT OR Apache-2.0
