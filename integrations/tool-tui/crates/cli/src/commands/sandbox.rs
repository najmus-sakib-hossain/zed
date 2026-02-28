//! Sandbox command implementation

use anyhow::{Context, Result};
use clap::{Args, Subcommand};
use std::path::PathBuf;

use crate::sandbox::{
    NetworkMode, ResourceLimits, SandboxBackendType, SandboxConfig, SandboxManager,
};
use crate::ui;

#[derive(Debug, Args)]
pub struct SandboxCommand {
    #[command(subcommand)]
    pub command: SandboxSubcommand,
}

#[derive(Debug, Subcommand)]
pub enum SandboxSubcommand {
    /// Create a new sandbox environment
    Create {
        /// Name of the sandbox
        name: String,

        /// Backend to use (docker, podman, native, wasm, auto)
        #[arg(short, long, default_value = "auto")]
        backend: String,

        /// Memory limit in MB
        #[arg(long)]
        memory: Option<u64>,

        /// CPU shares (0-1024)
        #[arg(long)]
        cpu: Option<u64>,

        /// Enable network access
        #[arg(long)]
        network: bool,

        /// Working directory inside sandbox
        #[arg(long, default_value = "/workspace")]
        workdir: PathBuf,

        /// Skip opening interactive shell after creation
        #[arg(long)]
        no_shell: bool,
    },

    /// List all sandboxes
    List {
        /// Output format (table, json)
        #[arg(short, long, default_value = "table")]
        format: String,
    },

    /// Execute command in sandbox
    Run {
        /// Sandbox name or ID
        sandbox: String,

        /// Command to execute
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        command: Vec<String>,
    },

    /// Open interactive shell in sandbox
    Shell {
        /// Sandbox name or ID
        sandbox: String,

        /// Shell to use (sh, bash, zsh)
        #[arg(short, long, default_value = "sh")]
        shell: String,
    },

    /// Copy file into sandbox
    CopyIn {
        /// Sandbox name or ID
        sandbox: String,

        /// Host file path
        host_path: PathBuf,

        /// Sandbox file path
        sandbox_path: PathBuf,
    },

    /// Copy file out of sandbox
    CopyOut {
        /// Sandbox name or ID
        sandbox: String,

        /// Sandbox file path
        sandbox_path: PathBuf,

        /// Host file path
        host_path: PathBuf,
    },

    /// Destroy a sandbox
    Destroy {
        /// Sandbox name or ID
        sandbox: String,

        /// Force destruction without confirmation
        #[arg(short, long)]
        force: bool,
    },

    /// Destroy all sandboxes
    DestroyAll {
        /// Force destruction without confirmation
        #[arg(short, long)]
        force: bool,
    },

    /// Show sandbox information
    Info {
        /// Sandbox name or ID
        sandbox: String,
    },
}

impl SandboxCommand {
    pub async fn run(&self) -> Result<()> {
        match &self.command {
            SandboxSubcommand::Create {
                name,
                backend,
                memory,
                cpu,
                network,
                workdir,
                no_shell,
            } => self.create(name, backend, *memory, *cpu, *network, workdir, !*no_shell).await,
            SandboxSubcommand::List { format } => self.list(format).await,
            SandboxSubcommand::Run { sandbox, command } => self.run_command(sandbox, command).await,
            SandboxSubcommand::Shell { sandbox, shell } => self.open_shell(sandbox, shell).await,
            SandboxSubcommand::CopyIn {
                sandbox,
                host_path,
                sandbox_path,
            } => self.copy_in(sandbox, host_path, sandbox_path).await,
            SandboxSubcommand::CopyOut {
                sandbox,
                sandbox_path,
                host_path,
            } => self.copy_out(sandbox, sandbox_path, host_path).await,
            SandboxSubcommand::Destroy { sandbox, force } => self.destroy(sandbox, *force).await,
            SandboxSubcommand::DestroyAll { force } => self.destroy_all(*force).await,
            SandboxSubcommand::Info { sandbox } => self.info(sandbox).await,
        }
    }

