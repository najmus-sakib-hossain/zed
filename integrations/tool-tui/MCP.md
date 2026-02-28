# DX MCP Server Documentation

Last updated: February 20, 2026

## Overview

The DX MCP server provides screenshot capabilities for web pages and native applications, designed for AI-assisted development workflows.

## Key Features

1. **Web page screenshots** - Capture any URL without interrupting user focus
2. **Native app screenshots** - Capture screenshots of running applications (Windows)
3. **OCR support** - Extract text from screenshots automatically
4. **Auto-attachment** - Screenshots are automatically returned as image attachments for AI vision models

## Screenshot Modes

### `mode: "page"` - Headless Browser (RECOMMENDED)
- Uses Playwright to capture actual web content
- **No focus change** - doesn't interrupt user's current work
- **No visible browser window** - runs completely in background
- Perfect for capturing web URLs without disruption
- Example: `{"url":"https://www.google.com/","mode":"page","open":false}`

### `mode: "vscode"` or `mode: "screen"` - Desktop Screenshot
- Captures whatever is currently visible on screen
- Useful for VS Code internal previews
- **With `open: true`**: Opens URL in browser first, then captures (focus change)
- **With `open: false`**: Captures current screen without opening URL
- **Use case**: Fallback when headless mode fails, or when you need real browser experience
- Example: `{"url":"https://www.google.com/","mode":"vscode","open":true}`

### Native App Screenshots
- Use `screenshot_window` tool with app title
- Windows-only feature using PowerShell
- Example: `{"title":"DX","ocr":true}`

## Best Practices (February 20, 2026)

**For web URLs - No focus change (RECOMMENDED):**
```json
{
  "url": "https://www.google.com/",
  "mode": "page",
  "open": false,
  "waitMs": 1500,
  "ocr": true
}
```

**For web URLs - With focus change (fallback/real experience):**
```json
{
  "url": "https://www.google.com/",
  "mode": "vscode",
  "open": true,
  "waitMs": 1500,
  "ocr": true
}
```

**For local dev servers - No focus change:**
```json
{
  "port": 3000,
  "mode": "page",
  "open": false,
  "waitMs": 1500
}
```

**For local dev servers - With focus change (fallback):**
```json
{
  "port": 3000,
  "mode": "vscode",
  "open": true,
  "waitMs": 1500
}
```

**For native apps:**
```json
{
  "title": "MyApp",
  "ocr": true,
  "ocrLang": "eng"
}
```

## Storage

All screenshots are automatically saved to `.dx/mcp/` directory with timestamps. The latest screenshot is always available at `.dx/mcp/latest.png`.

## VS Code Copilot Integration

As of February 2026, VS Code Copilot Chat supports vision and can receive image attachments from MCP tools. The DX MCP server automatically returns screenshots as image/png attachments that vision-capable models can analyze.

**Workflow:**
1. AI makes UI changes
2. AI calls `#screenshot_preview` with `mode: "page"` and `ocr: true`
3. Screenshot is captured without focus change
4. Image + OCR text returned to AI
5. AI analyzes and continues development

## Technical Details

- **Playwright**: Used for headless browser screenshots
- **screenshot-desktop**: Used for desktop/screen captures
- **Tesseract.js**: Used for OCR text extraction
- **PowerShell**: Used for Windows native app capture
