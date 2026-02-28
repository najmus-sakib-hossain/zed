//! Cross-platform service manager for the DX gateway daemon.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tracing::info;

/// Service configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceConfig {
    pub name: String,
    pub display_name: String,
    pub description: String,
    pub port: u16,
    pub auto_start: bool,
    pub working_dir: Option<PathBuf>,
    pub log_path: Option<PathBuf>,
    pub exe_path: Option<PathBuf>,
}

impl Default for ServiceConfig {
    fn default() -> Self {
        Self {
            name: "dx-gateway".into(),
            display_name: "DX Agent Gateway".into(),
            description: "DX Agent WebSocket Gateway Service".into(),
            port: 31337,
            auto_start: true,
            working_dir: None,
            log_path: None,
            exe_path: None,
        }
    }
}

/// Service status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ServiceStatus {
    Running,
    Stopped,
    Starting,
    Stopping,
    Unknown,
}

/// Detected service type for the current platform
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServiceType {
    WindowsService,
    Systemd,
    Launchd,
    Unknown,
}

impl ServiceType {
    /// Detect the service type for the current platform
    pub fn detect() -> Self {
        if cfg!(target_os = "windows") {
            Self::WindowsService
        } else if cfg!(target_os = "macos") {
            Self::Launchd
        } else if cfg!(target_os = "linux") {
            Self::Systemd
        } else {
            Self::Unknown
        }
    }
}

/// Cross-platform service manager
pub struct ServiceManager {
    pub config: ServiceConfig,
    pub service_type: ServiceType,
}

impl ServiceManager {
    pub fn new(config: ServiceConfig) -> Self {
        Self {
            config,
            service_type: ServiceType::detect(),
        }
    }

    /// Install the service on the current platform
    pub fn install(&self) -> Result<()> {
        match self.service_type {
            ServiceType::WindowsService => self.install_windows(),
            ServiceType::Systemd => self.install_systemd(),
            ServiceType::Launchd => self.install_launchd(),
            ServiceType::Unknown => anyhow::bail!("Unsupported platform for service installation"),
        }
    }

    /// Uninstall the service
    pub fn uninstall(&self) -> Result<()> {
        match self.service_type {
            ServiceType::WindowsService => self.uninstall_windows(),
            ServiceType::Systemd => self.uninstall_systemd(),
            ServiceType::Launchd => self.uninstall_launchd(),
            ServiceType::Unknown => anyhow::bail!("Unsupported platform"),
        }
    }

    /// Start the service
    pub fn start(&self) -> Result<()> {
        match self.service_type {
            ServiceType::WindowsService => self.start_windows(),
            ServiceType::Systemd => self.start_systemd(),
            ServiceType::Launchd => self.start_launchd(),
            ServiceType::Unknown => anyhow::bail!("Unsupported platform"),
        }
    }

    /// Stop the service
    pub fn stop(&self) -> Result<()> {
        match self.service_type {
            ServiceType::WindowsService => self.stop_windows(),
            ServiceType::Systemd => self.stop_systemd(),
            ServiceType::Launchd => self.stop_launchd(),
            ServiceType::Unknown => anyhow::bail!("Unsupported platform"),
        }
    }

    /// Restart the service
    pub fn restart(&self) -> Result<()> {
        self.stop()?;
        std::thread::sleep(std::time::Duration::from_secs(1));
        self.start()
    }

    /// Get service status
    pub fn status(&self) -> Result<ServiceStatus> {
        match self.service_type {
            ServiceType::WindowsService => self.status_windows(),
            ServiceType::Systemd => self.status_systemd(),
            ServiceType::Launchd => self.status_launchd(),
            ServiceType::Unknown => Ok(ServiceStatus::Unknown),
        }
    }

    // --- Windows ---

    fn install_windows(&self) -> Result<()> {
        let exe = self.get_exe_path()?;
        let args = format!("gateway start --port {} --daemon", self.config.port);

        // Use schtasks for Windows Task Scheduler (works without admin in some cases)
        // For true Windows Service, we'd use windows-service crate
        let output = std::process::Command::new("schtasks")
            .args([
                "/Create",
                "/SC",
                "ONSTART",
                "/TN",
                &self.config.name,
                "/TR",
                &format!("\"{}\" {}", exe.display(), args),
                "/RU",
                "SYSTEM",
                "/F",
            ])
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Failed to install service: {}", stderr);
        }

