//! Native OS-specific sandbox implementations

use anyhow::{Context, Result};
use async_trait::async_trait;
use std::path::Path;
use std::time::Instant;

use super::backend::{SandboxBackend, SandboxBackendType, SandboxResult};
use super::config::SandboxConfig;

/// Native sandbox implementation (platform-specific)
pub struct NativeSandbox {
    root_dir: std::path::PathBuf,
    config: SandboxConfig,
    #[cfg(unix)]
    inner: UnixSandbox,
    #[cfg(windows)]
    inner: WindowsSandbox,
}

impl NativeSandbox {
    pub fn new(root_dir: std::path::PathBuf) -> Result<Self> {
        Ok(Self {
            root_dir: root_dir.clone(),
            config: SandboxConfig::default(),
            #[cfg(unix)]
            inner: UnixSandbox::new(root_dir)?,
            #[cfg(windows)]
            inner: WindowsSandbox::new(root_dir)?,
        })
    }
}

#[async_trait]
impl SandboxBackend for NativeSandbox {
    async fn create(&mut self, config: &SandboxConfig) -> Result<()> {
        self.config = config.clone();
        self.inner.create(config).await
    }

    async fn execute(&self, command: &[String]) -> Result<SandboxResult> {
        self.inner.execute(command).await
    }

    async fn copy_in(&self, host_path: &Path, sandbox_path: &Path) -> Result<()> {
        self.inner.copy_in(host_path, sandbox_path).await
    }

    async fn copy_out(&self, sandbox_path: &Path, host_path: &Path) -> Result<()> {
        self.inner.copy_out(sandbox_path, host_path).await
    }

    async fn destroy(&mut self) -> Result<()> {
        self.inner.destroy().await
    }

    fn is_available() -> bool {
        true // Native is always available
    }

    fn backend_type(&self) -> SandboxBackendType {
        SandboxBackendType::Native
    }
}

// Unix implementation (Linux/macOS)
#[cfg(unix)]
mod unix_impl {
    use super::*;
    use nix::sched::{CloneFlags, unshare};
    use nix::sys::resource::{Resource, setrlimit};
    use nix::unistd::{Pid, chroot};
    use std::os::unix::process::CommandExt;
    use std::process::Command;

    pub struct UnixSandbox {
        root_dir: std::path::PathBuf,
    }

    // SAFETY: UnixSandbox only contains PathBuf which is Send+Sync
    unsafe impl Send for UnixSandbox {}
    unsafe impl Sync for UnixSandbox {}

    impl UnixSandbox {
        pub fn new(root_dir: std::path::PathBuf) -> Result<Self> {
            Ok(Self { root_dir })
        }

        pub async fn create(&mut self, config: &SandboxConfig) -> Result<()> {
            // Create sandbox root directory
            tokio::fs::create_dir_all(&self.root_dir).await?;

            // Create basic filesystem structure
            for dir in &["bin", "lib", "tmp", "workspace"] {
                tokio::fs::create_dir_all(self.root_dir.join(dir)).await?;
            }

            // Copy essential binaries (sh, ls, etc.)
            self.copy_essential_binaries().await?;

            Ok(())
        }

        pub async fn execute(&self, command: &[String]) -> Result<SandboxResult> {
            let start_time = Instant::now();
            let root_dir = self.root_dir.clone();

            // Spawn in blocking task due to fork/exec
            let cmd_vec = command.to_vec();
            let result = tokio::task::spawn_blocking(move || {
                let mut cmd = Command::new(&cmd_vec[0]);
                cmd.args(&cmd_vec[1..]);

                // SAFETY: We're using standard Unix syscalls for sandboxing
                unsafe {
                    cmd.pre_exec(move || {
                        // Unshare namespaces (Linux only)
                        #[cfg(target_os = "linux")]
                        {
                            let flags = CloneFlags::CLONE_NEWNS
                                | CloneFlags::CLONE_NEWPID
                                | CloneFlags::CLONE_NEWNET
                                | CloneFlags::CLONE_NEWUTS;
                            unshare(flags)
                                .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
                        }

                        // Change root directory
                        std::env::set_current_dir(&root_dir)?;
                        chroot(&root_dir)
                            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

                        Ok(())
                    });
                }

                cmd.output()
            })
            .await??;

            let duration_ms = start_time.elapsed().as_millis() as u64;

            Ok(SandboxResult {
                exit_code: result.status.code().unwrap_or(1),
                stdout: String::from_utf8_lossy(&result.stdout).to_string(),
                stderr: String::from_utf8_lossy(&result.stderr).to_string(),
                duration_ms,
            })
        }

        pub async fn copy_in(&self, host_path: &Path, sandbox_path: &Path) -> Result<()> {
            let dest = self.root_dir.join(sandbox_path.strip_prefix("/").unwrap_or(sandbox_path));
            if let Some(parent) = dest.parent() {
                tokio::fs::create_dir_all(parent).await?;
            }
            tokio::fs::copy(host_path, dest).await?;
            Ok(())
        }

