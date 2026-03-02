//! Platform-specific screen capture using native tools.
//!
//! - **Windows**: PowerShell + .NET `System.Drawing` bitmap capture
//! - **macOS**: `screencapture` CLI tool
//! - **Linux**: `scrot` or `import` (ImageMagick)
//!
//! All methods capture to a temporary PNG file, then read bytes.

use anyhow::Result;
use std::path::PathBuf;

/// Capture the full primary screen, returning raw PNG bytes.
pub fn capture_full_screen() -> Result<Vec<u8>> {
    let tmp = temp_screenshot_path();

    #[cfg(target_os = "windows")]
    {
        let script = format!(
            r#"
Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName System.Drawing
$screen = [System.Windows.Forms.Screen]::PrimaryScreen
$bitmap = New-Object System.Drawing.Bitmap($screen.Bounds.Width, $screen.Bounds.Height)
$graphics = [System.Drawing.Graphics]::FromImage($bitmap)
$graphics.CopyFromScreen($screen.Bounds.Location, [System.Drawing.Point]::Empty, $screen.Bounds.Size)
$bitmap.Save("{path}")
$graphics.Dispose()
$bitmap.Dispose()
"#,
            path = tmp.to_string_lossy().replace('\\', "\\\\"),
        );
        let status = std::process::Command::new("powershell")
            .args(["-NoProfile", "-Command", &script])
            .status()
            .map_err(|e| anyhow::anyhow!("PowerShell screenshot failed: {}", e))?;
        if !status.success() {
            return Err(anyhow::anyhow!("PowerShell screenshot exited with {:?}", status.code()));
        }
    }

    #[cfg(target_os = "macos")]
    {
        let status = std::process::Command::new("screencapture")
            .args(["-x", "-t", "png", &tmp.to_string_lossy()])
            .status()
            .map_err(|e| anyhow::anyhow!("screencapture failed: {}", e))?;
        if !status.success() {
            return Err(anyhow::anyhow!("screencapture exited with {:?}", status.code()));
        }
    }

    #[cfg(target_os = "linux")]
    {
        // Try scrot first, fall back to import (ImageMagick)
        let scrot_result = std::process::Command::new("scrot")
            .arg(&tmp.to_string_lossy().to_string())
            .status();
        match scrot_result {
            Ok(s) if s.success() => {}
            _ => {
                let status = std::process::Command::new("import")
                    .args(["-window", "root", &tmp.to_string_lossy().to_string()])
                    .status()
                    .map_err(|e| anyhow::anyhow!("Neither scrot nor import available: {}", e))?;
                if !status.success() {
                    return Err(anyhow::anyhow!("import (ImageMagick) exited with {:?}", status.code()));
                }
            }
        }
    }

    let data = std::fs::read(&tmp)?;
    let _ = std::fs::remove_file(&tmp);
    Ok(data)
}

/// Capture a region of the screen, returning raw PNG bytes.
pub fn capture_region(x: i32, y: i32, width: u32, height: u32) -> Result<Vec<u8>> {
    let tmp = temp_screenshot_path();

    #[cfg(target_os = "windows")]
    {
        let script = format!(
            r#"
Add-Type -AssemblyName System.Drawing
$bitmap = New-Object System.Drawing.Bitmap({w}, {h})
$graphics = [System.Drawing.Graphics]::FromImage($bitmap)
$graphics.CopyFromScreen({x}, {y}, 0, 0, (New-Object System.Drawing.Size({w}, {h})))
$bitmap.Save("{path}")
$graphics.Dispose()
$bitmap.Dispose()
"#,
            x = x,
            y = y,
            w = width,
            h = height,
            path = tmp.to_string_lossy().replace('\\', "\\\\"),
        );
        let status = std::process::Command::new("powershell")
            .args(["-NoProfile", "-Command", &script])
            .status()
            .map_err(|e| anyhow::anyhow!("PowerShell region capture failed: {}", e))?;
        if !status.success() {
            return Err(anyhow::anyhow!("PowerShell region capture exited with {:?}", status.code()));
        }
    }

    #[cfg(target_os = "macos")]
    {
        // screencapture -R x,y,w,h
        let rect = format!("{},{},{},{}", x, y, width, height);
        let status = std::process::Command::new("screencapture")
            .args(["-x", "-t", "png", "-R", &rect, &tmp.to_string_lossy()])
            .status()
            .map_err(|e| anyhow::anyhow!("screencapture region failed: {}", e))?;
        if !status.success() {
            return Err(anyhow::anyhow!("screencapture exited with {:?}", status.code()));
        }
    }

    #[cfg(target_os = "linux")]
    {
        // import -crop WxH+X+Y
        let geometry = format!("{}x{}+{}+{}", width, height, x, y);
        let status = std::process::Command::new("import")
            .args(["-window", "root", "-crop", &geometry, &tmp.to_string_lossy().to_string()])
            .status()
            .map_err(|e| anyhow::anyhow!("import region capture failed: {}", e))?;
        if !status.success() {
            return Err(anyhow::anyhow!("import exited with {:?}", status.code()));
        }
    }

    let data = std::fs::read(&tmp)?;
    let _ = std::fs::remove_file(&tmp);
    Ok(data)
}

