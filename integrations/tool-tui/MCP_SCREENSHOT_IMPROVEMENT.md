# MCP Screenshot Tool Improvement Request

## Current Issue

The `last-window-capture.ps1` PowerShell script in `.dx/mcp/` is a **workaround** for capturing screenshots of GPU/WebView2 windows (like GPUI apps). This script shouldn't be necessary - the MCP tool should handle this internally.

## What the PowerShell Script Does

Located at `.dx/mcp/last-window-capture.ps1`, this script:

1. **Takes a window handle (HWND)** as input (hardcoded: `1114562`)
2. **Tries PrintWindow API first** - standard Windows API for capturing window content
3. **Detects if PrintWindow fails** (returns blank/black image for GPU windows)
4. **Falls back to screen capture** if PrintWindow fails:
   - Temporarily raises the window to topmost (without activating/stealing focus)
   - Uses `CopyFromScreen` to capture actual pixels
   - Restores window z-order
5. **Saves PNG** to specified path
6. **Returns base64 + metadata** as JSON

### Key Code Sections

```powershell
# Hardcoded window handle
$hWnd = [IntPtr]::new(1114562)

# Try PrintWindow first
$ok = [Win32]::PrintWindow($hWnd, $hdc, 0)

# Detect blank/black output (GPU rendering issue)
function Test-ProbablyBlank([System.Drawing.Bitmap]$b) {
    # Samples pixels to detect if image is mostly uniform color
}

# Fallback: Raise window and capture from screen
if ($needsFallback) {
    [void][Win32]::SetWindowPos($hWnd, $HWND_TOPMOST, 0, 0, 0, 0, $flags)
    Start-Sleep -Milliseconds 120
    $gfx.CopyFromScreen($x, $y, 0, 0, $bmp.Size)
    [void][Win32]::SetWindowPos($hWnd, $HWND_NOTOPMOST, 0, 0, 0, 0, $flags)
}
```

## Why This Script Exists

**GPU/WebView2 rendering issue**: Windows' `PrintWindow` API doesn't work with GPU-accelerated windows (GPUI, Chromium WebView2, etc.) because:
- GPU renders directly to screen, bypassing GDI
- PrintWindow captures GDI surface, which is blank for GPU apps
- Need to capture actual screen pixels instead

## Current MCP Tool Behavior

The `mcp_DX_screenshot_window` tool has an `allowRaise` parameter that triggers this fallback behavior internally. However, the PowerShell script suggests this logic might be:
1. Duplicated/external to the MCP tool
2. Hardcoded for specific window handles
3. Not fully integrated into the MCP tool

## Problems with Current Approach

1. **Hardcoded window handle**: Script has `$hWnd = [IntPtr]::new(1114562)` - this is specific to one window instance
2. **External script dependency**: Should be built into MCP tool, not a separate file
3. **Manual invocation**: Script appears to be called separately rather than integrated
4. **Maintenance burden**: Two places to maintain screenshot logic

## Desired Solution

The MCP tool should handle this internally:

```rust
// Inside mcp_DX_screenshot_window implementation
fn screenshot_window(title: &str, allow_raise: bool) -> Result<Screenshot> {
    let hwnd = find_window_by_title(title)?;
    
    // Try PrintWindow first
    let bitmap = try_print_window(hwnd)?;
    
    // Detect if blank (GPU rendering)
    if is_probably_blank(&bitmap) && allow_raise {
        // Fallback: Raise window and capture from screen
        temporarily_raise_window(hwnd)?;
        sleep(120); // Let window render
        let screen_bitmap = capture_from_screen(hwnd)?;
        restore_window_order(hwnd)?;
        return Ok(screen_bitmap);
    }
    
    Ok(bitmap)
}
```

## Questions

1. **Is the PowerShell script still needed?** Or is it legacy code that can be removed?
2. **Does `mcp_DX_screenshot_window` already implement this logic?** The `allowRaise: true` parameter suggests it does
3. **Why is the window handle hardcoded?** Should be dynamically found by title
4. **Can we remove this script?** If the MCP tool already handles GPU windows properly

## Recommendation

1. **Verify MCP tool has built-in GPU window support** with `allowRaise: true`
2. **Remove the PowerShell script** if redundant
3. **Update documentation** to clarify when to use `allowRaise: true`
4. **Test with GPUI apps** to ensure screenshots work without external scripts

## Testing

```bash
# Should work without PowerShell script
#list_windows
#screenshot_window {"title":"DX","allowRaise":true,"savePath":".dx/mcp/test.png"}
```

If this works reliably, the PowerShell script can be deleted.
