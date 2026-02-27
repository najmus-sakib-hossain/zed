# GPUI Framework Guide - Complete Reference

Based on Zed codebase analysis (February 2026)

## Table of Contents
1. [What is GPUI?](#what-is-gpui)
2. [Core Concepts](#core-concepts)
3. [Styling System](#styling-system)
4. [Component Architecture](#component-architecture)
5. [Layout System](#layout-system)
6. [UI Component Locations](#ui-component-locations)
7. [Common Patterns](#common-patterns)

---

## What is GPUI?

GPUI is a **hybrid immediate and retained mode, GPU-accelerated UI framework** for Rust, designed specifically for building the Zed code editor. It provides:

- **State management** with Entities
- **Declarative UI** with Views
- **Low-level control** with Elements
- **Tailwind-style API** for styling
- **Cross-platform support** (macOS, Linux, Windows)

### Key Philosophy
- Elements are the building blocks
- Views are entities that can be rendered
- Styling uses method chaining (fluent API)
- Layout properties apply to **container elements** (`div()`), not components

---

## Core Concepts

### 1. Elements vs Components

**Elements** (Low-level):
- `div()` - The swiss-army knife container
- `h_flex()` - Horizontal flex container
- `v_flex()` - Vertical flex container
- Direct control over rendering and layout

**Components** (High-level):
- `Label` - Text display
- `Button` - Interactive button
- `Icon` - Icon display
- Built on top of elements

### 2. The `div()` Element

The fundamental building block. All layout and styling methods apply here:

```rust
div()
    .flex()              // Enable flexbox
    .flex_col()          // Column direction
    .gap_2()             // Gap between children
    .p_4()               // Padding
    .bg(color)           // Background color
    .child(Label::new("Hello"))
```

### 3. Contexts

GPUI uses different contexts for different operations:
- `App` - Application-level context
- `Context<T>` - Entity-specific context
- `Window` - Window-specific operations

---

## Styling System

### Location
- **Base**: `crates/ui/src/styles/`
- **Color**: `crates/ui/src/styles/color.rs`
- **Spacing**: `crates/ui/src/styles/spacing.rs`
- **Typography**: `crates/ui/src/styles/typography.rs`

### Styling Rules

#### ✅ CORRECT: Styling on `div()`
```rust
div()
    .flex()
    .min_w_0()
    .overflow_x_hidden()
    .flex_shrink()
    .child(Label::new("Text").truncate())
```

#### ❌ WRONG: Styling on Components
```rust
// This will NOT work!
Label::new("Text")
    .overflow_x_hidden()  // ❌ No such method
    .flex_shrink()        // ❌ No such method
```

### Common Styling Methods

#### Layout (on `div()`)
```rust
.flex()                  // Enable flexbox
.flex_col()              // Column direction
.flex_row()              // Row direction (default)
.flex_shrink()           // Allow shrinking
.flex_grow()             // Allow growing
.min_w_0()               // Minimum width 0 (enables truncation)
.max_w_full()            // Maximum width 100%
.w_full()                // Width 100%
.h_full()                // Height 100%
```

#### Spacing
```rust
.gap_0p5()               // ~2px gap
.gap_1()                 // ~4px gap
.gap_1p5()               // ~6px gap
.gap_2()                 // ~8px gap
.p_1()                   // Padding 1 unit
.px_2()                  // Horizontal padding
.py_2()                  // Vertical padding
.m_1()                   // Margin 1 unit
```

#### Overflow
```rust
.overflow_hidden()       // Hide overflow
.overflow_x_hidden()     // Hide horizontal overflow
.overflow_y_hidden()     // Hide vertical overflow
.overflow_scroll()       // Enable scrolling
```

#### Colors
```rust
.bg(cx.theme().colors().background)
.text_color(Color::Accent.color(cx))
.border_color(cx.theme().colors().border)
```

### Semantic Colors

The `Color` enum provides theme-aware colors:

```rust
Color::Default          // Default text color
Color::Muted            // De-emphasized text
Color::Accent           // Accent/highlight color
Color::Error            // Error state
Color::Warning          // Warning state
Color::Success          // Success state
Color::Disabled         // Disabled state
Color::Placeholder      // Placeholder text
```

Usage:
```rust
Label::new("Text").color(Color::Accent)
```

### Dynamic Spacing

GPUI supports UI density settings (Compact, Default, Comfortable):

```rust
use crate::DynamicSpacing;

// Automatically adjusts based on UI density setting
div().gap(DynamicSpacing::Base16)
```

---

## Component Architecture

### Component Locations

#### Core UI Components
**Location**: `crates/ui/src/components/`

```
components/
├── label/              # Text components
│   ├── label.rs       # Basic label
│   ├── highlighted_label.rs
│   └── loading_label.rs
├── button/            # Button components
│   ├── button.rs
│   ├── icon_button.rs
│   └── toggle_button.rs
├── icon/              # Icon system
├── list/              # List components
│   ├── list_item.rs
│   └── list_header.rs
├── modal.rs           # Modal dialogs
├── popover.rs         # Popovers
├── tooltip.rs         # Tooltips
├── context_menu.rs    # Context menus
└── ...
```

### Label Component

**Location**: `crates/ui/src/components/label/label.rs`

```rust
// Basic usage
Label::new("Hello, World!")

// With styling
Label::new("Important")
    .size(LabelSize::Small)
    .color(Color::Accent)
    .weight(FontWeight::BOLD)
    .truncate()           // ✅ Truncate method exists on Label
    .truncate_start()     // Truncate from start
    .strikethrough()
    .italic()
    .underline()
```

**Key Methods**:
- `.truncate()` - Truncate with ellipsis
- `.truncate_start()` - Truncate from beginning
- `.single_line()` - Force single line
- `.color(Color)` - Set text color
- `.size(LabelSize)` - Set size
- `.weight(FontWeight)` - Set font weight

### Button Component

**Location**: `crates/ui/src/components/button/`

```rust
Button::new("click_me", "Click Me")
    .style(ButtonStyle::Filled)
    .size(ButtonSize::Default)
    .on_click(|event, window, cx| {
        // Handle click
    })
```

### Icon Component

**Location**: `crates/ui/src/components/icon/`

```rust
Icon::new(IconName::Check)
    .size(IconSize::Small)
    .color(Color::Accent)
```

---

## Layout System

### Flex Containers

#### Horizontal Flex (`h_flex()`)
```rust
h_flex()
    .gap_2()
    .child(Icon::new(IconName::File))
    .child(Label::new("File.txt"))
```

#### Vertical Flex (`v_flex()`)
```rust
v_flex()
    .gap_1()
    .child(Label::new("Title"))
    .child(Label::new("Subtitle"))
```

### Group Helpers

**Location**: `crates/ui/src/components/group.rs`

Pre-configured flex containers with consistent spacing:

```rust
h_group_sm()    // Horizontal, ~2px gap
h_group()       // Horizontal, ~4px gap
h_group_lg()    // Horizontal, ~6px gap
h_group_xl()    // Horizontal, ~8px gap

v_group_sm()    // Vertical, ~2px gap
v_group()       // Vertical, ~4px gap
v_group_lg()    // Vertical, ~6px gap
v_group_xl()    // Vertical, ~8px gap
```

### Responsive Layout Pattern

For elements that need to shrink/truncate:

```rust
h_flex()
    .gap_1()
    .child(Icon::new(IconName::File))
    .child(
        div()
            .flex_shrink()      // Allow container to shrink
            .min_w_0()          // Enable truncation
            .overflow_x_hidden() // Hide overflow
            .child(
                Label::new("Very long filename.txt")
                    .truncate()  // Truncate text
            )
    )
    .child(Icon::new(IconName::ChevronDown))
```

**Key Pattern**: 
1. Wrap the shrinkable content in a `div()`
2. Apply `.flex_shrink()`, `.min_w_0()`, `.overflow_x_hidden()` to the `div()`
3. Apply `.truncate()` to the `Label`

---

## UI Component Locations

### Workspace Structure

**Location**: `crates/workspace/src/workspace.rs`

The main application window structure:

```rust
pub struct Workspace {
    center: PaneGroup,           // Center editor area
    left_dock: Entity<Dock>,     // Left sidebar
    bottom_dock: Entity<Dock>,   // Bottom panel
    right_dock: Entity<Dock>,    // Right sidebar
    status_bar: Entity<StatusBar>,
    modal_layer: Entity<ModalLayer>,
    // ...
}
```

### Sidebar

**Location**: `crates/sidebar/src/sidebar.rs`

The workspace sidebar (recent projects, threads):

```rust
pub struct Sidebar {
    // Workspace picker and thread list
}
```

### Project Panel (File Browser)

**Location**: `crates/project_panel/src/project_panel.rs`

The file explorer/tree view:

```rust
pub struct ProjectPanel {
    // File tree rendering
}
```

**Key rendering method**: `render_entry()` - Renders individual files/folders

### Agent UI

**Location**: `crates/agent_ui/src/`

AI agent interface components:

```
agent_ui/src/
├── agent_panel.rs              # Main agent panel
├── text_thread_editor.rs       # Chat interface
├── language_model_selector.rs  # Model picker
├── ui/
│   └── model_selector_components.rs  # Model selector UI
└── acp/                        # Agent context protocol
```

### Title Bar

**Location**: `crates/title_bar/src/title_bar.rs`

Window title bar with tabs and controls.

### Status Bar

**Location**: `crates/workspace/src/` (part of workspace)

Bottom status bar showing file info, diagnostics, etc.

---

## Common Patterns

### Pattern 1: Responsive Model Selector

**Problem**: Show model name when space available, hide when constrained.

**Solution**:
```rust
h_flex()
    .gap_0p5()
    .min_w_0()
    .child(provider_icon)
    .child(
        div()
            .flex_shrink()
            .min_w_0()
            .overflow_x_hidden()
            .child(
                Label::new(model_name)
                    .truncate()
            )
    )
    .child(chevron_icon)
```

### Pattern 2: File Tree Entry

**Problem**: Long file/folder names overflow container.

**Solution**:
```rust
div()
    .flex_shrink()          // Allow shrinking
    .min_w_0()              // Enable truncation
    .overflow_x_hidden()    // Hide overflow
    .child(
        Label::new(filename)
            .truncate()
    )
```

### Pattern 3: List Item with Icon and Text

```rust
ListItem::new(index)
    .child(
        h_flex()
            .gap_1p5()
            .child(Icon::new(icon).size(IconSize::Small))
            .child(Label::new(text).truncate())
    )
```

### Pattern 4: Conditional Styling

```rust
div()
    .when(is_active, |div| {
        div.bg(cx.theme().colors().element_active)
    })
    .when_some(optional_value, |div, value| {
        div.child(Label::new(value))
    })
```

### Pattern 5: Interactive Element

```rust
div()
    .id("clickable-item")
    .cursor_pointer()
    .hover(|style| {
        style.bg(cx.theme().colors().element_hover)
    })
    .on_click(cx.listener(|this, event, window, cx| {
        // Handle click
    }))
    .child(Label::new("Click me"))
```

---

## Key Takeaways

1. **Layout properties go on `div()`, not components**
   - ✅ `div().flex_shrink().child(Label::new("text"))`
   - ❌ `Label::new("text").flex_shrink()`

2. **Truncation requires both container and component setup**
   - Container: `.flex_shrink()`, `.min_w_0()`, `.overflow_x_hidden()`
   - Component: `.truncate()`

3. **Use semantic colors from the `Color` enum**
   - Theme-aware and consistent across the app

4. **Flex is the primary layout system**
   - `h_flex()` for horizontal
   - `v_flex()` for vertical
   - Use gap methods for spacing

5. **Components are in `crates/ui/src/components/`**
   - Well-organized by functionality
   - Each has its own module

6. **Workspace structure is in `crates/workspace/`**
   - Main window layout
   - Dock system (left, right, bottom)
   - Pane management

7. **File browser is `crates/project_panel/`**
   - Tree view rendering
   - File/folder entry display

8. **Sidebar is `crates/sidebar/`**
   - Workspace picker
   - Thread list

---

## Additional Resources

- **GPUI README**: `crates/gpui/README.md`
- **Official Site**: https://www.gpui.rs/
- **UI Prelude**: `crates/ui/src/prelude.rs` - Common imports
- **Storybook**: `crates/storybook/` - Component examples
- **Zed Discord**: https://zed.dev/community-links

---

*This guide is based on the actual Zed codebase as of February 2026.*
