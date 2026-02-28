# DX Chat UI Requirements

## Recent Fixes

### Double Character Input Bug (FIXED)

**Problem**: When typing a single character (e.g., "a"), it would appear twice ("aa") in the input box.

**Root Cause**: On Windows, crossterm fires both `KeyPress` and `KeyRelease` events for each keystroke. The application was processing both events, causing characters to be inserted twice.

**Solution**: Added event kind filtering in `app.rs` to only process `KeyEventKind::Press` events:
```rust
if key.kind == crossterm::event::KeyEventKind::Press {
    self.handle_key(key);
}
```

### Cursor Rendering Issue (FIXED)

**Problem**: Cursor was replacing characters at cursor position with '▎' symbol.

**Solution**: Modified cursor rendering to preserve existing characters:
- Empty space: Show '▎' bar in accent color
- On character: Invert colors (accent background, dark foreground)

## Current Implementation Status

### ✅ Completed Features

- Vercel-inspired design system with clean colors
- Professional CLI symbols (no emojis)
- Combined input box and first bottom bar in single rounded border
- Second bottom bar with status information
- Rotating keyboard shortcuts (6 shortcuts, 3-second intervals)
- Custom blinking cursor (only visible during interaction)
- Cursor disappears after 3 seconds of inactivity
- Text selection with Ctrl+A and Shift+Arrow keys
- Visual selection rendering (white background, black text)
- History navigation with Up/Down arrow keys
- History storage for submitted prompts
- Model selector on left side (Add, Plan, Gemini 3 Pro)
- Audio mode toggle with sound effects
- Theme cycling
- Proper padding and spacing (2 chars horizontal, minimal vertical)
- No gap between input and first bottom bar
- Minimal gap between first box and second bottom bar

## Design System

- **Style**: Vercel-inspired design system
- **Colors**: Professional, minimal, clean
- **Symbols**: NO EMOJIS - use professional CLI symbols only (▸, ◆, ◉, ›, etc.)
- **Theme**: Dark background with subtle borders

## Layout Structure

### Main Chat Area

- Messages display area (scrollable)
- Loading indicator when AI is responding
- Clean message bubbles with timestamps

### Input Area

- Single-line input with border on top only
- Placeholder: "Type a prompt... (Enter to send, Shift+Enter for new line)"
- Cursor visible when typing
- NO double character input bug

### Bottom Bar 1 (Main Actions)

- **Left side**: Model selector (e.g., "Gemini 3 Pro")
- **Left-center**: Action buttons ("Add", "Plan")
- **Center**: Empty/flexible space
- **Right side**: Action buttons ("Audio", "Local", "Send")
- Audio button highlights when active
- Send button in accent color

### Bottom Bar 2 (Secondary Info)

- **Left side**: Status items ("Changes", "Tasks", "Agents")
- **Center**: Rotating keyboard shortcuts display (cycles through all shortcuts)
- **Right side**: Info items ("Temporary", "Tokens", "Working...")
- NO horizontal padding - full width usage
- Proper spacing between items

## Keyboard Shortcuts

### Global Shortcuts

- `Ctrl+C`: Exit application
- `Ctrl+M`: Toggle bottom menu visibility
- `Alt+A`: Toggle audio mode
- `Alt+T`: Cycle through themes
- `Alt+L`: Toggle loading indicator (debug)

### Mode Switching

- `Alt+1`: Switch to Agent mode
- `Alt+2`: Switch to Plan mode
- `Alt+3`: Switch to Ask mode

### Input Controls

- `Enter`: Send message (when not empty)
- `Shift+Enter`: New line in message
- `Ctrl+A`: Select all text
- `Shift+Left/Right`: Extend selection
- `Up/Down`: Navigate prompt history
- `Ctrl+E`: Move cursor to end
- `Ctrl+U`: Clear line before cursor
- `Ctrl+K`: Clear line after cursor
- `Ctrl+W`: Delete word before cursor
- `Ctrl+D`: Exit (when input empty)
- `Home/End`: Move to start/end of line
- `Backspace/Delete`: Delete characters (or selection)

### Navigation

- `Tab`: Switch focus between mode selector and input
- `Left/Right`: Navigate modes (when mode selector focused)

## Keyboard Shortcuts Display

- Display shortcuts in rotating loop in bottom bar 2 center
- Show one shortcut at a time
- Cycle through all shortcuts automatically
- Format: "Ctrl+M: Toggle Menu | Alt+A: Audio" (example)

## Audio Features

- Audio mode toggle with visual indicator
- Sound effects for interactions:
  - Click sound for button presses
  - Send sound for message submission
  - Mode switch sound for mode changes
- Audio visualizer when recording (future feature)
- Sounds loaded from `media/audio/` folder

## Input Handling Rules

- CRITICAL: Only process `KeyEventKind::Press` events (ignore Release/Repeat)
- Only accept character input with NONE or SHIFT modifiers
- Block ALT and CTRL character combinations from input
- Handle global shortcuts BEFORE passing to input handler
- Selection is cleared on typing or navigation (except Shift+Arrow)
- History navigation updates input content and cursor position

## Color Scheme (Vercel Theme)

- Background: `#000000` (pure black)
- Foreground: `#FFFFFF` (white)
- Border: `#333333` (dark gray)
- Border Focused: `#FFFFFF` (white)
- Accent: `#0070F3` (blue)
- Accent Secondary: `#FF0080` (pink)
- User Message BG: `#1A1A1A`
- AI Message BG: `#121212`

## Mode Colors

- Agent: `#00FF87` (green)
- Plan: `#FFB800` (yellow)
- Ask: `#0070F3` (blue)

## Professional Symbols Used

- `▸` - Agent mode, navigation
- `◆` - Plan mode
- `◉` - Ask mode, loading
- `›` - Message prefix, input indicator
- `│` - Separator
- `─` - Horizontal line

## Technical Notes

- Use `ratatui` for TUI rendering
- Use `crossterm` for terminal control and event handling
- Use `rodio` for audio playback
- Implement proper focus management
- Handle terminal resize gracefully
- Use efficient rendering (only redraw on change)
- Filter key events to only process `KeyEventKind::Press`
- Store prompt history in `Vec<String>` with `Option<usize>` index
- Cursor blinks every 500ms, disappears after 3 seconds of inactivity
- Selection rendering: iterate through characters and apply inverted style

## File Structure

- `app.rs` - Main application loop, event handling, state management
- `components.rs` - UI widgets (MessageList, InputBox, CombinedInputBar, etc.)
- `input.rs` - Input state and key handling logic
- `theme.rs` - Color schemes and theme variants
- `modes.rs` - Chat mode definitions (Agent, Plan, Ask)
- `effects.rs` - Visual effects (ShimmerEffect, TypingIndicator)
