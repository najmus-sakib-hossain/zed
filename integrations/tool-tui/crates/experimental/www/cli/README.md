# DX WWW CLI

Binary-first web framework CLI tool for creating and managing DX WWW projects.

## Installation

### From Source

```bash
# Clone the repository
git clone https://github.com/dx-www/dx-www-runtime
cd dx-www-runtime

# Build and install
cargo install --path crates/www-cli

# Or use the install script
./install-www-cli.sh  # Linux/macOS
install-www-cli.bat   # Windows
```

### From Cargo

```bash
cargo install dx-www-cli
```

## Usage

### Create a New Project

```bash
dx-www new my-app
cd my-app
```

### Start Development Server

```bash
dx-www dev
# Server runs at http://localhost:3000

# Custom port
dx-www dev --port 8080

# Disable hot reload
dx-www dev --no-hot-reload
```

### Build for Production

```bash
dx-www build

# Custom output directory
dx-www build --output dist

# Optimization levels: debug, release, size
dx-www build --optimization size
```

### Generate Files

```bash
# Generate a page
dx-www generate page about
dx-www g p about  # Short form

# Generate a component
dx-www generate component Button
dx-www g c Button

# Generate an API route
dx-www generate api users
dx-www g a users

# Generate a layout
dx-www generate layout dashboard
dx-www g l dashboard
```

### Preview Production Build

```bash
dx-www preview
# Runs at http://localhost:4173

# Custom port
dx-www preview --port 8080
```

### Clean Build Artifacts

```bash
# Clean build directory
dx-www clean

# Also clean cache
dx-www clean --cache
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

Create `dx.config.toml` in your project root:

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

## Features

- **File-System Routing**: Create routes by adding files to `pages/`
- **Multi-Language Support**: Write component logic in Rust, Python, JavaScript, or Go
- **Binary Compilation**: Zero-parse `.dxob` binary format for production
- **Hot Reload**: Instant updates during development
- **Layout System**: Nested layouts with automatic chain composition
- **API Routes**: Server-side endpoints in `api/` directory

## Binary Size

The CLI binary is optimized for size:
- Release build: ~8-12 MB (stripped)
- Includes all framework features
- No runtime dependencies

## License

MIT OR Apache-2.0
