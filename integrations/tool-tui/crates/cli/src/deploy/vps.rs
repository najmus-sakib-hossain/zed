//! VPS Deployment
//!
//! Deploy to VPS via SSH.

use super::{DeployConfig, DeployError, DeployPlatform, DeployResult, DeployStatus};
use std::process::Command;

/// Deploy to VPS via SSH
pub async fn deploy_to_vps(
    config: &DeployConfig,
    host: &str,
    user: &str,
) -> Result<DeployResult, DeployError> {
    let ssh_target = format!("{}@{}", user, host);
    let binary_path = get_binary_path(config)?;

    // Step 1: Ensure target directory exists
    let mkdir_status =
        Command::new("ssh").args([&ssh_target, "mkdir", "-p", "/opt/dx"]).status()?;

    if !mkdir_status.success() {
        return Err(DeployError::SshError("Failed to create target directory".to_string()));
    }

    // Step 2: Copy binary via SCP
    let scp_status = Command::new("scp")
        .args([
            binary_path.to_str().unwrap_or(""),
            &format!("{}:/opt/dx/{}", ssh_target, config.binary_name),
        ])
        .status()?;

    if !scp_status.success() {
        return Err(DeployError::SshError("Failed to copy binary".to_string()));
    }

    // Step 3: Set permissions
    let chmod_status = Command::new("ssh")
        .args([
            &ssh_target,
            "chmod",
            "+x",
            &format!("/opt/dx/{}", config.binary_name),
        ])
        .status()?;

    if !chmod_status.success() {
        return Err(DeployError::SshError("Failed to set permissions".to_string()));
    }

    // Step 4: Create systemd service
    let service_content = generate_systemd_service(config);
    let service_cmd = format!(
        "echo '{}' | sudo tee /etc/systemd/system/{}.service",
        service_content, config.binary_name
    );

    let service_status = Command::new("ssh").args([&ssh_target, &service_cmd]).status()?;

    if !service_status.success() {
        return Err(DeployError::SshError("Failed to create systemd service".to_string()));
    }

    // Step 5: Reload and restart service
    let reload_status = Command::new("ssh")
        .args([
            &ssh_target,
            &format!(
                "sudo systemctl daemon-reload && sudo systemctl enable {} && sudo systemctl restart {}",
                config.binary_name, config.binary_name
            ),
        ])
        .status()?;

    if !reload_status.success() {
        return Err(DeployError::SshError("Failed to start service".to_string()));
    }

    Ok(DeployResult {
        platform: DeployPlatform::Vps,
        url: Some(format!("http://{}:8080", host)),
        status: DeployStatus::Success,
        message: format!("Deployed to {} as systemd service", host),
    })
}

/// Get binary path
fn get_binary_path(config: &DeployConfig) -> Result<std::path::PathBuf, DeployError> {
    let path = config.project_root.join("target").join("release").join(&config.binary_name);

    #[cfg(windows)]
    let path = path.with_extension("exe");

    if path.exists() {
        Ok(path)
    } else {
        Err(DeployError::BuildFailed(format!("Binary not found: {:?}", path)))
    }
}

/// Generate systemd service file
fn generate_systemd_service(config: &DeployConfig) -> String {
    format!(
        r#"[Unit]
Description=DX Agent Service
After=network.target

[Service]
Type=simple
User=dx
Group=dx
WorkingDirectory=/opt/dx
ExecStart=/opt/dx/{binary}
Restart=always
RestartSec=10
StandardOutput=journal
StandardError=journal
Environment=RUST_LOG=info
Environment=DX_ENV=production

# Resource limits
MemoryMax={memory}
CPUQuota=100%

# Security hardening
NoNewPrivileges=true
ProtectSystem=strict
ProtectHome=true
PrivateTmp=true
PrivateDevices=true
ProtectKernelTunables=true
ProtectKernelModules=true
ProtectControlGroups=true

[Install]
WantedBy=multi-user.target
"#,
        binary = config.binary_name,
        memory = format!("{}M", config.target_memory / 1024 / 1024)
    )
}

