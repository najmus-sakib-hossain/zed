//! # Screen Capture Integration
//!
//! Screen capture and recording functionality.
//!
//! ## Usage
//!
//! ```rust,ignore
//! use driven::integrations::screen::{ScreenClient, ScreenConfig};
//!
//! let config = ScreenConfig::from_file("~/.dx/config/screen.sr")?;
//! let client = ScreenClient::new(&config)?;
//!
//! // Capture screenshot
//! let screenshot = client.capture_screenshot().await?;
//!
//! // Capture specific window
//! let window = client.capture_window("Terminal").await?;
//! ```

use crate::error::{DrivenError, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Screen configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScreenConfig {
    /// Whether screen integration is enabled
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Output directory
    #[serde(default = "default_output_dir")]
    pub output_dir: PathBuf,
    /// Default format (png, jpg, gif)
    #[serde(default = "default_format")]
    pub default_format: String,
    /// Include cursor in screenshots
    #[serde(default)]
    pub include_cursor: bool,
    /// Default monitor (0 = primary, -1 = all)
    #[serde(default)]
    pub default_monitor: i32,
}

fn default_true() -> bool {
    true
}

fn default_output_dir() -> PathBuf {
    dirs::picture_dir().unwrap_or_else(|| PathBuf::from(".")).join("Screenshots")
}

fn default_format() -> String {
    "png".to_string()
}

impl Default for ScreenConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            output_dir: default_output_dir(),
            default_format: default_format(),
            include_cursor: false,
            default_monitor: 0,
        }
    }
}

impl ScreenConfig {
    /// Load from .sr config file
    pub fn from_file(path: impl AsRef<std::path::Path>) -> Result<Self> {
        let content = std::fs::read_to_string(path.as_ref())
            .map_err(|e| DrivenError::Io(e))?;
        Self::parse_sr(&content)
    }

    fn parse_sr(_content: &str) -> Result<Self> {
        Ok(Self::default())
    }
}

/// Display/Monitor info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisplayInfo {
    /// Display ID
    pub id: u32,
    /// Display name
    pub name: String,
    /// Width
    pub width: u32,
    /// Height
    pub height: u32,
    /// Scale factor
    pub scale: f32,
    /// Is primary display
    pub is_primary: bool,
    /// X position
    pub x: i32,
    /// Y position
    pub y: i32,
}

/// Window info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowInfo {
    /// Window ID
    pub id: u64,
    /// Window title
    pub title: String,
    /// Owner application
    pub app: String,
    /// Window bounds
    pub bounds: WindowBounds,
    /// Is visible
    pub is_visible: bool,
    /// Is on screen
    pub is_on_screen: bool,
}

