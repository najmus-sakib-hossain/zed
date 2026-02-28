//! Platform-specific node capabilities.
//!
//! Provides cross-platform access to:
//! - Screen capture (via `scrap` crate concepts, using platform APIs)
//! - System command execution
//! - Platform information

use anyhow::Result;
use serde::{Deserialize, Serialize};

/// Platform node capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformCapabilities {
    pub notifications: bool,
    pub screen_capture: bool,
    pub camera: bool,
    pub location: bool,
    pub system_run: bool,
    pub platform: String,
}

impl PlatformCapabilities {
    /// Detect capabilities for the current platform
    pub fn detect() -> Self {
        Self {
            notifications: true, // notify-rust works everywhere
            screen_capture: cfg!(any(
                target_os = "windows",
                target_os = "macos",
                target_os = "linux"
            )),
            camera: false, // Requires additional setup
            location: cfg!(any(target_os = "windows", target_os = "macos")),
            system_run: true,
            platform: std::env::consts::OS.to_string(),
        }
    }
}

/// Execute a system command safely
pub async fn system_run(command: &str, args: &[&str]) -> Result<CommandOutput> {
    let output = tokio::process::Command::new(command).args(args).output().await?;

    Ok(CommandOutput {
        exit_code: output.status.code().unwrap_or(-1),
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
    })
}

/// Command execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandOutput {
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
}

/// Take a screenshot using platform-native APIs.
///
/// On Windows: Uses PowerShell + .NET to capture screen
/// On macOS: Uses `screencapture` command
/// On Linux: Uses `import` (ImageMagick) or `gnome-screenshot`
pub async fn capture_screen(output_path: &std::path::Path) -> Result<()> {
    #[cfg(target_os = "windows")]
    {
        let ps_script = format!(
            r#"
Add-Type -AssemblyName System.Windows.Forms
$screen = [System.Windows.Forms.Screen]::PrimaryScreen.Bounds
$bitmap = New-Object System.Drawing.Bitmap($screen.Width, $screen.Height)
$graphics = [System.Drawing.Graphics]::FromImage($bitmap)
$graphics.CopyFromScreen($screen.Location, [System.Drawing.Point]::Empty, $screen.Size)
$bitmap.Save("{}")
$graphics.Dispose()
$bitmap.Dispose()
"#,
            output_path.display()
        );
        let status = tokio::process::Command::new("powershell")
            .args(["-NoProfile", "-Command", &ps_script])
            .output()
            .await?;
        if !status.status.success() {
            anyhow::bail!("Screenshot failed: {}", String::from_utf8_lossy(&status.stderr));
        }
    }

    #[cfg(target_os = "macos")]
    {
        let status = tokio::process::Command::new("screencapture")
            .args(["-x", output_path.to_str().unwrap_or("")])
            .output()
            .await?;
        if !status.status.success() {
            anyhow::bail!("Screenshot failed");
        }
    }

    #[cfg(target_os = "linux")]
    {
        // Try gnome-screenshot first, then import (ImageMagick)
        let result = tokio::process::Command::new("gnome-screenshot")
            .args(["-f", output_path.to_str().unwrap_or("")])
            .output()
            .await;

        if result.is_err() || !result.as_ref().unwrap().status.success() {
            let status = tokio::process::Command::new("import")
                .args(["-window", "root", output_path.to_str().unwrap_or("")])
                .output()
                .await?;
            if !status.status.success() {
                anyhow::bail!("Screenshot failed. Install gnome-screenshot or ImageMagick.");
            }
        }
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    {
        let _ = output_path;
        anyhow::bail!("Screen capture not supported on this platform");
    }

    Ok(())
}

/// Get platform-specific location (if available).
/// Returns (latitude, longitude) or an error.
pub async fn get_location() -> Result<(f64, f64)> {
    #[cfg(target_os = "macos")]
    {
        // Use CoreLocation via swift script
        let output = tokio::process::Command::new("swift")
            .args(["-e", r#"
import CoreLocation
class Loc: NSObject, CLLocationManagerDelegate {
    let mgr = CLLocationManager()
    override init() { super.init(); mgr.delegate = self; mgr.requestWhenInUseAuthorization(); mgr.requestLocation() }
    func locationManager(_ m: CLLocationManager, didUpdateLocations l: [CLLocation]) {
        if let c = l.last?.coordinate { print("\(c.latitude),\(c.longitude)") }
        exit(0)
    }
    func locationManager(_ m: CLLocationManager, didFailWithError e: Error) { print("error"); exit(1) }
}
let l = Loc(); RunLoop.main.run(until: Date(timeIntervalSinceNow: 5))
"#])
            .output()
            .await?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        let parts: Vec<&str> = stdout.trim().split(',').collect();
        if parts.len() == 2 {
            let lat: f64 = parts[0].parse()?;
            let lon: f64 = parts[1].parse()?;
            return Ok((lat, lon));
        }
        anyhow::bail!("Location unavailable on macOS");
    }

    #[cfg(target_os = "windows")]
    {
        // Use Windows Location API via PowerShell
        let output = tokio::process::Command::new("powershell")
            .args([
                "-NoProfile",
                "-Command",
                r#"
Add-Type -AssemblyName System.Device
$watcher = New-Object System.Device.Location.GeoCoordinateWatcher
$watcher.Start()
Start-Sleep -Seconds 3
$coord = $watcher.Position.Location
Write-Output "$($coord.Latitude),$($coord.Longitude)"
$watcher.Stop()
"#,
            ])
            .output()
            .await?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        let parts: Vec<&str> = stdout.trim().split(',').collect();
        if parts.len() == 2 {
            let lat: f64 = parts[0].parse()?;
            let lon: f64 = parts[1].parse()?;
            return Ok((lat, lon));
        }
        anyhow::bail!("Location unavailable on Windows");
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        anyhow::bail!("Location services not available on this platform")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_platform_capabilities() {
        let caps = PlatformCapabilities::detect();
        assert!(caps.notifications);
        assert!(caps.system_run);
        assert!(!caps.platform.is_empty());
    }

    #[tokio::test]
    async fn test_system_run() {
        #[cfg(target_os = "windows")]
        let result = system_run("cmd", &["/C", "echo hello"]).await;
        #[cfg(not(target_os = "windows"))]
        let result = system_run("echo", &["hello"]).await;

        let output = result.unwrap();
        assert_eq!(output.exit_code, 0);
        assert!(output.stdout.contains("hello"));
    }
}