    async fn create(
        &self,
        name: &str,
        backend: &str,
        memory: Option<u64>,
        cpu: Option<u64>,
        network: bool,
        workdir: &PathBuf,
        open_shell: bool,
    ) -> Result<()> {
        let backend_type: SandboxBackendType = backend.parse()?;

        let mut limits = ResourceLimits::default();
        if let Some(mem) = memory {
            limits.memory_bytes = Some(mem * 1024 * 1024); // Convert MB to bytes
        }
        if let Some(cpu_shares) = cpu {
            limits.cpu_shares = Some(cpu_shares);
        }

        let config = SandboxConfig {
            limits,
            network: if network {
                NetworkMode::Bridge
            } else {
                NetworkMode::None
            },
            network_enabled: network,
            workdir: workdir.clone(),
            ..Default::default()
        };

        ui::logger::info(&format!("Creating sandbox '{}' with {} backend...", name, backend_type));

        let manager = SandboxManager::new();
        let id = manager.create(name.to_string(), backend_type, config).await?;

        ui::logger::success(&format!("Sandbox created: {}", id));

        // Open interactive shell if requested
        if open_shell {
            ui::logger::info("Opening interactive shell...");
            // Get sandbox info before opening shell
            let sandbox_info = manager.get(&id).await?;
            self.open_shell_direct(&sandbox_info, "sh").await?;
        }

        Ok(())
    }

    async fn open_shell_direct(
        &self,
        sandbox_info: &crate::sandbox::Sandbox,
        shell: &str,
    ) -> Result<()> {
        ui::logger::info(&format!("Opening {} shell in sandbox...", shell));
        ui::logger::info("Type 'exit' to leave the sandbox");

        match sandbox_info.backend {
            SandboxBackendType::Docker | SandboxBackendType::Podman => {
                // Check if docker is available
                if which::which("docker").is_err() {
                    ui::logger::warn("Docker is not installed or not in PATH");
                    ui::logger::info("Falling back to native sandbox mode...");
                    return self.open_native_shell(&sandbox_info.root_dir).await;
                }

                // For Docker backend, use docker exec -it with the container ID
                let status = std::process::Command::new("docker")
                    .args(&["exec", "-it", &sandbox_info.id, shell])
                    .status()
                    .context("Failed to execute docker exec")?;

                if !status.success() {
                    return Err(anyhow::anyhow!("Shell exited with error"));
                }
            }
            SandboxBackendType::Native | SandboxBackendType::Wasm | SandboxBackendType::Auto => {
                // For native backend, open shell in the sandbox directory
                return self.open_native_shell(&sandbox_info.root_dir).await;
            }
        }

        Ok(())
    }

    async fn open_shell_by_id(&self, sandbox_id: &str, shell: &str) -> Result<()> {
        ui::logger::info(&format!("Opening {} shell in sandbox...", shell));
        ui::logger::info("Type 'exit' to leave the sandbox");

        // For native backend, we need to get the sandbox info
        let manager = SandboxManager::new();

        // Try to find sandbox by ID
        let sandbox_info = manager.get(sandbox_id).await?;

        match sandbox_info.backend {
            SandboxBackendType::Docker | SandboxBackendType::Podman => {
                // Check if docker is available
                if which::which("docker").is_err() {
                    ui::logger::warn("Docker is not installed or not in PATH");
                    ui::logger::info("Falling back to native sandbox mode...");
                    return self.open_native_shell(&sandbox_info.root_dir).await;
                }

                // For Docker backend, use docker exec -it with the container ID
                let status = std::process::Command::new("docker")
                    .args(&["exec", "-it", sandbox_id, shell])
                    .status()
                    .context("Failed to execute docker exec")?;

                if !status.success() {
                    return Err(anyhow::anyhow!("Shell exited with error"));
                }
            }
            SandboxBackendType::Native | SandboxBackendType::Wasm | SandboxBackendType::Auto => {
                // For native backend, open shell in the sandbox directory
                return self.open_native_shell(&sandbox_info.root_dir).await;
            }
        }

        Ok(())
    }

