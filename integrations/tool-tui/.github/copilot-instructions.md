# Copilot instructions (DX)

Last updated: February 20, 2026

When you need visual verification (UI changes, browser previews, native app frames), use the DX MCP tools instead of guessing.

## GPUI Desktop App Development Workflow

### CRITICAL: Use Cargo Watch + MCP Screenshots

For GPUI/native desktop app development (apps/desktop, apps/webview):

1. **Start cargo watch ONCE** (keeps running in background):
   ```bash
   cargo watch -x run
   ```
   - Use `controlPwshProcess` with action "start" to run in background
   - Auto-recompiles and restarts app on file changes
   - Monitor output with `getProcessOutput` tool

2. **Take screenshots with MCP tool** (for GPU/WebView2 apps):
   - Use `#screenshot_window` with `allowRaise: true` for GPUI/GPU windows
   - Find window title with `#list_windows` first
   - Save to `.dx/mcp/` directory

3. **NEVER use these anti-patterns**:
   - ❌ Running `cargo run` repeatedly (inefficient)
   - ❌ Using `sleep` commands (pointless waiting)
   - ❌ Manual window switching

### Example Desktop App Workflow

```bash
# 1. Start watch process (once)
controlPwshProcess(action: "start", command: "cargo watch -x run", cwd: "apps/webview")

# 2. Make code changes
# (cargo watch auto-recompiles)

# 3. Check compilation output
getProcessOutput(terminalId: "14", lines: 20)

# 4. Take screenshot when ready
#list_windows  # Find window title
#screenshot_window {"title":"DX","allowRaise":true,"ocr":true}

# 5. Repeat steps 2-4 as needed
```

### Benefits

- **Faster iteration**: No repeated cargo startup overhead
- **Continuous feedback**: See compilation errors immediately
- **Reliable screenshots**: `allowRaise: true` works with GPU windows
- **Clean workflow**: No sleep commands or manual waiting

## AI Feedback Loop Pattern (Iterative Development)

GitHub Copilot agent mode supports **agentic loops** where you can make changes and verify them iteratively. Use this pattern for UI development:

1. **Make changes** to code/UI
2. **Screenshot** using `#screenshot_preview` to see results
3. **Analyze** the screenshot and identify issues
4. **Adjust** code based on visual feedback
5. **Repeat** steps 2-4 until satisfied

This creates a real-time feedback loop where AI can see its changes and iterate automatically.

### Example iterative workflow:

```
User: "Make the button blue and centered"
AI: [makes changes] → calls #screenshot_preview → sees result → "The button is blue but not perfectly centered, adjusting..."
AI: [adjusts code] → calls #screenshot_preview → sees result → "Perfect! The button is now blue and centered."
```

## Screenshot workflow

**CRITICAL: Use `mode: "page"` for web URLs to avoid focus changes**

- For web URLs: Use `#screenshot_preview` with `mode: "page"` and `open: false` to capture without interrupting user focus
- For native apps: Use `#screenshot_window` with the app title
- For VS Code previews: Use `mode: "vscode"` only when specifically testing VS Code internal preview
- Set `ocr: true` so the tool result contains readable text in addition to the PNG attachment
- Omit `savePath` unless the user requests a specific filename; by default screenshots are saved under `.dx/mcp/`
- After capturing a screenshot, immediately call `#dx_attach_latest_screenshot {"filename":"latest.png"}` so the chat model receives the screenshot as an explicit image attachment

### Mode selection guide

- `mode: "page"` - Headless browser (Playwright), no focus change, captures actual URL content ✓ RECOMMENDED
- `mode: "vscode"` - Desktop screenshot, captures whatever is currently visible (use only for VS Code previews)
- `mode: "screen"` - Same as vscode, desktop screenshot

### Examples

**Web URLs (no focus change):**
- `#screenshot_preview {"url":"https://www.google.com/","mode":"page","open":false,"waitMs":1500,"ocr":true,"ocrLang":"eng"}`
- `#screenshot_preview {"port":3000,"path":"/","mode":"page","open":false,"waitMs":1500,"ocr":true}`

**Web URLs (with focus change - fallback/real experience):**
- `#screenshot_preview {"url":"https://www.google.com/","mode":"vscode","open":true,"waitMs":1500,"ocr":true,"ocrLang":"eng"}`
- `#screenshot_preview {"port":3000,"path":"/","mode":"vscode","open":true,"waitMs":1500,"ocr":true}`

**Native apps:**
- `#screenshot_window {"title":"DX","ocr":true,"ocrLang":"eng"}`

**Attach latest:**
- `#dx_attach_latest_screenshot {"filename":"latest.png","ocr":true,"ocrLang":"eng"}`

## Feedback loop

After making a meaningful UI/preview change, take a new screenshot and compare against the previous OCR text/output before proceeding. Continue iterating until the desired result is achieved.