        info!("Installed Windows scheduled task: {}", self.config.name);
        Ok(())
    }

    fn uninstall_windows(&self) -> Result<()> {
        let output = std::process::Command::new("schtasks")
            .args(["/Delete", "/TN", &self.config.name, "/F"])
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Failed to uninstall service: {}", stderr);
        }

        info!("Uninstalled Windows scheduled task: {}", self.config.name);
        Ok(())
    }

    fn start_windows(&self) -> Result<()> {
        let output = std::process::Command::new("schtasks")
            .args(["/Run", "/TN", &self.config.name])
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Failed to start service: {}", stderr);
        }

        info!("Started Windows service: {}", self.config.name);
        Ok(())
    }

    fn stop_windows(&self) -> Result<()> {
        let output = std::process::Command::new("schtasks")
            .args(["/End", "/TN", &self.config.name])
            .output()?;

        if !output.status.success() {
            // Try taskkill as fallback
            let _ = std::process::Command::new("taskkill").args(["/F", "/IM", "dx.exe"]).output();
        }

        info!("Stopped Windows service: {}", self.config.name);
        Ok(())
    }

    fn status_windows(&self) -> Result<ServiceStatus> {
        let output = std::process::Command::new("schtasks")
            .args(["/Query", "/TN", &self.config.name, "/FO", "CSV", "/NH"])
            .output()?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        if stdout.contains("Running") {
            Ok(ServiceStatus::Running)
        } else if stdout.contains("Ready") {
            Ok(ServiceStatus::Stopped)
        } else {
            Ok(ServiceStatus::Unknown)
        }
    }

    // --- Systemd (Linux) ---

    fn install_systemd(&self) -> Result<()> {
        let exe = self.get_exe_path()?;
        let unit = format!(
            r#"[Unit]
Description={description}
After=network.target

[Service]
Type=simple
ExecStart={exe} gateway start --port {port}
Restart=always
RestartSec=5
StandardOutput=journal
StandardError=journal
{working_dir}

[Install]
WantedBy=multi-user.target
"#,
            description = self.config.description,
            exe = exe.display(),
            port = self.config.port,
            working_dir = self
                .config
                .working_dir
                .as_ref()
                .map(|d| format!("WorkingDirectory={}", d.display()))
                .unwrap_or_default(),
        );

        let unit_path = format!("/etc/systemd/system/{}.service", self.config.name);
        std::fs::write(&unit_path, unit)?;

        std::process::Command::new("systemctl").args(["daemon-reload"]).output()?;

        if self.config.auto_start {
            std::process::Command::new("systemctl")
                .args(["enable", &self.config.name])
                .output()?;
        }

        info!("Installed systemd unit: {}", unit_path);
        Ok(())
    }

    fn uninstall_systemd(&self) -> Result<()> {
        let _ = self.stop_systemd();
        let _ = std::process::Command::new("systemctl")
            .args(["disable", &self.config.name])
            .output();

        let unit_path = format!("/etc/systemd/system/{}.service", self.config.name);
        let _ = std::fs::remove_file(&unit_path);

        std::process::Command::new("systemctl").args(["daemon-reload"]).output()?;

        info!("Uninstalled systemd unit: {}", self.config.name);
        Ok(())
    }

    fn start_systemd(&self) -> Result<()> {
        let output = std::process::Command::new("systemctl")
            .args(["start", &self.config.name])
            .output()?;

        if !output.status.success() {
            anyhow::bail!("Failed to start systemd service");
        }
        Ok(())
    }

    fn stop_systemd(&self) -> Result<()> {
        let output = std::process::Command::new("systemctl")
            .args(["stop", &self.config.name])
            .output()?;

        if !output.status.success() {
            anyhow::bail!("Failed to stop systemd service");
        }
        Ok(())
    }

    fn status_systemd(&self) -> Result<ServiceStatus> {
        let output = std::process::Command::new("systemctl")
            .args(["is-active", &self.config.name])
            .output()?;

        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        match stdout.as_str() {
            "active" => Ok(ServiceStatus::Running),
            "inactive" => Ok(ServiceStatus::Stopped),
            "activating" => Ok(ServiceStatus::Starting),
            "deactivating" => Ok(ServiceStatus::Stopping),
            _ => Ok(ServiceStatus::Unknown),
        }
    }

    // --- Launchd (macOS) ---

    fn install_launchd(&self) -> Result<()> {
        let exe = self.get_exe_path()?;
        let plist = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>dev.dx.{name}</string>
    <key>ProgramArguments</key>
    <array>
        <string>{exe}</string>
        <string>gateway</string>
        <string>start</string>
        <string>--port</string>
        <string>{port}</string>
    </array>
    <key>RunAtLoad</key>
    <{auto_start}/>
    <key>KeepAlive</key>
    <true/>
    <key>StandardOutPath</key>
    <string>/tmp/{name}.stdout.log</string>
    <key>StandardErrorPath</key>
    <string>/tmp/{name}.stderr.log</string>
</dict>
</plist>"#,
            name = self.config.name,
            exe = exe.display(),
            port = self.config.port,
            auto_start = if self.config.auto_start {
                "true"
            } else {
                "false"
            },
        );

        let home = std::env::var("HOME")?;
        let plist_dir = format!("{}/Library/LaunchAgents", home);
        std::fs::create_dir_all(&plist_dir)?;
        let plist_path = format!("{}/dev.dx.{}.plist", plist_dir, self.config.name);
        std::fs::write(&plist_path, plist)?;

        info!("Installed launchd plist: {}", plist_path);
        Ok(())
    }

    fn uninstall_launchd(&self) -> Result<()> {
        let _ = self.stop_launchd();
        let home = std::env::var("HOME")?;
        let plist_path = format!("{}/Library/LaunchAgents/dev.dx.{}.plist", home, self.config.name);
        let _ = std::fs::remove_file(&plist_path);
        info!("Uninstalled launchd plist: {}", self.config.name);
        Ok(())
    }

    fn start_launchd(&self) -> Result<()> {
        let home = std::env::var("HOME")?;
        let plist_path = format!("{}/Library/LaunchAgents/dev.dx.{}.plist", home, self.config.name);
        std::process::Command::new("launchctl").args(["load", &plist_path]).output()?;
        Ok(())
    }

    fn stop_launchd(&self) -> Result<()> {
        let home = std::env::var("HOME")?;
        let plist_path = format!("{}/Library/LaunchAgents/dev.dx.{}.plist", home, self.config.name);
        std::process::Command::new("launchctl").args(["unload", &plist_path]).output()?;
        Ok(())
    }

    fn status_launchd(&self) -> Result<ServiceStatus> {
        let label = format!("dev.dx.{}", self.config.name);
        let output = std::process::Command::new("launchctl").args(["list", &label]).output()?;

        if output.status.success() {
            Ok(ServiceStatus::Running)
        } else {
            Ok(ServiceStatus::Stopped)
        }
    }

    // --- Helpers ---

    fn get_exe_path(&self) -> Result<PathBuf> {
        if let Some(ref exe) = self.config.exe_path {
            Ok(exe.clone())
        } else {
            std::env::current_exe().map_err(Into::into)
        }
    }
}

