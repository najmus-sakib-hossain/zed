# DX MCP Server

DX is a local **MCP (Model Context Protocol)** server (stdio transport) that provides tools for:

- Capturing screenshots of web pages and VS Code previews
- Capturing screenshots of native Windows app windows (e.g. the DX desktop app)
- Optionally running OCR on screenshots and returning extracted text
- Returning screenshots as MCP `image/png` tool output (so hosts that support images can display them)

## Requirements

- Windows (required for native window tools)
- Bun
- For `mode: "page"` web screenshots on Windows: Chrome or Edge installed (DX prefers a headless browser CLI path for reliability)

## Tools

### `open_preview`
Opens a URL (or `localhost` port + path) in the system default browser.

Inputs:
- `url` (http/https) OR `port` + `path`

### `screenshot_preview`
Takes a screenshot of a URL/port preview and returns an `image/png`.

Modes:
- `mode: "vscode" | "screen"` — desktop screenshot (captures what’s currently visible on screen; useful for VS Code integrated preview)
- `mode: "page"` — headless page screenshot (no visible browser redirect by default)

Key inputs:
- `url` OR `port` + `path`
- `open` — whether to open the URL in a visible browser first
  - Defaults: `true` for desktop modes, `false` for `mode: "page"`
- `savePath` — optional; if omitted saves under `.dx/mcp/`
- `ocr`, `ocrLang` — optional OCR on the captured PNG

Outputs:
- `image/png` (base64)
- `text` summary (includes save paths; includes OCR text if enabled)

### `list_windows` (Windows-only)
Lists visible native windows (title + bounds + process name), optionally filtered by a substring.

Input:
- `query` (optional)

### `screenshot_window` (Windows-only)
Captures a screenshot of a native app window by matching **title or process name**.

Inputs:
- `title` — substring match against window title or process name (case-insensitive)
- `savePath` — optional; if omitted saves under `.dx/mcp/`
- `ocr`, `ocrLang` — optional
- `allowRaise` — optional
  - `false` (default): **no-focus mode** (uses `PrintWindow` only; does not change z-order)
  - `true`: enables a fallback for GPU/WebView2 windows that temporarily raises the target window **without activation** to capture real pixels

Notes on focus/visibility:
- In `allowRaise: false` mode, DX uses `PrintWindow` (Win32). This can capture without focus/activation, but many GPU-accelerated apps (WebView2/DirectComposition) do not render into `PrintWindow`, resulting in blank output.
- In `allowRaise: true` mode, DX may temporarily raise the target window (with `SWP_NOACTIVATE`) and use a pixel copy fallback. This is usually reliable for GPU apps, but it can change z-order and may be visually noticeable.
- If the window is minimized, Windows may not have fresh pixels available to capture.

### `dx_attach_latest_screenshot` / `attach_latest_screenshot`
Reads `.dx/mcp/latest.png` (or another file in that folder) and re-attaches it as an MCP `image/png` response.

This is useful when a host shows an image to the user but the model didn’t receive the actual image bytes in a previous step.

Inputs:
- `filename` (optional, default `latest.png`)
- `ocr`, `ocrLang` (optional)

## Output files

All screenshots are written under the workspace root:

- `.dx/mcp/latest.png` — always updated with the most recent screenshot
- `.dx/mcp/latest-preview.png` / `.dx/mcp/latest-window.png` — per-category latest
- `.dx/mcp/*.png` — individual saved screenshots (either named via `savePath` or auto-generated)

## Running locally

From `mcp/`:

- Start server (stdio): `bun run src/server.ts`
- Run test client: `bun run src/test_client.ts`

## VS Code configuration

This server is meant to be registered via `.vscode/mcp.json` (stdio transport) and run with Bun.

## Limitations

- Web screenshots:
  - `mode: "page"` is headless and does not capture the VS Code integrated browser UI; it captures the URL content.
  - `mode: "vscode"` captures the desktop pixels (what is actually on screen).
- Native window screenshots on Windows:
  - `PrintWindow` can succeed without focus, but GPU-accelerated apps may not render into it.
  - The fallback may need the window to be visible (not fully occluded/minimized) to capture correct pixels.
