//! System Service/Daemon Management
//!
//! Cross-platform service management for running DX gateway as a background service:
//! - macOS: launchd (LaunchAgent)
//! - Linux: systemd (user service)
//! - Windows: Task Scheduler (schtasks)

use std::fs;
use std::path::PathBuf;
use std::process::Command;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

/// Service configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceConfig {
    /// Service name/label
    pub name: String,
    /// Gateway port
    pub port: u16,
    /// Enable mDNS
    pub mdns: bool,
    /// Require authentication
    pub require_auth: bool,
    /// Auto-start on boot
    pub auto_start: bool,
    /// Log file path
    pub log_path: Option<PathBuf>,
    /// Working directory
    pub working_dir: Option<PathBuf>,
}

impl Default for ServiceConfig {
    fn default() -> Self {
        Self {
            name: "dx-gateway".to_string(),
            port: 31337,
            mdns: true,
            require_auth: true,
            auto_start: true,
            log_path: None,
            working_dir: None,
        }
    }
}

/// Service status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceStatus {
    /// Whether the service is installed
    pub installed: bool,
    /// Whether the service is running
    pub running: bool,
    /// Process ID (if running)
    pub pid: Option<u32>,
    /// Last start time
    pub started_at: Option<String>,
    /// Service type
    pub service_type: ServiceType,
    /// Error message (if any)
    pub error: Option<String>,
}

/// Service type for current platform
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ServiceType {
    Launchd,
    Systemd,
    WindowsTask,
    Unknown,
}

impl ServiceType {
    /// Detect service type for current platform
    pub fn detect() -> Self {
        #[cfg(target_os = "macos")]
        return ServiceType::Launchd;

        #[cfg(target_os = "linux")]
        return ServiceType::Systemd;

        #[cfg(target_os = "windows")]
        return ServiceType::WindowsTask;

        #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
        return ServiceType::Unknown;
    }
}

/// Service manager
pub struct ServiceManager {
    config: ServiceConfig,
    service_type: ServiceType,
}

impl ServiceManager {
    /// Create a new service manager
    pub fn new(config: ServiceConfig) -> Self {
        Self {
            config,
            service_type: ServiceType::detect(),
        }
    }

    /// Get the DX executable path
    fn get_dx_path() -> Result<PathBuf> {
        std::env::current_exe().context("Failed to get current executable path")
    }

    /// Get the service file path
    fn service_file_path(&self) -> Result<PathBuf> {
        match self.service_type {
            ServiceType::Launchd => {
                let home = dirs::home_dir().context("Failed to get home directory")?;
                Ok(home
                    .join("Library")
                    .join("LaunchAgents")
                    .join(format!("com.dx.{}.plist", self.config.name)))
            }
            ServiceType::Systemd => {
                let config = dirs::config_dir().context("Failed to get config directory")?;
                Ok(config
                    .join("systemd")
                    .join("user")
                    .join(format!("{}.service", self.config.name)))
            }
            ServiceType::WindowsTask => {
                // Windows doesn't have a file, uses Task Scheduler API
                Ok(PathBuf::from(format!("\\DX\\{}", self.config.name)))
            }
            ServiceType::Unknown => {
                anyhow::bail!("Unsupported platform for service management")
            }
        }
    }

    /// Install the service
    pub fn install(&self) -> Result<()> {
        match self.service_type {
            ServiceType::Launchd => self.install_launchd(),
            ServiceType::Systemd => self.install_systemd(),
            ServiceType::WindowsTask => self.install_windows_task(),
            ServiceType::Unknown => {
                anyhow::bail!("Unsupported platform for service management")
            }
        }
    }

    /// Uninstall the service
    pub fn uninstall(&self) -> Result<()> {
        // Stop the service first
        let _ = self.stop();

        match self.service_type {
            ServiceType::Launchd => self.uninstall_launchd(),
            ServiceType::Systemd => self.uninstall_systemd(),
            ServiceType::WindowsTask => self.uninstall_windows_task(),
            ServiceType::Unknown => {
                anyhow::bail!("Unsupported platform for service management")
            }
        }
    }