/// Capture a specific window by title.
pub fn capture_window(title: &str) -> Result<Vec<u8>> {
    let tmp = temp_screenshot_path();

    #[cfg(target_os = "windows")]
    {
        // Use PowerShell to find window by title and capture it
        let script = format!(
            r#"
Add-Type -AssemblyName System.Drawing
Add-Type -TypeDefinition @"
using System;
using System.Runtime.InteropServices;
public class WindowCapture {{
    [DllImport("user32.dll")]
    public static extern IntPtr FindWindow(string lpClassName, string lpWindowName);
    [DllImport("user32.dll")]
    public static extern bool GetWindowRect(IntPtr hWnd, out RECT lpRect);
    [StructLayout(LayoutKind.Sequential)]
    public struct RECT {{
        public int Left, Top, Right, Bottom;
    }}
}}
"@
$hwnd = [WindowCapture]::FindWindow($null, "{title}")
if ($hwnd -eq [IntPtr]::Zero) {{ throw "Window not found: {title}" }}
$rect = New-Object WindowCapture+RECT
[WindowCapture]::GetWindowRect($hwnd, [ref]$rect)
$w = $rect.Right - $rect.Left
$h = $rect.Bottom - $rect.Top
$bitmap = New-Object System.Drawing.Bitmap($w, $h)
$graphics = [System.Drawing.Graphics]::FromImage($bitmap)
$graphics.CopyFromScreen($rect.Left, $rect.Top, 0, 0, (New-Object System.Drawing.Size($w, $h)))
$bitmap.Save("{path}")
$graphics.Dispose()
$bitmap.Dispose()
"#,
            title = title.replace('"', "`\""),
            path = tmp.to_string_lossy().replace('\\', "\\\\"),
        );
        let status = std::process::Command::new("powershell")
            .args(["-NoProfile", "-Command", &script])
            .status()
            .map_err(|e| anyhow::anyhow!("Window capture failed: {}", e))?;
        if !status.success() {
            return Err(anyhow::anyhow!("Window capture exited with {:?}", status.code()));
        }
    }

    #[cfg(target_os = "macos")]
    {
        // Use screencapture -l (window ID) — need to find window ID first
        let list_output = std::process::Command::new("osascript")
            .args(["-e", &format!(
                "tell application \"System Events\" to get (id of first window of (first process whose name is \"{}\"))",
                title
            )])
            .output()
            .map_err(|e| anyhow::anyhow!("osascript window lookup failed: {}", e))?;
        let window_id = String::from_utf8_lossy(&list_output.stdout).trim().to_string();

        let status = std::process::Command::new("screencapture")
            .args(["-x", "-t", "png", "-l", &window_id, &tmp.to_string_lossy()])
            .status()
            .map_err(|e| anyhow::anyhow!("screencapture window failed: {}", e))?;
        if !status.success() {
            return Err(anyhow::anyhow!("screencapture exited with {:?}", status.code()));
        }
    }

    #[cfg(target_os = "linux")]
    {
        // xdotool to find window, then import by window id
        let output = std::process::Command::new("xdotool")
            .args(["search", "--name", title])
            .output()
            .map_err(|e| anyhow::anyhow!("xdotool search failed: {}", e))?;
        let wid = String::from_utf8_lossy(&output.stdout)
            .lines()
            .next()
            .unwrap_or("0")
            .trim()
            .to_string();

        let status = std::process::Command::new("import")
            .args(["-window", &wid, &tmp.to_string_lossy().to_string()])
            .status()
            .map_err(|e| anyhow::anyhow!("import window capture failed: {}", e))?;
        if !status.success() {
            return Err(anyhow::anyhow!("import exited with {:?}", status.code()));
        }
    }

    let data = std::fs::read(&tmp)?;
    let _ = std::fs::remove_file(&tmp);
    Ok(data)
}

/// Encode raw PNG bytes to base64 string.
pub fn png_to_base64(png_data: &[u8]) -> String {
    // Simple base64 encoding without external dependency
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = String::with_capacity((png_data.len() + 2) / 3 * 4);
    for chunk in png_data.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = if chunk.len() > 1 { chunk[1] as u32 } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] as u32 } else { 0 };
        let triple = (b0 << 16) | (b1 << 8) | b2;
        result.push(CHARS[((triple >> 18) & 0x3F) as usize] as char);
        result.push(CHARS[((triple >> 12) & 0x3F) as usize] as char);
        if chunk.len() > 1 {
            result.push(CHARS[((triple >> 6) & 0x3F) as usize] as char);
        } else {
            result.push('=');
        }
        if chunk.len() > 2 {
            result.push(CHARS[(triple & 0x3F) as usize] as char);
        } else {
            result.push('=');
        }
    }
    result
}

/// Get PNG image dimensions from the header.
pub fn png_dimensions(png_data: &[u8]) -> Option<(u32, u32)> {
    // PNG IHDR: bytes 16-19 = width, 20-23 = height (big-endian)
    if png_data.len() < 24 {
        return None;
    }
    // Check PNG signature
    if &png_data[0..8] != b"\x89PNG\r\n\x1a\n" {
        return None;
    }
    let width = u32::from_be_bytes([png_data[16], png_data[17], png_data[18], png_data[19]]);
    let height = u32::from_be_bytes([png_data[20], png_data[21], png_data[22], png_data[23]]);
    Some((width, height))
}

fn temp_screenshot_path() -> PathBuf {
    let mut path = std::env::temp_dir();
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    path.push(format!("dx_screenshot_{}.png", ts));
    path
}
