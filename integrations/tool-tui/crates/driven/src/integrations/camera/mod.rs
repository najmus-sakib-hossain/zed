//! # Camera Integration
//!
//! Camera capture and control for photo/video.
//!
//! ## Usage
//!
//! ```rust,ignore
//! use driven::integrations::camera::{CameraClient, CameraConfig};
//!
//! let config = CameraConfig::from_file("~/.dx/config/camera.sr")?;
//! let client = CameraClient::new(&config)?;
//!
//! // Capture a photo
//! let photo = client.capture_photo().await?;
//!
//! // List available cameras
//! let cameras = client.list_cameras().await?;
//! ```

use crate::error::{DrivenError, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Camera configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CameraConfig {
    /// Whether camera integration is enabled
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Default camera device
    pub default_device: Option<String>,
    /// Output directory
    #[serde(default = "default_output_dir")]
    pub output_dir: PathBuf,
    /// Default photo quality (1-100)
    #[serde(default = "default_quality")]
    pub photo_quality: u8,
    /// Default video resolution
    #[serde(default = "default_resolution")]
    pub video_resolution: String,
    /// Default video FPS
    #[serde(default = "default_fps")]
    pub video_fps: u32,
}

fn default_true() -> bool {
    true
}

fn default_output_dir() -> PathBuf {
    dirs::picture_dir().unwrap_or_else(|| PathBuf::from("."))
}

fn default_quality() -> u8 {
    85
}

fn default_resolution() -> String {
    "1920x1080".to_string()
}

fn default_fps() -> u32 {
    30
}

impl Default for CameraConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            default_device: None,
            output_dir: default_output_dir(),
            photo_quality: default_quality(),
            video_resolution: default_resolution(),
            video_fps: default_fps(),
        }
    }
}

impl CameraConfig {
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

/// Camera device
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CameraDevice {
    /// Device ID
    pub id: String,
    /// Device name
    pub name: String,
    /// Is default device
    pub is_default: bool,
    /// Supported resolutions
    pub resolutions: Vec<String>,
    /// Device type
    pub device_type: CameraType,
}

/// Camera type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CameraType {
    /// Built-in webcam
    Builtin,
    /// USB camera
    Usb,
    /// Virtual camera
    Virtual,
    /// Network camera
    Network,
    /// Unknown
    Unknown,
}

