# GPUI Documentation

## Overview

GPUI is a hybrid immediate and retained mode, GPU accelerated, UI framework for Rust, designed to support a wide variety of applications.

### Getting Started

GPUI is still in active development as we work on the Zed code editor, and is still pre-1.0. There will often be breaking changes between versions. You'll also need to use the latest version of stable Rust and be on macOS or Linux. Add the following to your `Cargo.toml`:

```
gpui = { version = "*" }
```

Everything in GPUI starts with an `Application`. You can create one with `Application::new()`, and kick off your application by passing a callback to `Application::run()`. Inside this callback, you can create a new window with `App::open_window()`, and register your first root view.

### The Big Picture

GPUI offers three different registers depending on your needs:

- **State management and communication with Entities**: Whenever you need to store application state that communicates between different parts of your application, use GPUI's entities. Entities are owned by GPUI and are only accessible through an owned smart pointer similar to an `Rc`.

- **High level, declarative UI with views**: All UI in GPUI starts with a view. A view is simply an `Entity` that can be rendered, by implementing the `Render` trait. At the start of each frame, GPUI will call this render method on the root view of a given window. Views build a tree of `elements`, lay them out and style them with a tailwind-style API.

- **Low level, imperative UI with Elements**: Elements are the building blocks of UI in GPUI, providing a wrapper around an imperative API with total control over rendering.

Each of these registers has corresponding contexts that can be accessed from all GPUI services.

### Other Resources

GPUI provides additional services like actions for keyboard shortcuts, platform services, an async executor, and testing macros.

## Core Components

### Modules
- [Ownership and Data Flow](modules/ownership_and_data_flow.md)
- [Colors](modules/colors.md)

### Key Structs
- [App](api/structs/App.md)
- [Application](api/structs/Application.md)
- [Entity](api/structs/Entity.md)
- [Window](api/structs/Window.md)
- [Div](api/structs/Div.md)

### Key Traits
- [Render](api/traits/Render.md)
- [Action](api/traits/Action.md)
- [Element](api/traits/Element.md)

### Key Enums
- [Display](api/enums/Display.md)
- [Position](api/enums/Position.md)

### Functions
- [div](api/functions/div.md)
- [px](api/functions/px.md)

### Macros
- [actions](api/macros/actions.md)

## API Reference

See the `api/` folder for detailed documentation of all structs, traits, enums, functions, macros, types, and constants.

## Dependencies

GPUI has system dependencies for macOS (Xcode, Metal), Linux, etc. See the core documentation for details.