/// Check SSH connectivity
pub async fn check_ssh_connection(host: &str, user: &str) -> Result<bool, DeployError> {
    let ssh_target = format!("{}@{}", user, host);

    let status = Command::new("ssh")
        .args(["-o", "ConnectTimeout=5", &ssh_target, "echo", "ok"])
        .status()?;

    Ok(status.success())
}

/// Get server info via SSH
pub async fn get_server_info(host: &str, user: &str) -> Result<ServerInfo, DeployError> {
    let ssh_target = format!("{}@{}", user, host);

    // Get OS info
    let os_output = Command::new("ssh").args([&ssh_target, "cat", "/etc/os-release"]).output()?;

    let os_info = String::from_utf8_lossy(&os_output.stdout).to_string();

    // Get memory info
    let mem_output = Command::new("ssh").args([&ssh_target, "free", "-m"]).output()?;

    let mem_info = String::from_utf8_lossy(&mem_output.stdout).to_string();

    // Get disk info
    let disk_output = Command::new("ssh").args([&ssh_target, "df", "-h", "/"]).output()?;

    let disk_info = String::from_utf8_lossy(&disk_output.stdout).to_string();

    Ok(ServerInfo {
        os: parse_os_name(&os_info),
        memory_mb: parse_memory(&mem_info),
        disk_gb: parse_disk(&disk_info),
        raw_os: os_info,
    })
}

/// Server information
#[derive(Debug)]
pub struct ServerInfo {
    /// OS name
    pub os: String,
    /// Total memory in MB
    pub memory_mb: u64,
    /// Available disk in GB
    pub disk_gb: u64,
    /// Raw OS release info
    pub raw_os: String,
}

/// Parse OS name from /etc/os-release
fn parse_os_name(content: &str) -> String {
    for line in content.lines() {
        if line.starts_with("PRETTY_NAME=") {
            return line.trim_start_matches("PRETTY_NAME=").trim_matches('"').to_string();
        }
    }
    "Unknown".to_string()
}

/// Parse memory from `free -m` output
fn parse_memory(content: &str) -> u64 {
    for line in content.lines() {
        if line.starts_with("Mem:") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                return parts[1].parse().unwrap_or(0);
            }
        }
    }
    0
}

/// Parse disk from `df -h` output
fn parse_disk(content: &str) -> u64 {
    for line in content.lines() {
        if line.contains('/') && !line.starts_with("Filesystem") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 4 {
                let avail = parts[3];
                if avail.ends_with('G') {
                    return avail.trim_end_matches('G').parse().unwrap_or(0);
                }
            }
        }
    }
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_systemd_service() {
        let config = DeployConfig {
            binary_name: "test-app".to_string(),
            target_memory: 128 * 1024 * 1024,
            ..Default::default()
        };

        let service = generate_systemd_service(&config);

        assert!(service.contains("test-app"));
        assert!(service.contains("MemoryMax=128M"));
        assert!(service.contains("NoNewPrivileges=true"));
    }

    #[test]
    fn test_parse_os_name() {
        let content = r#"
NAME="Ubuntu"
VERSION="22.04.3 LTS (Jammy Jellyfish)"
PRETTY_NAME="Ubuntu 22.04.3 LTS"
"#;

        let name = parse_os_name(content);
        assert_eq!(name, "Ubuntu 22.04.3 LTS");
    }

    #[test]
    fn test_parse_memory() {
        let content = r#"
              total        used        free      shared  buff/cache   available
Mem:           7962        2341        3421          42        2199        5296
"#;

        let memory = parse_memory(content);
        assert_eq!(memory, 7962);
    }

    #[test]
    fn test_parse_disk() {
        let content = r#"
Filesystem      Size  Used Avail Use% Mounted on
/dev/sda1        50G   20G   28G  42% /
"#;

        let disk = parse_disk(content);
        assert_eq!(disk, 28);
    }
}
