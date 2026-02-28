# DX GPUI WebView Demo with Glow Card

Desktop application built with Zed's GPUI framework demonstrating:
- WebView integration for browsing
- Animated gradient glow card UI component
- Toggle between views with spacebar or button

## Features

### Glow Card Demo
Beautiful animated card with:
- Multi-color gradient border (purple → cyan → pink → orange)
- Floating action buttons with hover effects
- SVG icon rendering
- Smooth transitions

### WebView Integration
- Embedded web browser (Google.com by default)
- Custom titlebar
- Cross-platform support (Windows, macOS, Linux)

## Controls

- **Spacebar**: Toggle between Glow Card and WebView
- **Button**: Click the floating button in bottom-right corner to toggle

## Run

From repo root:

```bash
cargo run --manifest-path apps/webview/Cargo.toml
```

## Tutorial

Visit `/gpui-tutorial` in the www app to learn how to build desktop apps with GPUI.

## Architecture

- `main.rs` - Application entry point and window management
- `glow_card.rs` - Animated gradient card component
- `webview.rs` - WebView integration
- `titlebar.rs` - Custom titlebar component

Created: February 20, 2026

