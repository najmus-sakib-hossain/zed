//! Daemon service — manages the background agent lifecycle.

use serde::{Deserialize, Serialize};

/// State of the daemon process.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DaemonState {
    Stopped,
    Starting,
    Running,
    Paused,
    Stopping,
    Error,
}

/// Configuration for the daemon service.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaemonConfig {
    /// Whether to start on system boot.
    pub autostart: bool,
    /// Port for the local IPC socket.
    pub ipc_port: u16,
    /// Max memory budget in MB.
    pub max_memory_mb: u64,
    /// Log file path.
    pub log_path: Option<String>,
}

impl Default for DaemonConfig {
    fn default() -> Self {
        Self {
            autostart: false,
            ipc_port: 42069,
            max_memory_mb: 512,
            log_path: None,
        }
    }
}

/// The background daemon service.
pub struct DaemonService {
    state: DaemonState,
    config: DaemonConfig,
}

impl DaemonService {
    pub fn new(config: DaemonConfig) -> Self {
        Self {
            state: DaemonState::Stopped,
            config,
        }
    }

    pub fn state(&self) -> DaemonState {
        self.state
    }

    pub fn config(&self) -> &DaemonConfig {
        &self.config
    }

    /// Start the daemon.
    pub fn start(&mut self) -> anyhow::Result<()> {
        if self.state == DaemonState::Running {
            return Ok(());
        }
        log::info!("Starting DX daemon on port {}", self.config.ipc_port);
        self.state = DaemonState::Starting;
        // Placeholder — real implementation spawns the background process
        self.state = DaemonState::Running;
        Ok(())
    }

    /// Stop the daemon.
    pub fn stop(&mut self) -> anyhow::Result<()> {
        log::info!("Stopping DX daemon");
        self.state = DaemonState::Stopping;
        // Placeholder
        self.state = DaemonState::Stopped;
        Ok(())
    }

    /// Pause the daemon (stop processing, keep alive).
    pub fn pause(&mut self) {
        if self.state == DaemonState::Running {
            self.state = DaemonState::Paused;
        }
    }

    /// Resume from paused state.
    pub fn resume(&mut self) {
        if self.state == DaemonState::Paused {
            self.state = DaemonState::Running;
        }
    }

    /// Install the daemon as a system service.
    pub fn install_service(&self) -> anyhow::Result<()> {
        log::info!("Installing DX daemon as system service");
        let exe_path = std::env::current_exe()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| "dx-daemon".to_string());

        #[cfg(target_os = "linux")]
        self.install_systemd_service(&exe_path)?;

        #[cfg(target_os = "macos")]
        self.install_launchd_service(&exe_path)?;

        #[cfg(target_os = "windows")]
        self.install_windows_service(&exe_path)?;