    /// Start the service
    pub fn start(&self) -> Result<()> {
        match self.service_type {
            ServiceType::Launchd => {
                let path = self.service_file_path()?;
                let status = Command::new("launchctl")
                    .args(["load", "-w"])
                    .arg(&path)
                    .status()
                    .context("Failed to run launchctl")?;

                if !status.success() {
                    anyhow::bail!("Failed to start service via launchctl");
                }
                Ok(())
            }
            ServiceType::Systemd => {
                let status = Command::new("systemctl")
                    .args(["--user", "start", &self.config.name])
                    .status()
                    .context("Failed to run systemctl")?;

                if !status.success() {
                    anyhow::bail!("Failed to start service via systemctl");
                }
                Ok(())
            }
            ServiceType::WindowsTask => {
                let status = Command::new("schtasks")
                    .args(["/Run", "/TN", &format!("\\DX\\{}", self.config.name)])
                    .status()
                    .context("Failed to run schtasks")?;

                if !status.success() {
                    anyhow::bail!("Failed to start scheduled task");
                }
                Ok(())
            }
            ServiceType::Unknown => {
                anyhow::bail!("Unsupported platform")
            }
        }
    }

    /// Stop the service
    pub fn stop(&self) -> Result<()> {
        match self.service_type {
            ServiceType::Launchd => {
                let path = self.service_file_path()?;
                let status = Command::new("launchctl")
                    .args(["unload"])
                    .arg(&path)
                    .status()
                    .context("Failed to run launchctl")?;

                if !status.success() {
                    tracing::warn!(
                        "launchctl unload returned non-zero (service may not be running)"
                    );
                }
                Ok(())
            }
            ServiceType::Systemd => {
                let status = Command::new("systemctl")
                    .args(["--user", "stop", &self.config.name])
                    .status()
                    .context("Failed to run systemctl")?;

                if !status.success() {
                    tracing::warn!("systemctl stop returned non-zero (service may not be running)");
                }
                Ok(())
            }
            ServiceType::WindowsTask => {
                let status = Command::new("schtasks")
                    .args(["/End", "/TN", &format!("\\DX\\{}", self.config.name)])
                    .status()
                    .context("Failed to run schtasks")?;

                if !status.success() {
                    tracing::warn!("schtasks end returned non-zero (task may not be running)");
                }
                Ok(())
            }
            ServiceType::Unknown => {
                anyhow::bail!("Unsupported platform")
            }
        }
    }

    /// Restart the service
    pub fn restart(&self) -> Result<()> {
        self.stop()?;
        std::thread::sleep(std::time::Duration::from_millis(500));
        self.start()
    }

