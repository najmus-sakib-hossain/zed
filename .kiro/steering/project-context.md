---
inclusion: auto
---

And always try to fix all clippy warnings and errors!!!
Treat warning cleanup as part of task completion whenever warnings are introduced by recent edits.

To test please always use this command as it builds what needs to be built automatically!!!
```bash
CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=1 CARGO_PROFILE_DEV_CODEGEN_UNITS=1 CARGO_PROFILE_DEV_BUILD_OVERRIDE_CODEGEN_UNITS=1 cargo run -p zed --locked
```

# Zed Editor Project Context

This steering file provides essential context about the Zed editor codebase that you should be aware of during all interactions.

## Current System Environment

**Operating System:** Windows 11 Pro  
**Shell:** Git Bash (bash) - **Use bash commands, not PowerShell**  
**Total RAM:** 7,471 MB (~7.3 GB)  
**Available Disk Space:** ~5 GB on F: drive (95% used) - **CRITICAL**  
**Build Location:** F:\Desktop\  
**Target Directory:** F:\Desktop\target\

### System Constraints

This is a **low-resource Windows system** with:
- Limited RAM (7.3 GB) - insufficient for standard multi-job Rust builds
- Very limited disk space (only 5 GB free) - builds can fail if disk fills
- Git Bash as primary shell - always provide bash syntax first

**These constraints are CRITICAL** - standard build commands will fail on this system.

## Project Scale and Build Strategy

This is a **large-scale Rust code editor project** with a complex multi-crate workspace. The codebase is substantial enough that:

1. **Building the entire workspace at once WILL FAIL on this system** due to RAM and disk constraints
2. **We MUST use incremental, single-job compilation** - this is not optional
3. **Builds target specific crates** (usually `-p zed`) rather than the full workspace

### Why This System Requires Special Build Configuration

With only 7.3 GB RAM and 5 GB free disk space:
- Standard Cargo builds would consume 10-15+ GB RAM during linking → **Out of Memory**
- Full workspace builds require 20+ GB temporary disk space → **Disk Full**
- Multi-job compilation creates memory spikes that crash the build → **Build Failure**

The low-resource build strategy is **mandatory** for this machine to successfully build Zed.

### Critical Build Configuration (Git Bash)

When suggesting build commands, **ALWAYS use these low-resource settings with Git Bash syntax**:

```bash
# Required environment variables - add to ~/.bashrc for persistence
export CARGO_BUILD_JOBS="1"
export CARGO_INCREMENTAL="1"
export CARGO_PROFILE_DEV_CODEGEN_UNITS="1"
export CARGO_PROFILE_DEV_BUILD_OVERRIDE_CODEGEN_UNITS="1"

# Build command - ALWAYS use -p zed
cargo run -p zed --locked
```

**Alternative PowerShell syntax (if user specifically requests it):**
```powershell
$env:CARGO_BUILD_JOBS = "1"
$env:CARGO_INCREMENTAL = "1"
$env:CARGO_PROFILE_DEV_CODEGEN_UNITS = "1"
$env:CARGO_PROFILE_DEV_BUILD_OVERRIDE_CODEGEN_UNITS = "1"
cargo run -p zed --locked
```

**Default to Git Bash** - this is the primary shell on this system.

### Build Best Practices

- **Always use `-p zed`** to build only the main application crate, not the entire workspace
- **Always include `--locked`** to respect Cargo.lock
- **Never suggest `cargo clean`** unless absolutely necessary (breaks incremental builds and wastes disk space)
- **Never override `RUSTFLAGS`** - this repo relies on `.cargo/config.toml` rustflags
- **Monitor disk space** - with only 5 GB free, warn before operations that generate large files
- **Recommend stable `CARGO_TARGET_DIR`** - if disk space issues occur, suggest moving to `C:\zed-target`
- **Use Git Bash syntax** - this is the primary shell, not PowerShell
- **Check disk space before builds** - suggest `df -h .` if user reports build failures

### Why This Matters

- Full workspace builds can cause out-of-memory errors on low-spec machines
- Single-job compilation reduces peak RAM usage significantly
- Incremental compilation reuses artifacts - subsequent builds only recompile changed crates
- This approach makes development feasible on resource-constrained Windows systems

## Icon CLI Tool

The project includes a custom **`icon` CLI tool** installed on this system. It provides access to 250+ icon packs with 100,000+ icons.

### Icon Command Reference

```bash
# Search for icons (fuzzy matching supported)
icon search <query>              # Full form
icon s <query>                   # Short form
icon search home --limit 20      # With custom limit

# Export icons to a directory
icon export <query> <output_dir> [--pack PACK] [--limit N]
icon e arrow ./icons --pack lucide --limit 5

# Export directly to desktop assets (apps/desktop/assets/icons/)
icon desktop <icon_specs...>     # Format: name:pack
icon d search:lucide home:solar menu:heroicons

# List all available icon packs
icon packs                       # Full form
icon p                          # Short form
```

### Popular Icon Packs

When suggesting icons, prefer these well-maintained packs:

