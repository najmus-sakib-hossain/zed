//! Platform-specific mouse and keyboard input simulation.
//!
//! - **Windows**: Uses `SendInput` via PowerShell interop (no FFI needed)
//! - **macOS**: Uses `osascript` / `cliclick` for mouse, System Events for keyboard
//! - **Linux**: Uses `xdotool` (X11) or `ydotool` (Wayland)

use anyhow::Result;

/// Send a mouse move to absolute screen coordinates.
pub fn mouse_move(x: i32, y: i32) -> Result<()> {
    #[cfg(target_os = "windows")]
    {
        // Use PowerShell to call .NET System.Windows.Forms.Cursor
        let script = format!(
            "[System.Windows.Forms.Cursor]::Position = New-Object System.Drawing.Point({}, {})",
            x, y
        );
        run_powershell(&script)?;
    }

    #[cfg(target_os = "macos")]
    {
        // Use cliclick (brew install cliclick) or CoreGraphics via osascript
        let script = format!(
            "do shell script \"cliclick m:{},{}\" ",
            x, y
        );
        run_osascript(&script)?;
    }

    #[cfg(target_os = "linux")]
    {
        run_command("xdotool", &["mousemove", &x.to_string(), &y.to_string()])?;
    }

    Ok(())
}

/// Send a mouse click at the current cursor position.
pub fn mouse_click(button: MouseBtn, count: u32) -> Result<()> {
    let btn_str = match button {
        MouseBtn::Left => "1",
        MouseBtn::Right => "3",
        MouseBtn::Middle => "2",
    };

    #[cfg(target_os = "windows")]
    {
        // PowerShell: use Add-Type with user32.dll mouse_event
        let (down_flag, up_flag) = match button {
            MouseBtn::Left => ("0x0002", "0x0004"),   // MOUSEEVENTF_LEFTDOWN / UP
            MouseBtn::Right => ("0x0008", "0x0010"),   // MOUSEEVENTF_RIGHTDOWN / UP
            MouseBtn::Middle => ("0x0020", "0x0040"),  // MOUSEEVENTF_MIDDLEDOWN / UP
        };
        let script = format!(
            r#"
Add-Type -TypeDefinition @"
using System;
using System.Runtime.InteropServices;
public class MouseInput {{
    [DllImport("user32.dll")]
    public static extern void mouse_event(uint dwFlags, int dx, int dy, uint dwData, IntPtr dwExtraInfo);
}}
"@
for ($i = 0; $i -lt {count}; $i++) {{
    [MouseInput]::mouse_event({down}, 0, 0, 0, [IntPtr]::Zero)
    [MouseInput]::mouse_event({up}, 0, 0, 0, [IntPtr]::Zero)
    Start-Sleep -Milliseconds 50
}}
"#,
            count = count,
            down = down_flag,
            up = up_flag,
        );
        run_powershell(&script)?;
    }

    #[cfg(target_os = "macos")]
    {
        let click_char = match button {
            MouseBtn::Left => "c",
            MouseBtn::Right => "rc",
            MouseBtn::Middle => "mc",
        };
        for _ in 0..count {
            // cliclick: c:. for click at current pos
            let script = format!("do shell script \"cliclick {}:.\"", click_char);
            run_osascript(&script)?;
        }
    }

    #[cfg(target_os = "linux")]
    {
        for _ in 0..count {
            run_command("xdotool", &["click", btn_str])?;
        }
    }

    let _ = btn_str; // suppress unused on some platforms
    Ok(())
}