    /// Get service status
    pub fn status(&self) -> Result<ServiceStatus> {
        let installed = self.is_installed()?;

        match self.service_type {
            ServiceType::Launchd => {
                let output = Command::new("launchctl")
                    .args(["list"])
                    .output()
                    .context("Failed to run launchctl list")?;

                let stdout = String::from_utf8_lossy(&output.stdout);
                let label = format!("com.dx.{}", self.config.name);

                let running = stdout.lines().any(|line| line.contains(&label));

                let pid = if running {
                    stdout
                        .lines()
                        .find(|line| line.contains(&label))
                        .and_then(|line| line.split_whitespace().next())
                        .and_then(|pid| pid.parse().ok())
                } else {
                    None
                };

                Ok(ServiceStatus {
                    installed,
                    running,
                    pid,
                    started_at: None,
                    service_type: ServiceType::Launchd,
                    error: None,
                })
            }
            ServiceType::Systemd => {
                let output = Command::new("systemctl")
                    .args(["--user", "is-active", &self.config.name])
                    .output()
                    .context("Failed to run systemctl")?;

                let running = String::from_utf8_lossy(&output.stdout).trim() == "active";

                let pid = if running {
                    Command::new("systemctl")
                        .args(["--user", "show", &self.config.name, "-p", "MainPID"])
                        .output()
                        .ok()
                        .and_then(|o| {
                            String::from_utf8_lossy(&o.stdout)
                                .trim()
                                .strip_prefix("MainPID=")
                                .and_then(|s| s.parse().ok())
                        })
                } else {
                    None
                };

                Ok(ServiceStatus {
                    installed,
                    running,
                    pid,
                    started_at: None,
                    service_type: ServiceType::Systemd,
                    error: None,
                })
            }
            ServiceType::WindowsTask => {
                let output = Command::new("schtasks")
                    .args([
                        "/Query",
                        "/TN",
                        &format!("\\DX\\{}", self.config.name),
                        "/FO",
                        "CSV",
                    ])
                    .output()
                    .context("Failed to run schtasks")?;

                let stdout = String::from_utf8_lossy(&output.stdout);
                let running = stdout.contains("Running");

                Ok(ServiceStatus {
                    installed,
                    running,
                    pid: None,
                    started_at: None,
                    service_type: ServiceType::WindowsTask,
                    error: None,
                })
            }
            ServiceType::Unknown => Ok(ServiceStatus {
                installed: false,
                running: false,
                pid: None,
                started_at: None,
                service_type: ServiceType::Unknown,
                error: Some("Unsupported platform".to_string()),
            }),
        }
    }

    /// Check if service is installed
    fn is_installed(&self) -> Result<bool> {
        match self.service_type {
            ServiceType::Launchd | ServiceType::Systemd => {
                let path = self.service_file_path()?;
                Ok(path.exists())
            }
            ServiceType::WindowsTask => {
                let output = Command::new("schtasks")
                    .args(["/Query", "/TN", &format!("\\DX\\{}", self.config.name)])
                    .output()
                    .context("Failed to run schtasks")?;

                Ok(output.status.success())
            }
            ServiceType::Unknown => Ok(false),
        }
    }

    // ========================================================================
    // macOS launchd
    // ========================================================================

    fn install_launchd(&self) -> Result<()> {
        let dx_path = Self::get_dx_path()?;
        let plist_path = self.service_file_path()?;

        // Create LaunchAgents directory if needed
        if let Some(parent) = plist_path.parent() {
            fs::create_dir_all(parent).context("Failed to create LaunchAgents directory")?;
        }

        let log_path = self.config.log_path.clone().unwrap_or_else(|| {
            dirs::home_dir()
                .unwrap_or_default()
                .join("Library")
                .join("Logs")
                .join("dx-gateway.log")
        });

        let working_dir = self
            .config
            .working_dir
            .clone()
            .unwrap_or_else(|| dirs::home_dir().unwrap_or_default());

        let plist_content = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.dx.{name}</string>
    
    <key>ProgramArguments</key>
    <array>
        <string>{dx_path}</string>
        <string>gateway</string>
        <string>--port</string>
        <string>{port}</string>
        {mdns_arg}
        {auth_arg}
    </array>
    
    <key>WorkingDirectory</key>
    <string>{working_dir}</string>
    
    <key>StandardOutPath</key>
    <string>{log_path}</string>
    
    <key>StandardErrorPath</key>
    <string>{log_path}</string>
    
    <key>RunAtLoad</key>
    <{auto_start}/>
    
    <key>KeepAlive</key>
    <dict>
        <key>SuccessfulExit</key>
        <false/>
        <key>Crashed</key>
        <true/>
    </dict>
    
    <key>ThrottleInterval</key>
    <integer>10</integer>
    
    <key>ProcessType</key>
    <string>Background</string>
    
    <key>LowPriorityIO</key>
    <false/>
    
    <key>EnvironmentVariables</key>
    <dict>
        <key>RUST_LOG</key>
        <string>info</string>
    </dict>
</dict>
</plist>"#,
            name = self.config.name,
            dx_path = dx_path.display(),
            port = self.config.port,
            working_dir = working_dir.display(),
            log_path = log_path.display(),
            auto_start = if self.config.auto_start {
                "true"
            } else {
                "false"
            },
            mdns_arg = if self.config.mdns {
                "<string>--mdns</string>"
            } else {
                ""
            },
            auth_arg = if self.config.require_auth {
                "<string>--require-auth</string>"
            } else {
                ""
            },
        );

