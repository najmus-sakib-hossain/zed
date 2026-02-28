
# dx-www-state

Binary state management with memory slots and dirty tracking.

## Overview

This crate provides efficient state management for dx-www applications using zero-copy memory slots with automatic dirty tracking for minimal re-renders.

## Installation

```toml
[dependencies]
dx-www-state = { path = "../state" }
```

## Features

- `std` (default)
- Standard library support with parking_lot mutexes

## License

MIT OR Apache-2.0