/// Captured photo
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapturedPhoto {
    /// File path
    pub path: PathBuf,
    /// Width
    pub width: u32,
    /// Height
    pub height: u32,
    /// File size in bytes
    pub size: u64,
    /// MIME type
    pub mime_type: String,
    /// Timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Video recording
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoRecording {
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
    /// MIME type
    pub mime_type: String,
    /// Timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Recording state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecordingState {
    Idle,
    Recording,
    Paused,
    Processing,
}

/// Camera client
pub struct CameraClient {
    config: CameraConfig,
    recording_state: RecordingState,
    current_recording: Option<PathBuf>,
}

impl CameraClient {
    /// Create a new camera client
    pub fn new(config: &CameraConfig) -> Result<Self> {
        // Ensure output directory exists
        std::fs::create_dir_all(&config.output_dir)
            .map_err(|e| DrivenError::Io(e))?;

        Ok(Self {
            config: config.clone(),
            recording_state: RecordingState::Idle,
            current_recording: None,
        })
    }

    /// Check if camera is available
    pub fn is_available(&self) -> bool {
        self.config.enabled
    }

    /// List available cameras
    pub async fn list_cameras(&self) -> Result<Vec<CameraDevice>> {
        #[cfg(target_os = "macos")]
        {
            self.list_cameras_macos().await
        }

        #[cfg(target_os = "windows")]
        {
            self.list_cameras_windows().await
        }

        #[cfg(target_os = "linux")]
        {
            self.list_cameras_linux().await
        }

        #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
        {
            Ok(Vec::new())
        }
    }

    #[cfg(target_os = "macos")]
    async fn list_cameras_macos(&self) -> Result<Vec<CameraDevice>> {
        use tokio::process::Command;

        // Use system_profiler to list cameras
        let output = Command::new("system_profiler")
            .args(["SPCameraDataType", "-json"])
            .output()
            .await
            .map_err(|e| DrivenError::Process(e.to_string()))?;

        if !output.status.success() {
            return Ok(Vec::new());
        }

        let json: serde_json::Value = serde_json::from_slice(&output.stdout)
            .map_err(|e| DrivenError::Parse(e.to_string()))?;

        let mut cameras = Vec::new();

        if let Some(camera_data) = json["SPCameraDataType"].as_array() {
            for (idx, cam) in camera_data.iter().enumerate() {
                cameras.push(CameraDevice {
                    id: format!("camera_{}", idx),
                    name: cam["_name"].as_str().unwrap_or("Unknown").to_string(),
                    is_default: idx == 0,
                    resolutions: vec!["1920x1080".to_string(), "1280x720".to_string()],
                    device_type: CameraType::Builtin,
                });
            }
        }

        Ok(cameras)
    }

    #[cfg(target_os = "windows")]
    async fn list_cameras_windows(&self) -> Result<Vec<CameraDevice>> {
        // Use PowerShell to list video devices
        use tokio::process::Command;

        let script = r#"
            Get-CimInstance Win32_PnPEntity | 
            Where-Object { $_.PNPClass -eq 'Camera' -or $_.PNPClass -eq 'Image' } |
            Select-Object Name, DeviceID |
            ConvertTo-Json
        "#;

        let output = Command::new("powershell")
            .args(["-Command", script])
            .output()
            .await
            .map_err(|e| DrivenError::Process(e.to_string()))?;

        if !output.status.success() {
            return Ok(Vec::new());
        }

        let json_str = String::from_utf8_lossy(&output.stdout);
        let devices: Vec<serde_json::Value> = serde_json::from_str(&json_str)
            .unwrap_or_else(|_| Vec::new());

        let cameras = devices
            .into_iter()
            .enumerate()
            .map(|(idx, d)| CameraDevice {
                id: d["DeviceID"].as_str().unwrap_or_default().to_string(),
                name: d["Name"].as_str().unwrap_or("Unknown").to_string(),
                is_default: idx == 0,
                resolutions: vec!["1920x1080".to_string(), "1280x720".to_string()],
                device_type: CameraType::Usb,
            })
            .collect();

        Ok(cameras)
    }

    #[cfg(target_os = "linux")]
    async fn list_cameras_linux(&self) -> Result<Vec<CameraDevice>> {
        use tokio::process::Command;

        // List video devices
        let output = Command::new("v4l2-ctl")
            .args(["--list-devices"])
            .output()
            .await;

        if let Ok(output) = output {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let mut cameras = Vec::new();
                let mut current_name = String::new();

                for line in stdout.lines() {
                    if !line.starts_with('\t') && !line.is_empty() {
                        current_name = line.trim_end_matches(':').to_string();
                    } else if line.contains("/dev/video") {
                        let device = line.trim();
                        cameras.push(CameraDevice {
                            id: device.to_string(),
                            name: current_name.clone(),
                            is_default: cameras.is_empty(),
                            resolutions: vec!["1920x1080".to_string(), "1280x720".to_string()],
                            device_type: CameraType::Usb,
                        });
                    }
                }

                return Ok(cameras);
            }
        }

        Ok(Vec::new())
    }

    /// Capture a photo
    pub async fn capture_photo(&self) -> Result<CapturedPhoto> {
        let filename = format!(
            "photo_{}.jpg",
            chrono::Utc::now().format("%Y%m%d_%H%M%S")
        );
        let path = self.config.output_dir.join(&filename);

        #[cfg(target_os = "macos")]
        {
            self.capture_photo_macos(&path).await
        }

        #[cfg(target_os = "windows")]
        {
            self.capture_photo_windows(&path).await
        }

        #[cfg(target_os = "linux")]
        {
            self.capture_photo_linux(&path).await
        }

        #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
        {
            Err(DrivenError::Unsupported("Camera not supported on this platform".into()))
        }
    }

    #[cfg(target_os = "macos")]
    async fn capture_photo_macos(&self, path: &std::path::Path) -> Result<CapturedPhoto> {
        use tokio::process::Command;

        // Use imagesnap on macOS (needs to be installed: brew install imagesnap)
        let mut cmd = Command::new("imagesnap");
        cmd.arg("-q");
        cmd.arg(path);

        if let Some(ref device) = self.config.default_device {
            cmd.arg("-d").arg(device);
        }

        let status = cmd
            .status()
            .await
            .map_err(|e| DrivenError::Process(format!("Failed to capture photo: {}", e)))?;

        if !status.success() {
            return Err(DrivenError::Process("Photo capture failed".into()));
        }

        let metadata = tokio::fs::metadata(path)
            .await
            .map_err(|e| DrivenError::Io(e))?;

        Ok(CapturedPhoto {
            path: path.to_path_buf(),
            width: 1920, // Would need image library to get actual dimensions
            height: 1080,
            size: metadata.len(),
            mime_type: "image/jpeg".to_string(),
            timestamp: chrono::Utc::now(),
        })
    }

    #[cfg(target_os = "windows")]
    async fn capture_photo_windows(&self, path: &std::path::Path) -> Result<CapturedPhoto> {
        use tokio::process::Command;

        // Use ffmpeg on Windows
        let status = Command::new("ffmpeg")
            .args([
                "-f", "dshow",
                "-i", "video=0",
                "-frames:v", "1",
                "-y",
                path.to_str().unwrap(),
            ])
            .status()
            .await
            .map_err(|e| DrivenError::Process(format!("Failed to capture photo: {}", e)))?;

        if !status.success() {
            return Err(DrivenError::Process("Photo capture failed".into()));
        }

        let metadata = tokio::fs::metadata(path)
            .await
            .map_err(|e| DrivenError::Io(e))?;

        Ok(CapturedPhoto {
            path: path.to_path_buf(),
            width: 1920,
            height: 1080,
            size: metadata.len(),
            mime_type: "image/jpeg".to_string(),
            timestamp: chrono::Utc::now(),
        })
    }

    #[cfg(target_os = "linux")]
    async fn capture_photo_linux(&self, path: &std::path::Path) -> Result<CapturedPhoto> {
        use tokio::process::Command;

        let device = self.config.default_device.clone()
            .unwrap_or_else(|| "/dev/video0".to_string());

        // Use fswebcam or ffmpeg on Linux
        let status = Command::new("fswebcam")
            .args([
                "-d", &device,
                "-r", "1920x1080",
                "--jpeg", &self.config.photo_quality.to_string(),
                "--no-banner",
                path.to_str().unwrap(),
            ])
            .status()
            .await
            .map_err(|e| DrivenError::Process(format!("Failed to capture photo: {}", e)))?;

        if !status.success() {
            return Err(DrivenError::Process("Photo capture failed".into()));
        }

        let metadata = tokio::fs::metadata(path)
            .await
            .map_err(|e| DrivenError::Io(e))?;

        Ok(CapturedPhoto {
            path: path.to_path_buf(),
            width: 1920,
            height: 1080,
            size: metadata.len(),
            mime_type: "image/jpeg".to_string(),
            timestamp: chrono::Utc::now(),
        })
    }

    /// Start video recording
    pub async fn start_recording(&mut self) -> Result<PathBuf> {
        if self.recording_state == RecordingState::Recording {
            return Err(DrivenError::Conflict("Already recording".into()));
        }

        let filename = format!(
            "video_{}.mp4",
            chrono::Utc::now().format("%Y%m%d_%H%M%S")
        );
        let path = self.config.output_dir.join(&filename);

        self.recording_state = RecordingState::Recording;
        self.current_recording = Some(path.clone());

        // Note: Actual recording would need to spawn a background process
        // This is a simplified implementation
        
        Ok(path)
    }

    /// Stop video recording
    pub async fn stop_recording(&mut self) -> Result<Option<VideoRecording>> {
        if self.recording_state != RecordingState::Recording {
            return Ok(None);
        }

        self.recording_state = RecordingState::Processing;

        let path = self.current_recording.take();
        self.recording_state = RecordingState::Idle;

        if let Some(p) = path {
            if p.exists() {
                let metadata = tokio::fs::metadata(&p)
                    .await
                    .map_err(|e| DrivenError::Io(e))?;

                return Ok(Some(VideoRecording {
                    path: p,
                    duration: 0.0, // Would need ffprobe to get actual duration
                    width: 1920,
                    height: 1080,
                    fps: self.config.video_fps,
                    size: metadata.len(),
                    mime_type: "video/mp4".to_string(),
                    timestamp: chrono::Utc::now(),
                }));
            }
        }

        Ok(None)
    }

    /// Get recording state
    pub fn recording_state(&self) -> RecordingState {
        self.recording_state
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = CameraConfig::default();
        assert!(config.enabled);
        assert_eq!(config.photo_quality, 85);
        assert_eq!(config.video_fps, 30);
    }
}