        pub async fn copy_out(&self, sandbox_path: &Path, host_path: &Path) -> Result<()> {
            let src = self.root_dir.join(sandbox_path.strip_prefix("/").unwrap_or(sandbox_path));
            if let Some(parent) = host_path.parent() {
                tokio::fs::create_dir_all(parent).await?;
            }
            tokio::fs::copy(src, host_path).await?;
            Ok(())
        }

        pub async fn destroy(&mut self) -> Result<()> {
            tokio::fs::remove_dir_all(&self.root_dir).await?;
            Ok(())
        }

        async fn copy_essential_binaries(&self) -> Result<()> {
            // Copy sh for command execution
            let binaries = vec!["/bin/sh", "/bin/ls", "/bin/cat"];

            for binary in binaries {
                if let Ok(_) = tokio::fs::metadata(binary).await {
                    let dest = self.root_dir.join(binary.trim_start_matches('/'));
                    if let Some(parent) = dest.parent() {
                        tokio::fs::create_dir_all(parent).await?;
                    }
                    let _ = tokio::fs::copy(binary, dest).await;
                }
            }

            Ok(())
        }
    }
}

#[cfg(unix)]
pub use unix_impl::UnixSandbox;

// Windows implementation
#[cfg(windows)]
mod windows_impl {
    use super::*;
    use std::process::Command;
    use windows::Win32::System::JobObjects::*;
    // use windows::Win32::System::Threading::*;

    pub struct WindowsSandbox {
        root_dir: std::path::PathBuf,
        job_handle: Option<isize>, // Store as isize for Send/Sync
    }

    impl WindowsSandbox {
        pub fn new(root_dir: std::path::PathBuf) -> Result<Self> {
            Ok(Self {
                root_dir,
                job_handle: None,
            })
        }

        pub async fn create(&mut self, config: &SandboxConfig) -> Result<()> {
            // Create sandbox root directory
            tokio::fs::create_dir_all(&self.root_dir).await?;

            // Create workspace directory structure
            tokio::fs::create_dir_all(self.root_dir.join("workspace")).await?;
            tokio::fs::create_dir_all(self.root_dir.join("tmp")).await?;

            // Create job object for process isolation
            // SAFETY: Windows API call for job object creation
            unsafe {
                let job = CreateJobObjectW(None, None)?;

                // Set job limits
                let mut info = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
                info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;

                if let Some(memory) = config.limits.memory_bytes {
                    info.BasicLimitInformation.LimitFlags |= JOB_OBJECT_LIMIT_PROCESS_MEMORY;
                    info.ProcessMemoryLimit = memory as usize;
                }

                SetInformationJobObject(
                    job,
                    JobObjectExtendedLimitInformation,
                    &info as *const _ as *const _,
                    std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
                )?;

                // Store handle value as isize for Send/Sync
                self.job_handle = Some(job.0 as isize);
            }

            Ok(())
        }

        pub async fn execute(&self, command: &[String]) -> Result<SandboxResult> {
            let start_time = Instant::now();

            let mut cmd = Command::new(&command[0]);
            cmd.args(&command[1..]);
            cmd.current_dir(&self.root_dir);

            let output = cmd.output().context("Failed to execute command")?;

            let duration_ms = start_time.elapsed().as_millis() as u64;

            Ok(SandboxResult {
                exit_code: output.status.code().unwrap_or(1),
                stdout: String::from_utf8_lossy(&output.stdout).to_string(),
                stderr: String::from_utf8_lossy(&output.stderr).to_string(),
                duration_ms,
            })
        }

        pub async fn copy_in(&self, host_path: &Path, sandbox_path: &Path) -> Result<()> {
            let dest = self.root_dir.join(sandbox_path);
            if let Some(parent) = dest.parent() {
                tokio::fs::create_dir_all(parent).await?;
            }
            tokio::fs::copy(host_path, dest).await?;
            Ok(())
        }

        pub async fn copy_out(&self, sandbox_path: &Path, host_path: &Path) -> Result<()> {
            let src = self.root_dir.join(sandbox_path);
            if let Some(parent) = host_path.parent() {
                tokio::fs::create_dir_all(parent).await?;
            }
            tokio::fs::copy(src, host_path).await?;
            Ok(())
        }

        pub async fn destroy(&mut self) -> Result<()> {
            // Close job handle
            if let Some(handle_value) = self.job_handle.take() {
                // SAFETY: Closing Windows handle
                unsafe {
                    let handle = windows::Win32::Foundation::HANDLE(handle_value as *mut _);
                    let _ = windows::Win32::Foundation::CloseHandle(handle);
                }
            }

            // Remove directory
            tokio::fs::remove_dir_all(&self.root_dir).await?;
            Ok(())
        }
    }
}

#[cfg(windows)]
pub use windows_impl::WindowsSandbox;
