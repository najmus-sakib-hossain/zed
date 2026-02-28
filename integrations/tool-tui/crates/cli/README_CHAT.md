# DX AI Chat TUI

Professional AI chat interface with Vercel-inspired design system.

## Features

- **3 Interaction Modes**
  - 🚀 Agent: Execute tasks autonomously
  - 📋 Plan: Create execution plan first
  - 💬 Ask: Ask questions and get answers

- **5 Theme Variants**
  - Vercel (default black/white)
  - Dark (blue-gray)
  - Light (white/gray)
  - Ocean (deep blue)
  - Sunset (warm tones)

- **Professional Effects**
  - Shimmer loading animation
  - Typing indicator with dots
  - Smooth color transitions
  - Pulse effects

- **Keyboard Shortcuts**
  - `Tab` - Switch focus (Mode/Input)
  - `←/→` - Navigate modes (when focused)
  - `Alt+1/2/3` - Quick mode switch
  - `Alt+T` - Cycle themes
  - `Alt+L` - Toggle loading demo
  - `Ctrl+C` - Exit
  - `Ctrl+A/E` - Jump to start/end
  - `Ctrl+U/K` - Delete to start/end
  - `Ctrl+W` - Delete word
  - `Enter` - Send message
  - `Shift+Enter` - New line

## Usage

```bash
# Start chat with default settings
dx chat

# Start in specific mode
dx chat --mode agent
dx chat --mode plan
dx chat --mode ask

# Use specific theme
dx chat --theme vercel
dx chat --theme dark
dx chat --theme ocean

# Combined
dx chat --mode plan --theme sunset
```

## Architecture

- `app.rs` - Main application loop and state
- `theme.rs` - Color schemes and styling
- `modes.rs` - Chat mode definitions
- `input.rs` - Text input handling
- `effects.rs` - Shimmer, typing, pulse effects
- `components.rs` - UI widgets (messages, mode selector, input box)