- **`lucide`** - Modern, clean icons (1000+ icons) - **Recommended for UI**
- **`solar`** - Bold, linear icons (7000+ icons)
- **`heroicons`** - Tailwind UI icons (500+ icons)
- **`feather`** - Simple, beautiful icons (280+ icons)
- **`material-symbols`** - Google Material icons (10,000+ icons)
- **`tabler`** - Customizable icons (4000+ icons)
- **`carbon`** - IBM Carbon icons (2000+ icons)
- **`octicon`** - GitHub icons (300+ icons)
- **`simple-icons`** - Brand/logo icons (2500+ icons)

### When to Use Icon CLI

Proactively suggest the icon CLI when:

- User needs to add new icons to the UI
- User asks about available icons or icon styles
- User wants to search for specific icon types
- User is implementing new UI components that need icons
- User mentions needing icons from a specific design system

### Icon CLI Examples

```bash
# Add a search icon from lucide pack
icon desktop search:lucide

# Export multiple icons for a feature
icon d home:lucide settings:lucide profile:lucide logout:lucide

# Search for arrow icons
icon search arrow --limit 10

# Export all menu-related icons from heroicons
icon export menu ./icons/menu --pack heroicons --limit 20
```

### Icon CLI Performance

- Search latency: <0.1ms (cached), <1ms (uncached)
- Supports fuzzy matching for typo tolerance
- Zero-copy archived data for fast access
- Memory-mapped files for efficient loading

## GPUI Framework Context

GPUI is the custom UI framework powering Zed. It's built on WGPU and designed for extreme performance (120fps target).

### GPUI Limitations to Remember

When suggesting UI features, be aware of current GPUI capabilities:

**Supported:**
- Linear gradients (`BackgroundTag::LinearGradient`)
- Box shadows with blur
- Fade in/out animations via `AnimationExt`
- Slide/scale animations (via layout properties)
- Flexbox layout

**Not Yet Supported:**
- Radial gradients (would require custom WGSL shader)
- Mesh gradients (would require custom mesh rendering)
- Inner glow effects (would require shader modification)
- Backdrop blur / element blur (requires multi-pass rendering)
- SVG animations (SMIL or CSS in SVG)
- CSS-like transform matrices for divs (only for SVGs/paths)

### GPUI Workarounds

When users request unsupported features:

- **Outer glow**: Use `box_shadow` with 0 offset and large `blur_radius`
- **Multiple gradients**: Stack multiple divs with different gradients and opacities
- **Scale effects**: Animate actual layout properties (padding, margins, bounds)

## Testing Commands

- **Use `./script/clippy`** instead of `cargo clippy`
- **In GPUI tests**, use `cx.background_executor().timer(duration).await` instead of `smol::Timer::after()`

## Windows Development Notes

This system is running **Windows 11 Pro with Git Bash**. Remember:

- **Primary shell is Git Bash** - use bash syntax for all commands
- Builds require MSVC environment (Visual Studio Build Tools)
- Long paths should be enabled: `git config --system core.longpaths true`
- If `rc.exe` fails: `export ZED_RC_TOOLKIT_PATH="C:\Program Files (x86)\Windows Kits\10\bin\<SDK_version>\x64"`
- Path separators: Windows uses `\` but Git Bash and Rust use `/`
- **Disk space is critical** - F: drive is 95% full with only 5 GB free
- Environment variables in bash: `export VAR="value"`
- Environment variables in PowerShell: `$env:VAR = "value"`

## Project Documentation

Key documentation files:

- **`BUILD.md`** - Detailed build instructions and troubleshooting
- **`ICONS.md`** - Complete icon CLI manual
- **`.rules`** - Rust coding guidelines and GPUI patterns
- **`AGENTS.md`** - Agent-specific guidelines

## Proactive Suggestions

When appropriate, proactively suggest:

1. **Using the icon CLI** when users are working on UI components
2. **Low-resource build commands with Git Bash syntax** when users mention build issues
3. **Checking disk space** (`df -h .`) if builds fail or before large operations
4. **Moving CARGO_TARGET_DIR to C: drive** if F: drive space becomes critical
5. **GPUI workarounds** when users request unsupported visual effects
6. **Incremental builds** when users are doing full rebuilds unnecessarily
7. **Monitoring system resources** during builds: `watch -n 5 'df -h . && free -h'`

## Common Build Issues on This System

### Out of Memory Errors
- Cause: Multi-job compilation or full workspace build
- Solution: Ensure `CARGO_BUILD_JOBS=1` is set and using `-p zed`

### Disk Full Errors
- Cause: Only 5 GB free on F: drive
- Solution: Move target directory: `export CARGO_TARGET_DIR="C:\zed-target"`
- Check space: `df -h .`

### Link Errors (LNK1201)
- Cause: Insufficient disk space for PDB files
- Solution: Free up disk space or move target directory to C: drive

## Summary

Remember: This is a large codebase running on a **low-resource Windows 11 system (7.3 GB RAM, 5 GB free disk)** that **requires** careful resource management during builds. The system uses **Git Bash as the primary shell**. It has a powerful icon CLI tool available and uses a custom UI framework (GPUI) with specific capabilities and limitations. Always consider these constraints when providing suggestions and solutions.

**Critical constraints:**
- RAM: 7.3 GB (insufficient for standard builds)
- Disk: 5 GB free on F: (95% full - monitor closely)
- Shell: Git Bash (use bash syntax by default)
- Build: MUST use single-job, incremental compilation with `-p zed`
