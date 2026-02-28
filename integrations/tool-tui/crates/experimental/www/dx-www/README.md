# DX WWW Framework

Binary-first, multi-language web framework with file-system routing.

## Overview

DX WWW is a high-performance web framework that compiles `.pg` (page) and `.cp` (component) files to `.dxob` binary format for zero-parse performance. It supports multiple programming languages (Rust, Python, JavaScript, Go) in component scripts and integrates with dx-style for atomic CSS compilation.

## Features

- **File-System Routing**: Create routes by adding files to `pages/` directory
- **Multi-Language Support**: Write component logic in Rust, Python, JavaScript, or Go
- **Binary Compilation**: Zero-parse `.dxob` binary format for production
- **Hot Reload**: Instant updates during development without full page refresh
- **Layout System**: Nested layouts with automatic chain composition
- **API Routes**: Server-side endpoints in `api/` directory
- **Data Loaders**: Fetch data before page rendering

## Quick Start

```bash
# Create a new project
dx-www new my-app

# Start development server
cd my-app
dx-www dev

# Build for production
dx-www build
```

## Project Structure

```
my-app/
├── dx.config.toml          # Framework configuration
├── pages/                  # Routable pages (.pg files)
│   ├── index.pg           # Route: /
│   ├── about.pg           # Route: /about
│   └── _layout.pg         # Root layout
├── components/            # Reusable components (.cp files)
├── api/                   # Server-side API routes
├── public/                # Static assets
└── styles/                # Global styles
```

## Configuration

Create a `dx.config.toml` in your project root:

```toml
[project]
name = "my-app"
version = "0.1.0"

[build]
output_dir = ".dx/build"
optimization_level = "release"

[dev]
port = 3000
hot_reload = true
```

## File Formats

### Page File (.pg)

```html
<script lang="rust">
pub struct Props {
    title: String,
}

pub async fn load() -> Props {
    Props { title: "Hello".into() }
}
</script>

<template>
  <h1>{title}</h1>
</template>

<style>
h1 { color: blue; }
</style>
```

### Component File (.cp)

```html
<script lang="rust">
pub struct Props {
    label: String,
}
</script>

<template>
  <button class="btn">{label}</button>
</template>
```

## License

MIT OR Apache-2.0