/// Window bounds
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowBounds {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

/// Screenshot result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Screenshot {
    /// File path
    pub path: PathBuf,
    /// Width
    pub width: u32,
    /// Height
    pub height: u32,
    /// File size in bytes
    pub size: u64,
    /// Format
    pub format: String,
    /// Timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Screen recording
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScreenRecording {
    /// File path
    pub path: PathBuf,
    /// Duration in seconds
    pub duration: f64,
    /// Width
    pub width: u32,
    /// Height
    pub height: u32,
    /// FPS
    pub fps: u32,
    /// File size in bytes
    pub size: u64,
    /// Timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Region selection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Region {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

/// Screen client
pub struct ScreenClient {
    config: ScreenConfig,
    is_recording: bool,
    recording_process: Option<u32>,
}

impl ScreenClient {
    /// Create a new screen client
    pub fn new(config: &ScreenConfig) -> Result<Self> {
        std::fs::create_dir_all(&config.output_dir)
            .map_err(|e| DrivenError::Io(e))?;

        Ok(Self {
            config: config.clone(),
            is_recording: false,
            recording_process: None,
        })
    }

    /// Check if available
    pub fn is_available(&self) -> bool {
        self.config.enabled
    }

    /// List displays/monitors
    pub async fn list_displays(&self) -> Result<Vec<DisplayInfo>> {
        #[cfg(target_os = "macos")]
        {
            self.list_displays_macos().await
        }

        #[cfg(target_os = "windows")]
        {
            self.list_displays_windows().await
        }

        #[cfg(target_os = "linux")]
        {
            self.list_displays_linux().await
        }

        #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
        {
            Ok(Vec::new())
        }
    }

    #[cfg(target_os = "macos")]
    async fn list_displays_macos(&self) -> Result<Vec<DisplayInfo>> {
        use tokio::process::Command;

        let script = r#"
            tell application "System Events"
                set displayList to {}
                repeat with d in desktops
                    set displayInfo to (name of d) & "|" & (id of d)
                    set end of displayList to displayInfo
                end repeat
                return displayList
            end tell
        "#;

        let output = Command::new("osascript")
            .args(["-e", script])
            .output()
            .await;

        // Fallback to system_profiler
        let output = Command::new("system_profiler")
            .args(["SPDisplaysDataType", "-json"])
            .output()
            .await
            .map_err(|e| DrivenError::Process(e.to_string()))?;

        let json: serde_json::Value = serde_json::from_slice(&output.stdout)
            .unwrap_or_default();

        let mut displays = Vec::new();

        if let Some(data) = json["SPDisplaysDataType"].as_array() {
            for (idx, gpu) in data.iter().enumerate() {
                if let Some(monitors) = gpu["spdisplays_ndrvs"].as_array() {
                    for (m_idx, monitor) in monitors.iter().enumerate() {
                        let resolution = monitor["_spdisplays_resolution"]
                            .as_str()
                            .unwrap_or("1920 x 1080");
                        let parts: Vec<&str> = resolution.split(" x ").collect();
                        let width: u32 = parts.get(0).and_then(|s| s.parse().ok()).unwrap_or(1920);
                        let height: u32 = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(1080);

                        displays.push(DisplayInfo {
                            id: (idx * 10 + m_idx) as u32,
                            name: monitor["_name"].as_str().unwrap_or("Display").to_string(),
                            width,
                            height,
                            scale: 1.0,
                            is_primary: idx == 0 && m_idx == 0,
                            x: 0,
                            y: 0,
                        });
                    }
                }
            }
        }

        if displays.is_empty() {
            // Default display
            displays.push(DisplayInfo {
                id: 0,
                name: "Primary Display".to_string(),
                width: 1920,
                height: 1080,
                scale: 1.0,
                is_primary: true,
                x: 0,
                y: 0,
            });
        }

        Ok(displays)
    }

    #[cfg(target_os = "windows")]
    async fn list_displays_windows(&self) -> Result<Vec<DisplayInfo>> {
        use tokio::process::Command;

        let script = r#"
            Add-Type -AssemblyName System.Windows.Forms
            [System.Windows.Forms.Screen]::AllScreens | ForEach-Object {
                [PSCustomObject]@{
                    Name = $_.DeviceName
                    Width = $_.Bounds.Width
                    Height = $_.Bounds.Height
                    X = $_.Bounds.X
                    Y = $_.Bounds.Y
                    Primary = $_.Primary
                }
            } | ConvertTo-Json
        "#;

        let output = Command::new("powershell")
            .args(["-Command", script])
            .output()
            .await
            .map_err(|e| DrivenError::Process(e.to_string()))?;

        let json_str = String::from_utf8_lossy(&output.stdout);
        let screens: Vec<serde_json::Value> = serde_json::from_str(&json_str)
            .unwrap_or_else(|_| Vec::new());

        let displays = screens
            .into_iter()
            .enumerate()
            .map(|(idx, s)| DisplayInfo {
                id: idx as u32,
                name: s["Name"].as_str().unwrap_or("Display").to_string(),
                width: s["Width"].as_u64().unwrap_or(1920) as u32,
                height: s["Height"].as_u64().unwrap_or(1080) as u32,
                scale: 1.0,
                is_primary: s["Primary"].as_bool().unwrap_or(false),
                x: s["X"].as_i64().unwrap_or(0) as i32,
                y: s["Y"].as_i64().unwrap_or(0) as i32,
            })
            .collect();

        Ok(displays)
    }

    #[cfg(target_os = "linux")]
    async fn list_displays_linux(&self) -> Result<Vec<DisplayInfo>> {
        use tokio::process::Command;

        let output = Command::new("xrandr")
            .args(["--query"])
            .output()
            .await
            .map_err(|e| DrivenError::Process(e.to_string()))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut displays = Vec::new();
        let mut idx = 0;

        for line in stdout.lines() {
            if line.contains(" connected") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                let name = parts.get(0).unwrap_or(&"Display").to_string();
                let is_primary = line.contains("primary");

                // Parse resolution
                let mut width = 1920u32;
                let mut height = 1080u32;

                for part in &parts {
                    if part.contains('x') && part.contains('+') {
                        let res: Vec<&str> = part.split('+').next()
                            .unwrap_or("1920x1080")
                            .split('x')
                            .collect();
                        width = res.get(0).and_then(|s| s.parse().ok()).unwrap_or(1920);
                        height = res.get(1).and_then(|s| s.parse().ok()).unwrap_or(1080);
                        break;
                    }
                }

                displays.push(DisplayInfo {
                    id: idx,
                    name,
                    width,
                    height,
                    scale: 1.0,
                    is_primary,
                    x: 0,
                    y: 0,
                });
                idx += 1;
            }
        }

        Ok(displays)
    }

    /// List windows
    pub async fn list_windows(&self) -> Result<Vec<WindowInfo>> {
        #[cfg(target_os = "macos")]
        {
            self.list_windows_macos().await
        }

        #[cfg(target_os = "windows")]
        {
            self.list_windows_windows().await
        }

        #[cfg(target_os = "linux")]
        {
            self.list_windows_linux().await
        }

        #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
        {
            Ok(Vec::new())
        }
    }

    #[cfg(target_os = "macos")]
    async fn list_windows_macos(&self) -> Result<Vec<WindowInfo>> {
        use tokio::process::Command;

        let script = r#"
            tell application "System Events"
                set windowList to {}
                repeat with proc in (every process whose visible is true)
                    repeat with win in windows of proc
                        set windowInfo to (name of proc) & "|" & (name of win)
                        set end of windowList to windowInfo
                    end repeat
                end repeat
                return windowList
            end tell
        "#;

        let output = Command::new("osascript")
            .args(["-e", script])
            .output()
            .await
            .map_err(|e| DrivenError::Process(e.to_string()))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut windows = Vec::new();

        for (idx, line) in stdout.split(", ").enumerate() {
            let parts: Vec<&str> = line.split('|').collect();
            if parts.len() >= 2 {
                windows.push(WindowInfo {
                    id: idx as u64,
                    title: parts[1].trim().to_string(),
                    app: parts[0].to_string(),
                    bounds: WindowBounds {
                        x: 0,
                        y: 0,
                        width: 800,
                        height: 600,
                    },
                    is_visible: true,
                    is_on_screen: true,
                });
            }
        }

        Ok(windows)
    }

    #[cfg(target_os = "windows")]
    async fn list_windows_windows(&self) -> Result<Vec<WindowInfo>> {
        use tokio::process::Command;

        let script = r#"
            Get-Process | Where-Object { $_.MainWindowTitle -ne '' } |
            Select-Object Id, ProcessName, MainWindowTitle |
            ConvertTo-Json
        "#;

        let output = Command::new("powershell")
            .args(["-Command", script])
            .output()
            .await
            .map_err(|e| DrivenError::Process(e.to_string()))?;

        let json_str = String::from_utf8_lossy(&output.stdout);
        let procs: Vec<serde_json::Value> = serde_json::from_str(&json_str)
            .unwrap_or_else(|_| Vec::new());

        let windows = procs
            .into_iter()
            .map(|p| WindowInfo {
                id: p["Id"].as_u64().unwrap_or(0),
                title: p["MainWindowTitle"].as_str().unwrap_or("").to_string(),
                app: p["ProcessName"].as_str().unwrap_or("").to_string(),
                bounds: WindowBounds {
                    x: 0,
                    y: 0,
                    width: 800,
                    height: 600,
                },
                is_visible: true,
                is_on_screen: true,
            })
            .collect();

        Ok(windows)
    }

    #[cfg(target_os = "linux")]
    async fn list_windows_linux(&self) -> Result<Vec<WindowInfo>> {
        use tokio::process::Command;

        let output = Command::new("wmctrl")
            .args(["-l"])
            .output()
            .await
            .map_err(|e| DrivenError::Process(e.to_string()))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let windows = stdout
            .lines()
            .filter_map(|line| {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 4 {
                    let id = u64::from_str_radix(parts[0].trim_start_matches("0x"), 16).unwrap_or(0);
                    let title = parts[3..].join(" ");
                    Some(WindowInfo {
                        id,
                        title,
                        app: parts.get(2).unwrap_or(&"").to_string(),
                        bounds: WindowBounds {
                            x: 0,
                            y: 0,
                            width: 800,
                            height: 600,
                        },
                        is_visible: true,
                        is_on_screen: true,
                    })
                } else {
                    None
                }
            })
            .collect();

        Ok(windows)
    }

    /// Capture full screen screenshot
    pub async fn capture_screenshot(&self) -> Result<Screenshot> {
        let filename = format!(
            "screenshot_{}.{}",
            chrono::Utc::now().format("%Y%m%d_%H%M%S"),
            self.config.default_format
        );
        let path = self.config.output_dir.join(&filename);

        #[cfg(target_os = "macos")]
        {
            self.capture_macos(&path, None).await
        }

        #[cfg(target_os = "windows")]
        {
            self.capture_windows(&path, None).await
        }

        #[cfg(target_os = "linux")]
        {
            self.capture_linux(&path, None).await
        }

        #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
        {
            Err(DrivenError::Unsupported("Screenshot not supported".into()))
        }
    }

    /// Capture a region
    pub async fn capture_region(&self, region: &Region) -> Result<Screenshot> {
        let filename = format!(
            "screenshot_{}.{}",
            chrono::Utc::now().format("%Y%m%d_%H%M%S"),
            self.config.default_format
        );
        let path = self.config.output_dir.join(&filename);

        #[cfg(target_os = "macos")]
        {
            self.capture_macos(&path, Some(region)).await
        }

        #[cfg(target_os = "windows")]
        {
            self.capture_windows(&path, Some(region)).await
        }

        #[cfg(target_os = "linux")]
        {
            self.capture_linux(&path, Some(region)).await
        }

        #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
        {
            Err(DrivenError::Unsupported("Screenshot not supported".into()))
        }
    }

    #[cfg(target_os = "macos")]
    async fn capture_macos(&self, path: &std::path::Path, region: Option<&Region>) -> Result<Screenshot> {
        use tokio::process::Command;

        let mut cmd = Command::new("screencapture");
        
        if !self.config.include_cursor {
            cmd.arg("-x"); // No sound, no cursor
        }

        if let Some(r) = region {
            cmd.arg("-R")
                .arg(format!("{},{},{},{}", r.x, r.y, r.width, r.height));
        }

        cmd.arg(path);

        let status = cmd
            .status()
            .await
            .map_err(|e| DrivenError::Process(e.to_string()))?;

        if !status.success() {
            return Err(DrivenError::Process("Screenshot failed".into()));
        }

        let metadata = tokio::fs::metadata(path)
            .await
            .map_err(|e| DrivenError::Io(e))?;

        Ok(Screenshot {
            path: path.to_path_buf(),
            width: region.map(|r| r.width).unwrap_or(1920),
            height: region.map(|r| r.height).unwrap_or(1080),
            size: metadata.len(),
            format: self.config.default_format.clone(),
            timestamp: chrono::Utc::now(),
        })
    }

    #[cfg(target_os = "windows")]
    async fn capture_windows(&self, path: &std::path::Path, region: Option<&Region>) -> Result<Screenshot> {
        use tokio::process::Command;

        // Use PowerShell with .NET
        let script = if let Some(r) = region {
            format!(
                r#"
                Add-Type -AssemblyName System.Windows.Forms
                Add-Type -AssemblyName System.Drawing
                $bitmap = New-Object Drawing.Bitmap({}, {})
                $graphics = [Drawing.Graphics]::FromImage($bitmap)
                $graphics.CopyFromScreen({}, {}, 0, 0, $bitmap.Size)
                $bitmap.Save("{}")
                "#,
                r.width, r.height, r.x, r.y, path.display()
            )
        } else {
            format!(
                r#"
                Add-Type -AssemblyName System.Windows.Forms
                Add-Type -AssemblyName System.Drawing
                $screen = [System.Windows.Forms.Screen]::PrimaryScreen
                $bitmap = New-Object Drawing.Bitmap($screen.Bounds.Width, $screen.Bounds.Height)
                $graphics = [Drawing.Graphics]::FromImage($bitmap)
                $graphics.CopyFromScreen($screen.Bounds.Location, [Drawing.Point]::Empty, $screen.Bounds.Size)
                $bitmap.Save("{}")
                "#,
                path.display()
            )
        };

        let status = Command::new("powershell")
            .args(["-Command", &script])
            .status()
            .await
            .map_err(|e| DrivenError::Process(e.to_string()))?;

        if !status.success() {
            return Err(DrivenError::Process("Screenshot failed".into()));
        }

        let metadata = tokio::fs::metadata(path)
            .await
            .map_err(|e| DrivenError::Io(e))?;

        Ok(Screenshot {
            path: path.to_path_buf(),
            width: region.map(|r| r.width).unwrap_or(1920),
            height: region.map(|r| r.height).unwrap_or(1080),
            size: metadata.len(),
            format: self.config.default_format.clone(),
            timestamp: chrono::Utc::now(),
        })
    }

    #[cfg(target_os = "linux")]
    async fn capture_linux(&self, path: &std::path::Path, region: Option<&Region>) -> Result<Screenshot> {
        use tokio::process::Command;

        let mut cmd = Command::new("scrot");

        if let Some(r) = region {
            cmd.arg("-a")
                .arg(format!("{},{},{},{}", r.x, r.y, r.width, r.height));
        }

        cmd.arg(path);

        let status = cmd
            .status()
            .await
            .map_err(|e| DrivenError::Process(e.to_string()))?;

        if !status.success() {
            return Err(DrivenError::Process("Screenshot failed".into()));
        }

        let metadata = tokio::fs::metadata(path)
            .await
            .map_err(|e| DrivenError::Io(e))?;

        Ok(Screenshot {
            path: path.to_path_buf(),
            width: region.map(|r| r.width).unwrap_or(1920),
            height: region.map(|r| r.height).unwrap_or(1080),
            size: metadata.len(),
            format: self.config.default_format.clone(),
            timestamp: chrono::Utc::now(),
        })
    }

    /// Capture a window by title
    pub async fn capture_window(&self, title: &str) -> Result<Screenshot> {
        let filename = format!(
            "window_{}.{}",
            chrono::Utc::now().format("%Y%m%d_%H%M%S"),
            self.config.default_format
        );
        let path = self.config.output_dir.join(&filename);

        #[cfg(target_os = "macos")]
        {
            use tokio::process::Command;

            let status = Command::new("screencapture")
                .args(["-l", &self.find_window_id_macos(title).await?, "-x"])
                .arg(&path)
                .status()
                .await
                .map_err(|e| DrivenError::Process(e.to_string()))?;

            if !status.success() {
                return Err(DrivenError::Process("Window capture failed".into()));
            }
        }

        #[cfg(not(target_os = "macos"))]
        {
            // On other platforms, fall back to full screen
            return self.capture_screenshot().await;
        }

        let metadata = tokio::fs::metadata(&path)
            .await
            .map_err(|e| DrivenError::Io(e))?;

        Ok(Screenshot {
            path,
            width: 800,
            height: 600,
            size: metadata.len(),
            format: self.config.default_format.clone(),
            timestamp: chrono::Utc::now(),
        })
    }

    #[cfg(target_os = "macos")]
    async fn find_window_id_macos(&self, title: &str) -> Result<String> {
        use tokio::process::Command;

        let script = format!(
            r#"
            tell application "System Events"
                set targetWindow to ""
                repeat with proc in (every process whose visible is true)
                    repeat with win in windows of proc
                        if name of win contains "{}" then
                            return id of win
                        end if
                    end repeat
                end repeat
                return ""
            end tell
            "#,
            title
        );

        let output = Command::new("osascript")
            .args(["-e", &script])
            .output()
            .await
            .map_err(|e| DrivenError::Process(e.to_string()))?;

        let id = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if id.is_empty() {
            return Err(DrivenError::NotFound(format!("Window '{}' not found", title)));
        }

        Ok(id)
    }

    /// Check if recording
    pub fn is_recording(&self) -> bool {
        self.is_recording
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = ScreenConfig::default();
        assert!(config.enabled);
        assert_eq!(config.default_format, "png");
    }

    #[test]
    fn test_region() {
        let region = Region {
            x: 100,
            y: 100,
            width: 800,
            height: 600,
        };
        assert_eq!(region.width, 800);
    }
}
