# Zed Editor - GitHub Copilot Instructions

And always try to fix all clippy warnings and errors!!!
Treat warning cleanup as part of task completion whenever the warning is caused by your changes.

## 🚨🚨🚨 ABSOLUTE RULE: ONE BUILD COMMAND ONLY 🚨🚨🚨

**The ONLY command you are EVER allowed to run for building, checking, or testing code is:**
```bash
CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=1 CARGO_PROFILE_DEV_CODEGEN_UNITS=1 CARGO_PROFILE_DEV_BUILD_OVERRIDE_CODEGEN_UNITS=1 cargo run -p zed --locked
```

**FORBIDDEN commands (NEVER use these):**
- `cargo build -p <any_crate>` — FORBIDDEN
- `cargo check -p <any_crate>` — FORBIDDEN  
- `cargo test -p <any_crate>` — FORBIDDEN
- `cargo build` (without -p zed) — FORBIDDEN
- `cargo check` (without -p zed) — FORBIDDEN
- Any `cargo` command that is NOT `cargo run -p zed --locked` — FORBIDDEN

**Why:** Each separate crate build creates a NEW artifact tree in `target/`, wasting gigabytes on a system with only ~5 GB free. `cargo run -p zed --locked` compiles ALL changed crates incrementally in ONE shared artifact tree and then runs the app. It is the ONLY correct way to verify changes.

**If you violate this rule, you are actively destroying the development environment.**

## ⚠️ CRITICAL: NEVER Build Individual Crates

**NEVER run `cargo build -p <crate>`, `cargo check -p <crate>`, or `cargo test -p <crate>` for individual crates.**

Doing so creates a **separate build artifact tree** for that crate in `target/`, which wastes gigabytes of disk space on this already full system (95% used). The F: drive only has ~5 GB free.

**The ONLY allowed build command is:**
```bash
CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=1 CARGO_PROFILE_DEV_CODEGEN_UNITS=1 CARGO_PROFILE_DEV_BUILD_OVERRIDE_CODEGEN_UNITS=1 cargo run -p zed --locked
```

This single command compiles only what changed (incremental), reuses all existing artifacts, and runs the app. It is the correct way to verify all code changes on this system. Using any other build command is **explicitly forbidden** on this machine.

## Project Overview

This is the Zed code editor - a large-scale Rust project with a complex multi-crate workspace architecture. The codebase is substantial enough that building the entire workspace at once can overwhelm resource-constrained systems.

## Current Build Environment

**Operating System:** Windows 11 Pro  
**Shell:** Git Bash (bash)  
**Total RAM:** 7,471 MB (~7.3 GB)  
**Available Disk Space:** ~5 GB on F: drive (95% used)  
**Build Location:** F:\Desktop\

### Why Low-Resource Build Strategy is Required

With only 7.3 GB of RAM and limited disk space, this system **cannot handle** full workspace builds of the Zed editor. The standard multi-job Cargo build would:
- Consume 10-15+ GB of RAM during linking
- Require 20+ GB of temporary disk space
- Cause out-of-memory errors and build failures

**This is not optional** - the low-resource build configuration is **mandatory** for successful builds on this machine.

## Critical Build Information

### Low-Resource Build Strategy (MANDATORY)

This project **MUST use** incremental, single-job compilation on this Windows system. Use Git Bash commands:

```bash
# Required environment variables - ALWAYS set these
export CARGO_BUILD_JOBS="1"
export CARGO_INCREMENTAL="1"
export CARGO_PROFILE_DEV_CODEGEN_UNITS="1"
export CARGO_PROFILE_DEV_BUILD_OVERRIDE_CODEGEN_UNITS="1"

# Build only the zed package (not the entire workspace)
cargo run -p zed --locked
```

**For PowerShell (if needed):**
```powershell
$env:CARGO_BUILD_JOBS = "1"
$env:CARGO_INCREMENTAL = "1"
$env:CARGO_PROFILE_DEV_CODEGEN_UNITS = "1"
$env:CARGO_PROFILE_DEV_BUILD_OVERRIDE_CODEGEN_UNITS = "1"
cargo run -p zed --locked
```

