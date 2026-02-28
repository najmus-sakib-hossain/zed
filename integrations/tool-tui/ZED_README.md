# DX UI - GPUI Component Library

High-performance UI component library for GPUI applications, extracted and adapted from Zed's UI system.

## Overview

This crate provides 40+ reusable UI components built on GPUI, with simplified dependencies and no Zed-specific requirements. Perfect for building native desktop applications with Rust.

## Architecture

```
dx-ui/              # Main UI component library
├── components/     # 40+ reusable components
├── styles/         # Styling utilities
├── traits/         # Component traits
├── theme.rs        # Simplified theme system
└── settings.rs     # Minimal settings adapter

dx-ui-component/    # Base component traits
dx-ui-icons/        # Icon system (IconName enum)
dx-ui-macros/       # Proc macros for components
```

## Key Differences from Zed's UI

### Removed Dependencies
- ❌ `settings` - Zed's file-watching settings system
- ❌ `theme` - Zed's complex theme infrastructure  
- ❌ `util` - 40+ utility dependencies
- ❌ `menu` - Zed's menu system (replaced with simplified version)

### Added Adapters
- ✅ `theme.rs` - Minimal theme trait with `DxTheme`
- ✅ `settings.rs` - Simple settings interface
- ✅ `util.rs` - Only essential utilities

## Available Components

### Basic Components
- `Button` - Button with label and optional icon
- `IconButton` - Icon-only button
- `Label` - Text labels
- `Icon` - Icon rendering
- `Avatar` - User avatars
- `Chip` - Tag/chip components
- `Indicator` - Status indicators
- `Divider` - Visual separators

### Interactive Components
- `Toggle` - Toggle switches
- `Radio` - Radio buttons
- `Disclosure` - Collapsible sections
- `Keybinding` - Keyboard shortcut display

### Layout Components
- `Stack` - Stack layouts
- `Group` - Grouped elements
- `ContentGroup` - Content grouping
- `Tab` / `TabBar` - Tab navigation
- `TreeViewItem` - Tree view items

### Overlays
- `Modal` - Modal dialogs
- `Popover` - Popover menus
- `ContextMenu` - Context menus
- `DropdownMenu` - Dropdown menus
- `Tooltip` - Tooltips
- `Banner` - Banners
- `Notification` - Notifications

### Data Display
- `List` - List components
- `DataTable` - Data tables
- `Scrollbar` - Custom scrollbars
- `Progress` - Progress indicators
- `DiffStat` - Git diff statistics

## Usage

```rust
use dx_ui::prelude::*;

fn build_ui(cx: &mut Context) -> impl IntoElement {
    v_flex()
        .gap_4()
        .child(
            Button::new("my-button", "Click me!")
                .icon(IconName::Check)
                .on_click(|_event, _window, _cx| {
                    println!("Button clicked!");
                })
        )
        .child(
            Label::new("Hello from DX UI!")
                .size(LabelSize::Large)
        )
}
```

## Theme System

```rust
use dx_ui::theme::{DxTheme, DxColorScheme};
use gpui::Hsla;

let theme = DxTheme {
    colors: DxColorScheme {
        accent: Hsla::new(0.6, 0.7, 0.5, 1.0),
        background: Hsla::new(0.0, 0.0, 0.98, 1.0),
        ..Default::default()
    },
    ..Default::default()
};
```

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
dx-ui = { path = "../dx-ui" }
gpui = { git = "https://github.com/zed-industries/zed" }
```

## Status

🚧 **Work in Progress** - Components are being incrementally ported and tested.

### Phase 1: Core Infrastructure ✅
- [x] Extract crates from Zed
- [x] Create adapter traits (theme, settings)
- [x] Update imports and dependencies
- [x] Add to workspace

### Phase 2: Component Porting (In Progress)
- [ ] Port Button components
- [ ] Port Label components
- [ ] Port Icon system
- [ ] Port Layout components
- [ ] Port Overlay components
- [ ] Port Data display components

### Phase 3: Testing & Documentation
- [ ] Add component examples
- [ ] Create storybook/preview app
- [ ] Write comprehensive docs
- [ ] Add unit tests

## Contributing

When porting components:

1. Replace `theme::ActiveTheme` with `crate::theme::ActiveTheme`
2. Replace `settings::Settings` with `crate::settings::Settings`
3. Replace `util::*` with standard Rust crates or `crate::util::*`
4. Replace `icons::IconName` with `dx_icons::IconName`
5. Update imports to use `dx_*` crate names

## License

GPL-3.0-or-later (inherited from Zed)

## Credits

Components extracted and adapted from [Zed](https://github.com/zed-industries/zed) by Zed Industries.
