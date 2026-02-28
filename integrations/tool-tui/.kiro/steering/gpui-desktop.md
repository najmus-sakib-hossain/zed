---
inclusion: fileMatch
fileMatchPattern: "apps/desktop/**"
---

# GPUI Desktop Development Guidelines

## CRITICAL: Always Reference GPUI Documentation

Before working on `apps/desktop/` code, ALWAYS read the relevant GPUI documentation:

### Required Reading

1. **Core Documentation**: `#[[file:docs/gpui/README.md]]`
2. **GPUI Summary**: `#[[file:docs/gpui/GPUI_SUMMARY.md]]`
3. **API Reference**: Browse `docs/gpui/api/` for specific types/traits/functions

### GPUI is Evolving

- GPUI is in active development with changing APIs
- Do NOT assume APIs from memory or general Rust knowledge
- ALWAYS verify current API signatures in `docs/gpui/` before coding
- Check `docs/gpui/api/structs/`, `docs/gpui/api/traits/`, `docs/gpui/api/functions/`

## Desktop App Context

- Location: `apps/desktop/`
- Framework: GPUI (Zed's GPU-accelerated UI framework)
- Component Library: Shadcn-ui inspired components in `apps/desktop/src/components/`
- Icon System: Solar and Lucide icons from `apps/www/public/icons/`

## Development Workflow

1. Read relevant GPUI docs FIRST
2. Check existing component patterns in `apps/desktop/src/components/`
3. Verify API usage against `docs/gpui/api/`
4. Write code using verified APIs
5. Run `cargo check` and `cargo fmt`

## Testing & Visual Verification Workflow

### CRITICAL: Use Cargo Watch + MCP Screenshots

When developing GPUI desktop apps, use this efficient workflow:

1. **Start cargo watch ONCE** (keeps running in background):
   ```bash
   cargo watch -x run
   ```
   - Use `controlPwshProcess` with action "start" to run in background
   - This auto-recompiles and restarts the app on file changes
   - Monitor output with `getProcessOutput` tool

2. **Take screenshots with MCP tool** (for GPU/WebView2 apps):
   ```bash
   # Use mcp_DX_screenshot_window with allowRaise: true
   ```
   - ALWAYS use `allowRaise: true` for GPUI/GPU-accelerated windows
   - Find window title with `mcp_DX_list_windows` first
   - Save to `.dx/mcp/` directory for comparison

3. **NEVER use these anti-patterns**:
   - ❌ Running `cargo run` repeatedly (inefficient)
   - ❌ Using `sleep` commands (pointless waiting)
   - ❌ Manual window switching

### Example Workflow

```bash
# 1. Start watch process (once)
controlPwshProcess(action: "start", command: "cargo watch -x run", cwd: "apps/desktop")

# 2. Make code changes
# (cargo watch auto-recompiles)

# 3. Check compilation output
getProcessOutput(terminalId: "14", lines: 20)

# 4. Take screenshot when ready
mcp_DX_list_windows()  # Find window title
mcp_DX_screenshot_window(title: "DX", allowRaise: true, savePath: ".dx/mcp/latest.png")

# 5. Repeat steps 2-4 as needed
```

### Benefits

- **Faster iteration**: No repeated cargo startup overhead
- **Continuous feedback**: See compilation errors immediately
- **Reliable screenshots**: `allowRaise: true` works with GPU windows
- **Clean workflow**: No sleep commands or manual waiting

## Common GPUI Patterns

Refer to `docs/gpui/` for:
- Element rendering with `IntoElement` trait
- State management with `Context` and `Window`
- Styling with method chaining (`.flex()`, `.px()`, etc.)
- Event handling (`.on_click()`, etc.)
- Component composition

## Icon Export CLI

Use `icon` CLI to download SVG icons for the desktop app:

```bash
# Search for icons
icon search <query> --limit 10
icon s home --limit 5

# Export icons to desktop app (recommended)
icon desktop search:lucide home:solar menu:lucide
icon d search:lucide home:solar

# Export to custom directory
icon export <query> <output_dir> --limit 5 --pack lucide
icon e search ./icons --pack lucide

# List available icon packs (219 packs)
icon packs
icon p

# Help and version
icon help
icon version
```

### Icon Export Syntax

- Format: `name:pack` (e.g., `search:lucide`, `home:solar`)
- Icons exported to: `apps/desktop/assets/icons/`
- Available packs: lucide, solar, svgl, heroicons, feather, material-symbols, etc.
- 305,612+ icons available across 219 packs (includes 932 SVGL brand logos)
- Fast: ~2.7s initial load, cached for subsequent commands

### Quick Icon Workflow

1. Search: `icon s home --limit 5`
2. Export: `icon d home:lucide arrow-left:lucide check:solar`
3. Use in code: Load from `apps/desktop/assets/icons/home.svg`

### Production Features

- Cached engine initialization (fast subsequent runs)
- Proper error handling with exit codes
- Clean, minimal output with timing
- Help and version commands
- Command aliases (s, e, d, p)