/// Drag from current position to (to_x, to_y).
pub fn mouse_drag(to_x: i32, to_y: i32) -> Result<()> {
    #[cfg(target_os = "windows")]
    {
        let script = format!(
            r#"
Add-Type -TypeDefinition @"
using System;
using System.Runtime.InteropServices;
public class MouseDrag {{
    [DllImport("user32.dll")]
    public static extern void mouse_event(uint dwFlags, int dx, int dy, uint dwData, IntPtr dwExtraInfo);
    [DllImport("user32.dll")]
    public static extern bool SetCursorPos(int X, int Y);
}}
"@
[MouseDrag]::mouse_event(0x0002, 0, 0, 0, [IntPtr]::Zero)
Start-Sleep -Milliseconds 50
[MouseDrag]::SetCursorPos({x}, {y})
Start-Sleep -Milliseconds 50
[MouseDrag]::mouse_event(0x0004, 0, 0, 0, [IntPtr]::Zero)
"#,
            x = to_x,
            y = to_y,
        );
        run_powershell(&script)?;
    }

    #[cfg(target_os = "macos")]
    {
        let script = format!(
            "do shell script \"cliclick dd:. du:{},{}\"",
            to_x, to_y
        );
        run_osascript(&script)?;
    }

    #[cfg(target_os = "linux")]
    {
        // xdotool mousemove --sync combined with mousedown/mouseup
        run_command("xdotool", &["mousedown", "1"])?;
        run_command(
            "xdotool",
            &["mousemove", "--sync", &to_x.to_string(), &to_y.to_string()],
        )?;
        run_command("xdotool", &["mouseup", "1"])?;
    }

    Ok(())
}

/// Scroll the mouse wheel.
pub fn scroll(delta_x: i32, delta_y: i32) -> Result<()> {
    #[cfg(target_os = "windows")]
    {
        if delta_y != 0 {
            // MOUSEEVENTF_WHEEL = 0x0800, dwData = distance in multiples of WHEEL_DELTA (120)
            let wheel_amount = delta_y * 120;
            let script = format!(
                r#"
Add-Type -TypeDefinition @"
using System;
using System.Runtime.InteropServices;
public class MouseScroll {{
    [DllImport("user32.dll")]
    public static extern void mouse_event(uint dwFlags, int dx, int dy, uint dwData, IntPtr dwExtraInfo);
}}
"@
[MouseScroll]::mouse_event(0x0800, 0, 0, {amount}, [IntPtr]::Zero)
"#,
                amount = wheel_amount,
            );
            run_powershell(&script)?;
        }
        if delta_x != 0 {
            // MOUSEEVENTF_HWHEEL = 0x01000
            let wheel_amount = delta_x * 120;
            let script = format!(
                r#"
Add-Type -TypeDefinition @"
using System;
using System.Runtime.InteropServices;
public class MouseHScroll {{
    [DllImport("user32.dll")]
    public static extern void mouse_event(uint dwFlags, int dx, int dy, uint dwData, IntPtr dwExtraInfo);
}}
"@
[MouseHScroll]::mouse_event(0x01000, 0, 0, {amount}, [IntPtr]::Zero)
"#,
                amount = wheel_amount,
            );
            run_powershell(&script)?;
        }
    }

    #[cfg(target_os = "macos")]
    {
        // osascript doesn't support scroll directly; use cliclick or AppleScript hack
        if delta_y != 0 {
            let dir = if delta_y > 0 { "u" } else { "d" };
            let steps = delta_y.unsigned_abs();
            for _ in 0..steps {
                let script = format!("do shell script \"cliclick kp:{}\"", if dir == "u" { "page-up" } else { "page-down" });
                let _ = run_osascript(&script);
            }
        }
    }

    #[cfg(target_os = "linux")]
    {
        if delta_y > 0 {
            for _ in 0..delta_y {
                run_command("xdotool", &["click", "4"])?; // scroll up
            }
        } else if delta_y < 0 {
            for _ in 0..(-delta_y) {
                run_command("xdotool", &["click", "5"])?; // scroll down
            }
        }
        if delta_x > 0 {
            for _ in 0..delta_x {
                run_command("xdotool", &["click", "6"])?; // scroll left
            }
        } else if delta_x < 0 {
            for _ in 0..(-delta_x) {
                run_command("xdotool", &["click", "7"])?; // scroll right
            }
        }
    }

    Ok(())
}

