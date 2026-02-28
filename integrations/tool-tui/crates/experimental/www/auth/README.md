
# dx-www-auth

Binary authentication with Ed25519 tokens and passkey support.

## Overview

This crate provides secure authentication primitives for dx-www applications, including: -Ed25519 digital signatures for token-based auth -Argon2 password hashing -Optional WebAuthn/Passkey support

## Installation

```toml
[dependencies]
dx-www-auth = { path = "../auth" }
```

## Features

- `std` (default)
- Standard library support
- `passkey`
- WebAuthn/Passkey authentication

## License

MIT OR Apache-2.0