        fs::write(&plist_path, plist_content).context("Failed to write plist file")?;

        tracing::info!("Installed launchd service: {}", plist_path.display());
        Ok(())
    }

    fn uninstall_launchd(&self) -> Result<()> {
        let plist_path = self.service_file_path()?;

        if plist_path.exists() {
            fs::remove_file(&plist_path).context("Failed to remove plist file")?;
            tracing::info!("Removed launchd service: {}", plist_path.display());
        }

        Ok(())
    }

    // ========================================================================
    // Linux systemd
    // ========================================================================

    fn install_systemd(&self) -> Result<()> {
        let dx_path = Self::get_dx_path()?;
        let service_path = self.service_file_path()?;

        // Create systemd user directory if needed
        if let Some(parent) = service_path.parent() {
            fs::create_dir_all(parent).context("Failed to create systemd user directory")?;
        }

        let working_dir = self
            .config
            .working_dir
            .clone()
            .unwrap_or_else(|| dirs::home_dir().unwrap_or_default());

        let service_content = format!(
            r#"[Unit]
Description=DX Gateway Service
After=network.target

[Service]
Type=simple
ExecStart={dx_path} gateway --port {port} {mdns_arg} {auth_arg}
WorkingDirectory={working_dir}
Restart=on-failure
RestartSec=10
Environment=RUST_LOG=info

# Security hardening
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ProtectHome=read-only
ReadWritePaths=%h/.dx %h/.config/dx

[Install]
WantedBy=default.target
"#,
            dx_path = dx_path.display(),
            port = self.config.port,
            working_dir = working_dir.display(),
            mdns_arg = if self.config.mdns { "--mdns" } else { "" },
            auth_arg = if self.config.require_auth {
                "--require-auth"
            } else {
                ""
            },
        );

        fs::write(&service_path, service_content).context("Failed to write service file")?;

        // Reload systemd
        Command::new("systemctl")
            .args(["--user", "daemon-reload"])
            .status()
            .context("Failed to reload systemd")?;

        // Enable if auto_start
        if self.config.auto_start {
            Command::new("systemctl")
                .args(["--user", "enable", &self.config.name])
                .status()
                .context("Failed to enable service")?;
        }

        tracing::info!("Installed systemd service: {}", service_path.display());
        Ok(())
    }

    fn uninstall_systemd(&self) -> Result<()> {
        // Disable service
        let _ = Command::new("systemctl")
            .args(["--user", "disable", &self.config.name])
            .status();

        let service_path = self.service_file_path()?;

        if service_path.exists() {
            fs::remove_file(&service_path).context("Failed to remove service file")?;
        }

        // Reload systemd
        Command::new("systemctl")
            .args(["--user", "daemon-reload"])
            .status()
            .context("Failed to reload systemd")?;

        tracing::info!("Removed systemd service: {}", service_path.display());
        Ok(())
    }

    // ========================================================================
    // Windows Task Scheduler
    // ========================================================================

    fn install_windows_task(&self) -> Result<()> {
        let dx_path = Self::get_dx_path()?;

        let working_dir = self
            .config
            .working_dir
            .clone()
            .unwrap_or_else(|| dirs::home_dir().unwrap_or_default());

        // Create the DX folder in Task Scheduler
        let _ = Command::new("schtasks")
            .args([
                "/Create",
                "/TN",
                "\\DX",
                "/SC",
                "ONCE",
                "/ST",
                "00:00",
                "/TR",
                "cmd /c echo placeholder",
            ])
            .status();

        // Build the command arguments
        let mut args = vec![
            "gateway".to_string(),
            "--port".to_string(),
            self.config.port.to_string(),
        ];

        if self.config.mdns {
            args.push("--mdns".to_string());
        }

        if self.config.require_auth {
            args.push("--require-auth".to_string());
        }

        let command = format!("\"{}\" {}", dx_path.display(), args.join(" "));

        // Create the task
        let mut schtasks_args = vec![
            "/Create".to_string(),
            "/TN".to_string(),
            format!("\\DX\\{}", self.config.name),
            "/TR".to_string(),
            command,
            "/SC".to_string(),
        ];

        if self.config.auto_start {
            schtasks_args.push("ONLOGON".to_string());
        } else {
            schtasks_args.push("ONCE".to_string());
            schtasks_args.push("/ST".to_string());
            schtasks_args.push("00:00".to_string());
        }

        schtasks_args.push("/F".to_string()); // Force overwrite

        let status = Command::new("schtasks")
            .args(&schtasks_args)
            .status()
            .context("Failed to run schtasks")?;

        if !status.success() {
            anyhow::bail!("Failed to create scheduled task");
        }

        tracing::info!("Installed Windows scheduled task: \\DX\\{}", self.config.name);
        Ok(())
    }

    fn uninstall_windows_task(&self) -> Result<()> {
        let status = Command::new("schtasks")
            .args([
                "/Delete",
                "/TN",
                &format!("\\DX\\{}", self.config.name),
                "/F",
            ])
            .status()
            .context("Failed to run schtasks")?;

        if !status.success() {
            tracing::warn!("schtasks delete returned non-zero (task may not exist)");
        }

        tracing::info!("Removed Windows scheduled task: \\DX\\{}", self.config.name);
        Ok(())
    }
}