/// Type a text string by simulating keystrokes.
pub fn type_text(text: &str) -> Result<()> {
    #[cfg(target_os = "windows")]
    {
        // Use SendKeys via PowerShell — need to escape special chars
        let escaped = text
            .replace('{', "{{}")
            .replace('}', "{}}")
            .replace('+', "{+}")
            .replace('^', "{^}")
            .replace('%', "{%}")
            .replace('~', "{~}")
            .replace('(', "{(}")
            .replace(')', "{)}")
            .replace('[', "{[}")
            .replace(']', "{]}")
            ;
        let script = format!(
            r#"
Add-Type -AssemblyName System.Windows.Forms
[System.Windows.Forms.SendKeys]::SendWait("{text}")
"#,
            text = escaped,
        );
        run_powershell(&script)?;
    }

    #[cfg(target_os = "macos")]
    {
        // Use System Events to type text
        let escaped = text.replace('\\', "\\\\").replace('"', "\\\"");
        let script = format!(
            "tell application \"System Events\" to keystroke \"{}\"",
            escaped
        );
        run_osascript(&script)?;
    }

    #[cfg(target_os = "linux")]
    {
        run_command("xdotool", &["type", "--clearmodifiers", text])?;
    }

    Ok(())
}

/// Press a key combination (e.g., ["ctrl", "c"] for Ctrl+C).
pub fn key_press(keys: &[&str]) -> Result<()> {
    #[cfg(target_os = "windows")]
    {
        // Convert to SendKeys format
        let sendkeys = keys_to_sendkeys(keys);
        let script = format!(
            r#"
Add-Type -AssemblyName System.Windows.Forms
[System.Windows.Forms.SendKeys]::SendWait("{}")
"#,
            sendkeys,
        );
        run_powershell(&script)?;
    }

    #[cfg(target_os = "macos")]
    {
        // Build osascript key command
        let (modifiers, key_char) = parse_key_combo_mac(keys);
        let script = if modifiers.is_empty() {
            format!(
                "tell application \"System Events\" to key code {}",
                key_char
            )
        } else {
            format!(
                "tell application \"System Events\" to keystroke \"{}\" using {{{}}}",
                key_char,
                modifiers.join(", ")
            )
        };
        run_osascript(&script)?;
    }

    #[cfg(target_os = "linux")]
    {
        // xdotool key format: "ctrl+c", "alt+F4", etc.
        let combo = keys_to_xdotool(keys);
        run_command("xdotool", &["key", &combo])?;
    }

    Ok(())
}