**Current system uses Git Bash** - always provide bash commands as the primary option.

**Why this matters:**
- Building the full workspace can cause out-of-memory errors on low-spec machines
- Single-job compilation (`CARGO_BUILD_JOBS=1`) reduces peak RAM usage
- Incremental compilation reuses artifacts, so subsequent builds only recompile changed crates
- Always use `-p zed` to build only the main application crate

### Build Best Practices

1. **NEVER build individual crates** - Do not use `cargo build -p <crate>`, `cargo check -p <crate>`, or any per-crate command. This creates duplicate build artifacts and wastes the limited disk space.
2. **Always use `-p zed`** - The only allowed command is `cargo run -p zed --locked` with the required env vars.
3. **Respect incremental builds** - Don't suggest `cargo clean` unless absolutely necessary
4. **Monitor disk space** - F: drive has only ~5 GB free; warn if builds might fill disk
5. **Use stable target directory** - Current builds use `F:\Desktop\target`; consider moving to `C:\zed-target` if space issues occur
6. **Avoid `RUSTFLAGS` overrides** - This repo relies on `.cargo/config.toml` rustflags
7. **Use `--locked`** - Always include `--locked` flag to respect Cargo.lock
8. **Use Git Bash syntax** - This system uses bash, not PowerShell by default

## Icon CLI Tool

The project includes a custom `icon` CLI tool for managing SVG icons from 250+ icon packs (100,000+ icons total).

### Icon Command Usage

```bash
# Search for icons
icon search <query>              # or: icon s <query>
icon search home --limit 20

# Export icons to directory
icon export <query> <dir>        # or: icon e <query> <dir>
icon export arrow ./icons --pack lucide --limit 5

# Export to desktop assets (apps/desktop/assets/icons/)
icon desktop search:lucide home:solar menu:heroicons
icon d arrow:lucide              # Short form

# List available packs
icon packs                       # or: icon p
```

### Popular Icon Packs

- `lucide` - Modern, clean icons (1000+)
- `solar` - Bold, linear icons (7000+)
- `heroicons` - Tailwind UI icons (500+)
- `feather` - Simple, beautiful icons (280+)
- `material-symbols` - Google Material icons (10,000+)
- `tabler` - Customizable icons (4000+)
- `carbon` - IBM Carbon icons (2000+)

### When to Suggest Icon CLI

- User needs to add new icons to the project
- User asks about available icons or icon packs
- User wants to search for specific icon styles
- User needs to export icons for UI components

**Example suggestion:**
```
To add a search icon from the lucide pack:
icon desktop search:lucide
```

## Rust Coding Guidelines

### Code Quality

- **Prioritize correctness and clarity** over performance unless specified
- **Avoid panic-inducing functions** like `unwrap()` - use `?` to propagate errors
- **Never silently discard errors** with `let _ =` on fallible operations
  - Use `?` to propagate errors
  - Use `.log_err()` when ignoring errors but want visibility
  - Use explicit error handling with `match` or `if let Err(...)`
- **Handle async operation errors** - ensure errors propagate to UI for user feedback

### Code Organization

- **Implement in existing files** unless creating a new logical component
- **Never use `mod.rs`** - prefer `src/some_module.rs` over `src/some_module/mod.rs`
- **Specify library paths in Cargo.toml** using `[lib] path = "..."` (e.g., `gpui.rs` not `lib.rs`)
- **Use full words for variables** - no abbreviations like "q" for "queue"

### Comments

- **Don't write organizational comments** that summarize code
- **Only comment to explain "why"** when the reason is tricky or non-obvious

### Async and Concurrency

- **Use variable shadowing** to scope clones in async contexts:
  ```rust
  executor.spawn({
      let task_ran = task_ran.clone();
      async move {
          *task_ran.borrow_mut() = true;
      }
  });
  ```

## GPUI Framework

GPUI is the UI framework powering Zed - an immediate-mode-like framework built on WGPU.

### Key Concepts