    async fn open_native_shell(&self, sandbox_root: &std::path::Path) -> Result<()> {
        ui::logger::info(&format!("Sandbox directory: {}", sandbox_root.display()));
        ui::logger::info("You are now in an isolated directory environment");
        ui::logger::info("Files created here are isolated from your main system");

        // Determine shell to use
        let shell_cmd = if cfg!(windows) {
            // On Windows, use cmd or powershell
            if which::which("pwsh").is_ok() {
                "pwsh"
            } else if which::which("powershell").is_ok() {
                "powershell"
            } else {
                "cmd"
            }
        } else {
            // On Unix, prefer bash, fall back to sh
            if which::which("bash").is_ok() {
                "bash"
            } else {
                "sh"
            }
        };

        // Open interactive shell in sandbox directory
        let status = std::process::Command::new(shell_cmd)
            .current_dir(sandbox_root)
            .status()
            .context("Failed to open shell")?;

        if !status.success() {
            return Err(anyhow::anyhow!("Shell exited with error"));
        }

        Ok(())
    }

    async fn list(&self, format: &str) -> Result<()> {
        let manager = SandboxManager::new();
        let sandboxes = manager.list().await;

        if sandboxes.is_empty() {
            ui::logger::info("No sandboxes found");
            return Ok(());
        }

        match format {
            "json" => {
                let json = serde_json::to_string_pretty(&sandboxes)?;
                println!("{}", json);
            }
            "table" | _ => {
                println!("{:<20} {:<40} {:<10} {:<10}", "NAME", "ID", "BACKEND", "STATUS");
                println!("{}", "-".repeat(80));
                for sandbox in sandboxes {
                    println!(
                        "{:<20} {:<40} {:<10} {:<10}",
                        sandbox.name, sandbox.id, sandbox.backend, sandbox.status
                    );
                }
            }
        }

        Ok(())
    }

    async fn run_command(&self, sandbox: &str, command: &[String]) -> Result<()> {
        if command.is_empty() {
            return Err(anyhow::anyhow!("No command provided"));
        }

        let manager = SandboxManager::new();

        // Try to find by name first, then by ID
        let sandbox_info = match manager.get_by_name(sandbox).await {
            Ok(info) => info,
            Err(_) => manager.get(sandbox).await?,
        };

        ui::logger::info(&format!("Executing in sandbox '{}'...", sandbox_info.name));

        let result = manager.execute(&sandbox_info.id, command).await?;

        if !result.stdout.is_empty() {
            println!("{}", result.stdout);
        }

        if !result.stderr.is_empty() {
            eprintln!("{}", result.stderr);
        }

        ui::logger::info(&format!(
            "Command completed in {}ms with exit code {}",
            result.duration_ms, result.exit_code
        ));

        std::process::exit(result.exit_code);
    }

    async fn copy_in(
        &self,
        sandbox: &str,
        host_path: &PathBuf,
        sandbox_path: &PathBuf,
    ) -> Result<()> {
        let manager = SandboxManager::new();
        let sandbox_info = match manager.get_by_name(sandbox).await {
            Ok(info) => info,
            Err(_) => manager.get(sandbox).await?,
        };

        ui::logger::info(&format!("Copying {} to sandbox...", host_path.display()));

        manager.copy_in(&sandbox_info.id, host_path, sandbox_path).await?;

        ui::logger::success("File copied successfully");
        Ok(())
    }

    async fn copy_out(
        &self,
        sandbox: &str,
        sandbox_path: &PathBuf,
        host_path: &PathBuf,
    ) -> Result<()> {
        let manager = SandboxManager::new();
        let sandbox_info = match manager.get_by_name(sandbox).await {
            Ok(info) => info,
            Err(_) => manager.get(sandbox).await?,
        };

        ui::logger::info(&format!("Copying from sandbox to {}...", host_path.display()));

        manager.copy_out(&sandbox_info.id, sandbox_path, host_path).await?;

        ui::logger::success("File copied successfully");
        Ok(())
    }