/// Get the current cursor position.
pub fn cursor_position() -> Result<(i32, i32)> {
    #[cfg(target_os = "windows")]
    {
        let output = std::process::Command::new("powershell")
            .args([
                "-NoProfile",
                "-Command",
                "Add-Type -AssemblyName System.Windows.Forms; $p = [System.Windows.Forms.Cursor]::Position; \"$($p.X),$($p.Y)\"",
            ])
            .output()
            .map_err(|e| anyhow::anyhow!("powershell failed: {}", e))?;
        let text = String::from_utf8_lossy(&output.stdout);
        let parts: Vec<&str> = text.trim().split(',').collect();
        if parts.len() == 2 {
            let x: i32 = parts[0].parse().unwrap_or(0);
            let y: i32 = parts[1].parse().unwrap_or(0);
            return Ok((x, y));
        }
        Ok((0, 0))
    }

    #[cfg(target_os = "macos")]
    {
        let output = std::process::Command::new("cliclick")
            .arg("p")
            .output()
            .map_err(|e| anyhow::anyhow!("cliclick failed: {}", e))?;
        let text = String::from_utf8_lossy(&output.stdout);
        // cliclick p outputs: "x,y"
        let parts: Vec<&str> = text.trim().split(',').collect();
        if parts.len() == 2 {
            let x: i32 = parts[0].parse().unwrap_or(0);
            let y: i32 = parts[1].parse().unwrap_or(0);
            return Ok((x, y));
        }
        Ok((0, 0))
    }

    #[cfg(target_os = "linux")]
    {
        let output = std::process::Command::new("xdotool")
            .args(["getmouselocation", "--shell"])
            .output()
            .map_err(|e| anyhow::anyhow!("xdotool failed: {}", e))?;
        let text = String::from_utf8_lossy(&output.stdout);
        let mut x = 0i32;
        let mut y = 0i32;
        for line in text.lines() {
            if let Some(val) = line.strip_prefix("X=") {
                x = val.parse().unwrap_or(0);
            } else if let Some(val) = line.strip_prefix("Y=") {
                y = val.parse().unwrap_or(0);
            }
        }
        return Ok((x, y));
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    {
        Ok((0, 0))
    }
}

/// Get the screen resolution.
pub fn screen_size() -> Result<(u32, u32)> {
    #[cfg(target_os = "windows")]
    {
        let output = std::process::Command::new("powershell")
            .args([
                "-NoProfile",
                "-Command",
                "Add-Type -AssemblyName System.Windows.Forms; $s = [System.Windows.Forms.Screen]::PrimaryScreen.Bounds; \"$($s.Width),$($s.Height)\"",
            ])
            .output()
            .map_err(|e| anyhow::anyhow!("powershell failed: {}", e))?;
        let text = String::from_utf8_lossy(&output.stdout);
        let parts: Vec<&str> = text.trim().split(',').collect();
        if parts.len() == 2 {
            let w: u32 = parts[0].parse().unwrap_or(1920);
            let h: u32 = parts[1].parse().unwrap_or(1080);
            return Ok((w, h));
        }
        Ok((1920, 1080))
    }

    #[cfg(target_os = "macos")]
    {
        let output = std::process::Command::new("system_profiler")
            .args(["SPDisplaysDataType"])
            .output()
            .map_err(|e| anyhow::anyhow!("system_profiler failed: {}", e))?;
        let text = String::from_utf8_lossy(&output.stdout);
        // Look for "Resolution: 1920 x 1080" pattern
        for line in text.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("Resolution:") {
                let parts: Vec<&str> = trimmed.split_whitespace().collect();
                if parts.len() >= 4 {
                    let w: u32 = parts[1].parse().unwrap_or(1920);
                    let h: u32 = parts[3].parse().unwrap_or(1080);
                    return Ok((w, h));
                }
            }
        }
        Ok((1920, 1080))
    }

    #[cfg(target_os = "linux")]
    {
        let output = std::process::Command::new("xrandr")
            .arg("--current")
            .output()
            .map_err(|e| anyhow::anyhow!("xrandr failed: {}", e))?;
        let text = String::from_utf8_lossy(&output.stdout);
        // Look for line with "*" (current resolution)
        for line in text.lines() {
            if line.contains('*') {
                let trimmed = line.trim();
                if let Some(resolution) = trimmed.split_whitespace().next() {
                    let parts: Vec<&str> = resolution.split('x').collect();
                    if parts.len() == 2 {
                        let w: u32 = parts[0].parse().unwrap_or(1920);
                        let h: u32 = parts[1].parse().unwrap_or(1080);
                        return Ok((w, h));
                    }
                }
            }
        }
        Ok((1920, 1080))
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    {
        Ok((1920, 1080))
    }
}

// ── Mouse button type ────────────────────────────────────────────────

#[derive(Debug, Clone, Copy)]
pub enum MouseBtn {
    Left,
    Right,
    Middle,
}

// ── Platform helpers ─────────────────────────────────────────────────

#[cfg(target_os = "windows")]
fn run_powershell(script: &str) -> Result<()> {
    let status = std::process::Command::new("powershell")
        .args(["-NoProfile", "-Command", script])
        .status()
        .map_err(|e| anyhow::anyhow!("powershell failed: {}", e))?;
    if !status.success() {
        return Err(anyhow::anyhow!("PowerShell exited with {:?}", status.code()));
    }
    Ok(())
}

#[cfg(target_os = "macos")]
fn run_osascript(script: &str) -> Result<()> {
    let status = std::process::Command::new("osascript")
        .args(["-e", script])
        .status()
        .map_err(|e| anyhow::anyhow!("osascript failed: {}", e))?;
    if !status.success() {
        return Err(anyhow::anyhow!("osascript exited with {:?}", status.code()));
    }
    Ok(())
}

#[cfg(target_os = "linux")]
fn run_command(cmd: &str, args: &[&str]) -> Result<()> {
    let status = std::process::Command::new(cmd)
        .args(args)
        .status()
        .map_err(|e| anyhow::anyhow!("{} failed: {}", cmd, e))?;
    if !status.success() {
        return Err(anyhow::anyhow!("{} exited with {:?}", cmd, status.code()));
    }
    Ok(())
}

// ── Key combination converters ───────────────────────────────────────

#[cfg(target_os = "windows")]
fn keys_to_sendkeys(keys: &[&str]) -> String {
    let mut result = String::new();
    for key in keys {
        match key.to_lowercase().as_str() {
            "ctrl" | "control" => result.push('^'),
            "alt" => result.push('%'),
            "shift" => result.push('+'),
            "enter" | "return" => result.push_str("{ENTER}"),
            "tab" => result.push_str("{TAB}"),
            "escape" | "esc" => result.push_str("{ESC}"),
            "backspace" => result.push_str("{BACKSPACE}"),
            "delete" | "del" => result.push_str("{DELETE}"),
            "up" => result.push_str("{UP}"),
            "down" => result.push_str("{DOWN}"),
            "left" => result.push_str("{LEFT}"),
            "right" => result.push_str("{RIGHT}"),
            "home" => result.push_str("{HOME}"),
            "end" => result.push_str("{END}"),
            "pageup" => result.push_str("{PGUP}"),
            "pagedown" => result.push_str("{PGDN}"),
            "space" => result.push(' '),
            s if s.starts_with('f') && s.len() <= 3 => {
                // F1-F24
                result.push_str(&format!("{{{}}}", s.to_uppercase()));
            }
            s if s.len() == 1 => result.push_str(s),
            _ => result.push_str(key),
        }
    }
    result
}

#[cfg(target_os = "macos")]
fn parse_key_combo_mac(keys: &[&str]) -> (Vec<String>, String) {
    let mut modifiers = Vec::new();
    let mut key_char = String::new();
    for key in keys {
        match key.to_lowercase().as_str() {
            "ctrl" | "control" => modifiers.push("control down".to_string()),
            "alt" | "option" => modifiers.push("option down".to_string()),
            "shift" => modifiers.push("shift down".to_string()),
            "cmd" | "command" | "meta" | "super" => modifiers.push("command down".to_string()),
            other => key_char = other.to_string(),
        }
    }
    (modifiers, key_char)
}

#[cfg(target_os = "linux")]
fn keys_to_xdotool(keys: &[&str]) -> String {
    let mapped: Vec<String> = keys
        .iter()
        .map(|k| match k.to_lowercase().as_str() {
            "ctrl" | "control" => "ctrl".to_string(),
            "alt" => "alt".to_string(),
            "shift" => "shift".to_string(),
            "meta" | "super" | "cmd" | "command" => "super".to_string(),
            "enter" | "return" => "Return".to_string(),
            "tab" => "Tab".to_string(),
            "escape" | "esc" => "Escape".to_string(),
            "backspace" => "BackSpace".to_string(),
            "delete" | "del" => "Delete".to_string(),
            "up" => "Up".to_string(),
            "down" => "Down".to_string(),
            "left" => "Left".to_string(),
            "right" => "Right".to_string(),
            "home" => "Home".to_string(),
            "end" => "End".to_string(),
            "pageup" => "Prior".to_string(),
            "pagedown" => "Next".to_string(),
            "space" => "space".to_string(),
            s if s.starts_with('f') && s.len() <= 3 => s.to_uppercase(),
            other => other.to_string(),
        })
        .collect();
    mapped.join("+")
}