- **Context types**: `App`, `Context<T>`, `AsyncApp`, `AsyncWindowContext`
- **Window**: Manages window state, focus, actions, drawing
- **Entities**: `Entity<T>` handles to state with `.read()`, `.update()`, `.update_in()`
- **Concurrency**: `cx.spawn()` for foreground, `cx.background_spawn()` for background work
- **Rendering**: `Render` trait for views, `RenderOnce` for one-time components
- **Actions**: Keyboard-driven or programmatic via `window.dispatch_action()`

### GPUI Best Practices

- Use `cx.notify()` when state changes affect rendering
- Register event handlers with `cx.listener()` for entity updates
- Use `Task<R>` properly - await, detach, or store to prevent cancellation
- Prefer GPUI executor timers over `smol::Timer` in tests

## Testing

- **Use `./script/clippy`** instead of `cargo clippy`
- **Use GPUI timers in tests**: `cx.background_executor().timer(duration).await`
- **Avoid `smol::Timer::after()`** in tests that use `run_until_parked()`

## Pull Request Standards

When creating PRs:

- Use clear, imperative titles (e.g., "Fix crash in project panel")
- Avoid conventional commit prefixes (`fix:`, `feat:`, `docs:`)
- No trailing punctuation in titles
- Optional crate prefix when scope is clear (e.g., `git_ui: Add history view`)
- Include `Release Notes:` section with one bullet:
  - `- Added ...`, `- Fixed ...`, or `- Improved ...` for user-facing changes
  - `- N/A` for non-user-facing changes

Format:
```
Release Notes:

- Fixed crash when opening large files
```

## Current System Specifications

**OS:** Windows 11 Pro  
**Shell:** Git Bash (bash)  
**RAM:** 7.3 GB (low-resource system)  
**Disk:** F: drive with ~5 GB free (95% used) - **CRITICAL: Monitor disk space**  
**Build Path:** F:\Desktop\  
**Target Dir:** F:\Desktop\target (consider moving to C: if space issues)

### Windows Build Requirements

- Ensure MSVC environment variables are set (Visual Studio Build Tools)
- Long paths should be enabled:
  ```bash
  git config --system core.longpaths true
  ```
- If `rc.exe` selection fails:
  ```bash
  export ZED_RC_TOOLKIT_PATH="C:\Program Files (x86)\Windows Kits\10\bin\<SDK_version>\x64"
  ```

### Disk Space Management

With only 5 GB free on F:, be cautious about:
- Debug builds can generate 2-3 GB of artifacts
- Incremental builds accumulate in `target/` directory
- If disk space errors occur, suggest moving `CARGO_TARGET_DIR` to C: drive
- Avoid suggesting operations that generate large temporary files

## Current Project Status

The project is actively developing GPUI's rendering capabilities. Current limitations:

- **Gradients**: Linear gradients supported; radial and mesh gradients not yet implemented
- **Effects**: Box shadows supported; inner glow and backdrop blur not yet implemented
- **SVG Animations**: Not supported (resvg/usvg don't support SMIL or CSS animations)
- **UI Animations**: Fade/slide supported via `AnimationExt`; transform matrices limited

When suggesting UI features, be aware of these GPUI limitations and suggest workarounds when appropriate.

## Helpful Commands (Git Bash)

```bash
# Set required environment variables (add to ~/.bashrc for persistence)
export CARGO_BUILD_JOBS="1"
export CARGO_INCREMENTAL="1"
export CARGO_PROFILE_DEV_CODEGEN_UNITS="1"
export CARGO_PROFILE_DEV_BUILD_OVERRIDE_CODEGEN_UNITS="1"

# Low-resource build (ALWAYS use this)
cargo run -p zed --locked

# Check disk space before building
df -h .

# Run clippy
./script/clippy

# Search for icons
icon search <query>

# Export icon to desktop assets
icon desktop <name>:<pack>

# List icon packs
icon packs

# Check system memory
free -h

# Monitor build progress and disk usage
watch -n 5 'df -h . && free -h'
```

## Documentation References

- Build instructions: `BUILD.md`
- Icon CLI manual: `ICONS.md`
- Coding guidelines: `.rules`
- Agent guidelines: `AGENTS.md`