/// Get service logs
pub fn get_service_logs(config: &ServiceConfig, lines: usize) -> Result<String> {
    let service_type = ServiceType::detect();

    match service_type {
        ServiceType::Systemd => {
            let output = std::process::Command::new("journalctl")
                .args(["-u", &config.name, "-n", &lines.to_string(), "--no-pager"])
                .output()?;
            Ok(String::from_utf8_lossy(&output.stdout).into_owned())
        }
        ServiceType::Launchd => {
            let path = format!("/tmp/{}.stdout.log", config.name);
            if let Ok(content) = std::fs::read_to_string(&path) {
                let log_lines: Vec<&str> = content.lines().collect();
                let start = if log_lines.len() > lines {
                    log_lines.len() - lines
                } else {
                    0
                };
                Ok(log_lines[start..].join("\n"))
            } else {
                Ok("No logs found".into())
            }
        }
        ServiceType::WindowsService => {
            if let Some(ref log_path) = config.log_path {
                if let Ok(content) = std::fs::read_to_string(log_path) {
                    let log_lines: Vec<&str> = content.lines().collect();
                    let start = if log_lines.len() > lines {
                        log_lines.len() - lines
                    } else {
                        0
                    };
                    Ok(log_lines[start..].join("\n"))
                } else {
                    Ok("No logs found".into())
                }
            } else {
                Ok("No log path configured".into())
            }
        }
        ServiceType::Unknown => Ok("Unsupported platform".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_service_config_default() {
        let config = ServiceConfig::default();
        assert_eq!(config.name, "dx-gateway");
        assert_eq!(config.port, 31337);
        assert!(config.auto_start);
    }

    #[test]
    fn test_service_type_detect() {
        let st = ServiceType::detect();
        if cfg!(target_os = "windows") {
            assert_eq!(st, ServiceType::WindowsService);
        } else if cfg!(target_os = "macos") {
            assert_eq!(st, ServiceType::Launchd);
        } else if cfg!(target_os = "linux") {
            assert_eq!(st, ServiceType::Systemd);
        }
    }

    #[test]
    fn test_service_manager_creation() {
        let manager = ServiceManager::new(ServiceConfig::default());
        assert_eq!(manager.config.name, "dx-gateway");
    }
}
