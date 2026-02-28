
# dx-www-db

Zero-copy database layer with compile-time SQL verification.

## Overview

This crate provides database connectivity for dx-www applications with support for PostgreSQL and zero-copy operations.

## Installation

```toml
[dependencies]
dx-www-db = { path = "../db" }
```

## Features

- `postgres` (default)
- PostgreSQL support with connection pooling
- `std`
- Standard library support

## License

MIT OR Apache-2.0