/// Get logs from the service
pub fn get_service_logs(config: &ServiceConfig, lines: usize) -> Result<String> {
    match ServiceType::detect() {
        ServiceType::Launchd => {
            let log_path = config.log_path.clone().unwrap_or_else(|| {
                dirs::home_dir()
                    .unwrap_or_default()
                    .join("Library")
                    .join("Logs")
                    .join("dx-gateway.log")
            });

            if log_path.exists() {
                let output = Command::new("tail")
                    .args(["-n", &lines.to_string()])
                    .arg(&log_path)
                    .output()
                    .context("Failed to read log file")?;

                Ok(String::from_utf8_lossy(&output.stdout).to_string())
            } else {
                Ok("Log file not found".to_string())
            }
        }
        ServiceType::Systemd => {
            let output = Command::new("journalctl")
                .args([
                    "--user",
                    "-u",
                    &config.name,
                    "-n",
                    &lines.to_string(),
                    "--no-pager",
                ])
                .output()
                .context("Failed to run journalctl")?;

            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        }
        ServiceType::WindowsTask => {
            // Windows doesn't have centralized logging for tasks
            // Check Application event log
            let output = Command::new("powershell")
                .args([
                    "-Command",
                    &format!(
                        "Get-EventLog -LogName Application -Source 'dx*' -Newest {} | Format-List",
                        lines
                    ),
                ])
                .output()
                .context("Failed to read event log")?;

            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        }
        ServiceType::Unknown => {
            anyhow::bail!("Unsupported platform for log retrieval")
        }
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
        assert!(config.mdns);
        assert!(config.require_auth);
        assert!(config.auto_start);
    }

    #[test]
    fn test_service_type_detection() {
        let service_type = ServiceType::detect();

        #[cfg(target_os = "macos")]
        assert_eq!(service_type, ServiceType::Launchd);

        #[cfg(target_os = "linux")]
        assert_eq!(service_type, ServiceType::Systemd);

        #[cfg(target_os = "windows")]
        assert_eq!(service_type, ServiceType::WindowsTask);
    }

    #[test]
    fn test_service_manager_creation() {
        let config = ServiceConfig::default();
        let manager = ServiceManager::new(config);

        assert!(manager.service_file_path().is_ok());
    }
}