    async fn destroy(&self, sandbox: &str, force: bool) -> Result<()> {
        let manager = SandboxManager::new();
        let sandbox_info = match manager.get_by_name(sandbox).await {
            Ok(info) => info,
            Err(_) => manager.get(sandbox).await?,
        };

        if !force {
            let confirm = dialoguer::Confirm::new()
                .with_prompt(format!("Destroy sandbox '{}'?", sandbox_info.name))
                .interact()?;

            if !confirm {
                ui::logger::info("Cancelled");
                return Ok(());
            }
        }

        ui::logger::info(&format!("Destroying sandbox '{}'...", sandbox_info.name));

        manager.destroy(&sandbox_info.id).await?;

        ui::logger::success("Sandbox destroyed");
        Ok(())
    }

    async fn destroy_all(&self, force: bool) -> Result<()> {
        let manager = SandboxManager::new();
        let sandboxes = manager.list().await;

        if sandboxes.is_empty() {
            ui::logger::info("No sandboxes to destroy");
            return Ok(());
        }

        if !force {
            let confirm = dialoguer::Confirm::new()
                .with_prompt(format!("Destroy all {} sandboxes?", sandboxes.len()))
                .interact()?;

            if !confirm {
                ui::logger::info("Cancelled");
                return Ok(());
            }
        }

        ui::logger::info("Destroying all sandboxes...");

        manager.destroy_all().await?;

        ui::logger::success("All sandboxes destroyed");
        Ok(())
    }

    async fn open_shell(&self, sandbox: &str, shell: &str) -> Result<()> {
        let manager = SandboxManager::new();
        let sandbox_info = match manager.get_by_name(sandbox).await {
            Ok(info) => info,
            Err(_) => manager.get(sandbox).await?,
        };

        ui::logger::info(&format!("Opening {} shell in sandbox '{}'...", shell, sandbox_info.name));
        ui::logger::info("Type 'exit' to leave the sandbox");

        // For Docker backend, use docker exec -it
        if matches!(sandbox_info.backend, SandboxBackendType::Docker | SandboxBackendType::Podman) {
            let status = std::process::Command::new("docker")
                .args(&["exec", "-it", &sandbox_info.id, shell])
                .status()
                .context("Failed to execute docker exec")?;

            if !status.success() {
                return Err(anyhow::anyhow!("Shell exited with error"));
            }
        } else {
            // For native/wasm, run interactive loop
            ui::logger::warn("Interactive shell not fully supported for native/wasm backends");
            ui::logger::info("Use 'dx sandbox run' to execute commands");
        }

        Ok(())
    }

    async fn info(&self, sandbox: &str) -> Result<()> {
        let manager = SandboxManager::new();
        let sandbox_info = match manager.get_by_name(sandbox).await {
            Ok(info) => info,
            Err(_) => manager.get(sandbox).await?,
        };

        println!("Sandbox Information:");
        println!("  Name:    {}", sandbox_info.name);
        println!("  ID:      {}", sandbox_info.id);
        println!("  Backend: {}", sandbox_info.backend);
        println!("  Status:  {}", sandbox_info.status);
        println!("  Root:    {}", sandbox_info.root_dir.display());
        println!("\nConfiguration:");
        println!(
            "  Memory:  {:?}",
            sandbox_info
                .config
                .limits
                .memory_bytes
                .map(|b| format!("{} MB", b / 1024 / 1024))
        );
        println!("  CPU:     {:?}", sandbox_info.config.limits.cpu_shares);
        println!("  Network: {}", sandbox_info.config.network);
        println!("  Workdir: {}", sandbox_info.config.workdir.display());

        Ok(())
    }
}