        Ok(())
    }

    /// Uninstall the daemon system service.
    pub fn uninstall_service(&self) -> anyhow::Result<()> {
        log::info!("Uninstalling DX daemon system service");

        #[cfg(target_os = "linux")]
        self.uninstall_systemd_service()?;

        #[cfg(target_os = "macos")]
        self.uninstall_launchd_service()?;

        #[cfg(target_os = "windows")]
        self.uninstall_windows_service()?;

        Ok(())
    }

    /// Check whether the daemon is installed as a system service.
    pub fn is_service_installed(&self) -> bool {
        #[cfg(target_os = "linux")]
        {
            let path = format!(
                "{}/.config/systemd/user/dx-daemon.service",
                std::env::var("HOME").unwrap_or_default()
            );
            std::path::Path::new(&path).exists()
        }
        #[cfg(target_os = "macos")]
        {
            let path = format!(
                "{}/Library/LaunchAgents/com.dx.daemon.plist",
                std::env::var("HOME").unwrap_or_default()
            );
            std::path::Path::new(&path).exists()
        }
        #[cfg(target_os = "windows")]
        {
            let output = std::process::Command::new("sc")
                .args(["query", "DxDaemon"])
                .output();
            matches!(output, Ok(o) if o.status.success())
        }
        #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
        {
            false
        }
    }

    // ── Linux: systemd user service ──────────────────────────────────

    #[cfg(target_os = "linux")]
    fn install_systemd_service(&self, exe_path: &str) -> anyhow::Result<()> {
        let home = std::env::var("HOME")
            .map_err(|_| anyhow::anyhow!("HOME not set"))?;
        let dir = format!("{}/.config/systemd/user", home);
        std::fs::create_dir_all(&dir)?;

        let log_arg = self
            .config
            .log_path
            .as_deref()
            .map(|p| format!(" --log-file {}", p))
            .unwrap_or_default();

        let unit = format!(
            r#"[Unit]
Description=DX AI Daemon
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
ExecStart={exe_path} daemon --port {port}{log_arg}
Restart=on-failure
RestartSec=5
MemoryMax={mem}M
Environment=RUST_LOG=info

[Install]
WantedBy=default.target
"#,
            exe_path = exe_path,
            port = self.config.ipc_port,
            log_arg = log_arg,
            mem = self.config.max_memory_mb,
        );

        let path = format!("{}/dx-daemon.service", dir);
        std::fs::write(&path, unit)?;
        log::info!("Wrote systemd unit to {}", path);

        // Enable (but don't start) via systemctl --user
        let status = std::process::Command::new("systemctl")
            .args(["--user", "daemon-reload"])
            .status();
        if let Ok(s) = status {
            if !s.success() {
                log::warn!("systemctl daemon-reload exited with {}", s);
            }
        }

        if self.config.autostart {
            let _ = std::process::Command::new("systemctl")
                .args(["--user", "enable", "dx-daemon.service"])
                .status();
        }

        Ok(())
    }

    #[cfg(target_os = "linux")]
    fn uninstall_systemd_service(&self) -> anyhow::Result<()> {
        let _ = std::process::Command::new("systemctl")
            .args(["--user", "stop", "dx-daemon.service"])
            .status();
        let _ = std::process::Command::new("systemctl")
            .args(["--user", "disable", "dx-daemon.service"])
            .status();
        let home = std::env::var("HOME").unwrap_or_default();
        let path = format!("{}/.config/systemd/user/dx-daemon.service", home);
        if std::path::Path::new(&path).exists() {
            std::fs::remove_file(&path)?;
        }
        let _ = std::process::Command::new("systemctl")
            .args(["--user", "daemon-reload"])
            .status();
        Ok(())
    }

    // ── macOS: launchd user agent ────────────────────────────────────

    #[cfg(target_os = "macos")]
    fn install_launchd_service(&self, exe_path: &str) -> anyhow::Result<()> {
        let home = std::env::var("HOME")
            .map_err(|_| anyhow::anyhow!("HOME not set"))?;
        let dir = format!("{}/Library/LaunchAgents", home);
        std::fs::create_dir_all(&dir)?;

        let keep_alive = if self.config.autostart { "true" } else { "false" };

        let mut log_keys = String::new();
        if let Some(ref log_path) = self.config.log_path {
            log_keys.push_str(&format!(
                "    <key>StandardOutPath</key>\n    <string>{log_path}</string>\n    <key>StandardErrorPath</key>\n    <string>{log_path}.err</string>\n",
                log_path = log_path,
            ));
        }

        let plist = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN"
  "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.dx.daemon</string>
    <key>ProgramArguments</key>
    <array>
        <string>{exe_path}</string>
        <string>daemon</string>
        <string>--port</string>
        <string>{port}</string>
    </array>
    <key>RunAtLoad</key>
    <{keep_alive}/>
    <key>KeepAlive</key>
    <dict>
        <key>SuccessfulExit</key>
        <false/>
    </dict>
    <key>ProcessType</key>
    <string>Background</string>
    <key>SoftResourceLimits</key>
    <dict>
        <key>NumberOfFiles</key>
        <integer>4096</integer>
    </dict>
{log_keys}</dict>
</plist>
"#,
            exe_path = exe_path,
            port = self.config.ipc_port,
            keep_alive = keep_alive,
            log_keys = log_keys,
        );

        let path = format!("{}/com.dx.daemon.plist", dir);
        std::fs::write(&path, plist)?;
        log::info!("Wrote launchd plist to {}", path);
        Ok(())
    }

    #[cfg(target_os = "macos")]
    fn uninstall_launchd_service(&self) -> anyhow::Result<()> {
        let _ = std::process::Command::new("launchctl")
            .args(["unload", "-w"])
            .arg(format!(
                "{}/Library/LaunchAgents/com.dx.daemon.plist",
                std::env::var("HOME").unwrap_or_default()
            ))
            .status();

        let home = std::env::var("HOME").unwrap_or_default();
        let path = format!("{}/Library/LaunchAgents/com.dx.daemon.plist", home);
        if std::path::Path::new(&path).exists() {
            std::fs::remove_file(&path)?;
        }
        Ok(())
    }

    // ── Windows: Service Control Manager ─────────────────────────────

    #[cfg(target_os = "windows")]
    fn install_windows_service(&self, exe_path: &str) -> anyhow::Result<()> {
        // Use `sc.exe create` to register the service.
        let binpath = format!(
            "\"{}\" daemon --port {}",
            exe_path, self.config.ipc_port
        );

        let status = std::process::Command::new("sc")
            .args([
                "create",
                "DxDaemon",
                &format!("binPath= {}", binpath),
                "start= demand",
                "DisplayName= DX AI Daemon",
            ])
            .status()
            .map_err(|e| anyhow::anyhow!("Failed to execute sc.exe: {}", e))?;

        if !status.success() {
            return Err(anyhow::anyhow!(
                "sc create exited with code {:?}",
                status.code()
            ));
        }

        // Set description
        let _ = std::process::Command::new("sc")
            .args([
                "description",
                "DxDaemon",
                "DX Universal AI Platform background daemon",
            ])
            .status();

        // Set failure recovery: restart after 5 seconds on first 3 failures
        let _ = std::process::Command::new("sc")
            .args([
                "failure",
                "DxDaemon",
                "reset= 86400",
                "actions= restart/5000/restart/5000/restart/5000",
            ])
            .status();

        if self.config.autostart {
            let _ = std::process::Command::new("sc")
                .args(["config", "DxDaemon", "start= auto"])
                .status();
        }

        log::info!("Registered Windows service DxDaemon");
        Ok(())
    }

    #[cfg(target_os = "windows")]
    fn uninstall_windows_service(&self) -> anyhow::Result<()> {
        let _ = std::process::Command::new("sc")
            .args(["stop", "DxDaemon"])
            .status();
        let status = std::process::Command::new("sc")
            .args(["delete", "DxDaemon"])
            .status()
            .map_err(|e| anyhow::anyhow!("Failed to execute sc.exe: {}", e))?;
        if !status.success() {
            return Err(anyhow::anyhow!(
                "sc delete exited with code {:?}",
                status.code()
            ));
        }
        log::info!("Deleted Windows service DxDaemon");
        Ok(())
    }

    /// Generate the systemd unit / launchd plist / Windows service config
    /// as a string (for preview without writing).
    pub fn preview_service_config(&self) -> String {
        let exe = std::env::current_exe()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| "/usr/local/bin/dx-daemon".to_string());

        #[cfg(target_os = "linux")]
        {
            format!(
                "[Unit]\nDescription=DX AI Daemon\n\n[Service]\nExecStart={} daemon --port {}\nRestart=on-failure\nMemoryMax={}M\n\n[Install]\nWantedBy=default.target\n",
                exe, self.config.ipc_port, self.config.max_memory_mb
            )
        }
        #[cfg(target_os = "macos")]
        {
            format!(
                "<plist>\n  <dict>\n    <key>Label</key><string>com.dx.daemon</string>\n    <key>ProgramArguments</key><array><string>{}</string><string>daemon</string><string>--port</string><string>{}</string></array>\n  </dict>\n</plist>\n",
                exe, self.config.ipc_port
            )
        }
        #[cfg(target_os = "windows")]
        {
            format!(
                "sc create DxDaemon binPath= \"{}\" daemon --port {} start= demand\n",
                exe, self.config.ipc_port
            )
        }
        #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
        {
            "Service installation not supported on this platform".to_string()
        }
    }
}
