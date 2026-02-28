# GPUI Dx Desktop Apps

Dx Desktop app using GPUI from the Zed team.

## Building

The first build will take 10-30 minutes as it downloads and compiles the entire Zed repository.

```bash
cd crates/gpui-hello
cargo build --release
```

## Running

```bash
cargo run --release
```

## Windows Support Note

GPUI's Windows support is still in active development. If you encounter build errors, you may need to:

1. Ensure you have the latest Rust toolchain: `rustup update`
2. Install Visual Studio Build Tools with C++ development tools
3. Check the Zed repository for Windows-specific requirements: https://github.com/zed-industries/zed

## What it does

Creates a window with:
- Green background (#2e7d32)
- White centered text saying "Hello, world!"
- Uses GPUI's Tailwind-like styling API

## Code Structure

- `main.rs` - Contains the HelloWorld struct and app initialization
- Uses GPUI's `Render` trait for declarative UI
- Flexbox layout with centered content
